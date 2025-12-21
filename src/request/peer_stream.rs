use crate::parser::peers::Peer;
use crate::parser::torrent_file::TorrentFile;
use crate::request::client::ClientError;
use crate::request::client::ClientError::{HandshakeFailed, ServerDoesntHaveFile};
use crate::request::handshake::Handshake;
use crate::request::torrent_message::TorrentMessage;
use log::debug;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

const PAYLOAD_LENGTH: u32 = 16384;
const MAX_REQUEST_FOR_PIECE: usize = 20;

pub struct PeerStream {
    id: usize,
    stream: TcpStream,
    piece_length: usize,
    bitfield: TorrentMessage,
}

impl PeerStream {
    pub async fn new(
        id: usize,
        peer: &Peer,
        torrent_file: &TorrentFile,
        client_peer_id: &[u8; 20],
    ) -> Result<Self, ClientError> {
        //create connection to peer
        let socket = SocketAddr::new(peer.ip_addr, peer.port_number as u16);
        let mut stream = timeout(Duration::from_secs(5), TcpStream::connect(socket)).await??;
        //handshake
        let handshake = Handshake::new(torrent_file.info_hash, client_peer_id);
        Self::make_handshake(&mut stream, &handshake).await?;

        //looping until we saw a bitfield and we are unchoked
        let mut bitfield: Option<TorrentMessage> = None;
        let mut chocked: Option<bool> = None;
        loop {
            if bitfield.is_some() && chocked.is_some() {
                debug!("Ready for download from peer: {:?}", peer);
                return Ok(Self {
                    id: id,
                    stream: stream,
                    piece_length: torrent_file.piece_length as usize,
                    bitfield: bitfield.unwrap(),
                });
            }
            match Self::read_message(&mut stream).await? {
                msg => match msg {
                    TorrentMessage::Bitfield { .. } => {
                        bitfield = Some(msg);
                    }
                    TorrentMessage::Unchoke => {
                        chocked = Some(false);
                    }
                    _ => continue,
                },
            };
        }
    }

    pub async fn download_piece(
        &mut self,
        piece_id: usize,
    ) -> Result<(usize, Vec<u8>), ClientError> {
        if !self.bitfield.source_has_piece(piece_id) {
            return Err(ClientError::PieceNotPresent(piece_id));
        }

        let total_request_to_do =
            (self.piece_length as f32 / PAYLOAD_LENGTH as f32).ceil() as usize;
        let mut downloaded_blocks: Vec<Option<Vec<u8>>> = vec![None; total_request_to_do];
        let mut missing_block: HashSet<usize> = HashSet::with_capacity(total_request_to_do);
        let mut chocked: bool = false;
        missing_block.extend(0..total_request_to_do);
        loop {
            if missing_block.is_empty() {
                return Ok((
                    piece_id,
                    Self::build_piece_from_blocks(self, &mut downloaded_blocks),
                ));
            }
            if !chocked {
                Self::make_request_for_block(&mut self.stream, piece_id, &mut missing_block)
                    .await?;
            }

            let _ = tokio::time::timeout(Duration::from_millis(1500), async {
                loop {
                    if missing_block.is_empty() {
                        break;
                    }
                    match Self::read_message(&mut self.stream).await {
                        Ok(msg) => match msg {
                            TorrentMessage::Piece {
                                index,
                                begin,
                                block,
                            } => {
                                debug!(
                                    "{} - received piece.. index:{:?}, beign: {:?}",
                                    self.id, index, begin
                                );
                                let block_index = (begin as usize) / (PAYLOAD_LENGTH as usize);
                                let was_present = missing_block.remove(&block_index);
                                if was_present {
                                    downloaded_blocks[block_index] = Some(block);
                                }
                            }
                            TorrentMessage::KeepAlive => (),
                            TorrentMessage::Bitfield { .. } => (),
                            TorrentMessage::Choke => chocked = true,
                            TorrentMessage::Unchoke => chocked = false,
                            _ => {}
                        },
                        Err(_e) => (), //fixme dovrebbe ritornare un errore
                    }
                }
            })
            .await;
        }
    }

    async fn read_message(stream: &mut TcpStream) -> Result<TorrentMessage, ClientError> {
        let mut init_buf = [0u8; 4];
        stream
            .read_exact(&mut init_buf)
            .await
            .map_err(|_| ClientError::NoBytesInStream)?;
        let message_length = u32::from_be_bytes(init_buf) as usize;
        let mut message_buf = vec![0u8; message_length];
        stream
            .read_exact(&mut message_buf)
            .await
            .map_err(|_| ClientError::NoBytesInStream)?;
        Ok(TorrentMessage::read(&message_buf))
    }

    async fn make_handshake(
        stream: &mut TcpStream,
        handshake: &Handshake,
    ) -> Result<(), ClientError> {
        let data = handshake.to_bytes();
        stream.write_all(&data).await?;
        let mut buf = [0u8; 68];
        let n = stream.read(&mut buf).await?;
        if n > 0 {
            let received_handshake = Handshake::parse(buf);
            if received_handshake.info_hash != handshake.info_hash {
                return Err(ServerDoesntHaveFile);
            }
            Ok(())
        } else {
            Err(HandshakeFailed)
        }
    }
    async fn make_request_for_block(
        stream: &mut TcpStream,
        index: usize,
        missing_block: &mut HashSet<usize>,
    ) -> Result<(), ClientError> {
        for block in missing_block.iter().take(MAX_REQUEST_FOR_PIECE) {
            let request = TorrentMessage::Request {
                index: index as u32,
                begin: (*block * PAYLOAD_LENGTH as usize) as u32,
                length: PAYLOAD_LENGTH,
            }
            .to_bytes();
            let write = stream.write_all(&request).await;
            match write {
                Ok(_) => (),
                Err(e) => {
                    return Err(ClientError::Io(e));
                }
            }
        }
        Ok(())
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
