//! Thin imperative I/O shell for exporting generated Ghidra scripts to disk.

use super::render::{
    generate_ghidra_auto_unbrick_audit, generate_ghidra_memory_map,
    generate_ghidra_mn103_fix_decompilation, generate_ghidra_symbols,
    generate_ghidra_zero_undef_pipeline, render_pt_wrapper_script,
};
use super::specs::generate_ghidra_ssot_maps_json;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Exports all Ghidra scripts to the target directory.
pub fn export_all_ghidra_scripts(target_dir: &Path) -> Result<()> {
    if !target_dir.exists() {
        fs::create_dir_all(target_dir)
            .with_context(|| format!("Failed creating directory {:?}", target_dir))?;
    }

    let ssot_maps_json = generate_ghidra_ssot_maps_json()?;
    let artifacts: [(&str, String); 7] = [
        ("ghidra_am34_memory_map.py", generate_ghidra_memory_map()),
        (
            "ghidra_import_symbols_panadiga.py",
            generate_ghidra_symbols(),
        ),
        (
            "ghidra_auto_unbrick_audit.py",
            generate_ghidra_auto_unbrick_audit(),
        ),
        (
            "ghidra_mn103_fix_decompilation.py",
            generate_ghidra_mn103_fix_decompilation(),
        ),
        (
            "ghidra_importar_simbolos_panadiga.py",
            render_pt_wrapper_script().to_string(),
        ),
        (
            "ghidra_zero_undef_pipeline.py",
            generate_ghidra_zero_undef_pipeline(),
        ),
        ("ghidra_ssot_maps.json", ssot_maps_json),
    ];

    for (filename, contents) in &artifacts {
        let out_path = target_dir.join(filename);
        fs::write(&out_path, contents).with_context(|| format!("Failed writing {:?}", out_path))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_all_writes_valid_scripts_including_audit() {
        let dir = std::env::temp_dir().join(format!("panadiga_ghidra_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        export_all_ghidra_scripts(&dir).expect("export all");
        for file in [
            "ghidra_am34_memory_map.py",
            "ghidra_import_symbols_panadiga.py",
            "ghidra_importar_simbolos_panadiga.py",
            "ghidra_auto_unbrick_audit.py",
            "ghidra_mn103_fix_decompilation.py",
            "ghidra_zero_undef_pipeline.py",
            "ghidra_ssot_maps.json",
        ] {
            assert!(dir.join(file).is_file(), "missing {}", file);
        }
        let wrapper = fs::read_to_string(dir.join("ghidra_importar_simbolos_panadiga.py")).unwrap();
        assert!(!wrapper.contains("pathlib"));
        let _ = fs::remove_dir_all(&dir);
    }
}
