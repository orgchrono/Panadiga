//! Pure deterministic Jython 2.7 / Python-3 script renderers driven by `specs`.

mod audit_script;
mod core_scripts;
mod decompile_fix_script;
mod trap_resolvers;

pub use audit_script::generate_ghidra_auto_unbrick_audit;
pub use core_scripts::{generate_ghidra_memory_map, generate_ghidra_symbols};
pub use decompile_fix_script::generate_ghidra_mn103_fix_decompilation;

use super::specs::{AUDIT_KEYWORDS, LabeledAddrSpec, MemoryRegionSpec, SymbolSpec};
use std::fmt::Write as _;

pub(super) fn py_bool(value: bool) -> &'static str {
    if value { "True" } else { "False" }
}

pub fn format_py_byte_tuple(bytes: &[u8]) -> String {
    let mut out = String::from("(");
    for (idx, b) in bytes.iter().enumerate() {
        if idx > 0 {
            out.push_str(", ");
        }
        let _ = write!(&mut out, "0x{:02X}", b);
    }
    if bytes.len() == 1 {
        out.push(',');
    }
    out.push(')');
    out
}

pub(super) fn render_memory_regions(specs: &[MemoryRegionSpec]) -> String {
    let mut out = String::new();
    for spec in specs {
        let _ = writeln!(
            &mut out,
            "        ({:?}, 0x{:08X}, 0x{:08X}, {}, {}, {}, {}),",
            spec.name,
            spec.start,
            spec.size,
            py_bool(spec.read),
            py_bool(spec.write),
            py_bool(spec.execute),
            py_bool(spec.is_volatile),
        );
    }
    out
}

pub(super) fn render_symbol_specs(specs: &[SymbolSpec]) -> String {
    let mut out = String::new();
    for spec in specs {
        let _ = writeln!(
            &mut out,
            "        (0x{:08X}, {:?}, {:?}, {}),",
            spec.address,
            spec.name,
            spec.comment,
            py_bool(spec.is_primary),
        );
    }
    out
}

pub(super) fn render_labeled_tuples(specs: &[LabeledAddrSpec]) -> String {
    let mut out = String::new();
    for spec in specs {
        let _ = writeln!(&mut out, "    (0x{:08X}, {:?}),", spec.address, spec.label);
    }
    out
}

pub(super) fn render_keywords() -> String {
    let mut out = String::new();
    for kw in AUDIT_KEYWORDS {
        let _ = writeln!(&mut out, "    {:?},", kw);
    }
    out
}

pub fn render_pt_wrapper_script() -> &'static str {
    r#"# Ghidra Script: Panasonic DMR-EH55 Symbol Importer (Backward-Compatibility Wrapper)
# @category Forensics.Panasonic
# @author Panadiga Engineering Team (Jython 2.7 + Python 3 compatible)
from __future__ import print_function

import os
import runpy

_target = os.path.join(os.path.dirname(os.path.abspath(__file__)), "ghidra_import_symbols_panadiga.py")
runpy.run_path(_target, init_globals={"currentProgram": globals().get("currentProgram")}, run_name="__main__")
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_ghidra_scripts_use_verified_sc0_and_jython_safe_syntax() {
        let map_py = generate_ghidra_memory_map();
        let sym_py = generate_ghidra_symbols();
        let audit_py = generate_ghidra_auto_unbrick_audit();
        let fix_py = generate_ghidra_mn103_fix_decompilation();

        assert!(map_py.contains("SC0_UART0_MMIO") && map_py.contains("0xD4002000"));
        assert!(map_py.contains("SDCD_HOST_MMIO") && map_py.contains("0x89001800"));
        assert!(
            map_py.contains("FLASH_NOR_CS1_IPL")
                && map_py.contains("0x84000000")
                && map_py.contains("0x00130000")
        );
        assert!(
            map_py.contains("FLASH_NOR_CS1_EH55_BGO")
                && map_py.contains("0x84130000")
                && map_py.contains("0x002D0000")
        );
        assert!(map_py.contains("PROG061_SDRAM_UNCACHED") && map_py.contains("0x90400000"));
        assert!(map_py.contains("AIOF_TBC_MMIO") && map_py.contains("0x88000000"));
        assert!(sym_py.contains("DO_NOT_READ_LETHAL_0xC000100C") && sym_py.contains("0xC000100C"));
        assert!(sym_py.contains("EH55_BGO_GEOMETRY_TABLE") && sym_py.contains("0x440337A4"));
        assert!(sym_py.contains("UPPER_EEMUL_BASE") && sym_py.contains("0x84400000"));
        assert!(sym_py.contains("SC0TXB_UART0_TX_BYTE") && sym_py.contains("0xD4002008"));
        assert!(sym_py.contains("SDCD_BUS_WIDTH_CFG") && sym_py.contains("0x89001850"));
        assert!(!sym_py.contains("0xC0000100"));
        assert!(
            fix_py.contains("decode_mn103_regmask") && fix_py.contains("extract_mn103_call_frame")
        );
        assert!(fix_py.contains("setStackPurgeSize(0)") && fix_py.contains("COMPUTED_JUMP"));

        assert!(
            !sym_py.contains("sym_table.setPrimary") && !sym_py.contains("listing.PLATE_COMMENT")
        );
        assert!(sym_py.contains("sym.setPrimary()") && sym_py.contains("CodeUnit.PLATE_COMMENT"));
        assert!(audit_py.contains("jpype.JArray(jpype.JByte)"));
        for script in [&map_py, &sym_py, &audit_py, &fix_py] {
            assert!(!script.contains("from pathlib import"));
            assert!(!script.contains("0xFFFFFFFFL"));
        }
    }
}
