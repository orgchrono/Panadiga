//! CLI argument definitions for HDD MEIHDFS inspection and physical drive writes.

use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct HddArgs {
    #[command(subcommand)]
    pub action: HddActions,
}

#[derive(Subcommand, Debug)]
pub enum HddActions {
    /// Performs zero-copy structural inspection of MBR, mirror table, allocation descriptors, and partitions
    Inspect {
        /// Path to the HDD dump image (defaults to firmware_and_images/panasonic DMR EH-55 Firmware hdd_100.bin)
        #[arg(value_name = "IMAGE_PATH")]
        image: Option<PathBuf>,

        /// Displays visual density distribution bar per 1 MB block
        #[arg(short, long)]
        density: bool,

        /// Exports structured JSON report to specified file
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Surgically extracts firmware partitions and filesystem descriptors with SHA256SUMS manifest
    Extract {
        /// Path to the HDD dump image (defaults to firmware_and_images/panasonic DMR EH-55 Firmware hdd_100.bin)
        #[arg(value_name = "IMAGE_PATH")]
        image: Option<PathBuf>,

        /// Output directory for extracted partition binaries
        #[arg(
            short,
            long,
            default_value = "extracted_partitions",
            value_name = "DIR"
        )]
        outdir: PathBuf,

        /// Exports structured JSON report to specified file
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Decodes the bit-inverted (One's Complement XOR 0xFF) MEIHDFS allocation descriptors and extents
    DecodeDescriptors {
        /// Path to the HDD dump image or extracted allocation descriptors file (defaults to firmware_and_images/panasonic DMR EH-55 Firmware hdd_100.bin)
        #[arg(value_name = "IMAGE_PATH")]
        image: Option<PathBuf>,

        /// Exports structured JSON report to specified file
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,

        /// Include journal blocks analysis if inspecting a full HDD image
        #[arg(long)]
        journal: bool,
    },

    /// Lists connected physical drives & mounted partitions with optional media role filter (`all`, `hdd`, `sd`)
    ListDrives {
        /// Role filter: all, hdd, sd, or system
        #[arg(short, long, default_value = "all", value_name = "ROLE")]
        role: String,
    },

    /// Dedicated command listing detected SD Cards, MMC/SD readers, and removable flash media
    ListSdCards,

    /// Procedurally formats a non-system physical drive or disk image with MEIHDFS structures (mkfs.meihdfs)
    FormatMeihdfs {
        /// Target device path (e.g. \\.\PhysicalDrive1 or /dev/sdb)
        #[arg(short, long, value_name = "TARGET_DRIVE")]
        target: String,

        /// Total drive capacity in Gigabytes (e.g. 160, 250, 500, 1000)
        #[arg(short, long, default_value_t = 160, value_name = "SIZE_GB")]
        size_gb: u64,

        /// Optional path to PANA_DVD.FRM to inject MAIN AM34 OS kernel into Partition 1
        #[arg(long, value_name = "FRM_FILE")]
        inject_frm: Option<PathBuf>,

        /// Explicit safety confirmation token (e.g. CONFIRM-PhysicalDrive1)
        #[arg(long, value_name = "TOKEN")]
        confirm: String,

        /// Validate lockout, MEIHDFS layout, and FRM MAIN OS payloads without writing target
        #[arg(long)]
        dry_run: bool,

        /// Exports structured JSON report (`-` emits stdout JSON)
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Streams a raw image (.bin or transparently decompressed .bin.gz) directly to a non-system physical drive
    WriteImage {
        /// Target device path (e.g. \\.\PhysicalDrive1 or /dev/sdb)
        #[arg(short, long, value_name = "TARGET_DRIVE")]
        target: String,

        /// Path to the source raw or gzipped disk image
        #[arg(short, long, value_name = "IMAGE_PATH")]
        image: PathBuf,

        /// Explicit safety confirmation token (e.g. CONFIRM-PhysicalDrive1)
        #[arg(long, value_name = "TOKEN")]
        confirm: String,
    },

    /// Restores LBA 0 and LBA 24..31 primary structures atomically from dual backup mirrors (LBA 57344 and 57368)
    RestoreMirrors {
        /// Target device path (e.g. \\.\PhysicalDrive1 or /dev/sdb)
        #[arg(short, long, value_name = "TARGET_DRIVE")]
        target: String,

        /// Explicit safety confirmation token (e.g. CONFIRM-PhysicalDrive1)
        #[arg(long, value_name = "TOKEN")]
        confirm: String,
    },

    /// Inspects `src/hdd_cmd.c` (`MaxProductLBA`) capacity limits in a MAIN binary, EXTRHEAD Partition 1, or full HDD image
    InspectCapacity {
        /// Path to MAIN raw binary, EXTRHEAD Partition 1 (`hdd_partition_firmware_0x02000000.bin`), or full HDD image
        #[arg(value_name = "INPUT_PATH")]
        input: Option<PathBuf>,

        /// Optional `-i/--input` alias for INPUT_PATH
        #[arg(
            short = 'i',
            long = "input",
            visible_alias = "input-path",
            value_name = "INPUT_PATH"
        )]
        input_flag: Option<PathBuf>,

        /// Exports structured JSON report to specified file
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Patches `src/hdd_cmd.c` capacity ceiling (`dynamic` auto-detect up to 2TB+, `cap` target GB, or `factory_200gb`)
    PatchCapacity {
        /// Path to MAIN raw binary, EXTRHEAD Partition 1, or full HDD image
        #[arg(short, long, visible_alias = "input-path", value_name = "INPUT_PATH")]
        input: Option<PathBuf>,

        /// Output path for the patched binary/container/image (auto-generated beside input if omitted)
        #[arg(short, long, visible_alias = "output-path", value_name = "OUTPUT_PATH")]
        output: Option<PathBuf>,

        /// Patch mode: `dynamic` (bypass clamp for ANY HDD size), `cap` (RTSD/custom GB limit), or `factory_200gb`
        #[arg(short, long, default_value = "dynamic", value_name = "MODE")]
        mode: String,

        /// Target capacity in GB (e.g. 250, 300, 320, 400, 500, 1000, 2000)
        #[arg(short = 'g', long, visible_alias = "gb", value_name = "GB")]
        target_gb: Option<u32>,

        /// Optional MAIN AV TBC passthrough mode to co-apply (av_passthrough or factory_original)
        #[arg(long = "passthrough-mode", value_name = "PASSTHROUGH_MODE")]
        passthrough_mode: Option<String>,
    },

    /// Safely flashes a verified EH55 disk image + tail mirror with pre-64K CoW backup and SHA-256 readback
    FlashEh55(FlashEh55Args),

    /// Streams procedurally synthesized MEIHFS metadata + optional MAIN/DRV EXTRHEAD without a 100 MiB `.img`
    FlashComposed(FlashComposedArgs),
}

/// Shared drive capacity bounds and JSON export options for EH55 / composed HDD flashing.
#[derive(Args, Debug, Clone)]
pub struct HddExpectedRangeArgs {
    /// Optional minimum expected disk size in GB
    #[arg(long, value_name = "U64")]
    pub expected_min_gb: Option<u64>,

    /// Optional maximum expected disk size in GB
    #[arg(long, value_name = "U64")]
    pub expected_max_gb: Option<u64>,

    /// Allow 2201..=4096 GB extended ceiling validation (standard EH55 limit is 2200 GB)
    #[arg(long)]
    pub allow_extended_4tb: bool,

    /// Exports structured JSON flash & SHA-256 verification report (`-` emits stdout JSON)
    #[arg(short, long, value_name = "JSON_FILE")]
    pub json: Option<PathBuf>,
}

/// CLI parameters for safe physical/file-backed EH55 HDD flashing (`flash-eh55`).
#[derive(Args, Debug, Clone)]
pub struct FlashEh55Args {
    /// Target physical disk path or ID (e.g. \\.\PhysicalDrive1; PhysicalDrive0/NVMe hard-blocked)
    #[arg(long, value_name = "ID")]
    pub drive: String,

    /// Verified 100 MiB EH55 disk image path
    #[arg(long, value_name = "IMG")]
    pub image: PathBuf,

    /// Optional Partition 1 (`0x02000000`) mirror file (extracted from image if omitted)
    #[arg(long, value_name = "P1")]
    pub mirror_p1: Option<PathBuf>,

    /// Mandatory directory to store pre-flash 64 KiB backup + SHA-256 manifest
    #[arg(long, value_name = "DIR")]
    pub backup_dir: PathBuf,

    /// Explicit drive confirmation ID (e.g. PhysicalDrive1 or CONFIRM-PhysicalDrive1)
    #[arg(long, value_name = "ID")]
    pub confirm_physical_drive: String,

    #[command(flatten)]
    pub range: HddExpectedRangeArgs,
}

/// CLI parameters for streaming MEIHFS HDD composition flashing (`flash-composed`).
#[derive(Args, Debug, Clone)]
pub struct FlashComposedArgs {
    /// Target physical disk path or ID (e.g. `\\.\PhysicalDrive1` or `/dev/sdb`)
    #[arg(long, value_name = "ID")]
    pub drive: String,

    /// Mandatory directory to store pre-flash 64 KiB backup + SHA-256 manifest
    #[arg(long, value_name = "DIR")]
    pub backup_dir: PathBuf,

    /// Explicit drive confirmation ID (e.g. `PhysicalDrive1` or `CONFIRM-PhysicalDrive1`)
    #[arg(long, value_name = "ID")]
    pub confirm_physical_drive: String,

    /// Optional MAIN binary, EXTRHEAD, or `.FRM` source to inject into P1 & tail mirror
    #[arg(long, value_name = "FILE")]
    pub main_source: Option<PathBuf>,

    /// Optional DRV binary, EXTRHEAD, or `.FRM` source to inject into Partition 2
    #[arg(long, value_name = "FILE")]
    pub drv_source: Option<PathBuf>,

    /// Apply dynamic LBA48 HDD capacity unlock patch in memory before Mn103Vm gate
    #[arg(long)]
    pub unlock_capacity: bool,

    /// Target capacity ceiling in GB when applying bounded capacity patch
    #[arg(long, value_name = "GB")]
    pub target_gb: Option<u32>,

    /// Apply MAIN AV TBC passthrough patch in memory before Mn103Vm gate
    #[arg(long)]
    pub av_passthrough: bool,

    #[command(flatten)]
    pub range: HddExpectedRangeArgs,
}
