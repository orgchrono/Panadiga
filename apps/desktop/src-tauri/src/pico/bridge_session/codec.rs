//! Pure wire codecs and formatting helpers for interactive SC0 Bridge sessions.

/// Formats arbitrary bytes as uppercase space-separated hex octets.
pub(super) fn bytes_to_hex_spaced(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Pure encoder converting UI Bridge inputs (`text`, `line`, `hex`, `rsp`) into raw wire bytes.
pub fn encode_bridge_input(input_mode: &str, payload: &str) -> Result<Vec<u8>, String> {
    let mode = input_mode.trim().to_ascii_lowercase();
    match mode.as_str() {
        "text" | "raw" => Ok(payload.as_bytes().to_vec()),
        "line" | "cmd" => {
            let trimmed = payload.trim_end_matches(['\r', '\n']);
            let mut out = trimmed.as_bytes().to_vec();
            out.push(b'\r');
            Ok(out)
        }
        "hex" => decode_hex_payload(payload),
        "rsp" => {
            let trimmed = payload.trim();
            if trimmed.is_empty() {
                return Err("Payload RSP vazio.".to_string());
            }
            let csum: u8 = panadiga_core::pico::rsp_checksum(trimmed.as_bytes());
            Ok(format!("${}#{:02x}", trimmed, csum).into_bytes())
        }
        other => Err(format!(
            "Modo de entrada SC0 Bridge desconhecido: '{}'",
            other
        )),
    }
}

fn decode_hex_payload(raw_hex: &str) -> Result<Vec<u8>, String> {
    let cleaned: String = raw_hex
        .split_whitespace()
        .map(|tok| tok.trim_start_matches("0x").trim_start_matches("0X"))
        .collect::<Vec<_>>()
        .join("")
        .chars()
        .filter(|c| !matches!(c, ':' | '-' | ',' | '_'))
        .collect();

    if cleaned.is_empty() || !cleaned.len().is_multiple_of(2) {
        return Err(format!(
            "Hexadecimal inválido (comprimento ímpar ou vazio): '{}'",
            raw_hex
        ));
    }
    let mut bytes = Vec::with_capacity(cleaned.len() / 2);
    for idx in (0..cleaned.len()).step_by(2) {
        let byte = u8::from_str_radix(&cleaned[idx..idx + 2], 16)
            .map_err(|_| format!("Dígito hexadecimal inválido em '{}'", raw_hex))?;
        bytes.push(byte);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::encode_bridge_input;

    #[test]
    fn encodes_line_hex_and_rsp_frames_cleanly() {
        assert_eq!(encode_bridge_input("line", "PASS").unwrap(), b"PASS\r");
        assert_eq!(
            encode_bridge_input("hex", "0x15 0E:04:02").unwrap(),
            vec![0x15, 0x0E, 0x04, 0x02]
        );
        assert_eq!(encode_bridge_input("rsp", "g").unwrap(), b"$g#67");
        assert_eq!(encode_bridge_input("rsp", "Z0").unwrap(), b"$Z0#8a");
        assert_eq!(encode_bridge_input("text", "~~~").unwrap(), b"~~~");
        assert!(encode_bridge_input("hex", "ABC").is_err());
    }
}
