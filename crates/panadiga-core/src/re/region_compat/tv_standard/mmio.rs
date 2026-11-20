#![deny(unsafe_code)]

//! Clean-room declarative `AIOF`/`AVDD` MMIO sequence and profile builder for `AM34`.

use crate::constants::{
    AIOF_CONT_OFF, AIOF_FS0_OFF, AIOF_V_CLOCK_START, AIOF_V_OFF, AIOF_VMODESEL_OFF,
    AIOF_VMODESEL_PAL, AIOF_YC3_OFF, AM34_AIOF_BASE, AM34_AVDD_BASE,
};

use super::types::{LbsRegionPresetId, MmioWriteOp, TvCalibrationProfile, TvStandardKind};

pub(super) const AVDD_NPALCTL0_OFF: u32 = 0x0624;

fn mmio_op(
    addr: u32,
    width_bits: u8,
    value: u32,
    register_name: &str,
    note: impl Into<String>,
) -> MmioWriteOp {
    MmioWriteOp {
        addr,
        width_bits,
        value,
        register_name: register_name.to_string(),
        note: note.into(),
    }
}

/// Builds a clean-room declarative `AIOF`/`AVDD` MMIO calibration sequence for `standard`.
#[must_use]
pub fn build_tv_calibration_mmio_plan(standard: TvStandardKind) -> Vec<MmioWriteOp> {
    let vmodesel = standard.vmodesel_value().unwrap_or(AIOF_VMODESEL_PAL);
    let vmodesel_note = match standard {
        TvStandardKind::Ntsc => "AIOF cps_ntsc (525-line NTSC-M/J colorimetric mode)",
        TvStandardKind::PalM => "AIOF cps_pal_m (525-line Brazilian PAL-M mode for M6930LB/LBS)",
        TvStandardKind::Pal => "AIOF cps_pal (625-line PAL B/G/I mode)",
        TvStandardKind::Secam => {
            "SECAM 625-line base; verify SECAM0..1 chroma demodulation via live capture"
        }
    };
    let cont_val: u32 = match standard {
        TvStandardKind::Ntsc => 0x8000,
        TvStandardKind::PalM | TvStandardKind::Pal | TvStandardKind::Secam => 0x4000,
    };
    let npal_hint: u32 = match standard {
        TvStandardKind::Ntsc | TvStandardKind::PalM => 0,
        TvStandardKind::Pal | TvStandardKind::Secam => 1,
    };

    vec![
        mmio_op(
            AM34_AIOF_BASE + AIOF_V_OFF,
            16,
            u32::from(AIOF_V_CLOCK_START),
            "AIOF_V",
            "AIOF Preset [93] clock_start (0x8001) prior to VMODESEL",
        ),
        mmio_op(
            AM34_AIOF_BASE + AIOF_VMODESEL_OFF,
            16,
            u32::from(vmodesel),
            "AIOF_VMODESEL",
            vmodesel_note,
        ),
        mmio_op(
            AM34_AIOF_BASE + AIOF_FS0_OFF,
            16,
            0x40F0,
            "AIOF_FS0",
            "Standard Frame Sync (0x40F0); toggle bit 15 (0xC0F0) for non-standard 240p/LaserDisc",
        ),
        mmio_op(
            AM34_AIOF_BASE + AIOF_YC3_OFF,
            16,
            0x0020,
            "AIOF_YC3",
            "Enable MVAGC Line TBC (bit 5 = 0x0020) for horizontal VHS/analog jitter removal",
        ),
        mmio_op(
            AM34_AIOF_BASE + AIOF_CONT_OFF,
            16,
            cont_val,
            "AIOF_CONT",
            if standard == TvStandardKind::Ntsc {
                "NTSC black setup darker (7.5 IRE = 0x8000; set 0x4000 for 0 IRE NTSC-J)"
            } else {
                "Neutral 0 IRE black setup (0x4000)"
            },
        ),
        mmio_op(
            AM34_AVDD_BASE + AVDD_NPALCTL0_OFF,
            32,
            npal_hint,
            "AVDD_NPALCTL0",
            "Declarative 525/625-line AVDD back-end anchor (requires live M1 RMW verification before unattended write)",
        ),
    ]
}

/// Builds a declarative TV/region calibration profile from `preset` and optional standard override.
#[must_use]
pub fn build_tv_calibration_profile(
    preset: LbsRegionPresetId,
    standard_override: Option<TvStandardKind>,
) -> TvCalibrationProfile {
    let tv_standard = standard_override.unwrap_or_else(|| preset.default_tv_standard());
    TvCalibrationProfile {
        preset,
        tv_standard,
        chassis_family: preset.expected_chassis_family().to_string(),
        dvd_region_code: preset.dvd_region_code(),
        expected_oem_char: preset.expected_oem_char(),
        metadata_and_dev_oem_only: true,
        mmio_plan: build_tv_calibration_mmio_plan(tv_standard),
        notes: vec![
            "Unverified ifodev2 trailing region (0x400..0x7FF) is read-only metadata_and_dev_oem_only=true; only DEV.BIN/ifodev2 model (0x04..0x0C) and OEM (0x0C) are mutated by patch_dev_bin_bytes.".to_string(),
            "Clean-room MMIO ops target volatile SDRAM/hardware registers only and never copy proprietary Panasonic executable tables.".to_string(),
        ],
    }
}
