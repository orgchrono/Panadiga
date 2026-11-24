use anyhow::{Result, bail};
use byteorder::{ByteOrder, LittleEndian};

use super::ssot_synth::build_main_bgo_retarget_spec_from_ssot;
use super::{
    BytePatchSite, MAIN_BGO_STATE_MISMATCH, MainBgoGeometryStatus, MainBgoRetargetSpec,
    OwnedMainBgoRetargetSpec, OwnedSlicePatchSite, U32RelocSpec, WordPatchSite,
};
use crate::re::ssot_patches::{find_main_build_by_size, verify_main_native_bgo_from_spec};

fn count_u32_le(slice: &[u8], needle: u32) -> usize {
    let target = needle.to_le_bytes();
    let mut hits = 0usize;
    let mut idx = 0usize;
    while idx + 4 <= slice.len() {
        if slice[idx..idx + 4] == target {
            hits += 1;
            idx += 4;
        } else {
            idx += 1;
        }
    }
    hits
}

fn replace_u32_le(slice: &mut [u8], from: u32, to: u32) -> usize {
    let src = from.to_le_bytes();
    let dst = to.to_le_bytes();
    let mut hits = 0usize;
    let mut idx = 0usize;
    while idx + 4 <= slice.len() {
        if slice[idx..idx + 4] == src {
            slice[idx..idx + 4].copy_from_slice(&dst);
            hits += 1;
            idx += 4;
        } else {
            idx += 1;
        }
    }
    hits
}

fn pick_slice(s: &OwnedSlicePatchSite, target: bool) -> &[u8] {
    if target {
        s.replace.as_slice()
    } else {
        s.expect.as_slice()
    }
}

fn matches_retarget_state(buf: &[u8], spec: &OwnedMainBgoRetargetSpec, target: bool) -> bool {
    let scan = &buf[spec.reloc_scan_start..spec.reloc_scan_end];
    let pick_u32 = |s: &WordPatchSite| if target { s.replace } else { s.expect };
    let pick_u8 = |s: &BytePatchSite| if target { s.replace } else { s.expect };
    let reloc_matches = |r: &U32RelocSpec| {
        let (active, absent) = if target {
            (r.to, r.from)
        } else {
            (r.from, r.to)
        };
        count_u32_le(scan, active) == r.expected_count && count_u32_le(scan, absent) == 0
    };

    spec.word_sites
        .iter()
        .all(|s| LittleEndian::read_u32(&buf[s.offset..s.offset + 4]) == pick_u32(s))
        && spec.byte_sites.iter().all(|s| buf[s.offset] == pick_u8(s))
        && spec.slice_sites.iter().all(|s| {
            &buf[s.offset..s.offset + pick_slice(s, target).len()] == pick_slice(s, target)
        })
        && spec.ram_relocs.iter().all(reloc_matches)
}

fn classify_owned_sites(buf: &[u8], spec: &OwnedMainBgoRetargetSpec) -> (bool, bool) {
    (
        matches_retarget_state(buf, spec, false),
        matches_retarget_state(buf, spec, true),
    )
}

fn inspect_owned_bgo_geometry(
    buf: &[u8],
    spec: &OwnedMainBgoRetargetSpec,
) -> Result<MainBgoGeometryStatus> {
    if buf.len() != spec.expected_size {
        bail!(
            "MAIN BGO retarget '{}' requires {} B image, got {} B",
            spec.target_geometry_id,
            spec.expected_size,
            buf.len()
        );
    }
    let (is_factory, is_target) = classify_owned_sites(buf, spec);
    let (state, geom_id) = match (is_factory, is_target) {
        (true, false) => (
            spec.factory_geometry_id.clone(),
            Some(spec.factory_geometry_id.clone()),
        ),
        (false, true) => (
            spec.target_geometry_id.clone(),
            Some(spec.target_geometry_id.clone()),
        ),
        _ => (MAIN_BGO_STATE_MISMATCH.to_string(), None),
    };
    let table_va = spec.base_va + spec.table_detect_offset as u32;
    let flash_base =
        LittleEndian::read_u32(&buf[spec.table_detect_offset..spec.table_detect_offset + 4]);
    let bss_off = spec.word_sites[0].offset;
    let nfd_bss_end = LittleEndian::read_u32(&buf[bss_off..bss_off + 4]);
    Ok(MainBgoGeometryStatus {
        state,
        geometry_id: geom_id,
        table_va_hex: format!("0x{table_va:08X}"),
        bgo_flash_base_hex: format!("0x{flash_base:08X}"),
        nfd_bss_end_hex: format!("0x{nfd_bss_end:08X}"),
    })
}

fn apply_owned_bgo_retarget_in_place(
    buf: &mut [u8],
    spec: &OwnedMainBgoRetargetSpec,
    enable: bool,
) -> Result<()> {
    let pre = inspect_owned_bgo_geometry(buf, spec)?;
    if enable && pre.state == spec.target_geometry_id {
        return Ok(());
    }
    if !enable && pre.state == spec.factory_geometry_id {
        return Ok(());
    }
    if enable && pre.state != spec.factory_geometry_id {
        bail!(
            "Refusing to apply MAIN BGO retarget '{}': image in state '{}' (flash={}, bss_end={})",
            spec.target_geometry_id,
            pre.state,
            pre.bgo_flash_base_hex,
            pre.nfd_bss_end_hex
        );
    }
    if !enable && pre.state != spec.target_geometry_id {
        bail!(
            "Refusing to revert MAIN BGO retarget '{}': image in state '{}'",
            spec.target_geometry_id,
            pre.state
        );
    }

    for site in &spec.word_sites {
        let val = if enable { site.replace } else { site.expect };
        LittleEndian::write_u32(&mut buf[site.offset..site.offset + 4], val);
    }
    for site in &spec.byte_sites {
        buf[site.offset] = if enable { site.replace } else { site.expect };
    }
    for site in &spec.slice_sites {
        let slice = if enable {
            site.replace.as_slice()
        } else {
            site.expect.as_slice()
        };
        buf[site.offset..site.offset + slice.len()].copy_from_slice(slice);
    }
    let scan = &mut buf[spec.reloc_scan_start..spec.reloc_scan_end];
    for reloc in &spec.ram_relocs {
        let (src, dst) = if enable {
            (reloc.from, reloc.to)
        } else {
            (reloc.to, reloc.from)
        };
        let replaced = replace_u32_le(scan, src, dst);
        if replaced != reloc.expected_count {
            bail!(
                "MAIN BGO RAM relocation 0x{src:08X}->0x{dst:08X} count mismatch: {replaced} != {}",
                reloc.expected_count
            );
        }
    }

    let post = inspect_owned_bgo_geometry(buf, spec)?;
    let expected = if enable {
        spec.target_geometry_id.as_str()
    } else {
        spec.factory_geometry_id.as_str()
    };
    if post.state != expected {
        bail!(
            "MAIN BGO post-retarget verification failed: expected '{expected}', got '{}'",
            post.state
        );
    }
    Ok(())
}

/// Inspects an unpacked `MAIN` binary against a declarative [`MainBgoRetargetSpec`] (`fail-closed`).
pub fn inspect_main_bgo_geometry_for_spec(
    buf: &[u8],
    spec: &MainBgoRetargetSpec,
) -> Result<MainBgoGeometryStatus> {
    inspect_owned_bgo_geometry(buf, &spec.to_owned_spec())
}

/// Applies or reverts a [`MainBgoRetargetSpec`] idempotently and fail-closed in place.
pub fn apply_main_bgo_retarget_for_spec_in_place(
    buf: &mut [u8],
    spec: &MainBgoRetargetSpec,
    enable: bool,
) -> Result<()> {
    apply_owned_bgo_retarget_in_place(buf, &spec.to_owned_spec(), enable)
}

fn resolve_owned_bgo_retarget_spec(len: usize) -> Result<OwnedMainBgoRetargetSpec> {
    let (_, build) = find_main_build_by_size(len)
        .ok_or_else(|| anyhow::anyhow!("Unsupported MAIN size {len} B for BGO retarget"))?;
    build_main_bgo_retarget_spec_from_ssot(build)
}

/// Inspects or natively verifies any SSOT-registered `MAIN` build's BGO/NFROM NOR geometry (`fail-closed`).
pub fn inspect_or_verify_main_bgo_geometry(buf: &[u8]) -> Result<MainBgoGeometryStatus> {
    let (_, build) = find_main_build_by_size(buf.len()).ok_or_else(|| {
        anyhow::anyhow!(
            "Unsupported MAIN size {} B for BGO geometry inspection",
            buf.len()
        )
    })?;
    if build.supports_bgo_retarget_to.is_empty()
        && build.bgo_retarget_anchors.is_none()
        && build.native_bgo_detect_va.is_some()
    {
        return verify_main_native_bgo_from_spec(buf, build);
    }
    let owned = build_main_bgo_retarget_spec_from_ssot(build)?;
    inspect_owned_bgo_geometry(buf, &owned)
}

/// Inspects any SSOT-registered `MAIN` BGO/NFROM NOR geometry (`4 MiB Top-Boot` vs `8 MiB Uniform`).
pub fn inspect_main_bgo_geometry(buf: &[u8]) -> Result<MainBgoGeometryStatus> {
    let owned = resolve_owned_bgo_retarget_spec(buf.len())?;
    inspect_owned_bgo_geometry(buf, &owned)
}

/// Applies or reverts the `8 MiB` (`BGO_45SEC_8MB_0X84130000`) retarget dynamically via SSOT [`find_main_build_by_size`].
pub fn apply_main_bgo_retarget_in_place(buf: &mut [u8], enable: bool) -> Result<()> {
    let owned = resolve_owned_bgo_retarget_spec(buf.len())?;
    apply_owned_bgo_retarget_in_place(buf, &owned, enable)
}
