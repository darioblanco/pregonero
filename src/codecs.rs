use encoding::all::ISO_8859_1;
use encoding::{DecoderTrap, Encoding};
use quoted_printable::{decode as qp_decode, ParseMode};
use regex::Regex;
use std::str;

pub fn decode_rfc2047(input: &str) -> Result<String, Box<dyn std::error::Error>> {
    let re = Regex::new(r"(?i)\=\?(.*?)\?(q|b)\?(.*?)\?\=").unwrap();

    if !re.is_match(input) {
        return Err("Input is not a valid RFC 2047 encoded-word".into());
    }

    let caps = re.captures(input).unwrap();

    let charset = &caps[1];
    let encoding = &caps[2].to_uppercase();
    let encoded_text = &caps[3];

    let decoded_bytes = match encoding.as_str() {
        "Q" => qp_decode(encoded_text.replace('_', " "), ParseMode::Robust)?,
        _ => return Err(format!("Unsupported encoding: {}", encoding).into()),
    };

    match charset.to_uppercase().as_str() {
        "UTF-8" | "UTF8" => {
            let decoded_str = str::from_utf8(&decoded_bytes)?;
            Ok(decoded_str.to_string())
        }
        "ISO-8859-1" => {
            let decoded_str = ISO_8859_1.decode(&decoded_bytes, DecoderTrap::Strict)?;
            Ok(decoded_str.to_string())
        }
        _ => Err(format!("Unsupported charset: {}", charset).into()),
    }
}
