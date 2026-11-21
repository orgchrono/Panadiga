//! NOR Flash geometry validation and safe probe-based model identification helpers.

use crate::ssot::fingerprint::detect_diga_model_from_slice_safe;

/// Pure check for valid Panasonic DIGA Parallel NOR Flash geometries (2 MB, 4 MB, 8 MB, 16 MB).
/// Explicitly excludes 100 MB+ HDD dumps and optical `.FRM` containers.
pub const fn is_valid_nor_flash_size(size_bytes: u64) -> bool {
    matches!(
        size_bytes,
        0x0020_0000 | 0x0040_0000 | 0x0080_0000 | 0x0100_0000
    )
}

/// Identifies the Panasonic DIGA model using safe structural/loader probes (skipping shared DRV),
/// never inventing a concrete model on dump byte size alone.
pub fn detect_diga_model_from_slice(data: &[u8]) -> String {
    detect_diga_model_from_slice_safe(data)
}
