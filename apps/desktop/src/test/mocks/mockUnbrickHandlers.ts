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
        load_addr: (args?.loadAddr as string) || (wrap ? '0x50400000' : '0x51800000'),
        raw_stub_bytes: 168,
        wrapped_digadiag: wrap,
        output_bytes: wrap ? 131_072 : 168,
        written_path: (args?.outPath as string) || null,
        stub_hex: '44500050400000...',
      };
      return rep;
    }

    case 'run_matshita_boot_emu': {
      const scenario = String(args?.scenario ?? 'healthy_with_disc');
      const trayError = scenario === 'tray_cannot_close_80045300';
      const laserError = scenario === 'laser_apc_failure_80040904';
      const unlockFrm = scenario === 'unlock_disc_frm';
      return {
        scenario,
        inquiry_id: 'MATSHITA R6-MV4          ',
        steps: [
          {
            step_index: 1,
            boot_fn_va: '0x440062B8',
            boot_fn_name: 'atapi_test_unit_ready',
            cdb_hex: '00 00 00 00 00 00 00 00 00 00 00 00',
            sense_hex: trayError
              ? '70 00 04 00 00 00 00 0A 00 00 02 01 53 00 00 00 00 00'
              : '00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00',
            boot_log_line: trayError ? '** can not close tray **' : '[RTSD] Drive Ready',
            passed: !trayError,
          },
          {
            step_index: 2,
            boot_fn_va: '0x440067D0',
            boot_fn_name: unlockFrm ? 'disc_pana_dvd_frm_verify' : 'drive_startup_check',
            cdb_hex: '1B 00 00 00 03 00 01 00 00 00 00 00',
            sense_hex: laserError
              ? '70 00 04 00 00 00 00 0A 00 00 08 03 A7 00 00 00 00 00'
              : '00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00',
            boot_log_line: laserError
              ? 'drive start error(80040904)'
              : unlockFrm
                ? 'PANA_DVD.FRM verified -> JMP 0x50400000'
                : 'Normal disc TOC verified',
            passed: !trayError && !laserError,
          },
        ],
        uart_boot_transcript: [
          '***** Loader ver0.61 [06/03/06] *****',
          trayError
            ? '[RTSD] === DRIVE ACCESS ERROR === ( 80045300 )'
            : laserError
              ? 'drive start error(80040904)'
              : '[MATSHITA R6-MV4] Boot check completed',
        ],
        startup_succeeded: !trayError && !laserError,
        unlock_media_verified: unlockFrm,
        jump_target_va_hex: unlockFrm ? '0x50400000' : null,
        final_boot_code_hex: trayError ? '0x80045300' : laserError ? '0x80040904' : '0x00000000',
      };
    }

    case 'inject_matshita_atapi_write': {
      const targetVaRaw = String(args?.targetVa ?? args?.target_va ?? '0x51800000').trim();
      const parsedVa = Number.parseInt(targetVaRaw, 16);
      const inIplLow = Number.isFinite(parsedVa) && parsedVa >= 0x80000000 && parsedVa <= 0x807effff;
      const inIplHigh = Number.isFinite(parsedVa) && parsedVa >= 0x84000000 && parsedVa <= 0x847effff;
      if (inIplLow || inIplHigh) {
        throw new Error(
          `E05 IPL NOR Write Protected: gravação bloqueada em ${targetVaRaw} (região 0x80000000..0x807EFFFF somente leitura).`,
        );
      }
      const useEd = Boolean(args?.useVendorEd ?? args?.use_vendor_ed);
      const payloadHex = String(args?.payloadHex ?? args?.payload_hex ?? 'DEADBEEF').trim();
      const lenBytes = Math.max(1, Math.floor(payloadHex.length / 2));
      const record = {
        seq: 1,
        source: useEd ? 'ATAPI_VENDOR_ED' : 'ATAPI_WRITE_BUFFER_3B',
        cdb0_or_cmd: useEd ? '0xED' : '0x3B',
        mode: Number(args?.mode ?? 2),
        target_va_or_lba: Number.isFinite(parsedVa) ? parsedVa : 0x51800000,
        len_bytes: lenBytes,
        payload_sha256: 'e3b0c44298fc1c149afbf4c8996fb924',
        status: 'OK',
        note: `Injeção SDRAM ${targetVaRaw} concluída (IPL protegido)`,
      };
      return {
        record,
        snapshot: {
          inquiry_vendor: 'MATSHITA',
          inquiry_product: 'R6-MV4          ',
          inquiry_revision: '1.00',
          tray_closed: true,
          media_present: true,
          sram_0cd8_auth: 1,
          sram_0d78_s211: 0,
          sram_0242_ed_mode: useEd ? 1 : 0,
          mmode: Number(args?.mode ?? 2),
          slog_enabled: false,
          slog_entries: [],
          buffer_len: lenBytes,
          buffer_sha256: record.payload_sha256,
          disc_lba_sectors: 2295104,
          history_count: 1,
          injections: [record],
        },
      };
    }

    case 'run_matshita_serial_cmd': {
      const command = String(args?.command ?? 'WW 0x51800000 0xDEADBEEF').trim();
      const upper = command.toUpperCase();
      if (upper.includes('0X80') || upper.includes('0X84')) {
        throw new Error('E05 IPL NOR Write Protected: escrita via DRV serial bloqueada na IPL.');
      }
      return {
        command,
        authorized_flag_cd8: 1,
        s211_flag_d78: 0,
        slog_enabled: false,
        output_lines: [`> ${command}`, 'OK (SDRAM/SRAM guarded R/W)'],
        status: 'OK',
      };
    }

    case 'verify_frm_unlock_command': {
      const kind = String(args?.containerKind ?? args?.container_kind ?? 'sd').trim();
      const isSd = kind.toLowerCase().includes('sd') || kind.toLowerCase().includes('extr');
      return {
        container_kind: kind,
        container_bytes: isSd ? 131_072 : 262_144,
        sha256: '9f86d081884c7d659a2feaa0c55ad015',
        entry_count: 1,
        bootloader_jump_va_hex: '0x50400000',
        extrhead_valid: isSd ? true : null,
        verified: true,
        notes: [
          isSd ? 'EXTRHEAD + JMP 0x50400000 válido' : 'Header MATSHITA PANA_DVD.FRM válido',
        ],
      };
    }

    case 'pack_unlock_frm_command': {
      const kind = String(args?.containerKind ?? args?.container_kind ?? 'sd').trim();
      const isSd = kind.toLowerCase().includes('sd') || kind.toLowerCase().includes('extr');
      const containerBytes = isSd ? 131_072 : 262_144;
      return {
        verification: {
          container_kind: kind,
          container_bytes: containerBytes,
          sha256: '9f86d081884c7d659a2feaa0c55ad015',
          entry_count: 1,
          bootloader_jump_va_hex: '0x50400000',
          extrhead_valid: isSd ? true : null,
          verified: true,
          notes: ['Container gerado e verificado contra regras Loader ver0.61'],
        },
        output_path: (args?.outPath as string) || (args?.out_path as string) || null,
        container_bytes: containerBytes,
      };
    }

    default:
      return undefined;
  }
}
