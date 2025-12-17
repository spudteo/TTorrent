use std::collections::{HashMap, HashSet};
use crate::parser::peers::Peer;
use crate::parser::torrent_file::TorrentFile;
use crate::request::handshake::Handshake;
use sha1::{Digest, Sha1};
use std::io::Error;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

use crate::request::message::Message;
use crate::request::torrent_message::TorrentMessage;
use thiserror::Error;
use tokio::time::error::Elapsed;


const PAYLOAD_LENGTH: u32 = 16384;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("the piece downloaded has a different hash than expected")]
    CorruptedPiece,
    #[error("problem with handshake")]
    Handshake,
    #[error("connection timeout")]
    Timeout,
    #[error("input non valido: {0}")]
    InvalidInput(String),
    #[error("Peer doesen't have the block id {0}")]
    BlockNotPresent(usize),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl From<Elapsed> for ClientError {
    fn from(_: Elapsed) -> Self {
        ClientError::Timeout
    }
}

pub struct Client {
    torrent_file: TorrentFile,
    peer: Peer,
    client_peer_id: [u8; 20],
}

impl Client {
    pub fn new(torrent_file: TorrentFile, peer: Peer) -> Client {
        let client_per_id = *b"01234567890123456789";
        Self {
            torrent_file,
            peer,
            client_peer_id: client_per_id,
        }
    }

    pub async fn handshake_done(
        stream: &mut TcpStream,
        handshake: Handshake,
    ) -> Result<bool, Error> {
        let data = handshake.to_bytes();
        stream.write_all(&data).await?;
        let mut buf = [0u8; 68];
        let n = stream.read(&mut buf).await?;
        if n > 0 {
            Ok(true)
            //check that handshake is ok in buf
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "peer closed connection without sending handshake, buff looks empty",
            ))
        }
    }

    async fn make_request_for_block(
        stream: &mut TcpStream,
        index: usize,
        missing_block: &mut HashSet<usize>,
    ) -> Result<(), ClientError> {
        //pick one block, download it, begin is exactly the block length per the index block
        let next_block = missing_block.iter().next();
        match next_block {
            Some(block) => {
                let request = TorrentMessage::Request {
                    index: index as u32,
                    begin: (block * PAYLOAD_LENGTH as usize) as u32,
                    length: PAYLOAD_LENGTH,
                }.to_bytes();
                println!("request done : {:?}", request);
                let write = stream.write_all(&request).await;
                match write {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        return Err(ClientError::Io(e));
                    }
                }
            }
            None => Err(ClientError::BlockNotPresent(index))
        }
    }

    fn build_piece_from_blocks(&self,downloaded_blocks: &mut Vec<Option<Vec<u8>>>) -> Vec<u8> {
        let mut final_piece = Vec::with_capacity(self.torrent_file.piece_length as usize);
        for block_opt in downloaded_blocks {
            if let Some(block_data) = block_opt {
                final_piece.extend_from_slice(&block_data);
            }
        }
        final_piece
    }

    async fn message_handler(
        &self,
        stream: &mut TcpStream,
        piece_id: u32
    ) -> Result<Vec<u8>, ClientError> {
        let mut chocked = true;
        let mut bitfield_message: Option<TorrentMessage> = None;
        let total_request_to_do = (self.torrent_file.piece_length as f32 / PAYLOAD_LENGTH as f32).ceil() as usize;
        let mut downloaded_blocks: Vec<Option<Vec<u8>>> = vec![None; total_request_to_do];
        let mut missing_block: HashSet<usize> = HashSet::with_capacity(total_request_to_do);
        missing_block.extend(0..total_request_to_do);

        loop {
            //exit the loop if all the blocks have been downloaded for the piece
            if missing_block.is_empty() {
                return Ok(Self::build_piece_from_blocks(self, &mut downloaded_blocks))
            }

            //request a block
            if let Some(ref msg) = bitfield_message {
                if msg.source_has_piece(piece_id) && !chocked {
                    Self::make_request_for_block(stream, piece_id as usize, &mut missing_block).await?;
                }
            }

            //reads the messages
            let mut init_buf = [0u8; 4];
            let n = stream.read(&mut init_buf).await?;
            let message_length = match n == 4 {
                true => u32::from_be_bytes(init_buf[0..4].try_into().unwrap()) as usize,
                false => 0,
            };
            let mut message_buf = vec![0u8; message_length];
            stream.read_exact(&mut message_buf).await?;
            let message = TorrentMessage::read(&message_buf);
            match message {
                TorrentMessage::KeepAlive => continue,
                TorrentMessage::Bitfield { bitfield: received } => {
                    println!("bitfield received");
                    match bitfield_message {
                        Some(_) => continue,
                        None => bitfield_message = Some(TorrentMessage::Bitfield { bitfield: received }),
                    }
                },
                TorrentMessage::Piece {
                    index,
                    begin,
                    block,
                } => {
                    println!("received piece.. {:?}, {:?}", index, begin);
                    let block_index = ((begin as usize) / (PAYLOAD_LENGTH as usize));
                    missing_block.remove(&block_index);
                    downloaded_blocks[block_index] = Some(block);
                }
                TorrentMessage::Unchoke => {
                    println!("unchoked by the server");
                    chocked = false
                }
                TorrentMessage::Choke => {
                    println!("Choked by the server");
                    chocked = true
                }
                _ => {}
            }
        }
    }

    fn piece_hash_is_correct(piece: &Vec<u8>, checksum: [u8; 20]) -> bool {
        let mut hasher = Sha1::new();
        hasher.update(&piece);
        let hash = hasher.finalize();
        let hash_value: [u8; 20] = hash.try_into().unwrap();
        hash_value == checksum
    }

    pub async fn download_from_peer(&self, piece_id: u32) -> Result<Vec<u8>, ClientError> {
        //create tcp connection
        let socket = SocketAddr::new(self.peer.ip_addr, self.peer.port_number as u16);
        let mut stream = timeout(Duration::from_secs(5), TcpStream::connect(socket)).await??;
        //handshake
        let handshake = Handshake::new(self.torrent_file.info_hash, self.client_peer_id);
        if !Client::handshake_done(&mut stream, handshake).await? {
            return Err(ClientError::Handshake);
        }


        loop {
            let piece =
                Self::message_handler(self, &mut stream, piece_id).await?;

            match Self::piece_hash_is_correct(&piece, self.torrent_file.pieces[piece_id as usize]) {
                true => return Ok(piece),
                false => continue,
            }
        }

    }
}
