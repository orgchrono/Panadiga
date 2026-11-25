#![deny(unsafe_code)]

//! Deterministic JSON Schema Draft 2020-12 generation (`schemars`) and anti-drift verification.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use schemars::schema_for;
use serde_json::Value;

use crate::types::{
    Authorization, BoardProfile, DigaOsConfigSpec, EepromChipProfile, FirmwareTrainProfile,
    FlashChipProfile, MediaFormatProfile, ModelProfileManifest, NorLayoutProfile, ProfilesBundle,
    ResolvedModelProfile, SocCoreProfile, SocProfile,
};

fn render_schema_value<T: schemars::JsonSchema>() -> Value {
    let schema = schema_for!(T);
    serde_json::to_value(schema).expect("schemars Schema serializes cleanly to Value")
}

type SchemaBuilderEntry = (&'static str, fn() -> Value);

/// Named JSON Schema Draft 2020-12 files emitted into `profiles/schema/`.
#[must_use]
pub fn generate_all_schemas() -> Vec<(&'static str, String)> {
    let entries: &[SchemaBuilderEntry] = &[
        (
            "profiles_bundle.schema.json",
            render_schema_value::<ProfilesBundle>,
        ),
        (
            "resolved_model_profile.schema.json",
            render_schema_value::<ResolvedModelProfile>,
        ),
        (
            "capability_authorization.schema.json",
            render_schema_value::<Authorization>,
        ),
        (
            "soc_core.schema.json",
            render_schema_value::<SocCoreProfile>,
        ),
        ("soc.schema.json", render_schema_value::<SocProfile>),
        (
            "flash_chip.schema.json",
            render_schema_value::<FlashChipProfile>,
        ),
        (
            "eeprom_chip.schema.json",
            render_schema_value::<EepromChipProfile>,
        ),
        (
            "firmware_train.schema.json",
            render_schema_value::<FirmwareTrainProfile>,
        ),
        (
            "nor_layout.schema.json",
            render_schema_value::<NorLayoutProfile>,
        ),
        (
            "media_format.schema.json",
            render_schema_value::<MediaFormatProfile>,
        ),
        ("board.schema.json", render_schema_value::<BoardProfile>),
        (
            "model_manifest.schema.json",
            render_schema_value::<ModelProfileManifest>,
        ),
        (
            "digaos_config.schema.json",
            render_schema_value::<DigaOsConfigSpec>,
        ),
    ];
    entries
        .iter()
        .map(|(name, builder)| {
            let mut txt =
                serde_json::to_string_pretty(&builder()).expect("Pretty JSON Schema serialization");
            txt.push('\n');
            (*name, txt)
        })
        .collect()
}

/// Writes all generated JSON Schema Draft 2020-12 files into `schema_dir`.
pub fn export_schemas_to_dir(schema_dir: &Path) -> Result<usize> {
    fs::create_dir_all(schema_dir)
        .with_context(|| format!("Creating schema dir {}", schema_dir.display()))?;
    let schemas = generate_all_schemas();
    for (filename, content) in &schemas {
        let target = schema_dir.join(filename);
        fs::write(&target, content)
            .with_context(|| format!("Writing schema {}", target.display()))?;
    }
    Ok(schemas.len())
}

/// Verifies every committed schema under `schema_dir` matches freshly generated output.
pub fn verify_schemas_match_dir(schema_dir: &Path) -> Result<()> {
    for (filename, expected) in generate_all_schemas() {
        let target = schema_dir.join(filename);
        let on_disk = fs::read_to_string(&target).with_context(|| {
            format!(
                "Missing generated JSON Schema {} (run `panadiga profiles export-schema`)",
                target.display()
            )
        })?;
        if on_disk.replace("\r\n", "\n") != expected {
            bail!(
                "Schema drift detected in {} — regenerate via `panadiga profiles export-schema`",
                target.display()
            );
        }
    }
    Ok(())
}
