#![deny(unsafe_code)]

//! Container-level buffer validators for `DIGADIAG.FRM` and `PANA__SD.FRM`.

mod digadiag;
mod dual_parity;
mod pana_sd;
#[cfg(test)]
mod tests;

pub(crate) use digadiag::{DigadiagInspection, verify_digadiag_buffer};
pub use dual_parity::{
    DualFamilyParityReport, DualFamilySlotResolution, SdKitAuditProfile,
    simulate_sd_find_firm_entry_dual,
};
pub(crate) use pana_sd::verify_pana_sd_buffer;
