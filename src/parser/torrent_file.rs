use std::collections::HashMap;
use percent_encoding::{NON_ALPHANUMERIC, percent_encode};
use serde::{Deserialize, Serialize};
use crate::parser::bencode::{BencodeValue, encode_bencode};
use sha1::{Digest, Sha1};
use url::Url;



#[derive(Debug, Clone,Deserialize, Serialize)]
pub struct TorrentInfo {
    pub name: String,
    length : usize,
    #[serde(rename = "piece length")]
    pub piece_length: usize,
    #[serde(with = "serde_bytes")]
    pieces: Vec<u8>
}


impl TorrentInfo {
    //fixme this function is copying data
    pub fn get_divided_pieces(&self) -> Vec<[u8; 20]> {
        let mut divided: Vec<[u8; 20]> = vec![];
        let mut chunk = 20;
        while chunk <= self.pieces.len() {
            let slice = &self.pieces[chunk - 20..chunk];
            let array: [u8; 20] = slice.try_into().expect("slice must be 20 bytes");
            divided.push(array);
            chunk += 20;
        }
        divided
    }
}

#[derive(Debug, Clone,Deserialize, Serialize)]
pub struct TorrentFile {
    announce: Option<String>,
    comment: Option<String>,
    pub info : TorrentInfo
}

impl TorrentFile {

    pub fn build_tracker_url(&self) -> Result<String, Box<dyn std::error::Error>> {
        let mut url = Url::parse(self.announce.as_ref().unwrap())?;
        let info_hash_encoded = percent_encode(&self.compute_info_hash(), NON_ALPHANUMERIC).to_string();
        let query = format!(
            "info_hash={}&peer_id={}",
            info_hash_encoded, "01234567890123456789"
        );
        url.set_query(Some(&query));
        Ok(url.to_string())
    }

    pub fn compute_info_hash(&self) -> [u8; 20] {
        let encoded = serde_bencode::to_bytes(&self.info).unwrap();
        let mut hasher = Sha1::new();
        hasher.update(&encoded);
        let hash = hasher.finalize();
        hash.into()
    }

}
