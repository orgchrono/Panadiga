//! Panasonic DIGA Dynamic HDD Capacity Unlock & `src/hdd_cmd.c` Patcher.
//!
//! Splits pure preset/DTO math (`specs`), pure `MAIN`/MBR codecs (`codec`),
//! and thin disk inspection/writing (`io`).

pub mod codec;
pub mod io;
pub mod specs;

pub use codec::{
    apply_capacity_patch_in_place, find_hdd_cmd_clamp_site, inspect_main_buffer_capacity,
};
pub use io::{
    default_capacity_patched_output_path, inspect_main_hdd_capacity, patch_main_hdd_capacity,
};
pub use specs::{
    DYNAMIC_UNCLAMPED_MAX_BLOCKS_2KB, HDD_CMD_M7920_200GB_BLOCKS, HDD_CMD_M7920_400GB_BLOCKS,
    HddCapacityPatchMode, MAIN_DEFAULT_BASE_VA, MainCapacityInspectionReport,
    MainCapacityPatchReport, RtsdCapacityPreset, blocks_2kb_for_gb, official_rtsd_presets,
};
