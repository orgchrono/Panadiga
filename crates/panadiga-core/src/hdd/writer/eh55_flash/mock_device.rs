#![deny(unsafe_code)]

//! Re-export of the shared offline [`FileBackedMockFlashDevice`] from `hdd::writer::raw_device`.

pub use super::super::raw_device::FileBackedMockFlashDevice;
