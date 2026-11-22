//! Build-time SSOT hardware manifest bundle validation and JSON Schema drift guard.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn emit_rerun_tree(dir: &Path) {
    println!("cargo:rerun-if-changed={}", dir.display());
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        let ext = path.extension().and_then(|s| s.to_str());
        if path.is_dir() {
            emit_rerun_tree(&path);
        } else if matches!(ext, Some("json" | "gz" | "bin")) {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}

fn main() {
    let manifest_dir = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is always set by Cargo"),
    );
    let profiles_dir = manifest_dir.join("../../profiles");
    emit_rerun_tree(&profiles_dir);

    let bundle = panadiga_profiles::load_bundle_from_dir(&profiles_dir).unwrap_or_else(|e| {
        panic!(
            "Failed to load & validate Panadiga v2 profiles directory {}: {e:#}",
            profiles_dir.display()
        )
    });

    let schema_dir = profiles_dir.join("schema");
    if schema_dir.join("profiles_bundle.schema.json").is_file() {
        panadiga_profiles::verify_schemas_match_dir(&schema_dir).unwrap_or_else(|e| {
            panic!("Panadiga JSON Schema drift guard failed: {e:#}");
        });
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is always set by Cargo"));
    let bundle_json = serde_json::to_string(&bundle)
        .expect("Validated ProfilesBundle must serialize cleanly to JSON");
    fs::write(out_dir.join("profiles.bundle.json"), bundle_json)
        .expect("Failed writing OUT_DIR/profiles.bundle.json");
}
