/**
 * Stateless pure mock IPC handlers for Segregated Forensic Vaults, Mandatory Mn103Vm Pre-Write Gate,
 * Streaming Composed MEIHFS HDD Flash, ISO 9660 Level 1 (1x) DVD Creator, and Relational JSON Hardware Profiles.
 */

import type {
  ComposedHddFlashReport,
  DvdCreatorReport,
  NorFingerprintReport,
  OpticalBurnerDriveInfo,
  PreWriteGateReport,
  ProfileAuthorizationOutcome,
  RegistryValidationReport,
  ResolvedModelProfile,
  SdLoaderProfile,
  StrictWorkflowStep,
  VaultArtifactRecord,
  VaultBucket,
  VaultSessionManifest,
} from '@/models';
import { readBooleanProp, readStringProp } from '@/utils/typeGuards';
import { publishMockEvent, readMockRequest } from './mockEventBus';

const MOCK_CREATED_TS = 1_763_650_000;
const MOCK_GATE_STEPS = 5199;
const MOCK_BIN_BYTES = 8_388_608;
const MOCK_FRM_BYTES = 7_183_272;
const MOCK_ISO_BYTES = 327_680;
const MOCK_ISO_SECTOR_SIZE = 2048;
const MOCK_ISO_SECTORS = 160;
const MOCK_ISO_EXTENT_LBA = 21;
const MOCK_ISO_FILE_SECTORS = 128;
const MOCK_DIGADIAG_RAW_BYTES = 262_144;
const MOCK_DVD_1X_KBPS = 1385;
const MOCK_DVD_4X_KBPS = 5540;
const MOCK_HDD_DISK_BYTES = 250_059_350_016;
const MOCK_HDD_SECTORS = 488_397_168;
const MOCK_HDD_MIRROR_LBA = 488_150_176;
const MOCK_MAIN_WRITTEN_BYTES = 13_003_264;
const MOCK_SHA_A = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const MOCK_SHA_B = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
const MOCK_SHA_C = 'cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc';
const MOCK_PERCENT_100 = 100;
const MOCK_3V3_MV = 3300;
const MOCK_VOLT_MIN_MV = 2700;
const MOCK_VOLT_MAX_MV = 3600;
const MOCK_UART_BAUD = 115200;
const MOCK_SECTOR_64K = 65_536;
const MOCK_BOOT_SECTORS_8 = 8;
const MOCK_MAIN_SECTORS_126 = 126;
const MOCK_TOC_48 = 48;
const MOCK_EEMUL_MARKER_OFF = 5;
const MOCK_DISK_SECTOR_512 = 512;
const MOCK_SD_START_LBA = 63;
const MOCK_SD_VOL_MB = 32;
const MOCK_PROFILE_MODEL_COUNT = 89;
const MOCK_PROFILE_SOC_CORE_COUNT = 5;
const MOCK_PROFILE_SOC_COUNT = 12;
const MOCK_PROFILE_FLASH_COUNT = 18;
const MOCK_PROFILE_EEPROM_COUNT = 6;
const MOCK_PROFILE_TRAIN_COUNT = 18;
const MOCK_PROFILE_LAYOUT_COUNT = 3;
const MOCK_PROFILE_BOARD_COUNT = 31;
const MOCK_PROFILE_CHECKS_COUNT = 796;
const MOCK_PROBE_BPS = 9800;

function buildMockPrewriteGate(kind: string, passed = true): PreWriteGateReport {
  return {
    passed,
    artifactKind: kind,
    artifactSha256: MOCK_SHA_A,
    iplUntouched: true,
    lethalMmioBlocked: true,
    executedSteps: MOCK_GATE_STEPS,
    checks: [
      {
        id: 'mn103vm_ipl_lockout',
        passed: true,
        detail: 'SA0..SA10 IPL write protection verified (IplWriteProtected)',
      },
      {
        id: 'mn103vm_lethal_0xc000100c',
        passed: true,
        detail: 'Lethal MMIO register 0xC000100C blocked cleanly',
      },
    ],
    notes: [],
  };
}

function buildEh55ResolvedProfile(): ResolvedModelProfile {
  const claim = (tier: 'bench_verified' | 'service_manual_only', reference: string) => ({
    tier,
    sources: [{ kind: 'bench_log', reference }],
  });
  return {
    manifest: {
      model_id: 'DMR-EH55',
      family_label: 'DMR-EH55',
      series: 'Série EH (HDD+DVD)',
      region_suffix: 'EC/LB',
      tv_standard: 'PAL/PAL-M',
      hw_code: 'M6930',
      board_ref: 'vep79138',
      train_ref: 'loader_ver061_toc48_main57',
      nor_layout_ref: 'eh55_ver061_on_s29jl064h',
      exact_aliases: ['DMR-EH55LB-S', 'DMR-EH55EC', 'DMR-EH55EP', 'DMR-EH55S', 'DMR-EH55P'],
      fixture_ref: 'profiles/fixtures/dmr_eh55.fixture.json',
      claims: {
        board: claim('bench_verified', 'docs/STATUS.md'),
        flash_chip: claim('bench_verified', 'firmware_and_images/eh55_nor_8mb_ic5001_dump.bin'),
        nor_layout: claim('bench_verified', 'firmware_and_images/eh55_nor_8mb_ic5001_dump.bin'),
        train: claim('bench_verified', 'docs/debug_logs/sddiag_ver061_live_20261119.log'),
        eeprom: claim('service_manual_only', 'DSD0603003CE IC7404 C3EBFC000053/RFKFM6016KT'),
      },
    },
    board: {
      id: 'vep79138',
      pcb_code: 'VEP79138',
      service_module_code: 'VEP70161A',
      soc_ref: 'mn2ds0009',
      flash_ref: 'spansion_s29jl064h_8mb',
      eeprom_refs: ['panasonic_c3ebfc000053_timer'],
      p7501_header_present: true,
      has_hdd_interface: true,
      has_sd_slot: true,
      evidence: claim('bench_verified', 'docs/STATUS.md'),
    },
    socCore: {
      id: 'mn10300_am34',
      isa: 'MN10300/AM34',
      endianness: 'little',
      logic_voltage_mv: MOCK_3V3_MV,
      evidence: claim('bench_verified', 'docs/debug_logs/sddiag_ver061_live_20261119.log'),
    },
    soc: {
      id: 'mn2ds0009',
      part_number: 'MN2DS0009',
      core_ref: 'mn10300_am34',
      bus_windows: [
        {
          name: 'nor_cached',
          base: '0x80000000',
          size: '0x04000000',
          uncached_alias_base: '0x84000000',
        },
      ],
      mmio: { sc0_uart: '0xD4002000' },
      lethal_mmio: ['0xC000100C'],
      p7501_uart_baud: MOCK_UART_BAUD,
      p7501_nsw5v_hiz_only: true,
      evidence: claim('bench_verified', 'docs/STATUS.md'),
    },
    flashChip: {
      id: 'spansion_s29jl064h_8mb',
      vendor: 'Spansion',
      part_number: 'S29JL064H70TFI00',
      package: 'TSOP-48',
      voltage_min_mv: MOCK_VOLT_MIN_MV,
      voltage_max_mv: MOCK_VOLT_MAX_MV,
      nominal_voltage_mv: MOCK_3V3_MV,
      writable: true,
      total_bytes: '0x00800000',
      jedec: { mfg: '0x0001', device_cycles: ['0x227E', '0x220A', '0x2201'] },
      sector_banks: [
        { name: 'bottom_boot', count: MOCK_BOOT_SECTORS_8, sector_size: '0x00002000' },
        { name: 'main_uniform', count: MOCK_MAIN_SECTORS_126, sector_size: '0x00010000' },
        { name: 'top_nvram', count: MOCK_BOOT_SECTORS_8, sector_size: '0x00002000' },
      ],
      rw_banks: [],
      hw_protect: null,
      evidence: claim('bench_verified', 'firmware_and_images/eh55_nor_8mb_ic5001_dump.bin'),
    },
    eepromChips: [
      {
        id: 'panasonic_c3ebfc000053_timer',
        vendor: 'Panasonic',
        part_number: 'C3EBFC000053',
        service_replacement_part: 'RFKFM6016KT',
        bus: 'i2c',
        function: 'timer_main_config',
        i2c_addr_7bit: '0x00000050',
        total_bytes: '0x00000800',
        page_size: 16,
        voltage_mv: MOCK_3V3_MV,
        evidence: claim('service_manual_only', 'DSD0603003CE IC7404'),
      },
    ],
    firmwareTrain: {
      id: 'loader_ver061_toc48_main57',
      loader_version: '0.61',
      loader_date: '06/03/06',
      loader_profile_key: 'ver061',
      loader_fingerprint: {
        offset_start: '0x00000000',
        offset_end: '0x00040000',
        ascii_marker: 'Loader ver0.61',
      },
      frm: { toc_entry_bytes: MOCK_TOC_48, toc_size: '0x1800', cipher: 'feistel64_ecb', key_id: 'pana_dvd_eh55' },
      prog_load_va: '0x50400000',
      main_load_va: '0x57000000',
      drv_load_va: '0x90202000',
      symbols: { bgo_table: '0x440337A4' },
      sd_boot_filenames: ['DIGADIAG.FRM', 'PANA__SD.FRM'],
      digadiag_raw_bytes: MOCK_DIGADIAG_RAW_BYTES,
      media_refs: ['sd_fat16b_0x06', 'dvd_iso9660_level1', 'hdd_meihdfs_v1'],
      evidence: claim('bench_verified', 'docs/STATUS.md'),
    },
    norLayout: {
      id: 'eh55_ver061_on_s29jl064h',
      flash_ref: 'spansion_s29jl064h_8mb',
      train_ref: 'loader_ver061_toc48_main57',
      regions: [
        {
          kind: 'ipl',
          start_offset: '0x00000000',
          end_offset: '0x0003FFFF',
          policy: 'read_only_permanent',
          tag: 'Loader ver0.61 (SA0..SA10)',
          signature: null,
        },
        {
          kind: 'bgo',
          start_offset: '0x00130000',
          end_offset: '0x003FFFFF',
          policy: 'gated_upper_nor',
          tag: 'BGO (SA26..SA70)',
          signature: null,
        },
        {
          kind: 'eemul',
          start_offset: '0x007F0000',
          end_offset: '0x007FFFFF',
          policy: 'nvram_sanitize',
          tag: 'EEMUL NVRAM (SA134..SA141)',
          signature: { marker_byte_offset: MOCK_EEMUL_MARKER_OFF, marker_value: '0xF1' },
        },
      ],
      evidence: claim('bench_verified', 'firmware_and_images/eh55_nor_8mb_ic5001_dump.bin'),
    },
    mediaFormats: [
      {
        id: 'sd_fat16b_0x06',
        media_kind: 'sd_card',
        filesystem: 'FAT16B',
        partition_type: '0x00000006',
        sector_size: MOCK_DISK_SECTOR_512,
        start_lba: MOCK_SD_START_LBA,
        default_volume_mb: MOCK_SD_VOL_MB,
        root_filenames: ['DIGADIAG.FRM', 'PANA__SD.FRM'],
        magic_hex: '55AA',
        write_speed_policy: 'raw_block_verified',
        evidence: claim('bench_verified', 'docs/STATUS.md'),
      },
      {
        id: 'dvd_iso9660_level1',
        media_kind: 'optical_dvd',
        filesystem: 'ISO9660_LEVEL1',
        partition_type: '0x00000000',
        sector_size: MOCK_ISO_SECTOR_SIZE,
        start_lba: 16,
        default_volume_mb: 1,
        root_filenames: ['PANA_DVD.FRM', 'DIGADIAG.FRM'],
        magic_hex: '4344303031',
        write_speed_policy: '1x_or_drive_minimum_dao',
        evidence: claim('bench_verified', 'docs/STATUS.md'),
      },
      {
        id: 'hdd_meihdfs_v1',
        media_kind: 'ata_hdd',
        filesystem: 'MEIHFS1.0',
        partition_type: '0x00000000',
        sector_size: MOCK_DISK_SECTOR_512,
        start_lba: 0,
        default_volume_mb: 100,
        root_filenames: ['MAIN', 'DRV'],
        magic_hex: '5348',
        write_speed_policy: 'composed_streaming_verified',
        evidence: claim('bench_verified', 'docs/STATUS.md'),
      },
    ],
    nvramCachedVaHex: '0x807F0000',
    nvramUncachedVaHex: '0x847F0000',
    iplProtectedRangeHex: '0x84000000..0x8403FFFF',
    bgoSectorRangeDerived: 'SA26..SA70',
    nvramSectorRangeDerived: 'SA134..SA141',
    derivedSectorGeometry: '8x8KB + 126x64KB + 8x8KB (S29JL064H70TFI00, SA0..SA141)',
    effectiveCapabilities: {
      inspectReadOnly: true,
      nvramSanitizeOfflineCow: true,
      nvramWriteLive: true,
      upperNorPatchOfflineCow: true,
      upperNorWriteLive: true,
      officialFrmPassThrough: true,
      customRecoveryMedia: true,
      labOverrideOfflineCowEligible: true,
    },
  };
}

function buildMockSession(
  sessionId: string,
  modelId: string,
  loaderProfile: SdLoaderProfile,
): VaultSessionManifest {
  return {
    sessionId,
    modelId,
    loaderProfile,
    createdAtEpochS: MOCK_CREATED_TS,
    sessionDir: `vault/sessions/${sessionId}`,
    completedSteps: ['select_model'],
    currentStep: 'build_cleanroom_sd',
    rawArtifacts: [],
    frmArtifacts: [],
    recompiledArtifacts: [],
    lastEmuGate: null,
  };
}

function parseMockBucket(raw?: string): VaultBucket | undefined {
  if (raw === 'raw' || raw === 'frm' || raw === 'recompiled') {
    return raw;
  }
  return undefined;
}

function parseMockStep(raw?: string): StrictWorkflowStep {
  if (
    raw === 'select_model' ||
    raw === 'build_cleanroom_sd' ||
    raw === 'setup_pico_probe' ||
    raw === 'capture_and_backup' ||
    raw === 'organize_and_recompile' ||
    raw === 'emulate_and_flash'
  ) {
    return raw;
  }
  return 'select_model';
}

function resolveLoaderProfile(raw?: string): SdLoaderProfile {
  return raw === 'ver063' ? 'ver063' : 'ver061';
}

function extractBasename(pathStr: string): string {
  const normalized = pathStr.replace(/\\/g, '/').trim();
  const idx = normalized.lastIndexOf('/');
  return idx >= 0 ? normalized.slice(idx + 1) : normalized;
}

export function handleMockVaultCommand(
  cmd: string,
  args?: Record<string, unknown>,
): unknown | undefined {
  const req = readMockRequest(args);
  switch (cmd) {
    case 'vault_init_session_command': {
      const modelId = readStringProp(req, 'modelId') ?? 'DMR-EH55';
      const loaderProfile = resolveLoaderProfile(readStringProp(req, 'loaderProfile'));
      const sessionId =
        readStringProp(req, 'sessionId') ?? `sess_${modelId.toLowerCase().replace(/[^a-z0-9]+/g, '_')}`;
      return buildMockSession(sessionId, modelId, loaderProfile);
    }

    case 'vault_list_sessions_command':
      return [buildMockSession('sess_dmr_eh55_default', 'DMR-EH55', 'ver061')];

    case 'vault_get_session_command': {
      const sid = readStringProp(args ?? {}, 'sessionId') ?? 'sess_dmr_eh55_default';
      return buildMockSession(sid, 'DMR-EH55', 'ver061');
    }

    case 'vault_ingest_file_command': {
      const src = readStringProp(req, 'sourcePath') ?? 'ic5001.bin';
      const filename = extractBasename(src) || 'ic5001.bin';
      const isFrm = filename.toLowerCase().endsWith('.frm');
      const override = parseMockBucket(readStringProp(req, 'bucketOverride'));
      if (isFrm && override === 'raw') {
        throw new Error(`Vault segregation violation: .FRM file '${filename}' belongs in frm/, NEVER raw/`);
      }
      if (!isFrm && override === 'frm') {
        throw new Error(`Vault segregation violation: frm/ accepts ONLY .FRM files (got '${filename}')`);
      }
      const bucket: VaultBucket = override ?? (isFrm ? 'frm' : 'raw');
      const rec: VaultArtifactRecord = {
        id: `${bucket}:${filename}`,
        bucket,
        filename,
        relativePath: `${bucket}/${filename}`,
        sizeBytes: isFrm ? MOCK_FRM_BYTES : MOCK_BIN_BYTES,
        sha256: isFrm ? MOCK_SHA_B : MOCK_SHA_A,
        origin: readStringProp(req, 'origin') ?? 'desktop_manual_ingest',
        createdAtEpochS: MOCK_CREATED_TS,
        emuVerified: bucket === 'recompiled',
      };
      return rec;
    }

    case 'vault_mark_step_command': {
      const sid = readStringProp(args ?? {}, 'sessionId') ?? 'sess_dmr_eh55_default';
      const step = parseMockStep(readStringProp(args ?? {}, 'step'));
      return {
        ...buildMockSession(sid, 'DMR-EH55', 'ver061'),
        completedSteps: ['select_model', step],
        currentStep: step,
      };
    }

    case 'vault_run_prewrite_emu_gate_command': {
      const kind = readStringProp(req, 'targetKind') ?? 'cleanroom_digadiag';
      return buildMockPrewriteGate(kind, true);
    }

    case 'flash_composed_hdd_safe_command': {
      const devPath = readStringProp(req, 'devicePath') ?? '';
      if (!devPath || devPath.includes('PhysicalDrive0')) {
        throw new Error('FATAL SAFETY LOCKOUT: PhysicalDrive0 is permanently blocked.');
      }
      const token = (readStringProp(req, 'confirmDiskModelToken') ?? '').trim();
      const shortDev = devPath.replace(/^\\\\.\\/, '');
      const expected = `CONFIRM-${shortDev}`;
      if (token !== expected && token !== shortDev) {
        throw new Error(`FATAL SAFETY LOCKOUT: expected ${expected}`);
      }
      publishMockEvent('hdd://flash-progress', {
        devicePath: devPath,
        stage: 'done',
        bytesDone: MOCK_SECTOR_64K,
        bytesTotal: MOCK_SECTOR_64K,
        detail: 'MEIHFS streaming composition & SHA-256 readback verified',
      });
      const report: ComposedHddFlashReport = {
        devicePath: devPath,
        diskBytes: MOCK_HDD_DISK_BYTES,
        totalSectors: MOCK_HDD_SECTORS,
        busTypeId: 7,
        mirrorMainLba: MOCK_HDD_MIRROR_LBA,
        backupBinPath: 'backups/cow_64k_backup.bin',
        backupManifestPath: 'backups/cow_64k_manifest.json',
        backup64kSha256: MOCK_SHA_C,
        layoutSha256: MOCK_SHA_A,
        p1MainBytesWritten: MOCK_MAIN_WRITTEN_BYTES,
        p1MainSha256: MOCK_SHA_A,
        p2DrvBytesWritten: 0,
        p2DrvSha256: null,
        mirrorMainBytesWritten: MOCK_MAIN_WRITTEN_BYTES,
        mirrorMainSha256: MOCK_SHA_A,
        emuGate: buildMockPrewriteGate('composed_hdd_stream', true),
        readbackVerified: true,
      };
      return report;
    }

    case 'inspect_optical_burners_command': {
      const burners: readonly OpticalBurnerDriveInfo[] = [
        {
          deviceId: '/dev/sr0',
          displayName: 'HL-DT-ST DVDRAM GH24NSC0',
          osBackend: 'mock_imapi2',
          supportedWriteSpeedsKbps: [MOCK_DVD_1X_KBPS, MOCK_DVD_4X_KBPS],
          minWriteSpeedKbps: MOCK_DVD_1X_KBPS,
          minWriteSpeedLabel: '1385 KB/s (1x DVD)',
          safeOpticalTarget: true,
          expectedConfirmToken: 'CONFIRM-sr0',
        },
      ];
      return burners;
    }

    case 'create_dvd_recovery_disc_command': {
      const burnerId = readStringProp(req, 'burnerId');
      if (burnerId) {
        const expected = `CONFIRM-${burnerId.replace(/^\/dev\//, '')}`;
        if ((readStringProp(req, 'confirmToken') ?? '').trim() !== expected) {
          throw new Error(`DVD SAFETY LOCKOUT: confirm_token must equal '${expected}'`);
        }
      }
      const target = burnerId ?? (readStringProp(req, 'outputIsoPath') ?? 'backups/eh55_dvd_recovery_iso9660.iso');
      const model = readStringProp(req, 'model') ?? 'DMR-EH55';
      const volumeLabel = readStringProp(req, 'volumeLabel') ?? 'PANADIGA_REC';
      const cleanroomOnly = readBooleanProp(req, 'cleanroomOnly') ?? true;
      publishMockEvent('dvd://burn-progress', {
        target,
        stage: 'readback_verify',
        percent: MOCK_PERCENT_100,
        detail: 'ISO 9660 Level 1 (1x DAO) verified',
      });
      const rep: DvdCreatorReport = {
        target,
        targetKind: burnerId ? 'optical_disc' : 'iso_file',
        model,
        volumeLabel,
        cleanroomOnly,
        bytesWritten: MOCK_ISO_BYTES,
        isoSha256: MOCK_SHA_A,
        writeSpeedPolicy: '1x_or_drive_minimum_dao',
        burnCommandPreview: burnerId ? `mock --speed=1 --dev=${burnerId}` : null,
        files: [
          {
            name_8_3: 'DIGADIAG.FRM',
            extent_lba: MOCK_ISO_EXTENT_LBA,
            size_bytes: MOCK_DIGADIAG_RAW_BYTES,
            sector_count: MOCK_ISO_FILE_SECTORS,
            sha256: MOCK_SHA_A,
          },
        ],
        emuGate: buildMockPrewriteGate('cleanroom_digadiag', true),
        inspection: {
          valid_pvd: true,
          valid_terminator: true,
          volume_id: volumeLabel,
          logical_block_size: MOCK_ISO_SECTOR_SIZE,
          volume_space_sectors: MOCK_ISO_SECTORS,
          files: [
            {
              name_8_3: 'DIGADIAG.FRM',
              extent_lba: MOCK_ISO_EXTENT_LBA,
              size_bytes: MOCK_DIGADIAG_RAW_BYTES,
              sector_count: MOCK_ISO_FILE_SECTORS,
              sha256: MOCK_SHA_A,
            },
          ],
          overall_status: 'PASS',
        },
        nextSteps: [
          '1. Burn this verified ISO 9660 Level 1 .iso strictly in DAO mode at 1x (or lowest drive speed).',
          `2. Insert disc into ${model} to run recovery update.`,
        ],
      };
      return rep;
    }

    case 'list_hardware_profiles_command':
      return [buildEh55ResolvedProfile()];

    case 'get_resolved_model_profile_command':
      return buildEh55ResolvedProfile();

    case 'validate_hardware_profiles_command': {
      const val: RegistryValidationReport = {
        valid: true,
        socCoreCount: MOCK_PROFILE_SOC_CORE_COUNT,
        socCount: MOCK_PROFILE_SOC_COUNT,
        flashChipCount: MOCK_PROFILE_FLASH_COUNT,
        eepromChipCount: MOCK_PROFILE_EEPROM_COUNT,
        firmwareTrainCount: MOCK_PROFILE_TRAIN_COUNT,
        norLayoutCount: MOCK_PROFILE_LAYOUT_COUNT,
        mediaFormatCount: MOCK_PROFILE_LAYOUT_COUNT,
        boardCount: MOCK_PROFILE_BOARD_COUNT,
        modelCount: MOCK_PROFILE_MODEL_COUNT,
        checksRun: MOCK_PROFILE_CHECKS_COUNT,
        checksFailed: 0,
        safetyChecksPassed: MOCK_PROFILE_CHECKS_COUNT,
        errors: [],
      };
      return val;
    }

    case 'authorize_profile_capability_command': {
      const req = readMockRequest(args);
      const modelId =
        readStringProp(req, 'modelId') ?? readStringProp(req, 'model_id') ?? 'DMR-EH55';
      const rawCap = readStringProp(req, 'capability') ?? 'inspect_read_only';
      const capability =
        rawCap === 'nvram_sanitize_offline_cow' ||
        rawCap === 'nvram_write_live' ||
        rawCap === 'upper_nor_patch_offline_cow' ||
        rawCap === 'upper_nor_write_live' ||
        rawCap === 'official_frm_pass_through' ||
        rawCap === 'custom_recovery_media' ||
        rawCap === 'critical_ipl_or_lethal_mmio'
          ? rawCap
          : 'inspect_read_only';
      const blocked =
        modelId === 'DMR-EZ485V' ||
        modelId === 'DMR-ES10' ||
        capability === 'critical_ipl_or_lethal_mmio';
      const outcome: ProfileAuthorizationOutcome = blocked
        ? {
            allowed: false,
            authorization: null,
            denial: {
              modelId,
              capability,
              reason: `Mock policy denied ${capability} on ${modelId}`,
              labOverrideEligible: false,
            },
          }
        : {
            allowed: true,
            authorization: {
              modelId,
              capability,
              mode: 'verified_policy',
              effectiveTier: 'bench_verified',
              warnings: [],
            },
            denial: null,
          };
      return outcome;
    }

    case 'fingerprint_nor_dump_command': {
      const fp: NorFingerprintReport = {
        detectedModel: 'DMR-EH55',
        displayLabel: 'DMR-EH55',
        isSizeOnly: false,
        matchedProbe: 'loader_banner:Loader ver0.61@0x0',
        hasEemulF1Marker: true,
        dumpSizeBytes: MOCK_BIN_BYTES,
        candidates: [
          {
            modelId: 'DMR-EH55',
            matchedProbe: 'loader_banner:Loader ver0.61@0x0',
            confidenceBps: MOCK_PROBE_BPS,
          },
        ],
      };
      return fp;
    }

    default:
      return undefined;
  }
}
