mod parser;
mod request;
mod traits;

use crate::parser::bencode::parse_bencode;
use crate::parser::peers::AnnounceResponse;
use crate::parser::torrent_file::TorrentFile;
use crate::request::client::Client;
use crate::request::handshake::Handshake;
use crate::traits::from_bencode::CreateFromBencode;
use clap::Parser;
use std::fs;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long)]
    file: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let bencode_byte = fs::read(&args.file)?;

    let bencode_input = parse_bencode(&bencode_byte);
    let torrent =
        TorrentFile::new_from_bencode(&bencode_input.0).expect("Failed to parse TorrentFile");
    println!("requesting peers...");
    let response = reqwest::get(torrent.build_tracker_url()?).await?;
    let body_bytes = response.bytes().await?;
    let announce_response = parse_bencode(&body_bytes);
    let announce = AnnounceResponse::parse(&announce_response.0);
    println!("{:?}", announce);

    //185.111.109.15, port_number: 38915
    //ip_addr: 185.239.193.44, port_number: 12765
    //ip_addr: 87.90.58.136, port_number: 56844

    let one_client = Client::new(torrent, announce.peers[0]);
    let piece_received = one_client.download_from_peer(3).await?;
    println!("{:?}", piece_received.len());

    // for peer in &announce.peers {
    //     let peer_id = *b"01234567890123456789";
    //     let handshake = Handshake::new(torrent.info_hash, peer_id);
    //     let peer_hand_result = handshake.shake(peer).await;
    //     match peer_hand_result {
    //         Ok(peer_hand) => {
    //             println!("handshake done {:?}", peer_hand);
    //             let peer_handshake = Handshake::parse(peer_hand);
    //             println!("handshake done {:?}", peer_handshake);
    //         }
    //         Err(e) => continue
    //     }
    // }

    Ok(())
}
