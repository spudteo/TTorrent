use std::io::Read;
use crate::parser::bencode;
use crate::parser::bencode::BencodeValue;

#[derive(Debug)]
struct TorrentFile{
    announce : String
}

impl TorrentFile {
    pub fn new(announce: String) -> Self {
        Self { announce }
    }

    pub fn new_from_bencode(bencode: BencodeValue) -> Result<Self, String> {
        let announce = Self::find_key_in_bencode(bencode, "announce".to_string());
        match announce {
            Err(e) => Err(format!("Unable to find key announce: {}", e)),
            Ok(url) => {
                Ok(TorrentFile::new(url))
            }
        }
    }

    fn find_key_in_bencode(input_bencode: BencodeValue, key: String) -> Result<String, String> {
        match input_bencode {
            BencodeValue::Dictionary(input) => {
                match input.get(&key.as_bytes().to_vec()) {
                    None => Err("Key not found".to_string()),
                    Some(value) => match value {
                        BencodeValue::String(bytes) => {
                            String::from_utf8(bytes.clone()).map_err(|_| "Invalid UTF-8 bytes".to_string())
                        }
                        _ => Err("Not a string in the key".to_string()),
                    },
                }
            }
            _ => Err("Not a dictionary provided".to_string()),
        }
    }


}


#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use super::*;

    #[test]
    fn find_key_in_bencode() {
        let mut input_map = HashMap::new();
        input_map.insert("announce".as_bytes().to_vec(),BencodeValue::String("url".as_bytes().to_vec()));
        let input_bencode = BencodeValue::Dictionary(input_map);

        let result = TorrentFile::find_key_in_bencode(input_bencode, "announce".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "url");
    }

    #[test]
    fn create_torrent_from_bencode() {
        let mut input_map = HashMap::new();
        input_map.insert("announce".as_bytes().to_vec(),BencodeValue::String("url".as_bytes().to_vec()));
        let input_bencode = BencodeValue::Dictionary(input_map);
        let result = TorrentFile::new_from_bencode(input_bencode);
        println!("{:?}", result)
            }
}