#![forbid(unsafe_code)]

use std::path::Path;

use panadiga_profiles::types::NorRegionKind;
use panadiga_profiles::{derive_sector_table, load_bundle_from_dir, resolve_model_in_bundle};
use serde_json::json;

fn region_span_hex(region: Option<&panadiga_profiles::types::NorRegion>) -> Option<String> {
    region.map(|r| format!("0x{:06X}..0x{:06X}", r.start_offset, r.end_offset))
}

#[test]
fn snapshot_eh55_resolved_profile_contract() {
    let profiles_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../profiles");
    let bundle = load_bundle_from_dir(&profiles_dir).expect("profiles bundle loads");
    let resolved = resolve_model_in_bundle(&bundle, "DMR-EH55").expect("DMR-EH55 resolves");
    let chip = resolved.flash_chip.as_ref().expect("EH55 flash chip");
    let layout = resolved.nor_layout.as_ref().expect("EH55 NOR layout");
    let sectors = derive_sector_table(chip).expect("sector table derives");

    let summary = json!({
        "model_id": resolved.manifest.model_id,
        "soc_id": resolved.soc.as_ref().map(|s| s.id.clone()),
        "flash_id": chip.id,
        "total_bytes": chip.total_bytes,
        "sector_count": sectors.len(),
        "ipl_region": region_span_hex(layout.find_region(NorRegionKind::Ipl)),
        "bgo_region": region_span_hex(layout.find_region(NorRegionKind::Bgo)),
        "nvram_region": region_span_hex(layout.find_region(NorRegionKind::Eemul)),
        "capabilities": resolved.effective_capabilities,
    });
    insta::assert_json_snapshot!("eh55_resolved_profile", summary);
}
