#![deny(unsafe_code)]

use std::fs;
use std::path::Path;

use super::*;
use crate::constants::resolve_workspace_root;

#[test]
fn synthetic_mn103_symbol_index_tracks_callers_mmio_and_transitive_nor_bgo() {
    // Construct a small MN103S binary @ 0x50400000:
    // func A @ 0x50400000: movm [d2],(sp); call32 -> 0x50400020 (callee C); rets
    // func C @ 0x50400020: movm [a3],(sp); mov 0x84380000, a0 (0xFC D4 ...); rets
    // string @ 0x50400040: "BGO_TEST\0"
    let base_va = 0x5040_0000u32;
    let mut bin = vec![0xCBu8; 0x80];

    // Func A @ 0x00..0x18: 0xCF 0x80 | 0xDD disp(i32) 0x80 0x04 | 0xF0 0xFC
    bin[0x00] = 0xCF;
    bin[0x01] = 0x80;
    bin[0x02] = 0xDD;
    let disp_a_to_c: i32 = 0x20 - 0x02; // +0x1E (MN103S PC-relative from insn start!)
    bin[0x03..0x07].copy_from_slice(&disp_a_to_c.to_le_bytes());
    bin[0x07] = 0x80;
    bin[0x08] = 0x04;
    bin[0x09] = 0xF0;
    bin[0x0A] = 0xFC;

    // Func C @ 0x20..0x38: 0xCF 0x10 | 0xFC 0xDC 0x84380000 | 0xFC 0xDC 0x50400040 | 0xF0 0xFC
    bin[0x20] = 0xCF;
    bin[0x21] = 0x10;
    bin[0x22] = 0xFC;
    bin[0x23] = 0xDC;
    bin[0x24..0x28].copy_from_slice(&0x8438_0000u32.to_le_bytes());
    bin[0x28] = 0xFC;
    bin[0x29] = 0xDC;
    bin[0x2A..0x2E].copy_from_slice(&0x5040_0040u32.to_le_bytes());
    bin[0x2E] = 0xF0;
    bin[0x2F] = 0xFC;

    bin[0x40..0x49].copy_from_slice(b"BGO_TEST\0");

    let idx = build_symbol_index(&bin, "loader_ver061_toc48_main57", "SYNTH", base_va, &[]);
    assert_eq!(idx.firmware_train_id, "loader_ver061_toc48_main57");
    assert_eq!(idx.module, "SYNTH");
    assert!(idx.function_count >= 2);

    let func_c = idx
        .functions
        .iter()
        .find(|f| f.va == "0x50400020")
        .expect("callee C present");
    assert_eq!(func_c.side_effects, SymbolSideEffect::WritesNorBgo);
    assert!(func_c.callers.contains(&"0x50400000".to_string()));
    assert!(func_c.mmio_xrefs.contains(&"0x84380000".to_string()));
    assert!(func_c.string_xrefs.iter().any(|s| s.contains("BGO_TEST")));

    let func_a = idx
        .functions
        .iter()
        .find(|f| f.va == "0x50400000")
        .expect("caller A present");
    assert_eq!(func_a.side_effects, SymbolSideEffect::WritesNorBgo);
    assert!(func_a.callees.contains(&"0x50400020".to_string()));

    // Write & query via isolated temp workspace root
    let dir = std::env::temp_dir().join(format!("panadiga_sym_idx_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let out_p =
        resolve_default_symbol_output_path(&dir, &idx.firmware_train_id, &idx.module).unwrap();
    write_symbol_index(&idx, &out_p).unwrap();

    let res = lookup_symbols(
        &dir,
        &SymbolLookupQuery {
            va: Some("0x50400020".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(res.total_matches, 1);
    assert!(res.safety_gate.blocked_for_unattended_call);
    assert_eq!(res.safety_gate.destructive_matches, 1);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn real_prog061_checked_in_symbol_index_blocks_bgo_mutators_if_present() {
    let root = resolve_workspace_root();
    let json_path =
        Path::new(&root).join("profiles/symbols/loader_ver061_toc48_main57/PROG061.json");
    if !json_path.is_file() {
        return;
    }
    let idx = load_symbol_index(&root, "loader_ver061_toc48_main57", "PROG061").unwrap();
    assert_eq!(idx.firmware_train_id, "loader_ver061_toc48_main57");
    assert!(idx.function_count > 50);

    for target in ["0x50409DF2", "0x50409E65"] {
        let lookup = lookup_symbols(
            &root,
            &SymbolLookupQuery {
                train_id: Some("loader_ver061_toc48_main57".to_string()),
                module: Some("PROG061".to_string()),
                va: Some(target.to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(lookup.total_matches, 1, "missing {target} in PROG061.json");
        assert_eq!(
            lookup.matches[0].symbol.side_effects,
            SymbolSideEffect::WritesNorBgo
        );
        assert!(lookup.safety_gate.blocked_for_unattended_call);
        assert!(
            lookup.matches[0]
                .symbol
                .callers
                .contains(&"0x50404FA7".to_string()),
            "{target} should list 0x50404FA7 in callers"
        );
    }
}
