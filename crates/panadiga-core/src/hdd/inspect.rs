//! Pure structural forensic inspector for Panasonic DMR-EH55 HDD images.

mod io;
mod probe;

#[cfg(test)]
mod tests;

pub use io::inspect_hdd_image;
pub use probe::inspect_hdd_slice;

#[cfg(test)]
use crate::constants::*;
