#![deny(unsafe_code)]

//! Unified 1-command BGO live heal planner and guarded executor (`bgo-live-heal`).
//!
//! Combines deterministic tag-hole & CoW FTL repair planning with SSOT-bounded
//! (`<= P7501_MAX_FAST_WRITE_BYTES_SSOT`) Pico CLI fast-command emission, CoW backup,
//! pre/post SHA-256 verification, and mandatory RAII `U LOCK` relock.

use std::path::Path;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use super::super::repair_plan::{
    BgoRepairOp, BgoRepairPlan, build_bgo_repair_plan, validate_bgo_op_bounds,
};
use super::super::{BgoSectorGeometry, eh55_bgo_sector_offset, resolve_nfd_file_page_pid};
use super::{BgoLiveApplyProgress, BgoLiveApplyReport, apply_bgo_repair_plan_live};
use crate::constants::AM34_FLASH_UNCACHED_BASE;
use crate::pico::panamon1::{Panamon1Client, Panamon1Transport};
use crate::re::region_compat::TvStandardKind;
use crate::ssot::pico_header::P7501_MAX_FAST_WRITE_BYTES_SSOT;

/// `[nfrom]` `config1` (`NFD_Read(0x0B)`) parameter ID `0x0117` (`MAIN 3410PQ` table `0x57AED0A8`
/// entry `0x53185E34 - 0x53184B44 = 0x12F0`, i.e. page `9` + `0x00F0`): `0x00` = `NTSC`, `0x01` = `PAL`.
const NFROM_CONFIG1_TV_SYS_PAGE_INDEX: usize = 9;
const NFROM_CONFIG1_TV_SYS_PAGE_OFF_HEX: &str = "0x00F0";

/// Request options controlling 1-command automated BGO live heal execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BgoLiveHealRequest {
    pub confirm_upper_nor: bool,
    pub tv_standard: Option<TvStandardKind>,
    pub dry_run: bool,
}

/// Metadata describing the `NTSC` (`0x00`) / `PAL` (`0x01`) `[nfrom]` `config1 + 0x12F0` (`page 9 + 0x00F0`, `ID 0x0117`) selector byte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BgoTvStandardNote {
    pub tv_standard: String,
    pub nfd_config1_pid_hex: String,
    pub page_offset_hex: String,
    pub value_byte_hex: String,
}

/// Deterministic Pico P7501 CLI command plan synthesized from a verified [`BgoRepairPlan`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BgoLiveHealCommandPlan {
    pub max_fast_write_bytes: usize,
    pub target_sectors: Vec<usize>,
    pub commands: Vec<String>,
    pub tv_standard_note: Option<BgoTvStandardNote>,
}

/// Full report emitted by [`heal_bgo_live_with_client`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BgoLiveHealReport {
    pub dry_run: bool,
    pub source_bgo_sha256: String,
    pub expected_post_sha256: String,
    pub erased_sas: Vec<usize>,
    pub total_write_bytes: usize,
    pub post_healthy: bool,
    pub command_plan: BgoLiveHealCommandPlan,
    pub live_apply: Option<BgoLiveApplyReport>,
}

fn build_tv_standard_note(
    std_kind: TvStandardKind,
    resolved_pid: Option<u16>,
) -> Result<BgoTvStandardNote> {
    let val_byte = match std_kind {
        TvStandardKind::Ntsc => 0x00u8,
        TvStandardKind::Pal => 0x01u8,
        other => {
            bail!(
                "[nfrom] ID=0x0117 config1+0x12F0 TV-system toggle supports 'ntsc' (0x00) or 'pal' (0x01), got '{}'",
                other.as_str()
            );
        }
    };
    let nfd_config1_pid_hex = resolved_pid.map_or_else(
        || format!("DYNAMIC(config1[page={NFROM_CONFIG1_TV_SYS_PAGE_INDEX}])"),
        |pid| format!("0x{pid:04X}"),
    );
    Ok(BgoTvStandardNote {
        tv_standard: std_kind.as_str().to_string(),
        nfd_config1_pid_hex,
        page_offset_hex: NFROM_CONFIG1_TV_SYS_PAGE_OFF_HEX.to_string(),
        value_byte_hex: format!("0x{val_byte:02X}"),
    })
}

/// Translates a CoW-verified [`BgoRepairPlan`] into Pico CLI commands chunked at `<= 128` B.
pub fn plan_live_bgo_heal_commands(
    plan: &BgoRepairPlan,
    tv_standard: Option<TvStandardKind>,
) -> Result<BgoLiveHealCommandPlan> {
    let max_chunk = P7501_MAX_FAST_WRITE_BYTES_SSOT as usize;
    let mut commands = Vec::new();
    commands.push("BGO_INSPECT DETAIL".to_string());

    for &sa in &plan.target_sectors {
        let sector_off = eh55_bgo_sector_offset(sa)?;
        let sector_va = AM34_FLASH_UNCACHED_BASE + sector_off as u32;
        commands.push(format!("FAST_READ {sector_va:08X} 00010000"));
    }

    for op in &plan.ops {
        match op {
            BgoRepairOp::EraseSector { phys_va, .. } => {
                let _ = validate_bgo_op_bounds(*phys_va, 0x1_0000)?;
                commands.push(format!("FAST_ERASE {phys_va:08X} CONFIRM_UPPER_NOR"));
            }
            BgoRepairOp::WriteBytes { phys_va, data, .. } => {
                if data.is_empty() {
                    bail!("FAST_WRITE at 0x{phys_va:08X} requires non-empty payload");
                }
                // Pad odd start/end boundaries to Spansion 16-bit halfwords using 0xFF (1->0 no-op mask).
                let aligned_va = phys_va & !1u32;
                let mut aligned_data = Vec::with_capacity(data.len() + 2);
                if (phys_va & 1) != 0 {
                    aligned_data.push(0xFF);
                }
                aligned_data.extend_from_slice(data);
                if (aligned_data.len() & 1) != 0 {
                    aligned_data.push(0xFF);
                }
                let _ = validate_bgo_op_bounds(aligned_va, aligned_data.len())?;
                for (idx, chunk) in aligned_data.chunks(max_chunk).enumerate() {
                    let chunk_va = aligned_va + (idx * max_chunk) as u32;
                    let hex_payload = hex::encode_upper(chunk);
                    commands.push(format!(
                        "FAST_WRITE {chunk_va:08X} {hex_payload} CONFIRM_UPPER_NOR"
                    ));
                }
            }
        }
    }

    commands.push("BGO_INSPECT DETAIL".to_string());
    let tv_standard_note = tv_standard
        .map(|std_kind| build_tv_standard_note(std_kind, None))
        .transpose()?;

    Ok(BgoLiveHealCommandPlan {
        max_fast_write_bytes: max_chunk,
        target_sectors: plan.target_sectors.clone(),
        commands,
        tv_standard_note,
    })
}

/// Plans (and when `dry_run == false`, executes via [`apply_bgo_repair_plan_live`]) a full BGO heal.
pub fn heal_bgo_live_with_client<T: Panamon1Transport>(
    client: Option<&mut Panamon1Client<T>>,
    nor_baseline: &[u8],
    backup_dir: &Path,
    req: &BgoLiveHealRequest,
    progress: &mut dyn FnMut(BgoLiveApplyProgress),
) -> Result<BgoLiveHealReport> {
    let plan = build_bgo_repair_plan(nor_baseline, None)?;
    let mut command_plan = plan_live_bgo_heal_commands(&plan, req.tv_standard)?;
    let geom = BgoSectorGeometry::eh55_default();
    let resolved_cfg_pid = resolve_nfd_file_page_pid(
        nor_baseline,
        &geom,
        "config1",
        NFROM_CONFIG1_TV_SYS_PAGE_INDEX,
    )
    .ok()
    .or_else(|| {
        plan.repaired_nor_image.as_deref().and_then(|repaired| {
            resolve_nfd_file_page_pid(repaired, &geom, "config1", NFROM_CONFIG1_TV_SYS_PAGE_INDEX)
                .ok()
        })
    });
    command_plan.tv_standard_note = req
        .tv_standard
        .map(|std_kind| build_tv_standard_note(std_kind, resolved_cfg_pid))
        .transpose()?;

    let live_apply = if req.dry_run {
        None
    } else {
        if !req.confirm_upper_nor {
            bail!("Upper NOR mutation refused fail-closed: --confirm-upper-nor is required");
        }
        let connected = client.ok_or_else(|| {
            anyhow::anyhow!("Live serial client required when --dry-run is not set")
        })?;
        Some(apply_bgo_repair_plan_live(
            connected,
            &plan,
            backup_dir,
            req.confirm_upper_nor,
            Some(15_000),
            progress,
        )?)
    };

    Ok(BgoLiveHealReport {
        dry_run: req.dry_run,
        source_bgo_sha256: plan.source_bgo_sha256,
        expected_post_sha256: plan.expected_post_sha256,
        erased_sas: plan.erased_sas,
        total_write_bytes: plan.total_write_bytes,
        post_healthy: plan.post_audit.healthy,
        command_plan,
        live_apply,
    })
}
