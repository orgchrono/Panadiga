//! `MAIN` `src/hdd_cmd.c` >200 GB LBA48 resize verification check.

use anyhow::{Context, Result, bail};
use byteorder::{ByteOrder, LittleEndian};

use crate::hdd::{find_hdd_cmd_clamp_site, inspect_main_buffer_capacity};
use crate::re::MAIN_BASE_VA;

use super::fixtures::new_spansion_nor_vm;
use super::types::PatchVerificationCheck;

const TEST_500GB_BLOCKS_2KB: u32 = 0x0E8D_4A51;
const TEST_200GB_BLOCKS_2KB_M1: u32 = 0x05D2_1DB9;

fn read_u32_le(slice: &[u8]) -> u32 {
    LittleEndian::read_u32(slice)
}

fn map_bss_page_if_needed(vm: &mut crate::re::Mn103Vm, va: u32) {
    let page = va & !0x0FFF;
    if vm.read_u32(page).is_err() {
        vm.map_segment(page, vec![0u8; 0x2000], true);
    }
}

fn exec_hdd_cmd_block(main_buf: &[u8], site_off: usize, phys_blocks: u32) -> Result<u32> {
    let max_lba_va = read_u32_le(&main_buf[site_off + 0x14..site_off + 0x18]);
    let phys_blocks_va = read_u32_le(&main_buf[site_off + 0x56..site_off + 0x5A]);

    let mut vm = new_spansion_nor_vm();
    vm.map_segment(MAIN_BASE_VA, main_buf.to_vec(), false);
    map_bss_page_if_needed(&mut vm, max_lba_va);
    map_bss_page_if_needed(&mut vm, phys_blocks_va);
    vm.write_u32(phys_blocks_va, phys_blocks)?;

    let site_va = MAIN_BASE_VA + site_off as u32;
    // 1. Run `mov slot_200gb, d0; mov d0, (MaxProductLBA)` at `site_va + 0x27..0x33`
    vm.pc = site_va + 0x27;
    vm.run_until(site_va + 0x33, 16)?;
    // 2. Run comparator & clamp at `site_va + 0x4E..0x6F`
    vm.pc = site_va + 0x4E;
    vm.run_until(site_va + 0x6F, 32)?;
    Ok(vm.read_u32(phys_blocks_va)?)
}

pub(super) fn verify_hdd_capacity_unlock(
    main_factory: &[u8],
    main_patched: &[u8],
) -> Result<PatchVerificationCheck> {
    let site_off = find_hdd_cmd_clamp_site(main_factory)
        .context("Missing src/hdd_cmd.c clamp site in factory MAIN binary")?;
    let rep_pat = inspect_main_buffer_capacity(
        main_patched,
        MAIN_BASE_VA,
        "EXTRHEAD_UNPACKED",
        None,
        "patched",
    )?;
    if !rep_pat.is_dynamic_unclamped {
        bail!("Patched MAIN binary does not report DynamicUnclamped mode");
    }

    let factory_result = exec_hdd_cmd_block(main_factory, site_off, TEST_500GB_BLOCKS_2KB)?;
    let patched_result = exec_hdd_cmd_block(main_patched, site_off, TEST_500GB_BLOCKS_2KB)?;
    if factory_result != TEST_200GB_BLOCKS_2KB_M1 {
        bail!(
            "Unpatched MAIN failed to clamp 500GB disk to 200GB limit: got 0x{factory_result:08X}"
        );
    }
    if patched_result != TEST_500GB_BLOCKS_2KB - 1 {
        bail!("Patched MAIN failed to preserve 500GB (>200GB) drive: got 0x{patched_result:08X}");
    }

    Ok(PatchVerificationCheck {
        id: "hdd_resize_over_200gb_lba48".to_string(),
        passed: true,
        detail: format!(
            "500GB input (0x{TEST_500GB_BLOCKS_2KB:08X} blks): factory clamped=0x{factory_result:08X} (200GB), patched preserved=0x{patched_result:08X}"
        ),
    })
}
