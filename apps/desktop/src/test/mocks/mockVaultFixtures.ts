/**
 * Stateless pure mock IPC handlers for Segregated Forensic Vaults, Mandatory Mn103Vm Pre-Write Gate,
 * Streaming Composed MEIHFS HDD Flash, ISO 9660 Level 1 (1x) DVD Creator, and Relational JSON Hardware Profiles.
 */

import type {
  PreWriteGateReport,
  ResolvedModelProfile,
  SdLoaderProfile,
  StrictWorkflowStep,
  VaultBucket,
  VaultSessionManifest,
} from '@/models';

export const MOCK_CREATED_TS = 1_763_650_000;
const MOCK_GATE_STEPS = 5199;
export const MOCK_BIN_BYTES = 8_388_608;
export const MOCK_FRM_BYTES = 7_183_272;
export const MOCK_ISO_BYTES = 327_680;
export const MOCK_ISO_SECTOR_SIZE = 2048;
export const MOCK_ISO_SECTORS = 160;
export const MOCK_ISO_EXTENT_LBA = 21;
export const MOCK_ISO_FILE_SECTORS = 128;
export const MOCK_DIGADIAG_RAW_BYTES = 262_144;
export const MOCK_DVD_1X_KBPS = 1385;
export const MOCK_DVD_4X_KBPS = 5540;
export const MOCK_HDD_DISK_BYTES = 250_059_350_016;
export const MOCK_HDD_SECTORS = 488_397_168;
export const MOCK_HDD_MIRROR_LBA = 488_150_176;
export const MOCK_MAIN_WRITTEN_BYTES = 13_003_264;
export const MOCK_SHA_A = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
export const MOCK_SHA_B = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
export const MOCK_SHA_C = 'cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc';
export const MOCK_PERCENT_100 = 100;
export const MOCK_3V3_MV = 3300;
export const MOCK_VOLT_MIN_MV = 2700;
export const MOCK_VOLT_MAX_MV = 3600;
export const MOCK_UART_BAUD = 115200;
export const MOCK_SECTOR_64K = 65_536;
export const MOCK_BOOT_SECTORS_8 = 8;
export const MOCK_MAIN_SECTORS_126 = 126;
export const MOCK_TOC_48 = 48;
export const MOCK_EEMUL_MARKER_OFF = 5;
export const MOCK_DISK_SECTOR_512 = 512;
export const MOCK_SD_START_LBA = 63;
export const MOCK_SD_VOL_MB = 32;
export const MOCK_PROFILE_MODEL_COUNT = 89;
export const MOCK_PROFILE_SOC_CORE_COUNT = 5;
export const MOCK_PROFILE_SOC_COUNT = 12;
export const MOCK_PROFILE_FLASH_COUNT = 18;
export const MOCK_PROFILE_EEPROM_COUNT = 6;
export const MOCK_PROFILE_TRAIN_COUNT = 18;
export const MOCK_PROFILE_LAYOUT_COUNT = 3;
export const MOCK_PROFILE_BOARD_COUNT = 31;
export const MOCK_PROFILE_CHECKS_COUNT = 796;
export const MOCK_PROBE_BPS = 9800;

export function buildMockPrewriteGate(kind: string, passed = true): PreWriteGateReport {
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

export function buildEh55ResolvedProfile(): ResolvedModelProfile {
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

export function buildMockSession(
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

export function parseMockBucket(raw?: string): VaultBucket | undefined {
  if (raw === 'raw' || raw === 'frm' || raw === 'recompiled') {
    return raw;
  }
  return undefined;
}

export function parseMockStep(raw?: string): StrictWorkflowStep {
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

export function resolveLoaderProfile(raw?: string): SdLoaderProfile {
  return raw === 'ver063' ? 'ver063' : 'ver061';
}

export function extractBasename(pathStr: string): string {
  const normalized = pathStr.replace(/\\/g, '/').trim();
  const idx = normalized.lastIndexOf('/');
  return idx >= 0 ? normalized.slice(idx + 1) : normalized;
}

