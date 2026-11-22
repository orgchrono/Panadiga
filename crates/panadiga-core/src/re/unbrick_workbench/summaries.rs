#![deny(unsafe_code)]

//! Summary builders (`BGO NOR`, `DEV.BIN` / `MAIN` region calibration) and unit tests
//! for the Pre-MAIN DIGA Unbrick Workbench.

use std::fs;
use std::path::Path;

use super::super::eh55_bgo::audit_eh55_bgo_nor_8mb;
use super::super::region_compat::inspect_dev_bin_bytes;
use super::catalog::{WorkbenchBgoSummary, WorkbenchRegionSummary};

pub(super) const RECOMMENDED_RECOVERY_PROFILE: &str = "PROG061_UART_MONITOR";

pub(super) fn infer_tv_standard_slug(
    model: Option<&str>,
    build_profile: Option<&str>,
) -> &'static str {
    let m = model.unwrap_or("").to_ascii_uppercase();
    for region_tok in [
        "LBS", "LB", "EE", "GC", "GCS", "EC", "EB", "EG", "EF", "EP", "EN",
    ] {
        if m.contains(region_tok) {
            let interpolated =
                crate::ssot::registry::interpolate_region_variant(region_tok, &m, "", Some(&m));
            return match interpolated.tv_standard.as_str() {
                "PAL-M" => "pal_m",
                "PAL/SECAM" => "secam",
                "PAL" | "PAL/NTSC" => "pal",
                _ => "ntsc",
            };
        }
    }
    if build_profile == Some("3060NP") {
        "pal"
    } else {
        "ntsc"
    }
}

pub(super) fn inspect_workbench_bgo(nor_8mb: Option<&Path>) -> Option<WorkbenchBgoSummary> {
    let path = nor_8mb?;
    let bytes = fs::read(path).ok()?;
    let audit = audit_eh55_bgo_nor_8mb(&bytes).ok()?;
    Some(WorkbenchBgoSummary {
        nor_dump_path: path.to_string_lossy().into_owned(),
        healthy: audit.healthy,
        total_sectors: audit.total_sectors,
        p1_active_count: audit.p1_active_count,
        p2_present_count: audit.p2_present_ids.len(),
        p2_missing_count: audit.p2_missing_ids.len(),
        spare_count: audit.spare_sas.len(),
        corrupt_count: audit.corrupt_sas.len(),
        nfrom_file_count: audit.nfrom_file_count,
        ifodev2_model: audit.ifodev2_model,
    })
}

pub(super) fn inspect_workbench_region(
    dev_bin_path: Option<&Path>,
    main_region: Option<(String, String)>,
    bgo_summary: Option<&WorkbenchBgoSummary>,
) -> Option<WorkbenchRegionSummary> {
    let dev_rep = dev_bin_path
        .and_then(|p| fs::read(p).ok())
        .and_then(|b| inspect_dev_bin_bytes(&b).ok());
    let bgo_model = bgo_summary.and_then(|b| b.ifodev2_model.clone());
    if dev_rep.is_none() && main_region.is_none() && bgo_model.is_none() {
        return None;
    }
    let dev_bin_model = dev_rep.as_ref().map(|r| r.model.clone()).or(bgo_model);
    let dev_bin_oem = dev_rep.as_ref().map(|r| r.oem.clone());
    let (main_build_profile, main_devinfo_state) = match main_region {
        Some((profile, state)) => (Some(profile), Some(state)),
        None => (None, None),
    };
    let recommended_tv_standard =
        infer_tv_standard_slug(dev_bin_model.as_deref(), main_build_profile.as_deref()).to_string();
    Some(WorkbenchRegionSummary {
        dev_bin_path: dev_bin_path.map(|p| p.to_string_lossy().into_owned()),
        dev_bin_model,
        dev_bin_oem,
        main_build_profile,
        main_devinfo_state,
        recommended_tv_standard,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{PANA_DEV_BIN_MAGIC, PANA_DEV_BIN_SIG};
    use crate::crypto::codec::write_padded_ascii;
    use crate::re::unbrick_workbench::inspect_unbrick_workspace;

    #[test]
    fn empty_workspace_returns_none_summaries_with_recommended_profile() {
        let dir = std::env::temp_dir().join(format!("panadiga_wb_empty_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let rep = inspect_unbrick_workspace(Some(&dir));
        assert_eq!(
            rep.recommended_recovery_profile,
            RECOMMENDED_RECOVERY_PROFILE
        );
        assert!(rep.region_compat_summary.is_none());
        assert!(rep.bgo_audit_summary.is_none());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn synthetic_workspace_with_dev_bin_populates_region_compat_summary() {
        let dir = std::env::temp_dir().join(format!("panadiga_wb_reg_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let fw_dir = dir.join("firmware_and_images");
        fs::create_dir_all(&fw_dir).unwrap();

        let mut dev = vec![0xFFu8; 512];
        dev[0..4].copy_from_slice(&PANA_DEV_BIN_MAGIC);
        write_padded_ascii(&mut dev[0x04..0x0C], "M6930LB", 0);
        dev[0x0C] = b'M';
        dev[0x20..0x2C].copy_from_slice(PANA_DEV_BIN_SIG);
        fs::write(fw_dir.join("DEV.BIN"), &dev).unwrap();

        let rep = inspect_unbrick_workspace(Some(&dir));
        assert_eq!(
            rep.recommended_recovery_profile,
            RECOMMENDED_RECOVERY_PROFILE
        );
        let reg = rep.region_compat_summary.expect("DEV.BIN summary expected");
        assert_eq!(reg.dev_bin_model.as_deref(), Some("M6930LB"));
        assert_eq!(reg.dev_bin_oem.as_deref(), Some("M"));
        assert_eq!(reg.recommended_tv_standard, "pal_m");

        let _ = fs::remove_dir_all(&dir);
    }
}
