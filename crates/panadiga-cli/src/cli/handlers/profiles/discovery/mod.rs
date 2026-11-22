#![deny(unsafe_code)]

//! Completeness auditing and physical NOR intake discovery helpers for `panadiga profiles`.

mod completeness;
mod nor_discover;

pub(super) use completeness::build_completeness_report;
pub(super) use nor_discover::discover_nor_dump;
