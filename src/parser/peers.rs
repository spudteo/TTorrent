use crate::parser::bencode::BencodeValue;
use crate::traits::from_bencode::CreateFromBencode;
use std::net::{IpAddr, SocketAddr};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize,Deserialize)]
pub struct Peer {
    pub ip: String,
    pub port: usize,
}

#[derive(Debug,Serialize,Deserialize)]
pub struct AnnounceResponse {
    interval: usize,
    peers: Vec<Peer>,
}
impl AnnounceResponse {

    pub fn get_peers_number(&self) -> usize {
        self.peers.len()
    }
    fn parse_ip(p: &Peer) -> Option<SocketAddr> {
        let full_address = format!("{}:{}", p.ip, p.port);
        match full_address.parse::<SocketAddr>() {
            Err(_e) => None,
            Ok(ip) => Some(ip),
        }
    }
    pub fn get_peers(&self) -> Vec<SocketAddr> {
        self.peers.iter().map(|p|{
        let full_address = format!("{}:{}", p.ip, p.port);
        match full_address.parse::<SocketAddr>() {
            Err(_e) => None,
            Ok(ip) => Some(ip),
        }})
            .filter( |p| p.is_some())
            .flatten().collect()
    }
}

