use crate::request::message::Message;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TorrentMessage {
    KeepAlive,
    Choke,
    Unchoke,
    Bitfield {
        bitfield: Vec<u8>,
    },
    Piece {
        index: u32,
        begin: u32,
        block: Vec<u8>,
    },
    Request {
        index: u32,
        begin: u32,
        length: u32,
    },
}

impl TorrentMessage {
    pub fn read(input_stream: &[u8]) -> TorrentMessage {
        if input_stream.len() == 0 {
            return TorrentMessage::KeepAlive;
        }

        match input_stream[0] {
            0 => TorrentMessage::Choke,
            1 => TorrentMessage::Unchoke,
            5 => TorrentMessage::Bitfield {
                bitfield: input_stream[1..].to_vec(),
            },
            7 => TorrentMessage::Piece {
                index: u32::from_be_bytes(input_stream[1..5].try_into().unwrap()),
                begin: u32::from_be_bytes(input_stream[5..9].try_into().unwrap()),
                block: input_stream[9..].to_vec(),
            },
            _ => panic!("invalid torrent message received"), //fix me
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            TorrentMessage::Request {
                index,
                begin,
                length,
            } => {
                let mut out = Vec::with_capacity(17);

                // length prefix  13
                out.extend_from_slice(&13u32.to_be_bytes());

                // message id
                out.push(MessageID::Request as u8);

                // payload
                out.extend_from_slice(&index.to_be_bytes());
                out.extend_from_slice(&begin.to_be_bytes());
                out.extend_from_slice(&length.to_be_bytes());

                out
            }
            _ => Vec::new(),
        }
    }

    pub fn source_has_piece(&self, index: u32) -> bool {
        match self {
            TorrentMessage::Bitfield { bitfield } => {
                //the byte num represent the index byte where we can find if the index n is set.
                //then for every byte the byte_index is exactly the position where it is set
                //byte_index will always have only one 1 set and the other position set to 0
                //so by making the bitwise and if the result have a 1 it was in the same position
                //therefore if it is not a zero it means that it has matched the position and that the index
                //is contained. We used the mask since it is big endian
                let byte_num = (index / 8) as usize;
                if byte_num > bitfield.len() - 1 {
                    panic!(
                        "Bitfield index out of bounds: {} > {}",
                        index,
                        bitfield.len() * 8 - 1
                    );
                }
                let byte_index = (index % 8) as u8;
                let byte = bitfield[byte_num];
                let mask = 1 << (7 - byte_index);
                match mask & byte {
                    0 => false,
                    _ => true,
                }
            }
            _ => false,
        }
    }
}
