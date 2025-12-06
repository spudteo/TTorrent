use std::collections::HashMap;
use std::{fmt};

const START_INTEGER : u8 = b'i';
const START_LIST : u8 = b'l';
const START_DICTIONARY : u8 = b'd';
const END_INTEGER_LIST_DICTIONARY : u8= b'e';
const INTEGER_MINUS_SIGN : u8 = b'-';
const ZERO   : u8 = b'0';
const END_SIZE_OF_STRING : u8 = b':';

const START_STRING :u8 = b'0';
const END_STRING: u8 = b'9';

#[derive(Debug, PartialEq, Eq)]
pub enum BencodeValue {
    Integer(Vec<u8>),
    String(Vec<u8>),
    List(Vec<BencodeValue>),
    Dictionary(HashMap<Vec<u8>, BencodeValue>),
    Error(String),
}


fn parse_string(input_slice : &[u8]) -> (BencodeValue, usize)  {
    if input_slice.len() < 1 {
        return (BencodeValue::Error("Stringa troppo corta".to_string()),0)
    }
    let pos_end_string  = input_slice.iter().position(|&b| b == END_SIZE_OF_STRING).unwrap_or(input_slice.len());
    let string_size = std::str::from_utf8(&input_slice[..pos_end_string])
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);
    (BencodeValue::String(input_slice[pos_end_string +1 ..pos_end_string + 1 + string_size].to_vec()), pos_end_string + string_size)
}

fn parse_integer(input_slice : &[u8]) -> (BencodeValue, usize) {
    if input_slice.len() < 1 {
        return (BencodeValue::Error("Integer troppo corto".to_string()),0)
    }
    let end_of_integer = input_slice.iter().position(|&b| b == END_INTEGER_LIST_DICTIONARY).unwrap_or(input_slice.len());
    (BencodeValue::Integer(input_slice[1..end_of_integer].to_vec()), end_of_integer )
}

fn parse_dictionary(input_slice : &[u8]) -> (BencodeValue, usize) {
    let mut parsed_dict = HashMap::<Vec<u8>, BencodeValue>::new();

    let mut pos =1;
    while pos < input_slice.len() {
        if input_slice[pos] == END_INTEGER_LIST_DICTIONARY { break}
        let key = parse_string(&input_slice[pos..]);
        let start_index_value = pos + key.1 +1;
        let value = parse_bencode(&input_slice[start_index_value..]);
        pos  = pos + key.1 + value.1 + 1;
        let BencodeValue::String(key_str) = key.0 else { panic!("expected string"); };
        parsed_dict.insert(key_str, value.0);
        pos +=1;
    }
    (BencodeValue::Dictionary(parsed_dict), 0 )

}


fn parse_list(input_slice : &[u8]) -> (BencodeValue, usize) {
    let mut pos = 1;
    let mut values  = Vec::<BencodeValue>::new();
    if input_slice.len() < 1 {
        return (BencodeValue::Error("Error in parsing".to_string()),0)
    }
    while pos < input_slice.len() {
        //fix me se c'e' uan e a meta'; e non ci dovrebbe stare allora e' un errore
        if input_slice[pos] == END_INTEGER_LIST_DICTIONARY { break}
        let pars_result = parse_bencode(&input_slice[pos..]);
        pos += pars_result.1;
        values.push(pars_result.0);
        pos += 1;
    }
    (BencodeValue::List(values), pos + 1 )
}

pub fn parse_bencode(input_slice : &[u8]) -> (BencodeValue, usize) {
    match input_slice[0] {
        START_INTEGER => parse_integer(&input_slice),
        x if x > START_STRING && x < END_STRING => parse_string(&input_slice),
        START_LIST => parse_list(&input_slice),
        START_DICTIONARY => parse_dictionary(&input_slice),
        _ => (BencodeValue::Error("Error in parsing".to_string()),0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bencode_parse_integer() {
        let result = parse_integer("i3e".as_bytes());
        assert_eq!(result.0, BencodeValue::Integer("3".as_bytes().to_vec()));
        assert_eq!(result.1,2);
        let result_negative = parse_integer("i-3e".as_bytes());
        assert_eq!(result_negative.0, BencodeValue::Integer("-3".as_bytes().to_vec()));
        assert_eq!(result_negative.1,3);
        let result_zero = parse_integer("i0e".as_bytes());
        assert_eq!(result_zero.0, BencodeValue::Integer("0".as_bytes().to_vec()));
        assert_eq!(result_zero.1,2);
        //-0 oppure 02, 00 deve dare errore unparsable
        // let result_zero = parse_integer("i-0e".as_bytes());
        // assert_eq!(result_zero.0, BencodeValue::Integer(BencodeInteger::new("0".as_bytes().to_vec())));
        // assert_eq!(result_zero.1,2);
    }

    #[test]
    fn bencode_parse_string() {
        let result = parse_string("3:teo".as_bytes());
        assert_eq!(result.0, BencodeValue::String("teo".as_bytes().to_vec()));
        assert_eq!(result.1,4);
    }

    #[test]
    fn bencode_parse_list() {
        let result = parse_list("l3:teo2:spi-9ee".as_bytes());
        assert_eq!(result.0, BencodeValue::List(vec![
            BencodeValue::String("teo".as_bytes().to_vec()),
            BencodeValue::String("sp".as_bytes().to_vec()),
            BencodeValue::Integer("-9".as_bytes().to_vec())
        ]));
        assert_eq!(result.1,15);
        let result_list = parse_list("li1eli2ei3eee".as_bytes());
        assert_eq!(result_list.0, BencodeValue::List(vec![
            BencodeValue::Integer("1".as_bytes().to_vec()),
            BencodeValue::List(vec![
            BencodeValue::Integer("2".as_bytes().to_vec()),
            BencodeValue::Integer("3".as_bytes().to_vec()),
            ])
        ]));
    }
    #[test]
    fn bencode_parse_dictionary(){
        let mut result_map = HashMap::new();
        result_map.insert("teo".as_bytes().to_vec(),BencodeValue::Integer("3".as_bytes().to_vec()));
        let result = parse_dictionary("d3:teoi3ee".as_bytes());
        assert_eq!(result.0, BencodeValue::Dictionary(
            result_map
        ))

    }


}
