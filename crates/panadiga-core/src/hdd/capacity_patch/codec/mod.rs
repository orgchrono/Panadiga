//! Pure in-memory buffer codecs for `src/hdd_cmd.c` capacity clamping, `EXTRHEAD`, & MBR superblocks.

mod loader;
mod patcher;

pub(crate) use loader::{LoadedMainSource, inspect_unpacked_capacity, load_main_source};
pub use loader::{find_hdd_cmd_clamp_site, inspect_main_buffer_capacity};
pub use patcher::apply_capacity_patch_in_place;
pub(crate) use patcher::{
    apply_optional_passthrough, maybe_update_procedural_mbr_capacity, patch_and_repack_extrhead,
    verify_and_inspect_repacked,
};

#[cfg(test)]
pub(crate) mod test_helpers {
    use super::super::specs::HDD_CMD_CLAMP_WINDOW_TEMPLATE;

    pub(crate) fn build_synthetic_hdd_cmd_main() -> Vec<u8> {
        let mut buf = vec![0u8; 0x300];
        buf[0x80..0x80 + HDD_CMD_CLAMP_WINDOW_TEMPLATE.len()]
            .copy_from_slice(&HDD_CMD_CLAMP_WINDOW_TEMPLATE);
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::super::specs::*;
    use super::test_helpers::build_synthetic_hdd_cmd_main;
    use super::*;

    #[test]
    fn test_inspect_and_patch_dynamic_and_preset_250gb_and_restore() {
        let mut main = build_synthetic_hdd_cmd_main();

        let rep0 = inspect_main_buffer_capacity(
            &main,
            MAIN_DEFAULT_BASE_VA,
            "RAW_MAIN_BINARY",
            None,
            "synthetic",
        )
        .expect("Should locate clamp site");
        assert_eq!(rep0.patch_site_file_offset, 0x80);
        assert!(!rep0.is_dynamic_unclamped);
        assert_eq!(rep0.slot_200gb_blocks_2kb, HDD_CMD_M7920_200GB_BLOCKS);
        assert_eq!(rep0.slot_400gb_blocks_2kb, HDD_CMD_M7920_400GB_BLOCKS);

        let applied_dyn =
            apply_capacity_patch_in_place(&mut main, HddCapacityPatchMode::DynamicUnclamped, None)
                .expect("Dynamic patch succeeds");
        assert_eq!(applied_dyn, DYNAMIC_UNCLAMPED_MAX_BLOCKS_2KB);
        let rep_dyn = inspect_main_buffer_capacity(
            &main,
            MAIN_DEFAULT_BASE_VA,
            "RAW_MAIN_BINARY",
            None,
            "synthetic",
        )
        .expect("Should still match signature after dynamic patch");
        assert!(rep_dyn.is_dynamic_unclamped);
        assert_eq!(rep_dyn.clamp_branch_opcode_hex, "0xCA");

        let applied_250 = apply_capacity_patch_in_place(
            &mut main,
            HddCapacityPatchMode::FixedCapacity,
            Some(250),
        )
        .expect("250 GB patch succeeds");
        assert_eq!(applied_250, 0x0746_6528);
        let rep_250 = inspect_main_buffer_capacity(
            &main,
            MAIN_DEFAULT_BASE_VA,
            "RAW_MAIN_BINARY",
            None,
            "synthetic",
        )
        .expect("Should detect 250 GB preset");
        assert!(!rep_250.is_dynamic_unclamped);
        assert_eq!(rep_250.slot_200gb_blocks_2kb, 0x0746_6528);
        assert_eq!(rep_250.slot_400gb_blocks_2kb, 0x0746_6528);
        assert_eq!(rep_250.slot_fallback_blocks_2kb, 0x0746_6528);

        apply_capacity_patch_in_place(&mut main, HddCapacityPatchMode::Factory200Gb, None)
            .expect("Factory restore succeeds");
        let rep_fac = inspect_main_buffer_capacity(
            &main,
            MAIN_DEFAULT_BASE_VA,
            "RAW_MAIN_BINARY",
            None,
            "synthetic",
        )
        .expect("Should detect factory restore");
        assert_eq!(rep_fac.slot_200gb_blocks_2kb, HDD_CMD_M7920_200GB_BLOCKS);
        assert_eq!(rep_fac.slot_400gb_blocks_2kb, HDD_CMD_M7920_400GB_BLOCKS);
        assert_eq!(rep_fac.clamp_branch_opcode_hex, "0xC6");
    }
}
