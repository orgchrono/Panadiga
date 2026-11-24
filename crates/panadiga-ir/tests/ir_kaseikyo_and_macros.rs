#![forbid(unsafe_code)]

use panadiga_ir::{
    decode_kaseikyo_from_pronto, format_kaseikyo_wire_hex, import_flipper_ir, parse_pronto_hex,
    synthesize_kaseikyo_pronto, verified_service_macros,
};

#[test]
fn kaseikyo_wire_checksum_and_pronto_roundtrip_multiple_triplets() {
    for (dev, sub, obc) in [
        (0xB0u8, 0x00u8, 0x3Du8),
        (0xB0, 0x01, 0x41),
        (0xB0, 0x10, 0x33),
        (0xB0, 0x20, 0x89),
    ] {
        let hex_frame = format_kaseikyo_wire_hex(dev, sub, obc);
        let bytes: Vec<u8> = hex_frame
            .split_whitespace()
            .map(|tok| u8::from_str_radix(tok, 16).expect("valid byte token"))
            .collect();
        assert_eq!(bytes.len(), 6);
        assert_eq!(bytes[0], 0x02);
        assert_eq!(bytes[1], 0x20);
        assert_eq!(bytes[2], dev);
        assert_eq!(bytes[3], sub);
        assert_eq!(bytes[4], obc);
        assert_eq!(bytes[5], dev ^ sub ^ obc);

        let pronto = synthesize_kaseikyo_pronto(dev, sub, obc);
        assert_eq!(decode_kaseikyo_from_pronto(&pronto), Some((dev, sub, obc)));
    }
}

#[test]
fn service_macros_and_malformed_ir_inputs_behave_safely() {
    let macros = verified_service_macros();
    assert!(!macros.is_empty());
    for m in macros {
        assert!(!m.id.trim().is_empty());
        assert!(!m.steps.is_empty());
    }

    assert_eq!(decode_kaseikyo_from_pronto(""), None);
    assert!(parse_pronto_hex("not hex").is_err());
    assert!(parse_pronto_hex("0000 006D").is_err());

    let imported = import_flipper_ir("corrupt junk\n# comment\nname: Bad\n", "BadInput");
    assert!(imported.keys.is_empty());
}
