use std::path::Path;

fn main() -> anyhow::Result<()> {
    panadiga_profiles::export_schemas_to_dir(Path::new("profiles/schema"))?;
    Ok(())
}
