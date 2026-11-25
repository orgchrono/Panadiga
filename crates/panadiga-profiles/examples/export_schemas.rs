use std::path::Path;

use anyhow::Result;

fn main() -> Result<()> {
    let schema_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../profiles/schema");
    let count = panadiga_profiles::export_schemas_to_dir(&schema_dir)?;
    println!(
        "Exported {count} Draft 2020-12 JSON Schemas to {}",
        schema_dir.display()
    );
    Ok(())
}
