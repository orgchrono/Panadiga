#![deny(unsafe_code)]

//! Opt-in, fail-closed firmware-hosted Stage-2 HDD boot hook for SATA-to-PATA bridges
//! (e.g. StarTech `IDE2SAT2` / Marvell `88SA8052`).
//!
//! When the cold NOR bootloader hardcodes a short post-`RESET-` delay (`2 ms` in
//! `Loader ver0.61`), cold/warm boots time out against slow SATA-to-PATA bridge silicon
//! before loading `MAIN` from the HDD.
//!
//! When paired with [`super::sata_bridge_fix`] (`1000 ms` post-`RESET-` wait in RAM `PROG`),
//! this hook intercepts `call check_update_flag` inside the RAM updater payload to:
//! 1. Enforce three explicit pre-HDD recovery escape hatches (UART0 `'F'`/`'f'`, SD
//!    `FORCEFVU.TXT`, and `Nor-FROM` boot with SD `PANA__SD.FRM` present) so stock
//!    `Fvu` recovery remains reachable even if the HDD holds an incompatible image;
//! 2. Query `HDD_MAX_LBA` and attempt primary (`0x10000`) then outer mirror
//!    (`max_lba - 0x20000`) HDD `EXTRHEAD` load/Feistel decrypt/Adler-32 uncompress,
//!    rejecting null `EXTRHEAD+0x1C` (`src_addr == 0`);
//! 3. Validate the exact 32-word power-of-two bit-walk at `*(entry + 8)`;
//! 4. Purge D-cache, execute exact `Loader`-equivalent timer/IRQ teardown
//!    (with ZERO NOR/BGO writes), and jump to `MAIN` (`CALLS (A2)`).

mod apply;
mod asm;
mod payload;
#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

use super::bgo_unbrick::PROG061_BASE_VA;

pub use apply::{
    apply_hdd_autoboot_for_loader_profile_in_place, apply_hdd_autoboot_in_place_for_spec,
    apply_prog061_hdd_autoboot_in_place, hdd_autoboot_spec_for_loader_profile,
    inspect_hdd_autoboot_for_loader_profile, inspect_hdd_autoboot_site_for_spec,
    inspect_prog061_hdd_autoboot_site,
};
pub use payload::{assemble_hdd_autoboot_payload_for_spec, assemble_prog061_hdd_autoboot_payload};

#[allow(deprecated)]
pub use apply::{
    apply_prog061_stage2_boot_in_place, apply_stage2_boot_for_loader_profile_in_place,
    apply_stage2_boot_in_place_for_spec, inspect_prog061_stage2_boot_site,
    inspect_stage2_boot_for_loader_profile, inspect_stage2_boot_site_for_spec,
    stage2_spec_for_loader_profile,
};
#[allow(deprecated)]
pub use payload::{assemble_prog061_stage2_payload, assemble_stage2_payload_for_spec};

/// File offset inside raw `PROG061` (`0x40000` B, `VA 0x5040_50D3`) hooked for pre-FVU SATA HDD autoboot.
pub const OFF_PROG061_HDD_AUTOBOOT_HOOK_SITE: usize = 0x0000_50D3;
#[deprecated(note = "Use OFF_PROG061_HDD_AUTOBOOT_HOOK_SITE")]
pub const OFF_PROG061_STAGE2_HOOK_SITE: usize = OFF_PROG061_HDD_AUTOBOOT_HOOK_SITE;
/// Exact 7-byte factory instruction at `0x5040_50D3` (`CALL 0x5040_9FDB,[],0`).
pub const ORIG_PROG061_HDD_AUTOBOOT_HOOK_BYTES: [u8; 7] =
    [0xDD, 0x08, 0x4F, 0x00, 0x00, 0x00, 0x00];
#[deprecated(note = "Use ORIG_PROG061_HDD_AUTOBOOT_HOOK_BYTES")]
pub const ORIG_PROG061_STAGE2_HOOK_BYTES: [u8; 7] = ORIG_PROG061_HDD_AUTOBOOT_HOOK_BYTES;
/// File offset inside `PROG061` zero-padded tail slack (`0x39494..0x3FFFF`) hosting SATA HDD autoboot code.
pub const PROG061_HDD_AUTOBOOT_PAYLOAD_OFFSET: usize = 0x0003_9600;
#[deprecated(note = "Use PROG061_HDD_AUTOBOOT_PAYLOAD_OFFSET")]
pub const PROG061_STAGE2_PAYLOAD_OFFSET: usize = PROG061_HDD_AUTOBOOT_PAYLOAD_OFFSET;
/// MN103S virtual address of the embedded SATA HDD autoboot entry (`0x5043_9600`).
pub const PROG061_HDD_AUTOBOOT_PAYLOAD_VA: u32 =
    PROG061_BASE_VA + PROG061_HDD_AUTOBOOT_PAYLOAD_OFFSET as u32;
#[deprecated(note = "Use PROG061_HDD_AUTOBOOT_PAYLOAD_VA")]
pub const PROG061_STAGE2_PAYLOAD_VA: u32 = PROG061_HDD_AUTOBOOT_PAYLOAD_VA;

pub const PROG061_HDD_AUTOBOOT_STATE_FACTORY: &str = "FACTORY";
#[deprecated(note = "Use PROG061_HDD_AUTOBOOT_STATE_FACTORY")]
pub const PROG061_STAGE2_STATE_FACTORY: &str = PROG061_HDD_AUTOBOOT_STATE_FACTORY;
pub const PROG061_HDD_AUTOBOOT_STATE_ENABLED: &str = "SATA_HDD_AUTOBOOT";
#[deprecated(note = "Use PROG061_HDD_AUTOBOOT_STATE_ENABLED")]
pub const PROG061_STAGE2_STATE_ENABLED: &str = PROG061_HDD_AUTOBOOT_STATE_ENABLED;
pub const PROG061_HDD_AUTOBOOT_STATE_MISMATCH: &str = "UNKNOWN_OR_MISMATCH";
#[deprecated(note = "Use PROG061_HDD_AUTOBOOT_STATE_MISMATCH")]
pub const PROG061_STAGE2_STATE_MISMATCH: &str = PROG061_HDD_AUTOBOOT_STATE_MISMATCH;

const PROG_SLACK_FILL_BYTE: u8 = 0xFF;
const PROG_TRAILER_RESERVED_BYTES: usize = 4;

/// Declarative per-firmware-train parameters required to assemble and verify the pre-FVU SATA HDD autoboot hook.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HddAutobootTrainSpec {
    pub train_id: &'static str,
    pub base_va: u32,
    pub expected_size: usize,
    pub hook_offset: usize,
    pub hook_expect: [u8; 7],
    pub payload_offset: usize,
    pub require_reset_fix_1000ms: bool,
    pub uart_abort_polls: i8,
    pub uart_poll_ticks_10ms: i8,
    pub hdd_primary_lba: u32,
    pub hdd_mirror_lba_delta: u32,
    pub hdd_probe_sectors: u32,
    pub feistel_crypt_span: u32,
    pub main_va_min: u32,
    pub main_va_max: u32,
    pub dcache_purge_base: u32,
    pub dcache_purge_len: u32,
    pub sd_info_bss_va: u32,
    pub hdd_dma_buf_va: u32,
    pub fvu_load_uncached_va: u32,
    pub norfrom_fvu_base_va: u32,
    pub dcache_purge_va: u32,
    pub uart0_getc_nb_va: u32,
    pub uart0_printf_va: u32,
    pub check_update_flag_va: u32,
    pub hdd_max_lba_va: u32,
    pub hdd_read_va: u32,
    pub sd_init_va: u32,
    pub stop_timer_irq_va: u32,
    pub feistel_decrypt_va: u32,
    pub extrhead_get_size_va: u32,
    pub extrhead_check_hdr_va: u32,
    pub extrhead_is_comp_va: u32,
    pub extrhead_get_src_va: u32,
    pub extrhead_get_entry_va: u32,
    pub extrhead_uncomp_va: u32,
    pub fat_mount_va: u32,
    pub fat_open_va: u32,
    pub fat_close_va: u32,
    pub disable_irq_va: u32,
    pub memcpy_va: u32,
    pub sleep_10ms_va: u32,
}

#[deprecated(note = "Use HddAutobootTrainSpec")]
pub type Stage2BootTrainSpec = HddAutobootTrainSpec;

/// Computes the 7-byte MN103S `CALL d32rel, [], 0` instruction hooking `hook_offset` to `payload_offset`.
#[must_use]
pub const fn compute_hdd_autoboot_hook_bytes(spec: &HddAutobootTrainSpec) -> [u8; 7] {
    let hook_va = spec.base_va.wrapping_add(spec.hook_offset as u32);
    let target_va = spec.base_va.wrapping_add(spec.payload_offset as u32);
    let rel = target_va.wrapping_sub(hook_va).to_le_bytes();
    [0xDD, rel[0], rel[1], rel[2], rel[3], 0x00, 0x00]
}

#[deprecated(note = "Use compute_hdd_autoboot_hook_bytes")]
#[must_use]
pub const fn compute_stage2_hook_bytes(spec: &HddAutobootTrainSpec) -> [u8; 7] {
    compute_hdd_autoboot_hook_bytes(spec)
}

/// Inspection status for `PROG061` pre-FVU SATA HDD autoboot integration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Prog061HddAutobootStatus {
    pub hook_va_hex: String,
    pub payload_va_hex: String,
    pub payload_size_bytes: usize,
    pub state: String,
    pub payload_present: bool,
    pub hook_bytes_hex: String,
}

#[deprecated(note = "Use Prog061HddAutobootStatus")]
pub type Prog061Stage2BootStatus = Prog061HddAutobootStatus;
