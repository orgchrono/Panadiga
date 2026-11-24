#![deny(unsafe_code)]

//! Clean-room Diskless / Standalone TBC Appliance Planner (`DigadiagRamTbc` &
//! gated `ExperimentalSdRamChainload`) for Panasonic `DMR-EH55` (`VEP79138` + Main PCB).
//!
//! Strictly volatile SDRAM / MMIO planning: `IC5001` bootloader sectors `SA0..SA3` remain
//! untouched (`nor_sa0_to_sa3_untouched = true`) and lethal AM34 register `0xC000_100C`
//! is never targeted.

use std::str::FromStr;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::constants::{
    AM34_FLASH_BASE, AM34_FLASH_CRITICAL_IPL_SIZE, AM34_FORBIDDEN_LETHAL_REG,
    AM34_IC7501_TBUS_BASE, AM34_IC7501_TBUS_CHECKSUM_OFF, AM34_IIC0_BASE, AM34_IIC1_BASE,
    AM34_MAIN_BASE_VA, AM34_PROG061_BASE_VA, AM34_SC0_BASE, AM34_SDCD_MMIO_BASE,
    AM34_SYS_STATUS_REG, AM34_WDBC_REG, IC7501_TBUS_CHECKSUM_BIAS, IC7501_WDT_KEEPALIVE_CMD,
    PANA_DIGADIAG_RAW_VER061_SIZE,
};

use super::main_passthrough::{
    MAIN_BASE_VA, OFF_FPC_STATUS_CTRL, OFF_ODD_INIT_STATUS, OFF_ODD_STATE_BRANCH,
    ORIG_FPC_STATUS_CTRL, ORIG_ODD_INIT_STATUS, ORIG_ODD_STATE_BRANCH,
};
use super::region_compat::{MmioWriteOp, TvStandardKind, build_tv_calibration_mmio_plan};

/// Supported diskless TBC / passthrough execution architectures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TbcApplianceMode {
    /// Verified safe bare-metal `DIGADIAG.FRM` (`262,144 B` @ `0x5040_0000`) TBC mode.
    DigadiagRamTbc,
    /// Hardware-proven Door #1 (`EV-20261119-004`): headless `PANA__SD.FRM` `sd48` `PROG061` slot (`0x0B1800`) without `SW_UART` JIG.
    NoJigPanaSdProgSlot,
    /// Experimental RAM-only chainload (`PANA__SD.FRM` / `MAIN` -> `0x5700_0000`) bypassing `CH1` HDD.
    ExperimentalSdRamChainload,
}

impl TbcApplianceMode {
    /// Canonical lowercase mode slug.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DigadiagRamTbc => "digadiag_ram_tbc",
            Self::NoJigPanaSdProgSlot => "no_jig_pana_sd_prog_slot",
            Self::ExperimentalSdRamChainload => "experimental_sd_ram_chainload",
        }
    }
}

impl FromStr for TbcApplianceMode {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "stage15" | "stage15_digadiag_tbc" | "digadiag_ram_tbc" | "digadiag" | "tbc" => {
                Ok(Self::DigadiagRamTbc)
            }
            "no_jig" | "porta1" | "prog_slot" | "no_jig_pana_sd_prog_slot" => {
                Ok(Self::NoJigPanaSdProgSlot)
            }
            "experimental_sd_ram_chainload" | "ram_chainload" | "chainload" => {
                Ok(Self::ExperimentalSdRamChainload)
            }
            other => bail!(
                "Unsupported TBC appliance mode '{other}': expected digadiag_ram_tbc, no_jig_pana_sd_prog_slot, or experimental_sd_ram_chainload"
            ),
        }
    }
}

/// Reference to a verified `MAIN` (`3410PQ`) AV TBC passthrough patch site from `re::main_passthrough`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifiedMainPassthroughSiteRef {
    pub name: String,
    pub va_hex: String,
    pub file_offset_hex: String,
    pub length_bytes: usize,
    pub note: String,
}

/// Complete declarative execution plan and hardware safety invariants for Standalone TBC operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TbcAppliancePlan {
    pub mode: TbcApplianceMode,
    pub tv_standard: TvStandardKind,
    pub requires_experimental_flag: bool,
    pub warnings: Vec<String>,
    pub steps: Vec<String>,
    pub mmio_ops: Vec<MmioWriteOp>,
    pub nor_sa0_to_sa3_untouched: bool,
    pub verified_main_passthrough_sites: Vec<VerifiedMainPassthroughSiteRef>,
}

fn verified_passthrough_site_refs() -> Vec<VerifiedMainPassthroughSiteRef> {
    [
        (
            "fpc_status_ctrl",
            OFF_FPC_STATUS_CTRL,
            ORIG_FPC_STATUS_CTRL.len(),
            "Bypass front-panel status poll in MAIN 3410PQ (does not bypass Loader ver0.61 CH1)",
        ),
        (
            "odd_state_branch",
            OFF_ODD_STATE_BRANCH,
            ORIG_ODD_STATE_BRANCH.len(),
            "Skip ODD state branch in MAIN 3410PQ after MAIN has been loaded into SDRAM",
        ),
        (
            "odd_init_status",
            OFF_ODD_INIT_STATUS,
            ORIG_ODD_INIT_STATUS.len(),
            "Return success (D0=0) from ODD init in MAIN 3410PQ",
        ),
    ]
    .into_iter()
    .map(|(name, off, len, note)| VerifiedMainPassthroughSiteRef {
        name: name.to_string(),
        va_hex: format!("0x{:08X}", MAIN_BASE_VA + off as u32),
        file_offset_hex: format!("0x{off:06X}"),
        length_bytes: len,
        note: note.to_string(),
    })
    .collect()
}

fn validate_mmio_plan_safety(mmio_ops: &[MmioWriteOp]) -> Result<()> {
    let cached_ipl_end = AM34_FLASH_BASE + AM34_FLASH_CRITICAL_IPL_SIZE;
    let uncached_ipl_end =
        crate::constants::AM34_FLASH_UNCACHED_BASE + AM34_FLASH_CRITICAL_IPL_SIZE;
    for op in mmio_ops {
        if op.addr == AM34_FORBIDDEN_LETHAL_REG {
            bail!(
                "Safety invariant violation: MMIO plan targets lethal AM34 register 0x{:08X}",
                AM34_FORBIDDEN_LETHAL_REG
            );
        }
        if (AM34_FLASH_BASE..cached_ipl_end).contains(&op.addr)
            || (crate::constants::AM34_FLASH_UNCACHED_BASE..uncached_ipl_end).contains(&op.addr)
        {
            bail!(
                "Safety invariant violation: MMIO plan writes into read-only NOR IPL 0x{:08X} (E05)",
                op.addr
            );
        }
    }
    Ok(())
}

/// Builds a declarative standalone TBC appliance plan with hardware watchdog and NOR safety invariants.
pub fn build_tbc_appliance_plan(
    mode: TbcApplianceMode,
    standard: TvStandardKind,
    experimental_ram_chainload: bool,
) -> Result<TbcAppliancePlan> {
    if mode == TbcApplianceMode::ExperimentalSdRamChainload && !experimental_ram_chainload {
        bail!(
            "ExperimentalSdRamChainload is gated: pass experimental_ram_chainload = true (--experimental-ram-chainload) to acknowledge IC3001/IC7501 TBUS WDT prerequisites."
        );
    }

    let mmio_ops = build_tv_calibration_mmio_plan(standard);
    validate_mmio_plan_safety(&mmio_ops)?;

    let mut warnings = vec![
        format!(
            "Main PCB + IC3001 + IC7501 TBUS WDT required: Digital PCB VEP79138 cannot drive analog video clamps/75-ohm outputs alone and IC7501 TBUS @ 0x{:08X} cuts power (U99 / WDT reset after ~2s/255s) unless serviced.",
            AM34_IC7501_TBUS_BASE
        ),
        format!(
            "Never read or probe lethal AM34 register 0x{:08X}; feed WDBC @ 0x{:08X} and monitor SYS_STATUS @ 0x{:08X}.",
            AM34_FORBIDDEN_LETHAL_REG, AM34_WDBC_REG, AM34_SYS_STATUS_REG
        ),
        format!(
            "P7501 safety: keep NSW5V (GP3) Hi-Z input without pulls, XRESET (GP5) open-drain, and release SW_UART (GP26) to Hi-Z after serial boot entry so Q7514 unblocks SC0 UART0 RX @ 0x{:08X}.",
            AM34_SC0_BASE
        ),
        "IC5001 Spansion NOR IPL SA0..SA3 remains strictly untouched (E05 read-only).".to_string(),
    ];

    let common_tail = [
        format!(
            "3. Initialize IC7501 TBUS keepalive mailbox @ 0x{:08X} (0x80 -> 0x81 -> 0x{:02X}, checksum +0x{:02X} bias 0x{:02X}) and service/disarm WDBC @ 0x{:08X}.",
            AM34_IC7501_TBUS_BASE,
            IC7501_WDT_KEEPALIVE_CMD,
            AM34_IC7501_TBUS_CHECKSUM_OFF,
            IC7501_TBUS_CHECKSUM_BIAS,
            AM34_WDBC_REG
        ),
        format!(
            "4. Configure IC3001 analog matrix/clamps over IIC0 @ 0x{:08X} / IIC1 @ 0x{:08X} (respecting IC7501 I2C arbitration).",
            AM34_IIC0_BASE, AM34_IIC1_BASE
        ),
        format!(
            "5. Apply clean-room AIOF/AVDD register plan ({} ops) for TV standard '{}' without touching IC5001.",
            mmio_ops.len(),
            standard.as_str()
        ),
    ];

    let steps = match mode {
        TbcApplianceMode::DigadiagRamTbc => {
            let mut s = vec![
                format!(
                    "1. Strap P7501 SW_UART=LOW (JIG:2500 / PREJIG) on <=2 GiB SDSC FAT16 so Loader ver0.61 loads {} B raw DIGADIAG.FRM to 0x{:08X} (uncached 0x90400000) without touching CH1 HDD or BGO.",
                    PANA_DIGADIAG_RAW_VER061_SIZE, AM34_PROG061_BASE_VA
                ),
                format!(
                    "2. Release SW_UART to Hi-Z after 'serial boot mode' banner and keep SC0 UART0 @ 0x{:08X} (115200 8N1) active for headless TBC commands.",
                    AM34_SC0_BASE
                ),
            ];
            s.extend(common_tail);
            s
        }
        TbcApplianceMode::NoJigPanaSdProgSlot => {
            let mut s = vec![
                "1. Boot headless without CH1 HDD (HDD_MAX_LBA get failed[0xE8000000]) and without pulling P7501 SW_UART low (Door #1 / Porta #1, EV-20261119-004).".to_string(),
                format!(
                    "2. Place <= {} B clean-room TBC payload in PROG061 slot (0x0B1800) of dual-family sd48 PANA__SD.FRM (M6930LB+M6930, [0x18]='M'); Loader ver0.61 checks 32-bit FVU sum (no RSA), loads to 0x90400000, and jumps to 0x{:08X}.",
                    PANA_DIGADIAG_RAW_VER061_SIZE, AM34_PROG061_BASE_VA
                ),
            ];
            s.extend(common_tail);
            s
        }
        TbcApplianceMode::ExperimentalSdRamChainload => {
            warnings.push(
                "EXPERIMENTAL RAM CHAINLOAD: main_passthrough sites (0x5771ABFB, 0x574F7B8C, 0x574F7C5C) only execute AFTER MAIN is loaded to SDRAM and do not fix Loader ver0.61 CH1 HDD hangs unless SD DIGADIAG.FRM performs the RAM chainload directly."
                    .to_string(),
            );
            vec![
                format!(
                    "1. Boot DIGADIAG monitor/loader via DIGADIAG.FRM @ 0x{:08X} (volatile SDRAM; SA0..SA3 never flashed).",
                    AM34_PROG061_BASE_VA
                ),
                format!(
                    "2. Maintain IC7501 TBUS @ 0x{:08X} + WDBC @ 0x{:08X} keepalive while loading MAIN EXTRHEAD via SDCD @ 0x{:08X} or PANAMON1> into SDRAM @ 0x{:08X}.",
                    AM34_IC7501_TBUS_BASE, AM34_WDBC_REG, AM34_SDCD_MMIO_BASE, AM34_MAIN_BASE_VA
                ),
                "3. Patch MAIN 3410PQ AV passthrough sites + dynamic HDD capacity clamp strictly inside volatile SDRAM prior to branching.".to_string(),
                format!(
                    "4. Program IC3001 via IIC0/IIC1 (0x{:08X}/0x{:08X}) and apply '{}' AIOF/AVDD MMIO calibration ops before jumping to MAIN.",
                    AM34_IIC0_BASE, AM34_IIC1_BASE, standard.as_str()
                ),
            ]
        }
    };

    Ok(TbcAppliancePlan {
        mode,
        tv_standard: standard,
        requires_experimental_flag: mode == TbcApplianceMode::ExperimentalSdRamChainload,
        warnings,
        steps,
        mmio_ops,
        nor_sa0_to_sa3_untouched: true,
        verified_main_passthrough_sites: verified_passthrough_site_refs(),
    })
}

pub mod keepalive;

pub use keepalive::{
    build_ic7501_vfd_text_frame, build_ic7501_wdt_255s_frame, compute_ic7501_tbus_checksum,
    emit_m0_tbus_wdt_keepalive_stub,
};

#[cfg(test)]
mod tests;
