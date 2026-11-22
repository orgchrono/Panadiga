//! CLI argument definitions for the Pre-MAIN DIGA Unbrick, SD Kit Builder, & TBC Passthrough Workbench.

pub mod ext;
pub mod payloads;
pub mod recovery;

pub use ext::{
    BgoAuditArgs, BgoLiveApplyArgs, BgoRepairPlanArgs, RegionCompatAction, TbcPlanArgs,
    VerifySdArgs,
};
pub use payloads::{DecodeNorDumpArgs, ExtractFrmArgs, MatshitaEmuAction, NorStubArgs};
pub use recovery::{BgoArgs, PassthroughArgs, PrepareSdArgs, RebuildFromDumpArgs};

use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct UnbrickArgs {
    #[command(subcommand)]
    pub action: UnbrickActions,
}

#[derive(Subcommand, Debug)]
pub enum UnbrickActions {
    /// Inspects workspace sources, PROG061 BGO sites, MAIN TBC passthrough, _dev_SERIAL RSP plan, and SD state
    Status {
        /// Optional workspace directory override
        #[arg(long, value_name = "DIR")]
        workspace: Option<PathBuf>,

        /// Exports structured JSON report
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Extracts and Feistel-decrypts DRV/PROG061/MAIN modules from PANA_DVD.FRM or PANA__SD.FRM
    ExtractFrm(ExtractFrmArgs),

    /// Inspects or patches PROG061 BGO (U99) recovery sites or emits 128 KiB DIGADIAG.FRM
    Bgo(BgoArgs),

    /// Inspects/patches MAIN (3410PQ) AV TBC (E-E) passthrough sites or emits _dev_SERIAL RSP frames
    Passthrough(PassthroughArgs),

    /// Builds a complete post-verified SD recovery kit (DIGADIAG.FRM + PANA__SD.FRM + SHA256SUMS.txt)
    PrepareSd(PrepareSdArgs),

    /// Rebuilds a verified SD unbrick kit directly from an immutable NOR dump and optional HDD MAIN partition
    RebuildFromDump(RebuildFromDumpArgs),

    /// Verifies an SD card root directory (DIGADIAG.FRM, PANA__SD.FRM, and -91006 hardware checklist)
    VerifySd(VerifySdArgs),

    /// Plans a clean-room Diskless / Standalone TBC execution sequence (Stage 1.5 DIGADIAG or gated RAM chainload)
    TbcPlan(TbcPlanArgs),

    /// Audits the 45-sector DMR-EH55 BGO FTL (SA26..=SA70) inside an 8 MiB IC5001 NOR dump
    BgoAudit(BgoAuditArgs),

    /// Builds and CoW-simulates a deterministic BGO FTL repair plan (SA26..=SA70 only, E05-guarded)
    BgoRepairPlan(BgoRepairPlanArgs),

    /// Applies a verified BgoRepairPlan live via Pico PANAMON1> (requires --confirm-upper-nor)
    BgoLiveApply(BgoLiveApplyArgs),

    /// Audits raw SD card MBR/GPT/Superfloppy, FAT16 BPB, 8.3 root entries, and cluster chain
    InspectSd {
        /// Drive letter (`E:`), Windows volume (`\\.\E:`), `\\.\PhysicalDriveN`, or raw `.img` file
        #[arg(value_name = "TARGET")]
        target: String,

        /// Exports structured JSON disk audit report
        #[arg(short, long, value_name = "JSON_FILE")]
        json: Option<PathBuf>,
    },

    /// Assembles a read-only MN103S UART NOR flash dumper stub (optionally wrapped as 128 KiB DIGADIAG.FRM)
    NorStub(NorStubArgs),

    /// Decodes and checksum-verifies a `PANADMP1..PANADONE` UART NOR flash capture stream
    DecodeNorDump(DecodeNorDumpArgs),

    /// Matshita R6-MV4 optical drive emulator, boot verifier, ATAPI write injector, and SD/CD .FRM unlock tool
    MatshitaEmu {
        #[command(subcommand)]
        action: MatshitaEmuAction,
    },

    /// Deterministic Region & Model compatibility patcher (MAIN <deviceinfo>, DEV.BIN, and .FRM TOC)
    RegionCompat {
        #[command(subcommand)]
        action: RegionCompatAction,
    },
}
