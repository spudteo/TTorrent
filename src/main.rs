mod parser;
mod request;
mod traits;

use crate::parser::bencode::parse_bencode;
use crate::parser::peers::AnnounceResponse;
use crate::parser::torrent_file::{TorrentFile};
use crate::request::client::Client;
use crate::traits::from_bencode::CreateFromBencode;
use clap::Parser;
use std::fs;
use tokio::time::Instant;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long)]
    file: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = Args::parse();
    let bencode_byte = fs::read(&args.file)?;
    let torrent : TorrentFile = serde_bencode::from_bytes(&bencode_byte).unwrap();
    log::info!("requesting peers...");
    let response = reqwest::get(torrent.build_tracker_url()?).await?;
    let body_bytes = response.bytes().await?;
    println!("{}", String::from_utf8_lossy(&body_bytes));
    let announce: AnnounceResponse = serde_bencode::from_bytes(&body_bytes).unwrap();
    let max_peer = announce.get_peers_number();
    let one_client = Client::new(torrent, announce.get_peers());
    one_client.download_torrent(max_peer).await?;

    Ok(())
}
