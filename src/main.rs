mod parser;

use std::{fs};
use crate::parser::bencode::parse_bencode;


fn main() -> std::io::Result<()> {
    let becode_input = fs::read("/Users/teospadotto/Documents/project/Rust/study/resource/debian-12.10.0-amd64-netinst.iso.torrent")?;
    let oggetto_deserializzato = parse_bencode(&becode_input);

    println!("{:?}", oggetto_deserializzato.0);

    Ok(())
}