use anyhow::Result;

fn main() -> Result<()> {
    let root = panadiga_core::constants::resolve_workspace_root();
    panadiga_core::ssot::export_all_ssot_projections(&root)?;
    println!(
        "Exported all Panadiga SSOT projections under {}",
        root.display()
    );
    Ok(())
}
