use std::net::{IpAddr, Ipv4Addr};
use url::Host::Ipv4;
use crate::parser::bencode::BencodeValue;
use crate::traits::from_bencode::CreateFromBencode;

#[derive(Debug)]
pub struct Peer {
    ip_addr: IpAddr,
    port_number: i64,
}

#[derive(Debug)]
pub struct AnnounceResponse {
    interval : i32,
    peers: Vec<Peer>,
}

impl Peer {
    pub fn new(ip_addr: IpAddr, port_number: i64) -> Self {
        Self { ip_addr,port_number }
    }
}


impl AnnounceResponse {
    pub fn new(interval : i32, peers: Vec<Peer>) -> Self {
        Self { interval,peers}
    }
    fn find_interval_in_bencode(input_bencode: &BencodeValue) -> Result<i32, String> {
        let interval = "interval".as_bytes();
        match input_bencode {
            BencodeValue::Dictionary(dict) => {
                match dict.get(&interval.to_vec())
                {
                    Some(value) => {
                        match value {
                            BencodeValue::Integer(v ) => {
                                    let s = String::from_utf8(v.clone())
                                        .map_err(|_| "Invalid UTF-8 bytes".to_string())?;

                                    s.parse::<i32>()
                                        .map_err(|_| "Invalid integer format".to_string())

                            }
                            _ => Err("Not an integer in the interval key".to_string())
                        }
                    }
                    _ => Err("interval key not find in bencode provided".to_string())
                }

            }
            _ => Err("Not a dictionary provided".to_string())
        }
    }

    fn parse_ip(ip : &String) -> Result<IpAddr, String> {
        let parsed = ip.parse::<IpAddr>();
        match parsed {
            Err(e) => Err("problem wihile parsing ip {ip}".to_string()),
            Ok(ip) => Ok(ip),
        }
    }

    fn find_peer_in_bencode(input_bencode: &BencodeValue) -> Peer{
        let ip = "ip".as_bytes();
        let port = "port".as_bytes();
        match input_bencode {
            BencodeValue::Dictionary(dict) => {
                let found_ip = dict.get(&ip.to_vec()).unwrap().as_string_or_panic();
                let found_port = dict.get(&port.to_vec()).unwrap().as_int_or_panic();
                Peer::new(AnnounceResponse::parse_ip(&found_ip).unwrap(), found_port)

            }
            _ => Peer::new(AnnounceResponse::parse_ip(&"0.0.0.0".to_string()).unwrap(), 0)
        }
    }

    fn find_peers_in_bencode(input_bencode: &BencodeValue) -> Result<Vec<Peer>, String> {
        let peers = "peers".as_bytes();
        match input_bencode {
            BencodeValue::Dictionary(dict) => {
                match dict.get(&peers.to_vec())
                {
                    Some(value) => {
                        match value  {
                            BencodeValue::List(peer_list) => {
                                let mut peers: Vec<Peer> = Vec::new();
                                for peer in peer_list {
                                    peers.push(AnnounceResponse::find_peer_in_bencode(peer));
                                }
                                Ok(peers)

                            }
                            _ => Err("peers are not a list".to_string())
                        }

                }
                    _ => Err("peers key not find in bencode provided".to_string())
                }

            }
            _ => Err("Not a dictionary provided".to_string())
        }
    }
}

impl CreateFromBencode for AnnounceResponse {
    fn parse(input_bencode: &BencodeValue) -> Self{
        let interval = Self::find_interval_in_bencode(input_bencode).unwrap();
        let peers = Self::find_peers_in_bencode(input_bencode).unwrap();
        Self::new(interval,peers)
    }
}