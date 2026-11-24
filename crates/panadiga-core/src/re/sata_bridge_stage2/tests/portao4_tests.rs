use byteorder::{ByteOrder, LittleEndian};

use super::*;

struct Portao4Eh55Fixtures {
    root: PathBuf,
    loader: Vec<u8>,
    base_nor: Vec<u8>,
    stage2_prog: Vec<u8>,
    stage2_sha: String,
}

fn load_portao4_eh55_fixtures() -> Option<Portao4Eh55Fixtures> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let loader_path = root.join("firmware_and_images/eh55_loader_ver061_256k.bin");
    let nor_path = root.join("firmware_and_images/eh55_nor_8mb_ic5001_repaired.bin");
    let prog_path =
        root.join("firmware_and_images/extracted_partitions/frm_unpacked/prog061_dec.bin");
    if !loader_path.is_file() || !nor_path.is_file() || !prog_path.is_file() {
        return None;
    }
    let loader = std::fs::read(&loader_path).unwrap();
    let base_nor = std::fs::read(&nor_path).unwrap();
    let (stage2_prog, stage2_sha) = build_portao4_stage2_prog(&prog_path);
    Some(Portao4Eh55Fixtures {
        root,
        loader,
        base_nor,
        stage2_prog,
        stage2_sha,
    })
}

#[test]
fn mn103vm_proves_portao4_loader_ver061_norfrom_boot_and_escape_hatch() {
    use crate::constants::{
        AM34_FLASH_CRITICAL_IPL_SIZE, AM34_FLASH_SIZE, AM34_FLASH_UNCACHED_BASE,
    };
    use crate::pico::rsp_codec::policy::NorWritePolicy;

    let Some(Portao4Eh55Fixtures {
        loader,
        base_nor,
        stage2_prog,
        stage2_sha,
        ..
    }) = load_portao4_eh55_fixtures()
    else {
        return;
    };
    assert_eq!(loader.len(), PROG061_EXPECTED_SIZE);
    assert_eq!(base_nor.len(), AM34_FLASH_SIZE as usize);
    // Confirm SA11..SA14 (0x40000..0x80000) is virgin 0xFF and stock BGO lacks "fvu.exe".
    assert!(base_nor[0x4_0000..0x8_0000].iter().all(|&b| b == 0xFF));
    assert!(
        !base_nor[0x13_0000..0x40_0000]
            .windows(7)
            .any(|w| w == b"fvu.exe")
    );

    let run_norfrom_stage = |fvu_exists_rc: i32| -> (u32, Vec<u8>, Vec<u8>, u32) {
        let mut loader_img = loader.clone();
        // Patch only FUN_44005DEE (uart0_printf) and FUN_4400A1F1 (NFD_FileExists) to pure MN103S RET/stub.
        // Keep real Loader memcpy (0x4402A34A), memcmp (0x4402A30A), and 0x4400894B..0x44008B98 intact!
        loader_img[0x5DEE..0x5DF1].copy_from_slice(&[0xDF, 0x00, 0x00]); // RET [], 0
        let stub_exists = if fvu_exists_rc == 0 {
            vec![0x00u8, 0xDF, 0x00, 0x00] // CLR D0; RET [], 0
        } else {
            vec![0x2C, 0xFF, 0xFF, 0xDF, 0x00, 0x00] // MOV 0xFFFF, D0; RET [], 0
        };
        loader_img[0xA1F1..0xA1F1 + stub_exists.len()].copy_from_slice(&stub_exists);

        let mut nor_cow = base_nor.clone();
        // Program Stage-2 PROG061 in SA11..SA14 (0x40000..0x80000) in CoW memory only.
        nor_cow[0x4_0000..0x8_0000].copy_from_slice(&stage2_prog);
        let ipl_sha_before = calculate_hashes(&nor_cow[..AM34_FLASH_CRITICAL_IPL_SIZE as usize]).1;

        let mut data_seg = vec![0u8; 0x2000];
        data_seg[..0xB38].copy_from_slice(&loader[0x3_52A0..0x3_5DD8]);
        assert_eq!(&data_seg[0xAF..0xB7], b"fvu.exe\0");

        let mut vm = Mn103Vm::new();
        vm.nor_write_policy = NorWritePolicy::NvramOnly;
        vm.map_segment(0x4400_0000, loader_img, false);
        vm.map_segment(AM34_FLASH_UNCACHED_BASE, nor_cow, false);
        vm.map_segment(0x5048_0000, data_seg, true);
        vm.map_segment(0x9040_0000, vec![0xAAu8; PROG061_EXPECTED_SIZE], true);
        vm.map_segment(0x50FF_0000, vec![0u8; 0x1_0000], true);
        let frame_a3 = 0x50FF_F800u32;
        vm.a[3] = frame_a3;
        vm.sp = 0x50FF_F700;
        vm.pc = 0x4400_894B;

        let stop_pc = if fvu_exists_rc == 0 {
            0x4400_8B98
        } else {
            0x4400_89F5
        };
        vm.run_until(stop_pc, 4_000_000)
            .unwrap_or_else(|e| panic!("Loader ver0.61 Nor-FROM run failed: {e}"));
        let loaded_flag = vm.read_u32(frame_a3 - 12).unwrap();
        let ram_copy = vm.segment_bytes(0x9040_0000).unwrap().to_vec();
        let ipl_sha_after = calculate_hashes(
            &vm.segment_bytes(AM34_FLASH_UNCACHED_BASE).unwrap()
                [..AM34_FLASH_CRITICAL_IPL_SIZE as usize],
        )
        .1;
        assert_eq!(
            ipl_sha_before, ipl_sha_after,
            "IPL SA0..SA10 must remain bit-exact"
        );
        let nor_sa11_14 =
            vm.segment_bytes(AM34_FLASH_UNCACHED_BASE).unwrap()[0x4_0000..0x8_0000].to_vec();
        (vm.pc, ram_copy, nor_sa11_14, loaded_flag)
    };

    // 1. Negative control: when NFD finds no "fvu.exe" marker, Loader skips NOR copy and branches to Update-Disk.
    let (neg_pc, neg_ram, _, neg_flag) = run_norfrom_stage(-1);
    assert_eq!(neg_pc, 0x4400_89F5);
    assert_eq!(neg_flag, 0);
    assert!(neg_ram.iter().all(|&b| b == 0xAA));

    // 2. Positive Portao 4 proof: real Loader memcpy/memcmp copies SA11..SA14 -> 0x9040_0000 byte-for-byte.
    let (pos_pc, pos_ram, pos_nor, pos_flag) = run_norfrom_stage(0);
    assert_eq!(pos_pc, 0x4400_8B98);
    assert_eq!(pos_flag, 1);
    assert_eq!(calculate_hashes(&pos_ram).1, stage2_sha);
    assert_eq!(pos_ram, pos_nor);

    // 3. Chain into copied Stage-2 payload: without SD boots HDD MAIN; with PANA__SD.FRM hits Nor-FROM escape hatch.
    assert_stage2_ram_boots_hdd_and_escapes(&pos_ram, NorFromTestState::RawStage2);
}

#[allow(clippy::too_many_arguments)]
fn prove_self_unpacker_phase_c(
    loader_stubbed: &[u8],
    data_seg: &[u8],
    nor_backing: &[u8],
    packed_ram_256k: Vec<u8>,
    stage2_prog: &[u8],
    stage2_sha: &str,
    label: &str,
) -> Vec<u8> {
    use crate::constants::AM34_FLASH_UNCACHED_BASE;
    use crate::pico::rsp_codec::policy::NorWritePolicy;

    let expected_nor_sha = calculate_hashes(nor_backing).1;
    let mut data_ram = vec![0u8; 0x8_0000];
    data_ram[..data_seg.len()].copy_from_slice(data_seg);
    let mut chctr = vec![0u8; 0x100];
    chctr[0x70..0x72].copy_from_slice(&0x0003u16.to_le_bytes());

    let mut vm = Mn103Vm::new();
    vm.nor_write_policy = NorWritePolicy::NvramOnly;
    vm.map_segment(0x4400_0000, loader_stubbed.to_vec(), false);
    vm.map_segment(AM34_FLASH_UNCACHED_BASE, nor_backing.to_vec(), true);
    vm.map_segment(0x5048_0000, data_ram, true);
    vm.map_segment(0x9040_0000, packed_ram_256k.clone(), true);
    vm.map_segment(0x5040_0000, packed_ram_256k, false);
    vm.map_segment(0x9100_0000, vec![0u8; PROG061_EXPECTED_SIZE], true);
    vm.map_segment(0x9104_0000, vec![0u8; 0x100], true);
    vm.map_segment(0x50FF_0000, vec![0u8; 0x1_0000], true);
    vm.map_segment(0xC000_0000, chctr, true);
    vm.sp = 0x50FF_F800;
    vm.pc = 0x5040_0000;

    let mut saw_tramp = false;
    for _ in 0..20_000_000usize {
        if vm.pc == 0x9104_0000 {
            saw_tramp = true;
        }
        if saw_tramp && vm.pc == 0x5040_0000 {
            break;
        }
        vm.step()
            .unwrap_or_else(|e| panic!("{label} faulted @ 0x{:08X}: {e:?}", vm.pc));
    }
    assert!(
        saw_tramp,
        "{label} did not jump through 0x9104_0000 trampoline"
    );
    assert_eq!(
        vm.pc, 0x5040_0000,
        "{label} failed to return to 0x5040_0000"
    );
    assert_eq!(vm.read_u16(0xC000_0070).unwrap(), 0x0030);
    assert_eq!(
        calculate_hashes(vm.segment_bytes(AM34_FLASH_UNCACHED_BASE).unwrap()).1,
        expected_nor_sha,
        "{label} must cause ZERO NOR writes"
    );
    let unpacked_ram = vm.segment_bytes(0x9040_0000).unwrap().to_vec();
    assert_eq!(calculate_hashes(&unpacked_ram).1, stage2_sha);
    assert_eq!(unpacked_ram.as_slice(), stage2_prog);
    unpacked_ram
}

#[test]
fn mn103vm_proves_portao4_full_loader_ver061_case3_zero_u99_zero_sd_boot() {
    use std::collections::VecDeque;

    use crate::constants::{AM34_FLASH_CRITICAL_IPL_SIZE, AM34_FLASH_UNCACHED_BASE};
    use crate::pico::rsp_codec::policy::NorWritePolicy;
    use crate::re::eh55_bgo::build_portao4_norfrom_cow_image;
    use crate::re::region_compat::TvStandardKind;

    let Some(Portao4Eh55Fixtures {
        root,
        loader,
        base_nor,
        stage2_prog,
        stage2_sha,
    }) = load_portao4_eh55_fixtures()
    else {
        return;
    };
    let (cow_nor, report) =
        build_portao4_norfrom_cow_image(&base_nor, &stage2_prog, Some(TvStandardKind::Ntsc))
            .unwrap();

    assert!(report.post_audit.healthy);
    assert!(report.post_audit.corrupt_sas.is_empty());
    assert_eq!(report.post_audit.l2p_tag_mismatches, 0);
    assert_eq!(report.post_audit.fat_mirror_mismatches, 0);
    assert_eq!(report.config1_page9_pid, Some(0x000C));
    assert!(report.sa13_14_preserved_blank);
    assert!(report.sa11_12_compressed_bytes > 0);
    assert!(cow_nor[0x6_0000..0x8_0000].iter().all(|&b| b == 0xFF));
    assert_eq!(
        calculate_hashes(&cow_nor[..AM34_FLASH_CRITICAL_IPL_SIZE as usize]).1,
        report.ipl_sha256
    );
    assert_eq!(
        calculate_hashes(&base_nor[..AM34_FLASH_CRITICAL_IPL_SIZE as usize]).1,
        report.ipl_sha256
    );
    let cow_nor_sha = calculate_hashes(&cow_nor).1;

    let mut loader_stubbed = loader.clone();
    for va in [
        0x4400_5DEEu32,
        0x4400_7813,
        0x4400_7BA6,
        0x4400_7C17,
        0x4400_7C3F,
        0x4401_1340,
        0x4400_6AD7,
        0x4400_B1D5,
        0x4400_BF91,
        0x4400_D30E,
        0x4400_CC5F,
    ] {
        let off = (va - 0x4400_0000) as usize;
        loader_stubbed[off..off + 4].copy_from_slice(&[0x00, 0xDF, 0x00, 0x00]);
    }
    let off_devinfo = (0x4400_AD0Cu32 - 0x4400_0000) as usize;
    loader_stubbed[off_devinfo..off_devinfo + 6]
        .copy_from_slice(&[0x2C, 0x01, 0x00, 0xDF, 0x00, 0x00]);

    let caller_base = 0x5080_F000u32;
    let mut caller_stub = vec![0xCBu8; 0x100];
    let rel_init = 0x4400_A08Fu32.wrapping_sub(caller_base).to_le_bytes();
    caller_stub[0..7].copy_from_slice(&[
        0xDD,
        rel_init[0],
        rel_init[1],
        rel_init[2],
        rel_init[3],
        0x00,
        0x00,
    ]);
    let rel_disp = 0x4400_7F0Eu32.wrapping_sub(caller_base + 16).to_le_bytes();
    caller_stub[16..23].copy_from_slice(&[
        0xDD,
        rel_disp[0],
        rel_disp[1],
        rel_disp[2],
        rel_disp[3],
        0x00,
        0x00,
    ]);

    let tripwires = [
        (0x4400_6C70u32, "vfd_display_string"),
        (0x4400_9690u32, "u99_halt"),
        (0x4400_9BEDu32, "cleanup_bgo_temp_files"),
        (0x4400_B980u32, "HDD_MAX_LBA"),
        (0x4400_8BC5u32, "media_update_gate"),
    ];

    for startup_reason in [0x70u8, 0x41u8, 0x20u8] {
        let mut data_ram = vec![0u8; 0x8_0000];
        data_ram[..0xB38].copy_from_slice(&loader[0x3_52A0..0x3_5DD8]);

        let mut vm = Mn103Vm::new();
        vm.nor_write_policy = NorWritePolicy::UpperNorAppUnlocked;
        vm.map_segment(0x4400_0000, loader_stubbed.clone(), false);
        vm.map_segment(AM34_FLASH_UNCACHED_BASE, cow_nor.clone(), true);
        vm.map_segment(0xDB00_0000, vec![0u8; 0x1000], true);
        vm.map_segment(0x8980_0000, vec![0u8; 0x1000], true);
        vm.map_segment(0x5048_0000, data_ram, true);
        vm.map_segment(0x9040_0000, vec![0xAAu8; PROG061_EXPECTED_SIZE], true);
        vm.map_segment(0x50FF_0000, vec![0u8; 0x1_0000], true);
        vm.map_segment(caller_base, caller_stub.clone(), false);

        let run_phase = |vm: &mut Mn103Vm,
                         stops: &[u32],
                         budget: usize,
                         label: &str,
                         require_tramp: bool|
         -> [usize; 5] {
            let mut hits = [0usize; 5];
            let mut saw_tramp = !require_tramp;
            let mut ring = VecDeque::with_capacity(24);
            for _ in 0..budget {
                if vm.pc == 0x9104_0000 {
                    saw_tramp = true;
                }
                if saw_tramp && stops.contains(&vm.pc) {
                    return hits;
                }
                if ring.len() == 24 {
                    ring.pop_front();
                }
                ring.push_back(vm.pc);
                if let Some((idx, _)) = tripwires
                    .iter()
                    .enumerate()
                    .find(|(_, (va, _))| *va == vm.pc)
                {
                    hits[idx] += 1;
                }
                if vm.pc == 0x4400_CC5F {
                    vm.write_u8(vm.d[0] + 1, startup_reason).unwrap();
                }
                vm.step().unwrap_or_else(|e| {
                    let hist: Vec<String> = ring.iter().map(|p| format!("0x{p:08X}")).collect();
                    panic!(
                        "{label} (reason=0x{startup_reason:02X}) faulted: {e:?}\nlast_pcs=[{}]",
                        hist.join(", ")
                    );
                });
            }
            panic!(
                "{label} (reason=0x{startup_reason:02X}) exceeded {budget} steps @ 0x{:08X}",
                vm.pc
            );
        };

        vm.sp = 0x50FF_F800;
        vm.pc = caller_base;
        let hits_a = run_phase(
            &mut vm,
            &[caller_base + 7],
            8_000_000,
            "Phase A bgo_init",
            false,
        );
        assert_eq!(hits_a, [0; 5]);
        assert_eq!(vm.d[0], 0, "bgo_init(0x4400A08F) must return 0");
        assert_ne!(vm.read_u32(0x5048_0260).unwrap(), 0);
        assert_eq!(
            calculate_hashes(vm.segment_bytes(AM34_FLASH_UNCACHED_BASE).unwrap()).1,
            cow_nor_sha,
            "Phase A bgo_init must change ZERO NOR bits across 8 MiB flash"
        );

        vm.nor_write_policy = NorWritePolicy::NvramOnly;
        vm.sp = 0x50FF_F800;
        vm.d[0] = 0;
        vm.pc = caller_base + 16;
        let hits_b = run_phase(
            &mut vm,
            &[0x4400_8BBB, 0x5040_0000],
            12_000_000,
            "Phase B fmu_sd_dispatcher_ver061",
            false,
        );
        for (idx, (_, name)) in tripwires.iter().enumerate() {
            assert_eq!(
                hits_b[idx], 0,
                "Tripwire {name} hit for reason 0x{startup_reason:02X}"
            );
        }
        assert_eq!(
            calculate_hashes(vm.segment_bytes(AM34_FLASH_UNCACHED_BASE).unwrap()).1,
            cow_nor_sha,
            "Cold boot must cause ZERO NOR writes"
        );
        let ram_copy = vm.segment_bytes(0x9040_0000).unwrap().to_vec();
        assert_eq!(ram_copy.as_slice(), &cow_nor[0x4_0000..0x8_0000]);

        if startup_reason == 0x70 {
            let unpacked_ram = prove_self_unpacker_phase_c(
                &loader_stubbed,
                &loader[0x3_52A0..0x3_5DD8],
                &cow_nor,
                ram_copy,
                &stage2_prog,
                &stage2_sha,
                "Etapa 2 Phase C SA11..SA12 self-unpacker",
            );
            assert_stage2_ram_boots_hdd_and_escapes(
                &unpacked_ram,
                NorFromTestState::Portao4Sa11Magic,
            );
        }
    }

    // Etapa 1 physical SD dry-run proof: identical universal 256 KiB self-unpacker loaded from SD
    // against virgin SA11..SA14 blank NOR (zero NOR writes, boots HDD even with PANA__SD.FRM in slot).
    let (sd_self_unpack_256k, _) =
        crate::re::eh55_bgo::build_portao4_self_unpacking_prog061(&stage2_prog).unwrap();
    let unpacked_sd = prove_self_unpacker_phase_c(
        &loader_stubbed,
        &loader[0x3_52A0..0x3_5DD8],
        &base_nor,
        sd_self_unpack_256k,
        &stage2_prog,
        &stage2_sha,
        "Etapa 1 SD dry-run self-unpacker (zero NOR writes)",
    );
    let mut h_sd_boot = Stage2VmHarness::new_with_nor_state(&unpacked_sd, NorFromTestState::Blank);
    h_sd_boot.vm.write_u32(VM_CFG_SD_INIT_RC, 0).unwrap();
    h_sd_boot.vm.write_u32(VM_CFG_FAT_MOUNT_RC, 1).unwrap();
    h_sd_boot.vm.write_u32(VM_CFG_PANA_OPEN_RC, 0xC8).unwrap();
    h_sd_boot.run(0x5700_0000).unwrap();
    assert!(
        h_sd_boot
            .calls()
            .iter()
            .any(|&(t, lba, off)| t == TAG_HDD_READ && lba == 0x10000 && off == 0),
        "Etapa 1 SD dry-run must NOT false-trigger Nor-FROM escape hatch and must boot HDD MAIN"
    );

    let main3060_path =
        root.join("firmware_and_images/extracted_partitions/frm_unpacked/main3060np_unpacked.bin");
    if main3060_path.is_file() {
        crate::re::main_bgo_retarget::tests::prove_main_bgo_8mb_retarget_against_real_eh55_nor(
            &crate::re::main_bgo_retarget::tests::PROFILE_MAIN3060NP,
            std::fs::read(&main3060_path).unwrap(),
            cow_nor.clone(),
        );
    }
    let main3410_ctr_path =
        root.join("firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin");
    if main3410_ctr_path.is_file() {
        use crate::crypto::{ExtrheadCryptMode, unpack_extrhead};
        let raw_ctr = std::fs::read(&main3410_ctr_path).unwrap();
        let main3410 = unpack_extrhead(&raw_ctr, ExtrheadCryptMode::Main)
            .unwrap()
            .decompressed;
        crate::re::main_bgo_retarget::tests::prove_main_bgo_8mb_retarget_against_real_eh55_nor(
            &crate::re::main_bgo_retarget::tests::PROFILE_MAIN3410PQ,
            main3410,
            cow_nor,
        );
    }
}

#[test]
fn ssot_stage2_spec_matches_ver061_contract_field_for_field() {
    let from_ssot =
        stage2_spec_for_loader_profile("ver061").expect("SSOT must resolve ver061 stage2 spec");
    assert_eq!(from_ssot.train_id, "loader_ver061_toc48_main57");
    assert_eq!(from_ssot.base_va, PROG061_BASE_VA);
    assert_eq!(from_ssot.expected_size, PROG061_EXPECTED_SIZE);
    assert_eq!(from_ssot.hook_offset, OFF_PROG061_STAGE2_HOOK_SITE);
    assert_eq!(from_ssot.hook_expect, ORIG_PROG061_STAGE2_HOOK_BYTES);
    assert_eq!(from_ssot.payload_offset, PROG061_STAGE2_PAYLOAD_OFFSET);
    assert!(from_ssot.require_reset_fix_1000ms);
}

#[test]
fn mn103vm_proves_main3410pq_stage2_bitwalk_and_hdd_boot() {
    use crate::constants::AM34_MAIN_BASE_VA;
    use crate::crypto::{ExtrheadCryptMode, pack_extrhead, unpack_extrhead};
    use crate::hdd::{HddCapacityPatchMode, apply_capacity_patch_in_place};
    use crate::re::main_bgo_retarget::{
        MAIN_BGO_STATE_8MB, MAIN_BGO_STATE_FACTORY_4MB, apply_main_bgo_retarget_in_place,
        inspect_or_verify_main_bgo_geometry,
    };
    use crate::re::main_passthrough::EXPECTED_MAIN_3410PQ_SIZE;
    use crate::re::sata_bridge_fix::{
        SATA_BRIDGE_STATE_WAIT_1000MS, apply_main_sata_bridge_reset_fix_in_place,
        inspect_main_sata_bridge_reset_site,
    };

    let part1 = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin");
    if !part1.is_file() {
        return;
    }
    let raw_ctr = std::fs::read(&part1).unwrap();
    let unpacked_meta = unpack_extrhead(&raw_ctr, ExtrheadCryptMode::Main).unwrap();
    let mut main_3410 = unpacked_meta.decompressed;
    assert_eq!(main_3410.len(), EXPECTED_MAIN_3410PQ_SIZE);

    // Verify real MAIN3410PQ *(0x5700_0008) pointer & 32-word power-of-two bit-walk table.
    let bitwalk_va = LittleEndian::read_u32(&main_3410[8..12]);
    assert!(
        bitwalk_va >= AM34_MAIN_BASE_VA && bitwalk_va & 3 == 0,
        "Invalid MAIN3410PQ bitwalk pointer 0x{bitwalk_va:08X}"
    );
    let bw_off = (bitwalk_va - AM34_MAIN_BASE_VA) as usize;
    for bit in 0..32usize {
        let word = LittleEndian::read_u32(&main_3410[bw_off + bit * 4..bw_off + bit * 4 + 4]);
        assert_eq!(
            word,
            1u32 << bit,
            "MAIN3410PQ bitwalk mismatch at word {bit} (off 0x{bw_off:X}): got 0x{word:08X}"
        );
    }

    // Apply universal 1000 ms SATA fix + 250 GB HDD capacity + 4 MiB -> 8 MiB BGO retarget + pack EXTRHEAD(0x94000000).
    apply_main_sata_bridge_reset_fix_in_place(&mut main_3410, SataBridgeResetFixMode::Wait1000Ms)
        .unwrap();
    assert_eq!(
        inspect_main_sata_bridge_reset_site(&main_3410)
            .unwrap()
            .state,
        SATA_BRIDGE_STATE_WAIT_1000MS
    );
    apply_capacity_patch_in_place(
        &mut main_3410,
        HddCapacityPatchMode::FixedCapacity,
        Some(250),
    )
    .unwrap();
    assert_eq!(
        inspect_or_verify_main_bgo_geometry(&main_3410)
            .unwrap()
            .state,
        MAIN_BGO_STATE_FACTORY_4MB
    );
    apply_main_bgo_retarget_in_place(&mut main_3410, true).unwrap();
    assert_eq!(
        inspect_or_verify_main_bgo_geometry(&main_3410)
            .unwrap()
            .state,
        MAIN_BGO_STATE_8MB
    );
    let packed = pack_extrhead(
        &main_3410,
        "MAIN",
        AM34_MAIN_BASE_VA,
        "3410PQ",
        ExtrheadCryptMode::Main,
        unpacked_meta.header.adler_step.max(16),
        unpacked_meta.header.flag.max(1),
    )
    .unwrap();
    let repacked = unpack_extrhead(&packed, ExtrheadCryptMode::Main).unwrap();
    assert_eq!(repacked.header.src_addr, 0x9400_0000);

    // Execute Stage-2 assembly in Mn103Vm with REAL MAIN3410PQ mapped at 0x5700_0000
    // (no synthetic bit-walk stub — Stage-2 walks MAIN3410PQ's real *(0x5700_0008) table).
    let prog = build_stage2_prog_fixture();
    let mut h =
        Stage2VmHarness::new_full(&prog, NorFromTestState::Blank, Some(repacked.decompressed));
    h.vm.write_u32(VM_CFG_ENTRY_VA, repacked.header.dst_addr)
        .unwrap();
    h.vm.write_u32(VM_CFG_PRIMARY_SRC, repacked.header.src_addr)
        .unwrap();
    h.run(AM34_MAIN_BASE_VA).unwrap();
    assert_eq!(h.vm.pc, AM34_MAIN_BASE_VA);
    let calls = h.calls();
    assert!(calls.iter().any(|&(t, _, _)| t == TAG_UNCOMP));
    assert!(calls.iter().any(|&(t, _, _)| t == TAG_DCACHE));
    assert!(calls.iter().any(|&(t, _, _)| t == TAG_STOP_TMR));
    assert!(calls.iter().any(|&(t, _, _)| t == TAG_DISABLE_IRQ));
}
