//! SD Recovery Kit Generator (`DIGADIAG.FRM` + `PANA__SD.FRM`), Media Validator,
//! and Raw SD MBR/BPB/FAT16 Disk Auditor (`inspect_sd`).
//!
//! Splits pure media verification (`verify`), pure artifact composition (`compose`),
//! sector-aligned raw disk inspection (`inspect_sd`), and thin disk I/O (`io`).

pub mod compose;
pub mod inspect_sd;
pub mod io;
pub mod verify;

pub use compose::{SdUnbrickKitReport, SdUnbrickKitRequest};
pub use inspect_sd::{
    FatBpbReport, MbrPartitionRecord, RootDirFileReport, SdDiskInspectionReport,
    inspect_sd_disk_bytes, inspect_sd_reader, inspect_sd_target,
};
pub use io::{prepare_sd_unbrick_kit, verify_sd_unbrick_media};
pub use verify::{SdMediaVerificationReport, SdUnbrickCheck, verify_sd_buffers};
