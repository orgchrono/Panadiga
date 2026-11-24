//! Pure decoder and forensic analyzer for Panasonic MEIHFS allocation descriptors and extents.

mod decode;
mod io;

#[cfg(test)]
mod tests;

pub use decode::{
    MeihdfsError, aggregate_meihdfs_extents, decode_journal_blocks_slice, decode_meihdfs_slice,
    decode_single_meihdfs_record, try_decode_single_meihdfs_record,
};
pub use io::{decode_descriptors, decode_meihdfs_descriptors};

#[cfg(test)]
pub use tests::pure_meihdfs_tests;

#[cfg(test)]
use crate::constants::*;
