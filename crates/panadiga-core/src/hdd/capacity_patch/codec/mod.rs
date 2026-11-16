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
    use super::super::specs::WIN_LEN;

    pub(crate) fn build_synthetic_hdd_cmd_main() -> Vec<u8> {
        let mut buf = vec![0u8; 0x300];
        let sig: [u8; WIN_LEN] = [
            0xFD, 0xAA, 0x00, 0x8A, 0x00, 0x00, 0xFB, 0xE9, 0x00, 0x80, 0xC8, 0x1D, 0xFC, 0xCC,
            0x74, 0x3B, 0xA4, 0x0B, 0xFC, 0x81, 0xAC, 0xB9, 0x11, 0x53, 0xFC, 0xCC, 0x57, 0xE9,
            0xA5, 0x57, 0xDD, 0xC0, 0x2B, 0x01, 0x00, 0x00, 0x00, 0xCA, 0x29, 0xFC, 0xCC, 0xBA,
            0x1D, 0xD2, 0x05, 0xFC, 0x81, 0xAC, 0xB9, 0x11, 0x53, 0xFC, 0xCC, 0x71, 0xE9, 0xA5,
            0x57, 0xDD, 0xA5, 0x2B, 0x01, 0x00, 0x00, 0x00, 0xCA, 0x0E, 0xFC, 0xCC, 0xC8, 0xD7,
            0xA6, 0x04, 0xFC, 0x81, 0xAC, 0xB9, 0x11, 0x53, 0xFC, 0xA5, 0xAC, 0xB9, 0x11, 0x53,
            0xFC, 0xA4, 0x80, 0xBC, 0x11, 0x53, 0xA1, 0xC6, 0x0C, 0x29, 0xFF, 0xFC, 0x85, 0x80,
            0xBC, 0x11, 0x53, 0xCA, 0x0A, 0x28, 0xFF, 0xFC, 0x81, 0x80, 0xBC, 0x11, 0x53, 0xFC,
            0xA4, 0x80,
        ];
        buf[0x80..0x80 + sig.len()].copy_from_slice(&sig);
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
