#![deny(unsafe_code)]

//! Extended CLI args for SD verification, TV/Region bundles, TBC Appliance, and BGO NOR FTL repair.

use clap::{Args, Subcommand};
use std::path::PathBuf;

/// Verifies an SD card root directory (`DIGADIAG.FRM`, `PANA__SD.FRM`, and `-91006` checks).
#[derive(Args, Debug)]
pub struct VerifySdArgs {
    /// SD card root or staging directory to verify (defaults to firmware_and_images/sd_unbrick_kit)
    #[arg(
        default_value = panadiga_core::constants::DEFAULT_SD_UNBRICK_KIT_DIR,
        value_name = "SD_DIR"
    )]
    pub dir: PathBuf,

    /// Audit strictness profile: standard (default) or hardened
    #[arg(long, default_value = "standard", value_name = "PROFILE")]
    pub audit_profile: String,

    /// Exports structured JSON verification report
    #[arg(short, long, value_name = "JSON_FILE")]
    pub json: Option<PathBuf>,
}

/// Plans a clean-room Diskless / Standalone TBC execution sequence (`DIGADIAG` or gated RAM chainload).
#[derive(Args, Debug)]
pub struct TbcPlanArgs {
    /// Execution mode: digadiag_ram_tbc (default) or experimental_sd_ram_chainload
    #[arg(long, default_value = "digadiag_ram_tbc", value_name = "MODE")]
    pub mode: String,

    /// Target analog TV standard: ntsc, pal-m, pal, or secam
    #[arg(long, default_value = "ntsc", value_name = "STANDARD")]
    pub standard: String,

    /// Explicit opt-in required for experimental_sd_ram_chainload mode
    #[arg(long)]
    pub experimental_ram_chainload: bool,

    /// Exports structured JSON plan (`-` emits stdout JSON)
    #[arg(short, long, value_name = "JSON_FILE")]
    pub json: Option<PathBuf>,
}

/// Audits the 45-sector DMR-EH55 BGO FTL (`SA26..=SA70`) inside an 8 MiB `IC5001` NOR dump.
#[derive(Args, Debug)]
pub struct BgoAuditArgs {
    /// Path to 8 MiB Spansion `S29JL064H` (`IC5001`) binary dump
    #[arg(long, value_name = "BIN")]
    pub nor: PathBuf,

    /// Exports structured JSON audit report (`-` emits stdout JSON)
    #[arg(short, long, value_name = "JSON_FILE")]
    pub json: Option<PathBuf>,
}

/// Builds and CoW-simulates a deterministic BGO FTL repair plan (`SA26..=SA70` only, E05-guarded).
#[derive(Args, Debug)]
pub struct BgoRepairPlanArgs {
    /// Path to corrupt 8 MiB `IC5001` NOR binary dump
    #[arg(long, value_name = "BIN")]
    pub nor: PathBuf,

    /// Output path for verified `BgoRepairPlan` JSON
    #[arg(long, value_name = "JSON")]
    pub out_plan: PathBuf,

    /// Optional Copy-on-Write output path for simulated repaired 8 MiB NOR binary
    #[arg(long, value_name = "BIN")]
    pub cow_out: Option<PathBuf>,

    /// Exports structured JSON plan summary (`-` emits stdout JSON)
    #[arg(short, long, value_name = "JSON_FILE")]
    pub json: Option<PathBuf>,
}

/// Applies a CoW-verified `BgoRepairPlan` live via Pico `PANAMON1>` with pre/post SHA-256 and RAII `U LOCK`.
#[derive(Args, Debug)]
pub struct BgoLiveApplyArgs {
    /// Pico serial port identifier (e.g. COM10 or /dev/ttyACM0)
    #[arg(short, long, value_name = "PORT")]
    pub port: String,

    /// Path to verified `BgoRepairPlan` JSON file
    #[arg(long, value_name = "JSON")]
    pub plan: PathBuf,

    /// Mandatory directory to store verified pre-repair CoW NOR backup
    #[arg(long, value_name = "DIR")]
    pub backup_dir: PathBuf,

    /// Explicit operator confirmation unlocking Upper NOR (`SA26..=SA70`; `SA0..=SA3` remain locked E05)
    #[arg(long)]
    pub confirm_upper_nor: bool,

    /// Skip post-repair `BOOTTRACE 15000 NOJIG` verification pass
    #[arg(long)]
    pub no_boottrace: bool,

    /// Exports structured JSON live apply report (`-` emits stdout JSON)
    #[arg(short, long, value_name = "JSON_FILE")]
    pub json: Option<PathBuf>,
}

/// 1-command BGO FTL + tag-hole heal over Pico `PANAMON1>` (supports `--dry-run` + optional TV selector).
#[derive(Args, Debug)]
pub struct BgoLiveHealArgs {
    /// Pico serial port identifier (e.g. COM10 or /dev/ttyACM0; required unless --dry-run is set)
    #[arg(short, long, value_name = "PORT")]
    pub port: Option<String>,

    /// Path to 8 MiB IC5001 NOR baseline dump used to derive the deterministic heal plan
    #[arg(long, value_name = "BIN")]
    pub nor: PathBuf,

    /// Mandatory directory to store verified pre-repair CoW NOR backup
    #[arg(long, value_name = "DIR")]
    pub backup_dir: PathBuf,

    /// Explicit operator confirmation unlocking Upper NOR (`SA26..=SA70`; `SA0..=SA10` remain locked E05)
    #[arg(long)]
    pub confirm_upper_nor: bool,

    /// Optional `[nfrom]` TV standard target metadata (`ntsc` or `pal`)
    #[arg(long, value_name = "STANDARD")]
    pub tv_standard: Option<String>,

    /// Plan commands offline without mutating target Flash hardware
    #[arg(long)]
    pub dry_run: bool,

    /// Exports structured JSON live heal report (`-` emits stdout JSON)
    #[arg(short, long, value_name = "JSON_FILE")]
    pub json: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum RegionCompatAction {
    /// Inspects MAIN (3410PQ / 3060NP) src/fmupre/pre_update_firm.c model & OEM comparison guard
    InspectMain {
        /// Unpacked MAIN binary path
        #[arg(short, long, value_name = "MAIN_BIN")]
        input: PathBuf,

        /// Exports structured JSON report (`-` emits stdout JSON)
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Patches or restores MAIN <deviceinfo> model & OEM guard (bypass|factory_strict)
    PatchMain {
        /// Unpacked MAIN input binary path
        #[arg(short, long, value_name = "MAIN_BIN")]
        input: PathBuf,

        /// Patched MAIN output path (Copy-on-Write)
        #[arg(short, long, value_name = "OUT_BIN")]
        output: PathBuf,

        /// Mode: bypass (allow cross-model/OEM) or factory_strict
        #[arg(long, default_value = "bypass", value_name = "MODE")]
        mode: String,

        /// Exports structured JSON report (`-` emits stdout JSON)
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Inspects a 512-byte DEV.BIN service container device descriptor
    InspectDev {
        /// Path to 512-byte DEV.BIN
        #[arg(short, long, value_name = "DEV_BIN")]
        input: PathBuf,

        /// Exports structured JSON report (`-` emits stdout JSON)
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Edits model (<=8 ASCII) and optional OEM character in a 512-byte DEV.BIN
    PatchDev {
        /// Source 512-byte DEV.BIN path
        #[arg(short, long, value_name = "DEV_BIN")]
        input: PathBuf,

        /// Destination 512-byte DEV.BIN path (Copy-on-Write)
        #[arg(short, long, value_name = "OUT_DEV")]
        output: PathBuf,

        /// Target model string (1..=8 ASCII chars, e.g. EH55EG)
        #[arg(long, value_name = "MODEL")]
        model: String,

        /// Optional single ASCII OEM character (e.g. M or E)
        #[arg(long, value_name = "CHAR")]
        oem: Option<char>,

        /// Exports structured JSON report (`-` emits stdout JSON)
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Retargets PANA_DVD.FRM / PANA__SD.FRM TOC hardware_family, model_group, oem, and target_flag
    RetargetFrm {
        /// Source .FRM container path
        #[arg(short, long, value_name = "FRM_FILE")]
        input: PathBuf,

        /// Output .FRM container path (Copy-on-Write)
        #[arg(short, long, value_name = "OUT_FRM")]
        output: PathBuf,

        /// Optional hardware family string (<=8 ASCII chars, e.g. M6930)
        #[arg(long, value_name = "HW")]
        hw: Option<String>,

        /// Optional model group string (<=6 ASCII for DVD / <=12 for SD, e.g. NP-309)
        #[arg(long, value_name = "GROUP")]
        model_group: Option<String>,

        /// Optional OEM prefix byte (1 ASCII char, 48B TOC)
        #[arg(long, value_name = "CHAR")]
        oem: Option<char>,

        /// Optional TOC FormatFlag / target_flag byte
        #[arg(long, value_name = "FLAG")]
        target_flag: Option<u8>,

        /// Exports structured JSON report (`-` emits stdout JSON)
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Inspects DEV.BIN/ifodev2, MAIN, .FRM, and NVRAM SA134..SA141 against an LBS/TV calibration profile
    InspectTvBundle {
        /// Regional chassis preset: br-pal-m, us-ntsc, eu-pal, or custom
        #[arg(long, default_value = "br-pal-m", value_name = "PRESET")]
        preset: String,

        /// Optional TV standard override: ntsc, pal-m, pal, or secam
        #[arg(long, value_name = "STANDARD")]
        tv_standard: Option<String>,

        /// Optional 512 B DEV.BIN or 2048 B ifodev2 path (tail 0x400..0x7FF audited read-only)
        #[arg(long, value_name = "BIN")]
        dev_bin: Option<PathBuf>,

        /// Optional unpacked MAIN binary path
        #[arg(long, value_name = "BIN")]
        main: Option<PathBuf>,

        /// Optional PANA_DVD.FRM or PANA__SD.FRM path
        #[arg(long, value_name = "FRM")]
        frm: Option<PathBuf>,

        /// Optional 64 KiB NVRAM slice or 8 MiB IC5001 dump path
        #[arg(long, value_name = "BIN")]
        nvram: Option<PathBuf>,

        /// Exports structured JSON bundle inspection report (`-` emits stdout JSON)
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Inspects SSOT Brazilian (`M6930LB`) localization sites in unpacked `MAIN` (`3060NP` / `3410PQ`)
    InspectLocalization {
        /// Unpacked MAIN binary path
        #[arg(short, long, value_name = "MAIN_BIN")]
        input: PathBuf,

        /// Exports structured JSON report (`-` emits stdout JSON)
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Applies or restores SSOT Brazilian (`M6930LB`) localization sites in unpacked `MAIN`
    PatchLocalization {
        /// Unpacked MAIN input binary path
        #[arg(short, long, value_name = "MAIN_BIN")]
        input: PathBuf,

        /// Patched MAIN output path (Copy-on-Write)
        #[arg(short, long, value_name = "OUT_BIN")]
        output: PathBuf,

        /// Mode: m6930lb_brazil (default) or factory_original
        #[arg(long, default_value = "m6930lb_brazil", value_name = "MODE")]
        mode: String,

        /// Also patch MAIN `<deviceinfo>` model & OEM guard to `bypass_model_and_oem`
        #[arg(long)]
        bypass_devinfo: bool,

        /// Exports structured JSON report (`-` emits stdout JSON)
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },
}
