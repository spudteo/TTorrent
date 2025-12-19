use crate::parser::peers::Peer;
use crate::parser::torrent_file::TorrentFile;
use crate::request::handshake::Handshake;
use sha1::{Digest, Sha1};
use std::collections::{HashMap, HashSet};
use std::io::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

use crate::request::message::Message;
use crate::request::torrent_message::TorrentMessage;
use thiserror::Error;
use tokio::time::error::Elapsed;
use crate::request::peer_stream::PeerStream;
use async_channel::{unbounded, Receiver, Sender};
use tokio::time::sleep;

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
    peer: Vec<Peer>,
    client_peer_id: [u8; 20],
}

impl Client {
    pub fn new(torrent_file: TorrentFile, peer: Vec<Peer>) -> Client {
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

    pub async fn download_torrent(&self, number_of_peers: usize) -> Result<Vec<u8>, ClientError> {
        let (transmitter_work, receiver_work) = unbounded::<usize>();
        let (transmitter_piece, receiver_piece) = unbounded::<(usize,Vec<u8>)>();
        //fixme investigate arc
        let torrent_file = Arc::new(self.torrent_file.clone());
        let client_id = Arc::new(self.client_peer_id.clone());
        let peer_info = Arc::new(self.peer.clone());

        //create peer to download
        for slave_id in 1..= number_of_peers {
            let rx = receiver_work.clone();
            let tx = transmitter_piece.clone();
            let t_file = Arc::clone(&torrent_file);
            let c_id = Arc::clone(&client_id);
            let p_info = Arc::clone(&peer_info);

            tokio::spawn(async move {
                println!("Creating slave downloader {} ", slave_id);
                let mut peer_stream = PeerStream::new(slave_id,&p_info[slave_id], &t_file, &c_id).await;
                match peer_stream {
                    Ok(mut stream) => {
                        while let Ok(piece_id) = rx.recv().await {
                            let downloaded_piece = stream.download_piece(piece_id).await;
                            match downloaded_piece {
                                Ok(piece) => {
                                    let _ = tx.send(piece).await;
                                },
                                _ => {
                                    println!("errore downloading piece");
                                    //fixme try again for a specific number of times, or just reinsert the value in the to download queue
                                }
                            }
                        }
                    }
                    _ => {
                        //fixme try recreate the stream
                        println!("Error creating stream");
                    }
                }
            });
        }

        let number_of_pieces = self.torrent_file.pieces.len();
        let mut all_pieces = HashSet::with_capacity(number_of_pieces);
        all_pieces.extend(0..number_of_pieces);

        //fix me I already know the dimension of everything here following the torrent
        let mut downloaded_file: Vec<Option<Vec<u8>>> = vec![None; number_of_pieces];
        //send work to slave
        for piece in 0..self.torrent_file.pieces.len() {
            transmitter_work.send(piece).await.unwrap();
        }
        drop(transmitter_work); //fixme understand why drop this, i hsould not doning that

        let mut completed_pieces = 0;


        while let Ok((piece_id, data)) = receiver_piece.recv().await {
            if Self::piece_hash_is_correct(&data, self.torrent_file.pieces[piece_id]) {
                println!("Master: ricevuto pezzo {}", piece_id);
                downloaded_file[piece_id] = Some(data);
                completed_pieces += 1;
            } else {
                //fixme se ha fallito lo rimettiamo dentro a scaricare da qualcun'altro magari
                println!("Master: Hash fallito per pezzo {}, cosa facciamo?", piece_id);
            }

            if completed_pieces == 1000 {
                break;
            }
        }
        Ok(downloaded_file.into_iter().flatten().flatten().collect())
    }


}
