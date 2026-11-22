#![deny(unsafe_code)]

//! Auditable hexadecimal (`0x...`) and binary (`0b...`) serde serializers/deserializers
//! with optional, vector, and symbol-map support for external `.json` hardware manifests.

use std::collections::BTreeMap;

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serializer};

/// Parses decimal, `0x`/`0X` hexadecimal (underscores permitted), or `0b`/`0B` binary into `u64`.
pub fn parse_prefixed_u64(raw: &str) -> Result<u64, String> {
    let normalized: String = raw.trim().chars().filter(|&c| c != '_').collect();
    if let Some(hex) = normalized
        .strip_prefix("0x")
        .or_else(|| normalized.strip_prefix("0X"))
    {
        if hex.is_empty() {
            return Err("Empty hex literal".to_string());
        }
        u64::from_str_radix(hex, 16).map_err(|e| format!("Invalid hex literal '{raw}': {e}"))
    } else if let Some(bin) = normalized
        .strip_prefix("0b")
        .or_else(|| normalized.strip_prefix("0B"))
    {
        if bin.is_empty() {
            return Err("Empty binary literal".to_string());
        }
        u64::from_str_radix(bin, 2).map_err(|e| format!("Invalid binary literal '{raw}': {e}"))
    } else {
        normalized
            .parse::<u64>()
            .map_err(|e| format!("Invalid integer literal '{raw}': {e}"))
    }
}

/// Parses a hex byte sequence (spaces, `0x`, `_` permitted, e.g. `"55 AA"` or `"4344303031"`).
pub fn parse_hex_bytes(raw: &str) -> Result<Vec<u8>, String> {
    let trimmed = raw.trim();
    let no_prefix = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    let cleaned: String = no_prefix
        .chars()
        .filter(|c| !c.is_ascii_whitespace() && *c != '_')
        .collect();
    if cleaned.is_empty() || !cleaned.len().is_multiple_of(2) {
        return Err(format!(
            "Hex byte string '{raw}' must have an even non-empty nibble count"
        ));
    }
    (0..cleaned.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&cleaned[i..i + 2], 16)
                .map_err(|e| format!("Invalid byte hex '{}' in '{raw}': {e}", &cleaned[i..i + 2]))
        })
        .collect()
}

#[derive(Deserialize)]
#[serde(untagged)]
enum StringOrInt {
    Str(String),
    Int(u64),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum StringOrSignedInt {
    Str(String),
    Int(i64),
}

fn decode_u32(raw: StringOrInt) -> Result<u32, String> {
    let n = match raw {
        StringOrInt::Int(v) => v,
        StringOrInt::Str(s) => parse_prefixed_u64(&s)?,
    };
    u32::try_from(n).map_err(|e| e.to_string())
}

fn decode_u8(raw: StringOrInt) -> Result<u8, String> {
    let n = match raw {
        StringOrInt::Int(v) => v,
        StringOrInt::Str(s) => parse_prefixed_u64(&s)?,
    };
    u8::try_from(n).map_err(|e| e.to_string())
}

fn decode_i32(raw: StringOrSignedInt) -> Result<i32, String> {
    let val: i64 = match raw {
        StringOrSignedInt::Int(v) => v,
        StringOrSignedInt::Str(s) => {
            let trimmed = s.trim();
            if let Some(rest) = trimmed.strip_prefix('-') {
                let mag = parse_prefixed_u64(rest)?;
                let mag_i64 = i64::try_from(mag).map_err(|e| e.to_string())?;
                -mag_i64
            } else {
                let rest = trimmed.strip_prefix('+').unwrap_or(trimmed);
                let mag = parse_prefixed_u64(rest)?;
                i64::try_from(mag).map_err(|e| e.to_string())?
            }
        }
    };
    i32::try_from(val).map_err(|e| e.to_string())
}

pub mod hex_u8 {
    use super::*;

    pub fn serialize<S: Serializer>(val: &u8, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&format!("0x{val:02X}"))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<u8, D::Error> {
        decode_u8(StringOrInt::deserialize(de)?).map_err(D::Error::custom)
    }
}

pub mod hex_i32 {
    use super::*;

    pub fn serialize<S: Serializer>(val: &i32, ser: S) -> Result<S::Ok, S::Error> {
        if *val < 0 {
            ser.serialize_str(&format!("-0x{:08X}", val.unsigned_abs()))
        } else {
            ser.serialize_str(&format!("0x{val:08X}"))
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<i32, D::Error> {
        decode_i32(StringOrSignedInt::deserialize(de)?).map_err(D::Error::custom)
    }
}

pub mod hex_u32 {
    use super::*;

    pub fn serialize<S: Serializer>(val: &u32, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&format!("0x{val:08X}"))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<u32, D::Error> {
        decode_u32(StringOrInt::deserialize(de)?).map_err(D::Error::custom)
    }
}

pub mod opt_hex_u32 {
    use super::*;

    pub fn serialize<S: Serializer>(val: &Option<u32>, ser: S) -> Result<S::Ok, S::Error> {
        match val {
            Some(v) => ser.serialize_str(&format!("0x{v:08X}")),
            None => ser.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<Option<u32>, D::Error> {
        Option::<StringOrInt>::deserialize(de)?
            .map(decode_u32)
            .transpose()
            .map_err(D::Error::custom)
    }
}

pub mod hex_u32_vec {
    use super::*;

    pub fn serialize<S: Serializer>(vals: &[u32], ser: S) -> Result<S::Ok, S::Error> {
        let formatted: Vec<String> = vals.iter().map(|v| format!("0x{v:08X}")).collect();
        ser.collect_seq(formatted)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<Vec<u32>, D::Error> {
        Vec::<StringOrInt>::deserialize(de)?
            .into_iter()
            .map(decode_u32)
            .collect::<Result<Vec<_>, _>>()
            .map_err(D::Error::custom)
    }
}

pub mod hex_u32_map {
    use super::*;

    pub fn serialize<S: Serializer>(
        vals: &BTreeMap<String, u32>,
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        let formatted: BTreeMap<&String, String> = vals
            .iter()
            .map(|(k, v)| (k, format!("0x{v:08X}")))
            .collect();
        ser.collect_map(formatted)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        de: D,
    ) -> Result<BTreeMap<String, u32>, D::Error> {
        BTreeMap::<String, StringOrInt>::deserialize(de)?
            .into_iter()
            .map(|(k, v)| Ok((k, decode_u32(v).map_err(D::Error::custom)?)))
            .collect()
    }
}
