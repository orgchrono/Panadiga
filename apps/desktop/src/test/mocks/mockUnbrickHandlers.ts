/**
 * Mock IPC handlers for the Pre-MAIN DIGA Unbrick Workbench, SD Kit Builder, & TBC Passthrough.
 */

import type {
  FrmModuleExtractionReport,
  MainPassthroughReport,
  MainPassthroughRspPlan,
  NorDumperStubResult,
  SdMediaVerificationReport,
  SdUnbrickKitReport,
  UnbrickWorkspaceReport,
} from '@/models';

const MOCK_RSP_PLAN: MainPassthroughRspPlan = {
  handshake_hex: '0e0402',
  nak_byte_hex: '15',
  serial_state_base_va: '0x515B2E00',
  serial_callback_va: '0x515B2E40',
  isr_ctx_ptr_va: '0x5316AE8C',
  isr_saved_pc_offset: '0x64',
  passthrough_frames: [
    '$M5771ABFB,6:8001f0fccbcb#xx',
    '$M574F7B8C,4:ccd000cb#42',
    '$M574F7C5C,7:8000cbcbcbcbcb#7d',
  ],
  factory_restore_frames: [
    '$M5771ABFB,6:cfe0f8fed082#xx',
    '$M574F7B8C,4:bf03c110#xx',
    '$M574F7C5C,7:ddcd3c00000000#xx',
  ],
  callback_write_frame: '$M515B2E40,4:3c1f4050#95',
  pico_cli_shortcuts: [
    'PANARSP PASSTHROUGH',
    'PANARSP NORMAL MAGIC PASSTHROUGH',
    'PANARSP CALL:50401F3C',
  ],
};

const MOCK_SD_VERIFICATION: SdMediaVerificationReport = {
  sd_root_dir: 'sd_card_eh55',
  overall_status: 'READY_WITH_HW_WARNINGS',
  digadiag_present: true,
  digadiag_exact_size_ok: true,
  digadiag_dst_addr_hex: '0x50400000',
  digadiag_bgo_mode: 'BYPASS_CHECKS_ONLY',
  pana_sd_present: true,
  pana_sd_toc_entries: 3,
  pana_sd_list_sums_ok: true,
  pana_sd_prog061_mode: 'BYPASS_CHECKS_ONLY',
  pana_sd_main_build: '3410PQ',
  pana_sd_main_passthrough_mode: 'AV_PASSTHROUGH',
  pana_sd_main_hdd_dynamic: true,
  checks: [
    {
      id: 'digadiag_container',
      severity: 'pass',
      title: 'DIGADIAG.FRM (Stage-1.5 MN103S SDDIAG)',
      detail: 'size=131072 B (exact_128k=true), dst=0x50400000, adler_ok=true, bgo_mode=BYPASS_CHECKS_ONLY',
    },
    {
      id: 'pana_sd_container',
      severity: 'pass',
      title: 'PANA__SD.FRM (64B Plaintext SD TOC)',
      detail: 'entries=3, list_sums_ok=true, tbc_mode=AV_PASSTHROUGH',
    },
    {
      id: 'sd_hw_91006',
      severity: 'warn',
      title: 'BOOT.BIN SD-91006 Hardware Gate (1-bit -> 4-bit ACMD6)',
      detail: 'Requires SDSC <= 2 GiB (FAT16) + DAT1..DAT3 traces on P6801/P6802.',
    },
  ],
};

const MOCK_MAIN_PASSTHROUGH: MainPassthroughReport = {
  file_size: 13_003_052,
  expected_size_match: true,
  detected_mode: 'FACTORY_ORIGINAL',
  sites: [
    {
      name: 'fpc_status_ctrl',
      description: 'Front-panel status control -> mov 1, d0; ret; nop; nop',
      va_hex: '0x5771ABFB',
      file_offset_hex: '0x71ABFB',
      length: 6,
      state: 'FACTORY_ORIGINAL',
      current_bytes_hex: 'cfe0f8fed082',
      rsp_passthrough_frame: '$M5771ABFB,6:8001f0fccbcb#76',
      rsp_factory_frame: '$M5771ABFB,6:cfe0f8fed082#9c',
    },
    {
      name: 'odd_state_branch',
      description: 'ODD state check branch -> jmp16 +0x00D0 (0x574F7C5C); nop',
      va_hex: '0x574F7B8C',
      file_offset_hex: '0x4F7B8C',
      length: 4,
      state: 'FACTORY_ORIGINAL',
      current_bytes_hex: 'bf03c110',
      rsp_passthrough_frame: '$M574F7B8C,4:ccd000cb#42',
      rsp_factory_frame: '$M574F7B8C,4:bf03c110#8d',
    },
    {
      name: 'odd_init_status',
      description: 'ODD init return status -> clr d0; nop x5',
      va_hex: '0x574F7C5C',
      file_offset_hex: '0x4F7C5C',
      length: 7,
      state: 'FACTORY_ORIGINAL',
      current_bytes_hex: 'ddcd3c00000000',
      rsp_passthrough_frame: '$M574F7C5C,7:8000cbcbcbcbcb#7d',
      rsp_factory_frame: '$M574F7C5C,7:ddcd3c00000000#2a',
    },
  ],
  output_path: null,
  applied_mode: null,
};

const MOCK_UNBRICK_WORKSPACE: UnbrickWorkspaceReport = {
  workspace_root: 'C:/workspace/Panadiga',
  discovered_pana_dvd: 'firmware_and_images/PANA_DVD.FRM',
  discovered_prog061: 'firmware_and_images/extracted_partitions/frm_unpacked/prog061_dec.bin',
  discovered_main_3410pq: 'firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin',
  discovered_drv: 'firmware_and_images/extracted_partitions/frm_unpacked/drv_s211_dec.bin',
  discovered_sd_dir: 'sd_card_eh55',
  prog061_bgo: {
    raw_size: 262_144,
    raw_sha256: 'c7da9121174fcf4c000000000000000000000000000000000000000000000000',
    detected_mode: 'FACTORY_ORIGINAL',
    bgo_sum_site: { va_hex: '0x504230F0', file_offset_hex: '0x0230F0', state: 'FACTORY', bytes_hex: 'c804' },
    devinfo_site: { va_hex: '0x50405097', file_offset_hex: '0x005097', state: 'FACTORY', bytes_hex: 'c808' },
    bgo_init_hook_site: { va_hex: '0x5040A2CD', file_offset_hex: '0x00A2CD', state: 'FACTORY', bytes_hex: 'fcdf...' },
  },
  main_passthrough: MOCK_MAIN_PASSTHROUGH,
  main_capacity: {
    patch_site_va_hex: '0x574DD1FC',
    is_dynamic_unclamped: false,
    effective_default_limit_gb: 200.0,
  },
  rsp_plan: MOCK_RSP_PLAN,
  sd_verification: MOCK_SD_VERIFICATION,
  hardware_facts: [
    {
      topic: 'DIGADIAG.FRM vs PANA__SD.FRM / PANA_DVD.FRM',
      verdict: 'DISTINCT_STAGE15_VS_STAGE2_CONTAINERS',
      evidence:
        'DIGADIAG.FRM is fixed 128 KiB plaintext EXTRHEAD ("FVU ", 0x50400000); PANA__SD.FRM holds 64B plaintext TOC modules; PANA_DVD.FRM adds full Feistel encryption.',
    },
    {
      topic: 'Forcing SD 1-bit Mode via Pico P7501 (-91006 Bypass)',
      verdict: 'NOT_POSSIBLE_OVER_P7501_UART',
      evidence:
        'BOOT.BIN checks 1st GetSD_Status in 1-bit then switches to 4-bit via ACMD6 @ 0x44027F79 without UART console; use SDSC <=2GB FAT16 + DAT1..DAT3 repair.',
    },
    {
      topic: 'Standalone Digital PCB TBC Operation Outside Drive',
      verdict: 'ODD_HDD_REMOVABLE_MAIN_PCB_REQUIRED',
      evidence:
        'ODD + HDD can be eliminated via MAIN TBC Passthrough + CF/SSD, but P59001 still needs Main PCB P7402 (IC3001 video clamps/75R + IC7501 TBUS + 8 rails).',
    },
  ],
  digadiag_vs_pana_notes: [
    'DIGADIAG.FRM -> BOOT.BIN (NOR 0x44000000) loads 128 KiB plaintext EXTRHEAD directly.',
    'PANA__SD.FRM -> PROG061 reads 64B plaintext TOC slots for SD Update.',
  ],
};

export function handleMockUnbrickCommand(
  cmd: string,
  args?: Record<string, unknown>
): unknown | undefined {
  switch (cmd) {
    case 'inspect_unbrick_workbench':
      return MOCK_UNBRICK_WORKSPACE;

    case 'extract_frm_modules_command': {
      const outDir = (args?.outDir as string) || (args?.out_dir as string) || 'unbrick_extracted';
      const rep: FrmModuleExtractionReport = {
        source_frm_path: (args?.frmPath as string) || 'firmware_and_images/PANA_DVD.FRM',
        container_format: 'PANA_DVD_48B_FEISTEL',
        output_dir: outDir,
        modules: [
          {
            name: 'DRV S211',
            offset_hex: '0x00001800',
            encrypted_size: 720_896,
            decrypted_path: `${outDir}/drv_s211_dec.bin`,
            decrypted_sha256: 'aa11'.repeat(16),
            unpacked_path: `${outDir}/drv_s211_unpacked.bin`,
            unpacked_size: 1_441_792,
            dst_addr_hex: '0x90202000',
            checksum_valid: true,
          },
          {
            name: 'PROG061',
            offset_hex: '0x000B1800',
            encrypted_size: 262_144,
            decrypted_path: `${outDir}/prog061_dec.bin`,
            decrypted_sha256: 'c7da'.repeat(16),
            unpacked_path: null,
            unpacked_size: null,
            dst_addr_hex: null,
            checksum_valid: null,
          },
        ],
      };
      return rep;
    }

    case 'prepare_sd_unbrick_kit_command': {
      const req = (args?.request as Record<string, unknown>) || {};
      const outDir = (req.output_dir as string) || 'unbrick_sd_kit';
      const bgoMode = (req.bgo_mode as string) || 'bypass';
      const passthroughMode = (req.main_passthrough_mode as string) || 'av_passthrough';
      const includeSd = req.include_pana_sd !== false;
      const unlockDyn = req.unlock_hdd_dynamic !== false;
      const rep: SdUnbrickKitReport = {
        output_dir: outDir,
        prog061_source: 'firmware_and_images/PANA_DVD.FRM',
        main_source: includeSd ? 'firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin' : null,
        drv_source: includeSd ? 'drv_s211_dec.bin' : null,
        bgo_mode_applied: bgoMode,
        main_passthrough_applied: includeSd ? passthroughMode : null,
        hdd_capacity_applied: includeSd && unlockDyn ? 'dynamic' : null,
        digadiag_path: `${outDir}/DIGADIAG.FRM`,
        digadiag_sha256: 'deadbeef'.repeat(8),
        pana_sd_path: includeSd ? `${outDir}/PANA__SD.FRM` : null,
        pana_sd_sha256: includeSd ? 'cafebabe'.repeat(8) : null,
        sha256sums_path: `${outDir}/SHA256SUMS.txt`,
        verification: { ...MOCK_SD_VERIFICATION, sd_root_dir: outDir, pana_sd_present: includeSd },
      };
      return rep;
    }

    case 'verify_sd_unbrick_media_command': {
      const sdDir = (args?.sdRootDir as string) || (args?.sd_root_dir as string) || 'sd_card_eh55';
      return { ...MOCK_SD_VERIFICATION, sd_root_dir: sdDir };
    }

    case 'patch_main_passthrough_command': {
      const action = ((args?.action as string) || 'inspect').toLowerCase();
      if (action === 'rsp_plan') {
        const cb = args?.callbackTargetVa as string | undefined;
        return {
          ...MOCK_RSP_PLAN,
          callback_write_frame: cb ? `$M515B2E40,4:${cb.replace(/^0x/i, '').toLowerCase()}#00` : null,
        };
      }
      return {
        ...MOCK_MAIN_PASSTHROUGH,
        detected_mode: action === 'patch' ? 'AV_PASSTHROUGH' : MOCK_MAIN_PASSTHROUGH.detected_mode,
        output_path: action === 'patch' ? ((args?.outPath as string) || 'EH55_MAIN_PASSTHROUGH.bin') : null,
        applied_mode: action === 'patch' ? ((args?.mode as string) || 'av_passthrough') : null,
      };
    }

    case 'build_nor_dumper_stub_command': {
      const wrap = Boolean(args?.wrapAsDigadiag ?? args?.wrap_as_digadiag);
      const rep: NorDumperStubResult = {
        start_addr: (args?.startAddr as string) || '0x84000000',
        total_len_bytes: 8_388_608,
        block_size: 4096,
        load_addr: (args?.loadAddr as string) || '0x51800000',
        raw_stub_bytes: 168,
        wrapped_digadiag: wrap,
        output_bytes: wrap ? 131_072 : 168,
        written_path: (args?.outPath as string) || null,
        stub_hex: '44500051800000...',
      };
      return rep;
    }

    default:
      return undefined;
  }
}
