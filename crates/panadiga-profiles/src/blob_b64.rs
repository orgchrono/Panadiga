#![deny(unsafe_code)]

//! Pure-Rust RFC 4648 base64 codec for embedding referenced binary blobs inside `ProfilesBundle`.

use anyhow::{Result, bail};

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Encodes arbitrary bytes into canonical RFC 4648 base64 without line breaks.
#[must_use]
pub fn encode_base64(input: &[u8]) -> String {
    if input.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = u32::from(*chunk.get(1).unwrap_or(&0));
        let b2 = u32::from(*chunk.get(2).unwrap_or(&0));
        let triple = (b0 << 16) | (b1 << 8) | b2;

        out.push(ALPHABET[((triple >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((triple >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[((triple >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[(triple & 0x3F) as usize] as char
        } else {
            '='
        });
    }
    out
}

fn decode_sextet(byte: u8) -> Result<u8> {
    match byte {
        b'A'..=b'Z' => Ok(byte - b'A'),
        b'a'..=b'z' => Ok(byte - b'a' + 26),
        b'0'..=b'9' => Ok(byte - b'0' + 52),
        b'+' => Ok(62),
        b'/' => Ok(63),
        _ => bail!("Invalid base64 byte 0x{byte:02X}"),
    }
}

/// Decodes RFC 4648 base64 (ignoring ASCII whitespace) into raw bytes.
pub fn decode_base64(input: &str) -> Result<Vec<u8>> {
    let cleaned: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if cleaned.is_empty() {
        return Ok(Vec::new());
    }
    if !cleaned.len().is_multiple_of(4) {
        bail!(
            "Invalid base64 length {} (must be a multiple of 4)",
            cleaned.len()
        );
    }

    let mut out = Vec::with_capacity((cleaned.len() / 4) * 3);
    let (quartets, _) = cleaned.as_chunks::<4>();
    let total_chunks = quartets.len();
    for (idx, chunk) in quartets.iter().enumerate() {
        let is_last = idx + 1 == total_chunks;
        let pad_count = usize::from(chunk[2] == b'=') + usize::from(chunk[3] == b'=');
        if chunk[0] == b'=' || chunk[1] == b'=' || (chunk[2] == b'=' && chunk[3] != b'=') {
            bail!("Malformed base64 padding in quartet #{idx}");
        }
        if pad_count > 0 && !is_last {
            bail!("Unexpected non-trailing base64 padding in quartet #{idx}");
        }

        let s0 = u32::from(decode_sextet(chunk[0])?);
        let s1 = u32::from(decode_sextet(chunk[1])?);
        let s2 = if chunk[2] == b'=' {
            0
        } else {
            u32::from(decode_sextet(chunk[2])?)
        };
        let s3 = if chunk[3] == b'=' {
            0
        } else {
            u32::from(decode_sextet(chunk[3])?)
        };

        let triple = (s0 << 18) | (s1 << 12) | (s2 << 6) | s3;
        out.push(((triple >> 16) & 0xFF) as u8);
        if chunk[2] != b'=' {
            out.push(((triple >> 8) & 0xFF) as u8);
        }
        if chunk[3] != b'=' {
            out.push((triple & 0xFF) as u8);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_roundtrip_all_padding_cases_and_rejects_malformed() {
        assert_eq!(encode_base64(b""), "");
        assert_eq!(decode_base64(" \r\n ").unwrap(), b"");

        let vectors: &[(&[u8], &str)] = &[
            (b"f", "Zg=="),
            (b"fo", "Zm8="),
            (b"foo", "Zm9v"),
            (b"foobar", "Zm9vYmFy"),
        ];
        for &(raw, expected_b64) in vectors {
            let encoded = encode_base64(raw);
            assert_eq!(encoded, expected_b64);
            assert_eq!(decode_base64(&encoded).unwrap(), raw);
        }

        let binary: Vec<u8> = (0u8..=255).collect();
        let b64 = encode_base64(&binary);
        assert_eq!(decode_base64(&b64).unwrap(), binary);

        assert!(decode_base64("abc").is_err());
        assert!(decode_base64("ab=d").is_err());
        assert!(decode_base64("a===aaaa").is_err());
        assert!(decode_base64("ab==AAAA").is_err());
        assert!(decode_base64("ab!d").is_err());
    }
}
