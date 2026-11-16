//! Verified SSOT symbol catalog across NOR IPL, BOOT, PROG061, MAIN, and MMIO.

mod boot_bin;
mod hardware_mmio;
mod stages_prog_main;

use super::SymbolSpec;

const fn sym(address: u32, name: &'static str, comment: &'static str) -> SymbolSpec {
    SymbolSpec {
        address,
        name,
        comment,
        is_primary: true,
    }
}

/// Complete verified SSOT symbol catalog across NOR IPL, BOOT, PROG061, MAIN, and MMIO.
pub fn symbol_specs() -> Vec<SymbolSpec> {
    let mut out = Vec::with_capacity(
        hardware_mmio::SPECS.len() + boot_bin::SPECS.len() + stages_prog_main::SPECS.len(),
    );
    out.extend_from_slice(hardware_mmio::SPECS);
    out.extend_from_slice(boot_bin::SPECS);
    out.extend_from_slice(stages_prog_main::SPECS);
    out
}
