use super::*;

#[test]
fn resolves_all_seven_traps_and_am33_sleigh_gaps_in_synthetic_mn103s_images() {
    let mut buf = vec![0xCB_u8; 0x240];
    // Trap 4: 6-byte `mov imm32, a0` @ 0x40..0x46, targeted at +1 (`0x44000041`) by `bra` @ 0x60
    buf[0x40..0x46].copy_from_slice(&[0xFC, 0xCC, 0x11, 0x22, 0x33, 0x44]);
    let disp: i8 = -0x1F; // 0x60 + (-31) = 0x41
    buf[0x60..0x62].copy_from_slice(&[0xCA, disp as u8]);

    // Trap 5: CFI constant 0x84200554 @ 0x100
    buf[0x100..0x104].copy_from_slice(&0x8420_0554_u32.to_le_bytes());

    // Trap 1: 4-entry pointer array @ 0x120
    for (i, tgt) in [0x4400_0010_u32, 0x4400_0020, 0x4400_0030, 0x4400_0038]
        .iter()
        .enumerate()
    {
        buf[0x120 + i * 4..0x124 + i * 4].copy_from_slice(&tgt.to_le_bytes());
    }

    // Trap 6: SETLB (0xDB) + 4-byte loop body .. LNE (0xD9)
    buf[0x180] = 0xDB;
    buf[0x185] = 0xD9;

    // Trap 8 / AM33 Sleigh gap: `f9 ea c8` (MOVHU (A0+),D0) & `f9 3a 78 ff f0 fc` (pre-RETS 0xFF)
    buf[0x1A0..0x1A3].copy_from_slice(&[0xF9, 0xEA, 0xC8]);
    buf[0x1B0..0x1B6].copy_from_slice(&[0xF9, 0x3A, 0x78, 0xFF, 0xF0, 0xFC]);

    let extra = [InstructionSpan {
        start_va: 0x4400_0040,
        end_va: 0x4400_0046,
        mnemonic: "mov_imm32".to_string(),
    }];
    let overlaps = detect_mid_instruction_overlaps(&buf, 0x4400_0000, &extra);
    assert_eq!(overlaps.len(), 1);
    assert_eq!(overlaps[0].target_va, 0x4400_0041);

    let (genuine, mid) = filter_mid_instruction_constant_hits(&[0x4400_0042, 0x4400_0100], &extra);
    assert_eq!(genuine, vec![0x4400_0100]);
    assert_eq!(mid, vec![0x4400_0042]);

    let summary = analyze_seven_traps_in_slice(&buf, 0x4400_0000, 0x4400_0000, 0x4404_0000);
    assert_eq!(summary.cfi_constant_sites, vec![0x4400_0100]);
    assert_eq!(summary.orphan_code_pointers.len(), 4);
    assert_eq!(summary.setlb_loop_count, 1);
    assert_eq!(summary.am33_f9_ea_sites.len(), 1);
    assert_eq!(summary.am33_f9_ea_sites[0].operands, "(A0+),D0");
    assert_eq!(summary.am33_ff_pre_rets_sites, vec![0x4400_01B3]);
}

#[test]
fn parses_drv_cli_and_atapi_tables_with_pass_unlock_state() {
    let mut img = vec![0u8; 0x100];
    img[0..8].copy_from_slice(b"WB\0\0\0\0\0\0");
    img[8..12].copy_from_slice(&0x4013_4210_u32.to_le_bytes());
    img[12..16].copy_from_slice(&0x38_u32.to_le_bytes());
    img[20..28].copy_from_slice(b"PASS\0\0\0\0");
    img[28..32].copy_from_slice(&DRV_DECOMP_PASS_HANDLER_VA.to_le_bytes());

    let locked = parse_drv_cli_table(&img, 0, 0x400D_F5C0, 4, false);
    assert_eq!(locked.len(), 2);
    assert_eq!(locked[0].classification, "WRITE_BLOCKED");
    assert_eq!(locked[1].classification, "UNLOCK_AUTH");

    let unlocked = parse_drv_cli_table(&img, 0, 0x400D_F5C0, 4, true);
    assert_eq!(unlocked[0].classification, "UNLOCKED_WRITE");
}
