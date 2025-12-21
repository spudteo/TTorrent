use crate::parser::peers::Peer;
use crate::parser::torrent_file::TorrentFile;
use sha1::{Digest, Sha1};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::request::peer_stream::PeerStream;
use crate::request::storage::TorrentPersisted;
use async_channel::{RecvError, unbounded};
use log::{debug, error, info};
use thiserror::Error;
use tokio::time::error::Elapsed;

const PAYLOAD_LENGTH: u32 = 16384;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Couldn't read any data from the peer")]
    NoBytesInStream,
    #[error("the piece downloaded has a different hash than expected")]
    CorruptedPiece,
    #[error("problem with handshake")]
    HandshakeFailed,
    #[error("connection timeout")]
    Timeout,
    #[error("input non valido: {0}")]
    InvalidInput(String),
    #[error("Peer doesen't have the block id {0}")]
    BlockNotPresent(usize),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Peer doesn't have the piece id  {0}")]
    PieceNotPresent(usize),
    #[error("Handshake of the server was not the one we expected")]
    ServerDoesntHaveFile,
    #[error("Error in the channel receiver or transmitter")]
    ChannelReceiverError,
}

impl From<Elapsed> for ClientError {
    fn from(_: Elapsed) -> Self {
        ClientError::Timeout
    }
}
impl From<async_channel::RecvError> for ClientError {
    fn from(value: RecvError) -> Self {
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

    pub async fn download_torrent(
        &self,
        number_of_peers_downloader: usize,
    ) -> Result<(), ClientError> {
        let (transmitter_work, receiver_work) = unbounded::<usize>();
        let (transmitter_piece, receiver_piece) = unbounded::<(usize, Vec<u8>)>();
        //fixme investigate arc
        let torrent_file = Arc::new(self.torrent_file.clone());
        let client_id = Arc::new(self.client_peer_id.clone());
        let peer_info = Arc::new(self.peer.clone());

        //create peer to download
        for slave_id in 1..=number_of_peers_downloader {
            let r_work = receiver_work.clone();
            let t_piece = transmitter_piece.clone();
            let t_work = transmitter_work.clone();
            let t_file = Arc::clone(&torrent_file);
            let c_id = Arc::clone(&client_id);
            let p_info = Arc::clone(&peer_info);

            tokio::spawn(async move {
                println!("Creating slave downloader {} ", slave_id);
                let peer_stream =
                    PeerStream::new(slave_id, &p_info[slave_id], &t_file, &c_id).await;
                match peer_stream {
                    Ok(mut stream) => {
                        //keep reading if there is work to do
                        while let Ok(piece_id) = r_work.recv().await {
                            let downloaded_piece = stream.download_piece(piece_id).await;
                            match downloaded_piece {
                                Ok(piece) => {
                                    let _ = t_piece.send(piece).await;
                                }
                                _ => {
                                    //if it was unable to download the piece, put back the work in the queue
                                    t_work.send(piece_id).await;
                                }
                            }
                        }
                    }
                    _ => {
                        //fixme try recreate the stream until there is no more peer or we have exactly number of peers thread
                        println!("Error creating stream");
                    }
                }
            });
        }

        let number_of_pieces = self.torrent_file.pieces.len();
        let mut all_pieces = HashSet::with_capacity(number_of_pieces);
        all_pieces.extend(0..number_of_pieces);

        //fixme I already know the dimension of everything here following the torrent, i JUST NEED
        //to store the dimension of a flush, in order to save memory
        let mut downloaded_file: HashMap<usize, Vec<u8>> = HashMap::with_capacity(number_of_pieces);
        let file_dimension = number_of_pieces as u64 * self.torrent_file.piece_length as u64;
        let mut persisted_file =
            TorrentPersisted::new(&self.torrent_file.name, file_dimension).await?;

        //send work to slave reading from  checkpoint
        let mut piece_to_download: HashSet<usize> = HashSet::with_capacity(number_of_pieces);
        piece_to_download.extend(0..number_of_pieces);
        let piece_already_downloaded = persisted_file.read_checkpoint().await?;

        for piece in piece_to_download.difference(&piece_already_downloaded) {
            transmitter_work.send(*piece).await.unwrap();
        }

        info!(
            "Total pieces: {}, Pieces still to download: {}",
            number_of_pieces,
            piece_to_download
                .difference(&piece_already_downloaded)
                .count()
        );

        let mut completed_pieces = piece_already_downloaded.len();

        //keep up reading the piece that has been downloaded
        info! {"completed pieces {}", completed_pieces}

        loop {
            if completed_pieces == self.torrent_file.pieces.len() {
                persisted_file
                    .write_pieces(
                        &mut downloaded_file,
                        self.torrent_file.piece_length as usize,
                    )
                    .await?;
                break;
            }

            let received_piece = receiver_piece.recv().await?;
            info! {"completed pieces {}", completed_pieces}
            info! {"asd {}", self.torrent_file.pieces.len()}

            if completed_pieces % 100 == 0 {
                persisted_file
                    .write_pieces(
                        &mut downloaded_file,
                        self.torrent_file.piece_length as usize,
                    )
                    .await?;
            }
            if Self::piece_hash_is_correct(
                &received_piece.1,
                self.torrent_file.pieces[received_piece.0],
            ) {
                info!("Received piece number: {}", received_piece.0);
                downloaded_file.insert(received_piece.0, received_piece.1.clone());
                completed_pieces += 1;
            } else {
                info!("Resend the piece to queue {} ", &received_piece.0);
                transmitter_work.send(received_piece.0).await.unwrap();
            }
        }

        Ok(())
    }
}
