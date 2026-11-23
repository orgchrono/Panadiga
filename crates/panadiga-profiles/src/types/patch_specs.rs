#![deny(unsafe_code)]

//! Declarative binary patch sites, SATA-bridge reset specs, Stage-2 HDD boot ABI tables,
//! and per-build `MAIN` firmware descriptors stored in `profiles/firmware_trains/*.json`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::hex_serde::{hex_u16, hex_u32, opt_hex_u32};

/// Single fail-closed machine-code patch site inside a firmware image (`VA`, expected bytes, replacement).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BinaryPatchSiteSpec {
    pub name: String,
    pub description: String,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub va: u32,
    pub expect_hex: String,
    pub replace_hex: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol_ref: Option<String>,
}

/// Declarative post-`RESET-` ATA/SATA bridge polling wait site specification (`2 ms -> 1000 ms`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SataBridgeResetSpec {
    pub target_name: String,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub base_va: u32,
    pub expected_size: u32,
    pub factory_wait_ms: u32,
    pub patched_wait_ms: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub sleep_callee_va: u32,
    pub site: BinaryPatchSiteSpec,
}

/// Verified `PROG` / bootloader callee virtual addresses required to assemble the Stage-2 HDD boot hook.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Stage2BootSymbolsSpec {
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub dcache_purge_va: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub uart0_getc_nb_va: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub uart0_printf_va: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub check_update_flag_va: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub hdd_max_lba_va: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub hdd_read_va: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub sd_init_va: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub stop_timer_irq_va: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub feistel_decrypt_va: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub extrhead_get_size_va: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub extrhead_check_hdr_va: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub extrhead_is_comp_va: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub extrhead_get_src_va: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub extrhead_get_entry_va: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub extrhead_uncomp_va: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub fat_mount_va: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub fat_open_va: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub fat_close_va: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub disable_irq_va: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub memcpy_va: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub sleep_10ms_va: u32,
}

/// Complete declarative Stage-2 SATA HDD boot hook contract for a firmware train.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Stage2BootSpec {
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub base_va: u32,
    pub expected_size: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub hook_offset: u32,
    pub hook_expect_hex: String,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub payload_offset: u32,
    pub require_reset_fix_1000ms: bool,
    pub uart_abort_polls: i8,
    pub uart_poll_ticks_10ms: i8,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub hdd_primary_lba: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub hdd_mirror_lba_delta: u32,
    pub hdd_probe_sectors: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub feistel_crypt_span: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub main_va_min: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub main_va_max: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub dcache_purge_base: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub dcache_purge_len: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub sd_info_bss_va: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub hdd_dma_buf_va: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub fvu_load_uncached_va: u32,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub norfrom_fvu_base_va: u32,
    pub symbols: Stage2BootSymbolsSpec,
}

/// Declarative descriptor for an individual `MAIN` firmware build belonging to a firmware train.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MainBuildSpec {
    pub build_id: String,
    pub slot_name: String,
    #[serde(with = "hex_u16")]
    #[schemars(with = "String")]
    pub frm_version_u16: u16,
    #[serde(with = "hex_u32")]
    #[schemars(with = "String")]
    pub base_va: u32,
    pub unpacked_size: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region_tv_standard: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub model_markers: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chassis_family: Option<String>,
    pub native_bgo_geometry_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "opt_hex_u32")]
    #[schemars(with = "Option<String>")]
    pub native_bgo_detect_va: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "opt_hex_u32")]
    #[schemars(with = "Option<String>")]
    pub native_bgo_flash_base_va: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supports_bgo_retarget_to: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sata_bridge_reset_spec: Option<SataBridgeResetSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub av_passthrough_sites: Vec<BinaryPatchSiteSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unpacked_sha256: Option<String>,
}
