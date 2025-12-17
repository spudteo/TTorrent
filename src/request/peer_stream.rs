use crate::parser::peers::Peer;
use crate::parser::torrent_file::TorrentFile;
use crate::request::client::{Client, ClientError};
use crate::request::handshake::Handshake;
use crate::request::message::Bitfield;
use crate::request::torrent_message::TorrentMessage;
use std::collections::HashSet;
use std::io::Error;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

const PAYLOAD_LENGTH: u32 = 16384;

pub struct PeerStream {
    stream: TcpStream,
    piece_length: usize,
    bitfield: TorrentMessage,
    chocked: bool,
}

impl PeerStream {
    pub async fn new(
        peer: &Peer,
        torrent_file: &TorrentFile,
        client_peer_id: &[u8; 20],
    ) -> Result<Self, ClientError> {
        //create connection to peer
        let socket = SocketAddr::new(peer.ip_addr, peer.port_number as u16);
        let mut stream = timeout(Duration::from_secs(5), TcpStream::connect(socket)).await??;
        //handshake
        let handshake = Handshake::new(torrent_file.info_hash, client_peer_id);
        if !Self::handshake(&mut stream, handshake).await? {
            return Err(ClientError::Handshake);
        }

        //looping until we saw a bitfield and we are unchoked
        let mut bitfield: Option<TorrentMessage> = None;
        let mut chocked: Option<bool> = None;
        loop {
            if bitfield.is_some() && chocked.is_some() {
                println!("Ready for download from peer: {:?}",peer);
                return Ok(Self {
                    stream: stream,
                    piece_length: torrent_file.piece_length as usize,
                    bitfield: bitfield.unwrap(),
                    chocked: chocked.unwrap(), //fixme in this way we ca never be choked again
                });
            }
            match Self::read_message(&mut stream).await? {
                msg => match msg {
                    TorrentMessage::Bitfield { .. } => {
                        println!("received bitfield");
                        bitfield = Some(msg);
                    }
                    TorrentMessage::Unchoke => {
                        println!("received unchoked");
                        chocked = Some(false);
                    }
                    _ => continue,
                },
            };
        }
    }


    pub async fn download_piece(&mut self, piece_id : usize) -> Result<(Vec<u8>), ClientError> {
        if !self.bitfield.source_has_piece(piece_id){
            return Err(ClientError::BlockNotPresent(piece_id))
        }

        let total_request_to_do = (self.piece_length as f32 / PAYLOAD_LENGTH as f32).ceil() as usize;
        let mut downloaded_blocks: Vec<Option<Vec<u8>>> = vec![None; total_request_to_do];
        let mut missing_block: HashSet<usize> = HashSet::with_capacity(total_request_to_do);
        missing_block.extend(0..total_request_to_do);

        loop {
            //exit the loop if all the blocks have been downloaded for the piece
            if missing_block.is_empty() {
                return Ok(Self::build_piece_from_blocks(self, &mut downloaded_blocks));
            }
            Self::make_request_for_block(&mut self.stream, piece_id, &mut missing_block).await?;

            match Self::read_message(&mut self.stream).await? {
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
                _ => {}
            }
        }
    }

    async fn read_message(stream: &mut TcpStream) -> Result<TorrentMessage, ClientError> {
        let mut init_buf = [0u8; 4];
        let init_bytes_read = stream.read(&mut init_buf).await?;

        //fixme do something if there isn't any bytes in the stream with NoBytesInStream error
        //understand how to safe unwrap
        let message_length = match init_bytes_read == 4 {
            true => u32::from_be_bytes(init_buf[0..4].try_into().unwrap()) as usize,
            false => 0,
        };

        let mut message_buf = vec![0u8; message_length];
        stream.read_exact(&mut message_buf).await?;
        Ok(TorrentMessage::read(&message_buf))
    }

    async fn handshake(stream: &mut TcpStream, handshake: Handshake) -> Result<bool, Error> {
        let data = handshake.to_bytes();
        stream.write_all(&data).await?;
        let mut buf = [0u8; 68];
        let n = stream.read(&mut buf).await?;
        if n > 0 {
            Ok(true)
            //FIXME check that handshake is exactly what we have rquest comapring the hash
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
                }
                .to_bytes();
                println!("request done : {:?}", request);
                let write = stream.write_all(&request).await;
                match write {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        return Err(ClientError::Io(e));
                    }
                }
            }
            None => Err(ClientError::BlockNotPresent(index)),
        }
    }

    fn build_piece_from_blocks(&self, downloaded_blocks: &mut Vec<Option<Vec<u8>>>) -> Vec<u8> {
        let mut final_piece = Vec::with_capacity(self.piece_length);
        for block_opt in downloaded_blocks {
            if let Some(block_data) = block_opt {
                final_piece.extend_from_slice(&block_data);
            }
        }
        final_piece
    }
}
