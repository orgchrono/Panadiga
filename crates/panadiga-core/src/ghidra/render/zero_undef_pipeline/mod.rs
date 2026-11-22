//! Renders the 4-pass SSOT-driven Zero-`??` Ghidra recovery pipeline (`ghidra_zero_undef_pipeline.py`).

mod bootstrap;
mod bootstrap_layout;
mod decrypt_runtime;
mod pass2_typing;
mod pass3_functions;
mod pass3_gaps;
mod pass3_seeding;
mod pass4_verify_cli;

/// Generates the CPython 3 + PyGhidra 4-pass Zero-`??` pipeline script (`ghidra_zero_undef_pipeline.py`)
/// backed dynamically by `ghidra_ssot_maps.json` and supporting arbitrary future binaries via `--binary` / `--model`.
pub fn generate_ghidra_zero_undef_pipeline() -> String {
    format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        bootstrap::PY_BOOTSTRAP,
        bootstrap_layout::PY_BOOTSTRAP_LAYOUT,
        decrypt_runtime::PY_DECRYPT_RUNTIME,
        pass2_typing::PY_PASS2,
        pass3_seeding::PY_PASS3,
        pass3_functions::PY_PASS3_FUNCS,
        pass3_gaps::PY_PASS3_GAPS,
        pass4_verify_cli::PY_PASS4_CLI,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_undef_pipeline_contains_ssot_loader_all_passes_and_arbitrary_binary_cli() {
        let src = generate_ghidra_zero_undef_pipeline();
        for needle in [
            "ghidra_ssot_maps.json",
            "_load_ssot_maps",
            "infer_base_va",
            "import_binary",
            "apply_model_memory_layout",
            "infer_generic_entry_seeds",
            "decrypt-runtime",
            "feistel_decrypt_buf",
            "lzss_decompress_pana",
            "scan_dynamic_bench_anchors",
            "pass2_program",
            "pass3_program",
            "pass4_program",
            "verify_program",
            "_run_all_passes",
            "--binary",
            "--frm",
            "--region",
            "--base",
            "--program-name",
            "--model",
            "--project-dir",
            "--project-name",
            "from generic.stl import Pair",
            "Pointer32DataType()",
            "guard_project",
            "Panadiga_MN103S",
        ] {
            assert!(src.contains(needle), "missing {needle}");
        }
    }
}
