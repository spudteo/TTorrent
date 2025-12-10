use crate::parser::bencode::BencodeValue;

pub trait CreateFromBencode {
    fn parse(input_bencode: &BencodeValue) -> Self;
}


