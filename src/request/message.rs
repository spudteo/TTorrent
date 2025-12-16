use sha1::digest::typenum::Bit;
use thiserror::Error;

#[derive(Debug, PartialEq)]
enum MessageID {
    Choke = 0,
    Unchoke = 1,
    Interested = 2,
    NotInterested = 3,
    Have = 4,
    Bitfield = 5,
    Request = 6,
    Piece = 7,
    Cancel = 8,
    Port = 9,
}

impl TryFrom<u8> for MessageID {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(MessageID::Choke),
            1 => Ok(MessageID::Unchoke),
            2 => Ok(MessageID::Interested),
            3 => Ok(MessageID::NotInterested),
            4 => Ok(MessageID::Have),
            5 => Ok(MessageID::Bitfield),
            6 => Ok(MessageID::Request),
            7 => Ok(MessageID::Piece),
            8 => Ok(MessageID::Cancel),
            9 => Ok(MessageID::Port),
            _ => Err(()),
        }
    }
}

pub struct Message {
    length: u32,
    message_id: Option<MessageID>, //keep alive has no message id
    payload: Option<Vec<u8>>,
}
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("missing payload")]
    MissingPayload,
    #[error("invalid bitfield length: {0}")]
    InvalidLength(usize),
    #[error("bitfield payload is empty")]
    EmptyBitfield,
}

pub struct Bitfield {
    piece_index: Vec<u8>,
}

impl Bitfield {
    fn new(piece_index: Vec<u8>) -> Self {
        Self { piece_index }
    }

    fn contains(&self, index: i32) -> bool {
        //the byte num represent the index byte where we can find if the index n is set.
        //then for every byte the byte_index is exactly the position where it is set
        //byte_index will always have only one 1 set and the other position set to 0
        //so by making the bitwise and if the result have a 1 it was in the same position
        //therefore if it is not a zero it means that it has matched the position and that the index
        //is contained. We used the mask since it is big endian
        let byte_num = (index / 8) as usize;
        if byte_num > self.piece_index.len() - 1 {
            panic!(
                "Bitfield index out of bounds: {} > {}",
                index,
                self.piece_index.len() * 8 - 1
            );
        }
        let byte_index = (index % 8) as u8;
        let byte = self.piece_index[byte_num];
        let mask = 1 << (7 - byte_index);
        match mask & byte {
            0 => false,
            _ => true,
        }
    }
}

impl Message {
    fn new(length: u32, message_id: Option<MessageID>, payload: Option<Vec<u8>>) -> Self {
        Self {
            length,
            message_id,
            payload,
        }
    }

    fn parse_bitfield(payload: Vec<u8>) -> Bitfield {
        Bitfield {
            piece_index: payload,
        }
    }
    pub fn get_bitfield(self) -> Result<Bitfield, ParseError> {
        match self.payload {
            Some(p) => Ok(Self::parse_bitfield(p)),
            _ => Err(ParseError::MissingPayload),
        }
    }

    pub fn read(input_stream: &[u8]) -> Message {
        if input_stream.len() < 4 {
            panic!("the message is too short, it doesn't have at least 4 bytes");
        }
        let message_length = u32::from_be_bytes(input_stream[0..4].try_into().unwrap());

        match message_length {
            0 => Message::new(message_length, None, None), //keep alive
            1 => {
                let message_id = MessageID::try_from(input_stream[4]).unwrap();
                Message::new(message_length, Some(message_id), None)
            }
            _ => {
                let message_id = MessageID::try_from(input_stream[4]).unwrap();
                let total_len = 4 + message_length as usize;
                if total_len > input_stream.len() {
                    panic!("payload is too long");
                }
                Message::new(
                    message_length,
                    Some(message_id),
                    Some(input_stream[5..total_len].to_vec()),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_keep_alive() {
        let input = [0, 0, 0, 0];

        let message = Message::read(&input);

        assert_eq!(message.length, 0);
        assert!(message.message_id.is_none());
        assert!(message.payload.is_none());
    }

    #[test]
    fn read_unchoke() {
        let input = [0, 0, 0, 1, 1];

        let message = Message::read(&input);

        assert_eq!(message.length, 1);
        assert_eq!(message.message_id.unwrap(), MessageID::Unchoke);
        assert!(message.payload.is_none());
    }

    #[test]
    fn bitfield_contains() {
        let bf = Bitfield::new(vec![0b1100_0000, 0b1000_0001]);

        assert!(bf.contains(0));
        assert!(bf.contains(1));
        assert!(!bf.contains(2));
        assert!(bf.contains(8));
        assert!(!bf.contains(10));
        assert!(bf.contains(15));
    }
    #[test]
    #[should_panic]
    fn bitfield_contains_out_of_bounds() {
        let bf = Bitfield::new(vec![0b1100_0000, 0b1000_0001]);
        assert!(bf.contains(16));
    }
}
