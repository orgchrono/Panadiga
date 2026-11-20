#![deny(unsafe_code)]

//! Safe Panasonic DMR-EH55 Physical HDD Inspection, 64 KiB Pre-Write Backup,
//! Primary + High-LBA `MAIN` Mirror Flashing, and Full Read-Back SHA-256 Verification.
//!
//! Zero `unsafe` code: Windows Win32 IOCTLs are encapsulated in `win_ioctl::OwnedDiskHandle`,
//! and all logic operates over [`BlockFlashDevice`] so offline unit tests run
//! deterministically against [`FileBackedMockFlashDevice`] without touching any real drives.

mod backup_verify;
mod flash_ops;
mod mock_device;
#[cfg(test)]
mod tests;
mod types;

pub use flash_ops::execute_eh55_hdd_flash;
pub use mock_device::FileBackedMockFlashDevice;
pub use types::{
    BUS_TYPE_NVME, BlockFlashDevice, EH55_BACKUP_SECTOR_COUNT, EH55_FLASH_CHUNK_BYTES,
    EH55_MIRROR_MAIN_BACKSTEP_LBAS, Eh55FlashProgress, Eh55FlashStage, Eh55HddFlashReport,
    Eh55HddFlashRequest, SafeTargetDiskInfo, WindowsPhysicalFlashDevice,
    compute_eh55_mirror_main_lba, enrich_safe_target_disks, inspect_safe_target_disks,
};
