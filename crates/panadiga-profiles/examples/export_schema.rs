use std::path::Path;

fn main() -> anyhow::Result<()> {
    let schema_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../profiles/schema");
    let count = panadiga_profiles::export_schemas_to_dir(&schema_dir)?;
    println!("Exported {count} JSON Schemas to {}", schema_dir.display());
    Ok(())
}
