use std::collections::HashMap;
use std::fmt;

const START_INTEGER: u8 = b'i';
const START_LIST: u8 = b'l';
const START_DICTIONARY: u8 = b'd';
const END_INTEGER_LIST_DICTIONARY: u8 = b'e';
const INTEGER_MINUS_SIGN: u8 = b'-';
const ZERO: u8 = b'0';
const END_SIZE_OF_STRING: u8 = b':';

const START_STRING: u8 = b'1';
const END_STRING: u8 = b'9';

#[derive(Debug, PartialEq, Eq)]
pub enum BencodeValue {
    Integer(Vec<u8>),
    String(Vec<u8>),
    List(Vec<BencodeValue>),
    Dictionary(HashMap<Vec<u8>, BencodeValue>),
    Error(String),
}

impl fmt::Display for BencodeValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BencodeValue::Integer(bytes) => {
                // converte i bytes in stringa e poi stampa
                let s = std::str::from_utf8(bytes).unwrap_or("<invalid utf8>");
                write!(f, "{}", s)
            }
            BencodeValue::String(bytes) => {
                let s = std::str::from_utf8(bytes).unwrap_or("<invalid utf8>");
                write!(f, "\"{}\"", s) // mettiamo virgolette per distinguere
            }
            BencodeValue::List(list) => {
                write!(f, "[")?;
                for (i, item) in list.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            BencodeValue::Dictionary(dict) => {
                write!(f, "{{")?;
                for (i, (key, value)) in dict.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    let key_str = std::str::from_utf8(key).unwrap_or("<invalid utf8>");
                    write!(f, "\"{}\": {}", key_str, value)?;
                }
                write!(f, "}}")
            }
            BencodeValue::Error(msg) => write!(f, "Error: {}", msg),
        }
    }
}
impl BencodeValue {
    pub fn as_string_or_panic(&self) -> String {
        match self {
            BencodeValue::String(bytes) => {
                String::from_utf8(bytes.clone()).expect("BencodeValue string is not valid UTF-8")
            }
            _ => panic!("BencodeValue is not a string"),
        }
    }

    pub fn as_int_or_panic(&self) -> i64 {
        match self {
            BencodeValue::Integer(bytes) => {
                let s = std::str::from_utf8(bytes).expect("invalid UTF-8 inside Bencode integer");
                s.parse::<i64>()
                    .expect("BencodeValue integer is not a valid i64")
            }
            _ => panic!("BencodeValue is not an integer"),
        }
    }
}

fn parse_string(input_slice: &[u8]) -> (BencodeValue, usize) {
    if input_slice.len() < 1 {
        return (BencodeValue::Error("Stringa troppo corta".to_string()), 0);
    }
    let pos_end_string = input_slice
        .iter()
        .position(|&b| b == END_SIZE_OF_STRING)
        .unwrap_or(input_slice.len());
    let string_size = std::str::from_utf8(&input_slice[..pos_end_string])
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);
    (
        BencodeValue::String(
            input_slice[pos_end_string + 1..pos_end_string + 1 + string_size].to_vec(),
        ),
        pos_end_string + string_size + 1,
    )
}

fn parse_integer(input_slice: &[u8]) -> (BencodeValue, usize) {
    if input_slice.len() < 1 {
        return (BencodeValue::Error("Integer troppo corto".to_string()), 0);
    }
    let end_of_integer = input_slice
        .iter()
        .position(|&b| b == END_INTEGER_LIST_DICTIONARY)
        .unwrap_or(input_slice.len());
    (
        BencodeValue::Integer(input_slice[1..end_of_integer].to_vec()),
        end_of_integer + 1,
    )
}

fn parse_dictionary(input_slice: &[u8]) -> (BencodeValue, usize) {
    let mut parsed_dict = HashMap::<Vec<u8>, BencodeValue>::new();

    let mut pos = 1;
    while pos < input_slice.len() {
        if input_slice[pos] == END_INTEGER_LIST_DICTIONARY {
            break;
        }
        let key = parse_string(&input_slice[pos..]);
        let start_index_value = pos + key.1;
        let value = parse_bencode(&input_slice[start_index_value..]);
        pos = pos + key.1 + value.1;
        let BencodeValue::String(key_str) = key.0 else {
            panic!("expected string");
        };
        parsed_dict.insert(key_str, value.0);
    }
    (BencodeValue::Dictionary(parsed_dict), pos + 1)
}

fn parse_list(input_slice: &[u8]) -> (BencodeValue, usize) {
    let mut pos = 1;
    let mut values = Vec::<BencodeValue>::new();
    if input_slice.len() < 1 {
        return (BencodeValue::Error("Error in parsing".to_string()), 0);
    }
    while pos < input_slice.len() {
        //fix me se c'e' uan e a meta'; e non ci dovrebbe stare allora e' un errore
        if input_slice[pos] == END_INTEGER_LIST_DICTIONARY {
            break;
        }
        let pars_result = parse_bencode(&input_slice[pos..]);
        pos += pars_result.1;
        values.push(pars_result.0);
    }
    (BencodeValue::List(values), pos + 1)
}

pub fn parse_bencode(input_slice: &[u8]) -> (BencodeValue, usize) {
    match input_slice[0] {
        START_INTEGER => parse_integer(&input_slice),
        x if x >= START_STRING && x <= END_STRING => parse_string(&input_slice),
        START_LIST => parse_list(&input_slice),
        START_DICTIONARY => parse_dictionary(&input_slice),
        _ => (BencodeValue::Error("Error in parsing".to_string()), 0),
    }
}

pub fn encode_bencode(value: &BencodeValue) -> Vec<u8> {
    //ai generated, to check adn tested TBD
    match value {
        BencodeValue::Integer(bytes) => {
            let mut out = Vec::new();
            out.extend_from_slice(b"i");
            out.extend_from_slice(bytes);
            out.extend_from_slice(b"e");
            out
        }

        BencodeValue::String(bytes) => {
            let mut out = Vec::new();
            out.extend_from_slice(bytes.len().to_string().as_bytes());
            out.extend_from_slice(b":");
            out.extend_from_slice(bytes);
            out
        }

        BencodeValue::List(list) => {
            let mut out = Vec::new();
            out.extend_from_slice(b"l");
            for item in list {
                out.extend_from_slice(&encode_bencode(item));
            }
            out.extend_from_slice(b"e");
            out
        }

        BencodeValue::Dictionary(dict) => {
            let mut out = Vec::new();
            out.extend_from_slice(b"d");
            let mut keys: Vec<&Vec<u8>> = dict.keys().collect();
            keys.sort();
            for key in keys {
                out.extend_from_slice(key.len().to_string().as_bytes());
                out.extend_from_slice(b":");
                out.extend_from_slice(key);
                out.extend_from_slice(&encode_bencode(&dict[key]));
            }
            out.extend_from_slice(b"e");
            out
        }

        BencodeValue::Error(_) => {
            panic!("Cannot encode Error variant")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bencode_parse_integer() {
        let result = parse_integer("i3e".as_bytes());
        assert_eq!(result.0, BencodeValue::Integer("3".as_bytes().to_vec()));
        assert_eq!(result.1, 3);
        let result_negative = parse_integer("i-3e".as_bytes());
        assert_eq!(
            result_negative.0,
            BencodeValue::Integer("-3".as_bytes().to_vec())
        );
        assert_eq!(result_negative.1, 4);
        let result_zero = parse_integer("i0e".as_bytes());
        assert_eq!(
            result_zero.0,
            BencodeValue::Integer("0".as_bytes().to_vec())
        );
        assert_eq!(result_zero.1, 3);
        //-0 oppure 02, 00 deve dare errore unparsable
        // let result_zero = parse_integer("i-0e".as_bytes());
        // assert_eq!(result_zero.0, BencodeValue::Integer(BencodeInteger::new("0".as_bytes().to_vec())));
        // assert_eq!(result_zero.1,2);
    }

    #[test]
    fn bencode_parse_string() {
        let result = parse_string("3:teo".as_bytes());
        assert_eq!(result.0, BencodeValue::String("teo".as_bytes().to_vec()));
        assert_eq!(result.1, 5);
    }

    #[test]
    fn bencode_parse_long_string() {
        let value = "96:https://cdimage.debian.org/cdimage/release/12.10.0/amd64/iso-cd/debian-12.10.0-amd64-netinst.iso".as_bytes();
        let result = parse_string(value);
        assert_eq!(result.0, BencodeValue::String("https://cdimage.debian.org/cdimage/release/12.10.0/amd64/iso-cd/debian-12.10.0-amd64-netinst.iso".as_bytes().to_vec()));
    }
    #[test]
    fn bencode_parse_peers() {
        let mut result_map = HashMap::new();
        let mut ip_map1 = HashMap::new();
        ip_map1.insert(
            "ip".as_bytes().to_vec(),
            BencodeValue::Integer("46.5.64.254".as_bytes().to_vec()),
        );
        ip_map1.insert(
            "port".as_bytes().to_vec(),
            BencodeValue::Integer("6881".as_bytes().to_vec()),
        );
        result_map.insert(
            "interval".as_bytes().to_vec(),
            BencodeValue::Integer("900".as_bytes().to_vec()),
        );
        result_map.insert(
            "peers".as_bytes().to_vec(),
            BencodeValue::List(vec![BencodeValue::Dictionary(ip_map1)]),
        );
        let input = "d8:intervali900e5:peersld2:ip11:46.5.64.2544:porti6881eeee".as_bytes();
        let result = parse_bencode(input);
        println!("{}", result.0);
        println!("{}", BencodeValue::Dictionary(result_map));
        // assert_eq!(result.0, BencodeValue::Dictionary(result_map));
    }

    #[test]
    fn bencode_parse_list() {
        let result = parse_list("l3:teo2:spi-9ee".as_bytes());
        assert_eq!(
            result.0,
            BencodeValue::List(vec![
                BencodeValue::String("teo".as_bytes().to_vec()),
                BencodeValue::String("sp".as_bytes().to_vec()),
                BencodeValue::Integer("-9".as_bytes().to_vec())
            ])
        );
        assert_eq!(result.1, 15);
        let result_list = parse_list("li1eli2ei3eee".as_bytes());
        assert_eq!(
            result_list.0,
            BencodeValue::List(vec![
                BencodeValue::Integer("1".as_bytes().to_vec()),
                BencodeValue::List(vec![
                    BencodeValue::Integer("2".as_bytes().to_vec()),
                    BencodeValue::Integer("3".as_bytes().to_vec()),
                ])
            ])
        );
    }
    #[test]
    fn bencode_parse_dictionary() {
        let mut result_map = HashMap::new();
        result_map.insert(
            "teo".as_bytes().to_vec(),
            BencodeValue::Integer("3".as_bytes().to_vec()),
        );
        let result = parse_dictionary("d3:teoi3ee".as_bytes());
        assert_eq!(result.0, BencodeValue::Dictionary(result_map))
    }
}
