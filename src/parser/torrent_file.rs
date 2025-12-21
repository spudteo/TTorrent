use percent_encoding::{NON_ALPHANUMERIC, percent_encode};

use crate::parser::bencode::{BencodeValue, encode_bencode};
use sha1::{Digest, Sha1};
use url::Url;

#[derive(Debug, Clone)]
pub struct TorrentFile {
    pub announce: String,
    pub name: String,
    length: i64,
    pub piece_length: i64,
    pub pieces: Vec<[u8; 20]>,
    pub info_hash: [u8; 20],
}

impl TorrentFile {
    pub fn new(
        announce: String,
        name: String,
        length: i64,
        piece_length: i64,
        pieces: Vec<[u8; 20]>,
        info_hash: [u8; 20],
    ) -> Self {
        Self {
            announce,
            name,
            length,
            piece_length,
            pieces,
            info_hash,
        }
    }
    pub fn build_tracker_url(&self) -> Result<String, Box<dyn std::error::Error>> {
        let mut url = Url::parse(&self.announce)?;

        let info_hash_encoded = percent_encode(&self.info_hash, NON_ALPHANUMERIC).to_string();

        let query = format!(
            "info_hash={}&peer_id={}",
            info_hash_encoded, "01234567890123456789"
        );
        url.set_query(Some(&query));
        Ok(url.to_string())
    }

    pub fn new_from_bencode(bencode: &BencodeValue) -> Result<Self, String> {
        let announce = Self::find_key_string_in_bencode(bencode, "announce".to_string())?;
        let bencode_info = Self::find_key_dict_in_bencode(bencode, "info".to_string())?;
        let info_hash = Self::compute_info_hash(&bencode_info);
        let info = "info".as_bytes();
        let info_bencode = match bencode {
            BencodeValue::Dictionary(d) => d.get(&info.to_vec()),
            _ => Option::None,
        };

        match info_bencode {
            Some(info_bencode) => {
                let name = Self::find_key_string_in_bencode(info_bencode, "name".to_string())?;
                let length = Self::find_key_integer_in_bencode(info_bencode, "length".to_string())?;
                let piece_length =
                    Self::find_key_integer_in_bencode(info_bencode, "piece length".to_string())?;
                let all_pieces =
                    Self::find_key_bytes_in_bencode(info_bencode, "pieces".to_string())?;
                let pieces = Self::divide_pieces(&all_pieces);
                Ok(TorrentFile::new(
                    announce,
                    name,
                    length,
                    piece_length,
                    pieces,
                    info_hash,
                ))
            }
            None => Err("info key not found".to_string()),
        }
    }

    fn divide_pieces(pieces: &Vec<u8>) -> Vec<[u8; 20]> {
        let mut divided: Vec<[u8; 20]> = vec![];
        let mut chunk = 20;
        while chunk <= pieces.len() {
            let slice = &pieces[chunk - 20..chunk];
            let array: [u8; 20] = slice.try_into().expect("slice must be 20 bytes");
            divided.push(array);
            chunk += 20;
        }
        divided
    }

    fn compute_info_hash(info: &BencodeValue) -> [u8; 20] {
        let encoded = encode_bencode(info);
        let mut hasher = Sha1::new();
        hasher.update(&encoded);
        let hash = hasher.finalize();
        hash.into()
    }

    fn find_key_bytes_in_bencode(
        input_bencode: &BencodeValue,
        key: String,
    ) -> Result<Vec<u8>, String> {
        match input_bencode {
            BencodeValue::Dictionary(input) => match input.get(&key.as_bytes().to_vec()) {
                None => Err("Key not found".to_string()),
                Some(value) => match value {
                    BencodeValue::String(bytes) => Ok(bytes.to_vec()),
                    _ => Err("Not a integer in the key".to_string()),
                },
            },
            _ => Err("Not a dictionary provided".to_string()),
        }
    }

    fn find_key_dict_in_bencode(
        input_bencode: &BencodeValue,
        key: String,
    ) -> Result<&BencodeValue, String> {
        match input_bencode {
            BencodeValue::Dictionary(input) => match input.get(&key.as_bytes().to_vec()) {
                None => Err("Key not found".to_string()),
                Some(value) => match value {
                    BencodeValue::Dictionary(bytes) => Ok(value),
                    _ => Err("Not a integer in the key".to_string()),
                },
            },
            _ => Err("Not a dictionary provided".to_string()),
        }
    }

    fn find_key_integer_in_bencode(
        input_bencode: &BencodeValue,
        key: String,
    ) -> Result<i64, String> {
        match input_bencode {
            BencodeValue::Dictionary(input) => match input.get(&key.as_bytes().to_vec()) {
                None => Err("Key not found".to_string()),
                Some(value) => match value {
                    BencodeValue::Integer(bytes) => {
                        let s = String::from_utf8(bytes.clone())
                            .map_err(|_| "Invalid UTF-8 bytes".to_string())?;

                        s.parse::<i64>()
                            .map_err(|_| "Invalid integer format".to_string())
                    }
                    _ => Err("Not a integer in the key".to_string()),
                },
            },
            _ => Err("Not a dictionary provided".to_string()),
        }
    }

    fn find_key_string_in_bencode(
        input_bencode: &BencodeValue,
        key: String,
    ) -> Result<String, String> {
        match input_bencode {
            BencodeValue::Dictionary(input) => match input.get(&key.as_bytes().to_vec()) {
                None => Err("Key not found".to_string()),
                Some(value) => match value {
                    BencodeValue::String(bytes) => String::from_utf8(bytes.clone())
                        .map_err(|_| "Invalid UTF-8 bytes".to_string()),
                    _ => Err("Not a string in the key".to_string()),
                },
            },
            _ => Err("Not a dictionary provided".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn find_string_in_bencode() {
        let mut input_map = HashMap::new();
        input_map.insert(
            "announce".as_bytes().to_vec(),
            BencodeValue::String("url".as_bytes().to_vec()),
        );
        let input_bencode = BencodeValue::Dictionary(input_map);

        let result =
            TorrentFile::find_key_string_in_bencode(&input_bencode, "announce".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "url");
    }

    #[test]
    fn find_integer_in_bencode() {
        let mut input_map = HashMap::new();
        input_map.insert(
            "length".as_bytes().to_vec(),
            BencodeValue::Integer("54".as_bytes().to_vec()),
        );
        let input_bencode = BencodeValue::Dictionary(input_map);
        let result = TorrentFile::find_key_integer_in_bencode(&input_bencode, "length".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 54);
    }

    #[test]
    fn create_torrent_from_bencode() {
        let mut input_map = HashMap::new();
        input_map.insert(
            "announce".as_bytes().to_vec(),
            BencodeValue::String("url".as_bytes().to_vec()),
        );
        let input_bencode = BencodeValue::Dictionary(input_map);
        let result = TorrentFile::new_from_bencode(&input_bencode);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().announce, "url");
    }
}
