use crate::parser::peers::Peer;
use crate::parser::torrent_file::TorrentFile;
use crate::request::handshake::Handshake;
use sha1::{Digest, Sha1};
use std::collections::{HashMap, HashSet};
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
use crate::request::peer_stream::PeerStream;

const PAYLOAD_LENGTH: u32 = 16384;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Couldn't read any data from the peer")]
    NoBytesInStream,
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

    fn piece_hash_is_correct(piece: &Vec<u8>, checksum: [u8; 20]) -> bool {
        let mut hasher = Sha1::new();
        hasher.update(&piece);
        let hash = hasher.finalize();
        let hash_value: [u8; 20] = hash.try_into().unwrap();
        hash_value == checksum
    }

    pub async fn download_from_peer(&self, piece_id: u32) -> Result<Vec<u8>, ClientError> {
        let mut peer_stream = PeerStream::new(&self.peer, &self.torrent_file, &self.client_peer_id).await?;

        let piece = peer_stream.download_piece(piece_id as usize).await?;

        match Self::piece_hash_is_correct(&piece, self.torrent_file.pieces[piece_id as usize]) {
            true => Ok(piece),
            false => Err(ClientError::CorruptedPiece),
        }
    }
}
