use byteorder::{ByteOrder, LittleEndian};

use super::*;

fn encode_caller_stub(stub: &mut [u8], slot_off: usize, target_va: u32) {
    let rel = target_va
        .wrapping_sub(VA_MAIN_CALLER_STUB + slot_off as u32)
        .to_le_bytes();
    stub[slot_off..slot_off + 7]
        .copy_from_slice(&[0xDD, rel[0], rel[1], rel[2], rel[3], 0x00, 0x00]);
}

fn init_nfd_runner(
    profile: &MainBgoVmProfile,
    main_img: &[u8],
    nor_8mb: &[u8],
    allow_upper_nor_writes: bool,
) -> ItronTwoTaskRunner {
    let mut stub = vec![0xCBu8; 0x100];
    encode_caller_stub(&mut stub, 0, profile.nfd_init);
    encode_caller_stub(&mut stub, 16, profile.nfd_read);
    encode_caller_stub(&mut stub, 32, profile.nfd_write);

    let mut vm =
        map_main_bgo_vm_with_policy(profile, main_img, nor_8mb, stub, allow_upper_nor_writes);
    vm.sp = VA_MAIN_STACK_TOP;
    vm.pc = VA_MAIN_CALLER_STUB;
    let mut runner = ItronTwoTaskRunner::new(vm, *profile);
    runner
        .run_caller_until(VA_MAIN_CALLER_STUB + 7, 4_000_000)
        .unwrap_or_else(|e| panic!("MAIN{} NFD_Init failed:\n{e}", profile.build_id));
    assert_eq!(runner.vm.d[0], 0, "Retargeted NFD_Init() must return 0");
    runner
}

fn call_nfd_slot(
    runner: &mut ItronTwoTaskRunner,
    slot_off: u32,
    file_no: u32,
    buf_va: u32,
    stack_arg3: Option<u32>,
    label: &str,
) -> u32 {
    runner.vm.d[0] = file_no;
    runner.vm.d[1] = buf_va;
    runner.vm.sp = VA_MAIN_STACK_TOP;
    if let Some(arg3) = stack_arg3 {
        runner.vm.write_u32(VA_MAIN_STACK_TOP + 0x0C, arg3).unwrap();
    }
    runner.vm.pc = VA_MAIN_CALLER_STUB + slot_off;
    runner
        .run_caller_until(VA_MAIN_CALLER_STUB + slot_off + 7, 8_000_000)
        .unwrap_or_else(|e| panic!("{label}({file_no}) faulted:\n{e}"));
    runner.vm.d[0]
}

fn run_nfd_read_check(runner: &mut ItronTwoTaskRunner, file_no: u32, out_va: u32) {
    let rc = call_nfd_slot(runner, 16, file_no, out_va, None, "NFD_Read");
    assert_eq!(rc, 0, "Retargeted NFD_Read({file_no}) must return 0");
}

fn run_nfd_write_sequence(
    runner: &mut ItronTwoTaskRunner,
    profile: &MainBgoVmProfile,
    count: u8,
    xor_seed: u8,
    label: &str,
) -> (Vec<u8>, Option<(u8, u32)>) {
    run_nfd_read_check(runner, 22, VA_MAIN_OUT_BUF);
    let mut expected_payload = runner.vm.segment_bytes(VA_MAIN_OUT_BUF).unwrap()[..0x1F8].to_vec();
    for iter in 0u8..count {
        expected_payload[0] ^= xor_seed.wrapping_add(iter);
        for (idx, &byte) in expected_payload.iter().enumerate() {
            runner
                .vm
                .write_u8(VA_MAIN_OUT_BUF + idx as u32, byte)
                .unwrap();
        }
        let rc = call_nfd_slot(runner, 32, 22, VA_MAIN_OUT_BUF, Some(0x1F8), label);
        if rc != 0 {
            return (expected_payload, Some((iter, rc)));
        }
        assert_eq!(
            runner.vm.read_u32(profile.bgo_write_lock_va).unwrap(),
            0,
            "MAIN{} latched error lock after iter {iter} ({label})",
            profile.build_id
        );
    }
    (expected_payload, None)
}

fn verify_persisted_nfd22_payload(
    profile: &MainBgoVmProfile,
    main_img: &[u8],
    nor_img: &[u8],
    expected_payload: &[u8],
    ctx: &str,
    expect_zero_mutation: bool,
) {
    let (_, nor_in_sha) = calculate_hashes(nor_img);
    let (remount_bgo_rc, remount_bgo_sha) = call_main_va_with_policy(
        profile,
        main_img,
        nor_img,
        profile.bgo_init,
        (0, 0, 0),
        true,
    )
    .unwrap_or_else(|e| panic!("Fresh BGO_Init() ({ctx}) faulted: {e}"));
    assert_eq!(
        remount_bgo_rc, 0,
        "MAIN{} fresh BGO_Init() ({ctx}) must return 0",
        profile.build_id
    );
    if expect_zero_mutation {
        assert_eq!(
            remount_bgo_sha, nor_in_sha,
            "MAIN{} fresh BGO_Init() ({ctx}) mutated NOR bits",
            profile.build_id
        );
    }
    let mut fresh_runner = init_nfd_runner(profile, main_img, nor_img, true);
    let settled_nor = fresh_runner
        .vm
        .segment_bytes(AM34_FLASH_UNCACHED_BASE)
        .unwrap()
        .to_vec();
    let (_, fresh_init_sha) = calculate_hashes(&settled_nor);
    assert_eq!(
        fresh_init_sha, remount_bgo_sha,
        "MAIN{} fresh NFD_Init() ({ctx}) diverged from BGO_Init()",
        profile.build_id
    );
    if !expect_zero_mutation {
        let diffs: Vec<usize> = nor_img
            .iter()
            .zip(settled_nor.iter())
            .enumerate()
            .filter_map(|(off, (&a, &b))| (a != b).then_some(off))
            .collect();
        let outside_bgo: Vec<usize> = diffs
            .iter()
            .copied()
            .filter(|&off| !(0x13_0000..0x40_0000).contains(&off))
            .collect();
        assert!(
            outside_bgo.is_empty(),
            "MAIN{} ({ctx}) fresh BGO_Init() touched NOR outside BGO SA26..SA70: {:?}",
            profile.build_id,
            &outside_bgo[..outside_bgo.len().min(16)]
        );
        if let (Some(&first), Some(&last)) = (diffs.first(), diffs.last()) {
            println!(
                "[MAIN{}:{}] fresh remount settled {} bytes inside BGO (0x{:06X}..=0x{:06X})",
                profile.build_id,
                ctx,
                diffs.len(),
                first,
                last
            );
        }
        let (second_bgo_rc, second_bgo_sha) = call_main_va_with_policy(
            profile,
            main_img,
            &settled_nor,
            profile.bgo_init,
            (0, 0, 0),
            true,
        )
        .unwrap_or_else(|e| panic!("2nd BGO_Init() ({ctx}) faulted: {e}"));
        assert_eq!(
            second_bgo_rc, 0,
            "MAIN{} 2nd BGO_Init() ({ctx}) must return 0",
            profile.build_id
        );
        assert_eq!(
            second_bgo_sha, remount_bgo_sha,
            "MAIN{} 2nd BGO_Init() ({ctx}) was not idempotent",
            profile.build_id
        );
    }
    fresh_runner.vm.nor_write_policy = crate::pico::rsp_codec::policy::NorWritePolicy::NvramOnly;
    run_nfd_read_check(&mut fresh_runner, 22, VA_MAIN_OUT_BUF);
    assert_eq!(
        &fresh_runner.vm.segment_bytes(VA_MAIN_OUT_BUF).unwrap()[..0x1F8],
        expected_payload,
        "MAIN{} persisted NFD_Read(22) mismatch ({ctx})",
        profile.build_id
    );
}

fn prove_nfd_write_and_p1_advance(
    profile: &MainBgoVmProfile,
    synthesized: &OwnedMainBgoRetargetSpec,
    main_8mb: &[u8],
    nor_8mb: &[u8],
) {
    // Locate the active P1 (0xF101) sector, its first trailing blank subpage (> 0x1F), and virgin P5 spares.
    let p1_off = (0x13_0000usize..0x40_0000)
        .step_by(0x1_0000)
        .find(|&off| LittleEndian::read_u16(&nor_8mb[off + 4..off + 6]) == 0xF101)
        .expect("Active P1 (0xF101) sector missing from 8 MiB NOR");
    let next_blank_sp = (2..256usize)
        .rev()
        .take_while(|&sp| {
            nor_8mb[p1_off + sp * 0x100..p1_off + (sp + 1) * 0x100]
                .iter()
                .all(|&b| b == 0xFF)
        })
        .last()
        .expect("Active P1 sector has no trailing blank subpages");
    assert!(
        next_blank_sp > 0x1F && next_blank_sp < 0xFF,
        "Unexpected P1 next_blank_sp=0x{next_blank_sp:02X} at NOR 0x{p1_off:06X}"
    );
    let virgin_spare_offs: Vec<usize> = (0x13_0000usize..0x40_0000)
        .step_by(0x1_0000)
        .filter(|&off| LittleEndian::read_u16(&nor_8mb[off + 4..off + 6]) == 0xFFFF)
        .collect();
    assert!(
        !virgin_spare_offs.is_empty(),
        "No virgin P5 spare sector in 8 MiB NOR"
    );

    // Part A: Positive proof with full S1..S14: write 8x so P1 advances cleanly beyond sp=0x1F.
    let mut wr_runner = init_nfd_runner(profile, main_8mb, nor_8mb, true);
    let (expected_a, err_a) =
        run_nfd_write_sequence(&mut wr_runner, profile, 8, 0x5A, "NFD_Write[S1..S14]");
    assert!(
        err_a.is_none(),
        "MAIN{} Part A NFD_Write failed {:?} (lock={}, logs={:?})",
        profile.build_id,
        err_a,
        wr_runner.vm.read_u32(profile.bgo_write_lock_va).unwrap(),
        wr_runner.nfrom_logs
    );
    let nor_after = wr_runner
        .vm
        .segment_bytes(AM34_FLASH_UNCACHED_BASE)
        .unwrap()
        .to_vec();
    let adv_off = p1_off + next_blank_sp * 0x100;
    assert!(
        nor_after[adv_off..adv_off + 0x100]
            .iter()
            .any(|&b| b != 0xFF),
        "MAIN{} P1 @ 0x{p1_off:06X} did not advance into sp=0x{next_blank_sp:02X} (> 0x1F)",
        profile.build_id
    );
    run_nfd_read_check(&mut wr_runner, 22, VA_MAIN_OUT_BUF + 0x1000);
    assert_eq!(
        &wr_runner.vm.segment_bytes(VA_MAIN_OUT_BUF).unwrap()[0x1000..0x11F8],
        expected_a.as_slice(),
        "MAIN{} post-write NFD_Read(22) mismatch",
        profile.build_id
    );
    verify_persisted_nfd22_payload(profile, main_8mb, &nor_after, &expected_a, "Part A", true);

    // Part B: P1 sector rollover (`sp = 0xFF -> 0x100`) & P5 spare-sector allocation (`S13A + S13B`).
    let mut nor_rollover = nor_8mb.to_vec();
    for sp in next_blank_sp..=0xFEusize {
        let off = p1_off + sp * 0x100;
        nor_rollover[off..off + 4].copy_from_slice(&[0x80, 0x80, 0x80, 0x80]);
        nor_rollover[off + 4..off + 0x100].fill(0x00);
    }

    // Negative control (`main_no_s13a`): revert ONLY S13A (`mov 0x2000, a3` at `0x574A_FBB8..FBBA` shifted).
    let s13a_off = synthesized
        .byte_sites
        .iter()
        .find(|s| s.expect == 0x27 && s.replace == 0xCB)
        .expect("SSOT S13A byte site missing")
        .offset;
    let mut main_no_s13a = main_8mb.to_vec();
    main_no_s13a[s13a_off..s13a_off + 3].copy_from_slice(&[0x27, 0x00, 0x20]);

    let mut neg_runner = init_nfd_runner(profile, &main_no_s13a, &nor_rollover, true);
    let (_, neg_err) =
        run_nfd_write_sequence(&mut neg_runner, profile, 8, 0x70, "NFD_Write[no-S13A]");
    let (fail_iter, fail_rc) = neg_err.unwrap_or_else(|| {
        panic!(
            "MAIN{} without S13A unexpectedly survived P1 sp=0xFF rollover",
            profile.build_id
        )
    });
    let neg_lock = neg_runner.vm.read_u32(profile.bgo_write_lock_va).unwrap();
    let hit_bgo_err1 = neg_runner.nfrom_logs.iter().any(|l| {
        (l.contains("Status flag change error") || l.contains("File omap update error"))
            && l.contains("[0x1,")
    });
    let hit_rewrite_end_err = neg_runner
        .nfrom_logs
        .iter()
        .any(|l| l.contains("File rewrite end error") && l.contains("0xFFFFFFFD"));
    let hit_recovery_err19 = neg_runner
        .nfrom_logs
        .iter()
        .any(|l| l.contains("bgo_writeD") && l.contains("0xF07") && l.contains("0x13"));
    assert!(
        fail_rc == (-3i32 as u32)
            && hit_bgo_err1
            && hit_rewrite_end_err
            && hit_recovery_err19
            && neg_lock == 1,
        "MAIN{} without S13A iter {fail_iter} expected rc=-3 + BGO err[1] + File rewrite end error[-3] + bgo_writeD(0xF07) error(19) + lock=1, got rc={} lock={neg_lock} logs={:?}",
        profile.build_id,
        fail_rc as i32,
        neg_runner.nfrom_logs
    );
    // Immediate subsequent write must fail with rc=-2 (`error(19)` = `0x13`).
    let post_rc = call_nfd_slot(
        &mut neg_runner,
        32,
        22,
        VA_MAIN_OUT_BUF,
        Some(0x1F8),
        "NFD_Write[no-S13A-locked]",
    );
    assert_eq!(
        post_rc, -2i32 as u32,
        "MAIN{} locked write must return -2, got {}",
        profile.build_id, post_rc as i32
    );
    assert!(
        neg_runner
            .nfrom_logs
            .iter()
            .any(|l| l.contains("bgo_writeD") && l.contains("0x13")),
        "MAIN{} locked write missing bgo_writeD error(19=0x13): {:?}",
        profile.build_id,
        neg_runner.nfrom_logs
    );

    // Positive proof on `nor_rollover` with full S1..S14: 10 writes cross `sp=0xFF -> 0x100` cleanly.
    let mut ro_runner = init_nfd_runner(profile, main_8mb, &nor_rollover, true);
    let (expected_b, err_b) =
        run_nfd_write_sequence(&mut ro_runner, profile, 10, 0x80, "NFD_Write[rollover]");
    assert!(
        err_b.is_none(),
        "MAIN{} rollover NFD_Write failed {:?} (lock={}, logs={:?})",
        profile.build_id,
        err_b,
        ro_runner.vm.read_u32(profile.bgo_write_lock_va).unwrap(),
        ro_runner.nfrom_logs
    );

    let sec_map_plus1_va = synthesized
        .ram_relocs
        .iter()
        .find(|r| r.expected_count == 6)
        .expect("sector_map+1 relocation missing")
        .to;
    let p5_last_va = sec_map_plus1_va + (45 + 3) * 2;
    let p5_last_val = ro_runner.vm.read_u16(p5_last_va).unwrap();
    assert_eq!(
        p5_last_val, 0xFFFF,
        "MAIN{} S13B failed: P5[3] @ 0x{p5_last_va:08X} = 0x{p5_last_val:04X} (expected 0xFFFF)",
        profile.build_id
    );

    let nor_ro_after = ro_runner
        .vm
        .segment_bytes(AM34_FLASH_UNCACHED_BASE)
        .unwrap()
        .to_vec();
    let spare_activated = virgin_spare_offs.iter().any(|&off| {
        let lbn = LittleEndian::read_u16(&nor_ro_after[off + 4..off + 6]);
        lbn != 0xFFFF
            && nor_ro_after[off + 0x200..off + 0x300]
                .iter()
                .any(|&b| b != 0xFF)
    });
    assert!(
        spare_activated,
        "MAIN{} rollover did not allocate and program any P5 spare sector (SA41/67/68/70)",
        profile.build_id
    );
    verify_persisted_nfd22_payload(
        profile,
        main_8mb,
        &nor_ro_after,
        &expected_b,
        "Part B rollover",
        false,
    );
}

pub(crate) fn prove_main_bgo_8mb_retarget_against_real_eh55_nor(
    profile: &MainBgoVmProfile,
    main_orig: Vec<u8>,
    nor_8mb: Vec<u8>,
) {
    let (_, build) = find_main_build_by_id(profile.build_id)
        .unwrap_or_else(|| panic!("SSOT missing build {}", profile.build_id));
    let synthesized = build_main_bgo_retarget_spec_from_ssot(build).unwrap();
    assert!(!synthesized.word_sites.is_empty());

    let (_, orig_sha) = calculate_hashes(&main_orig);
    let (_, nor_before_sha) = calculate_hashes(&nor_8mb);

    // 1. Negative control: unpatched MAIN fails BGO_Init() against 8 MiB EH55 NOR.
    let (neg_rc, _) = call_main_va_with_policy(
        profile,
        &main_orig,
        &nor_8mb,
        profile.bgo_init,
        (0, 0, 0),
        true,
    )
    .unwrap_or_else(|e| panic!("Unpatched MAIN{} BGO_Init() faulted: {e}", profile.build_id));
    assert_eq!(
        neg_rc, 1,
        "Unpatched MAIN{} must reproduce live BGO driver init failed[1] against real 8 MiB NOR",
        profile.build_id
    );

    // 2. Apply SSOT 4 MiB -> 8 MiB retarget in place and verify BGO_Init(), NFD_Init(), and NFD_Read(17/22/12).
    let mut main_8mb = main_orig;
    assert_eq!(
        inspect_or_verify_main_bgo_geometry(&main_8mb)
            .unwrap()
            .state,
        MAIN_BGO_STATE_FACTORY_4MB
    );
    apply_main_bgo_retarget_in_place(&mut main_8mb, true).unwrap();
    assert_eq!(
        inspect_or_verify_main_bgo_geometry(&main_8mb)
            .unwrap()
            .state,
        MAIN_BGO_STATE_8MB
    );

    let (bgo_rc, bgo_nor_sha) = call_main_va_with_policy(
        profile,
        &main_8mb,
        &nor_8mb,
        profile.bgo_init,
        (0, 0, 0),
        true,
    )
    .expect("Retargeted BGO_Init() failed inside Mn103Vm");
    assert_eq!(
        bgo_rc, 0,
        "Retargeted MAIN{} BGO_Init() must return 0",
        profile.build_id
    );
    assert_eq!(
        bgo_nor_sha, nor_before_sha,
        "Retargeted BGO_Init() mutated NOR bits on clean 8 MiB image"
    );

    let mut runner = init_nfd_runner(profile, &main_8mb, &nor_8mb, true);
    assert_ne!(
        runner.vm.read_u32(profile.flag_init_va).unwrap(),
        0,
        "NFD initialized flag must be non-zero"
    );
    assert_eq!(
        runner.vm.read_u16(profile.mfr_id_va).unwrap(),
        0x0001,
        "Spansion manufacturer ID must be autodetected cleanly"
    );
    assert_eq!(
        runner.vm.read_u16(profile.dev_id_va).unwrap(),
        0x2202,
        "S29JL064H extended device ID cycle 2 stored after reading 0x227E at +0x02"
    );
    let init_nor_sha =
        calculate_hashes(runner.vm.segment_bytes(AM34_FLASH_UNCACHED_BASE).unwrap()).1;
    assert_eq!(
        init_nor_sha, nor_before_sha,
        "Retargeted NFD_Init() mutated NOR bits on clean 8 MiB image"
    );

    runner.vm.nor_write_policy = crate::pico::rsp_codec::policy::NorWritePolicy::NvramOnly;
    run_nfd_read_check(&mut runner, 17, VA_MAIN_OUT_BUF);

    run_nfd_read_check(&mut runner, 22, VA_MAIN_OUT_BUF);
    assert!(
        runner.vm.segment_bytes(VA_MAIN_OUT_BUF).unwrap()[..0x1F8]
            .iter()
            .any(|&b| b != 0),
        "NFD_Read(22 'sys_hdd') produced all-zero buffer"
    );

    run_nfd_read_check(&mut runner, 12, VA_MAIN_OUT_BUF + 0x0800);
    assert!(
        runner.vm.segment_bytes(VA_MAIN_OUT_BUF).unwrap()[0x0800..0x0800 + 0x30E4]
            .iter()
            .any(|&b| b != 0),
        "NFD_Read(12 'rtsd1') produced all-zero buffer"
    );

    let read_nor_sha =
        calculate_hashes(runner.vm.segment_bytes(AM34_FLASH_UNCACHED_BASE).unwrap()).1;
    assert_eq!(
        read_nor_sha, nor_before_sha,
        "Retargeted NFD_Read mutated NOR bits"
    );

    // 3. Prove S12 (0x574B_44C0 / 0x574A_CA0A) enables live BGO writes & P1 subpage advance past sp=0x1F.
    prove_nfd_write_and_p1_advance(profile, &synthesized, &main_8mb, &nor_8mb);

    // 4. Reversible roundtrip back to exact original SHA-256.
    apply_main_bgo_retarget_in_place(&mut main_8mb, false).unwrap();
    assert_eq!(calculate_hashes(&main_8mb).1, orig_sha);
}

#[test]
fn mn103vm_proves_main3060np_bgo_8mb_retarget_against_real_eh55_nor() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let main_path =
        root.join("firmware_and_images/extracted_partitions/frm_unpacked/main3060np_unpacked.bin");
    let nor_path = root.join("firmware_and_images/eh55_nor_8mb_ic5001_repaired.bin");
    if !main_path.is_file() || !nor_path.is_file() {
        return;
    }
    prove_main_bgo_8mb_retarget_against_real_eh55_nor(
        &PROFILE_MAIN3060NP,
        std::fs::read(&main_path).unwrap(),
        std::fs::read(&nor_path).unwrap(),
    );
}

#[test]
fn mn103vm_proves_main3410pq_bgo_8mb_retarget_against_real_eh55_nor() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let main_ctr_path =
        root.join("firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin");
    let nor_path = root.join("firmware_and_images/eh55_nor_8mb_ic5001_repaired.bin");
    if !main_ctr_path.is_file() || !nor_path.is_file() {
        return;
    }
    let raw_ctr = std::fs::read(&main_ctr_path).unwrap();
    let main_orig = unpack_extrhead(&raw_ctr, ExtrheadCryptMode::Main)
        .unwrap()
        .decompressed;
    prove_main_bgo_8mb_retarget_against_real_eh55_nor(
        &PROFILE_MAIN3410PQ,
        main_orig,
        std::fs::read(&nor_path).unwrap(),
    );
}
