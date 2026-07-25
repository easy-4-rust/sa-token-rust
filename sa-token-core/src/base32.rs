//! RFC 4648 Base32 without mandatory padding.

use thiserror::Error;

const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Base32Error {
    #[error("invalid Base32 character `{character}` at byte {index}")]
    InvalidCharacter { character: char, index: usize },
    #[error("Base32 padding may only appear at the end")]
    InvalidPadding,
    #[error("Base32 input has non-zero trailing bits")]
    NonZeroTrailingBits,
}

/// Encode bytes using RFC 4648 Base32 without `=` padding.
pub fn encode(bytes: &[u8]) -> String {
    let mut output = String::with_capacity((bytes.len() * 8).div_ceil(5));
    let mut buffer = 0_u16;
    let mut bits = 0_u8;

    for &byte in bytes {
        buffer = (buffer << 8) | u16::from(byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            output.push(ALPHABET[((buffer >> bits) & 0x1f) as usize] as char);
        }
    }

    if bits > 0 {
        output.push(ALPHABET[((buffer << (5 - bits)) & 0x1f) as usize] as char);
    }
    output
}

/// Decode RFC 4648 Base32.
///
/// Upper/lower case input and valid trailing `=` padding are accepted. Invalid
/// characters and non-zero trailing bits are rejected instead of silently
/// changing the secret.
pub fn decode(input: &str) -> Result<Vec<u8>, Base32Error> {
    let trimmed = input.trim();
    let data_end = trimmed.find('=').unwrap_or(trimmed.len());
    if trimmed[data_end..].bytes().any(|byte| byte != b'=') {
        return Err(Base32Error::InvalidPadding);
    }

    let data = &trimmed[..data_end];
    let mut output = Vec::with_capacity(data.len() * 5 / 8);
    let mut buffer = 0_u16;
    let mut bits = 0_u8;

    for (index, byte) in data.bytes().enumerate() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a',
            b'2'..=b'7' => byte - b'2' + 26,
            _ => {
                return Err(Base32Error::InvalidCharacter {
                    character: byte as char,
                    index,
                });
            }
        };
        buffer = (buffer << 5) | u16::from(value);
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            output.push(((buffer >> bits) & 0xff) as u8);
        }
    }

    if bits > 0 && (buffer & ((1_u16 << bits) - 1)) != 0 {
        return Err(Base32Error::NonZeroTrailingBits);
    }
    Ok(output)
}

pub fn encode_string(value: &str) -> String {
    encode(value.as_bytes())
}

pub fn decode_string(value: &str) -> Result<String, Base32Error> {
    let bytes = decode(value)?;
    String::from_utf8(bytes).map_err(|error| Base32Error::InvalidCharacter {
        character: error
            .as_bytes()
            .get(error.utf8_error().valid_up_to())
            .copied()
            .unwrap_or_default() as char,
        index: error.utf8_error().valid_up_to(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc4648_vectors_round_trip() {
        for (plain, encoded) in [
            ("", ""),
            ("f", "MY"),
            ("fo", "MZXQ"),
            ("foo", "MZXW6"),
            ("foob", "MZXW6YQ"),
            ("fooba", "MZXW6YTB"),
            ("foobar", "MZXW6YTBOI"),
        ] {
            assert_eq!(encode_string(plain), encoded);
            assert_eq!(decode_string(encoded).unwrap(), plain);
            assert_eq!(decode_string(&encoded.to_ascii_lowercase()).unwrap(), plain);
        }
    }

    #[test]
    fn accepts_padding_but_rejects_corruption() {
        assert_eq!(decode_string("MY======").unwrap(), "f");
        assert!(matches!(
            decode("M!"),
            Err(Base32Error::InvalidCharacter { .. })
        ));
        assert_eq!(decode("MZ"), Err(Base32Error::NonZeroTrailingBits));
        assert_eq!(decode("M=Y"), Err(Base32Error::InvalidPadding));
    }
}
