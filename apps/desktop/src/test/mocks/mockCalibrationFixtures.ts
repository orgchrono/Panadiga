/**
 * Fixture builders for Region Calibration (LBS) & Hardened SD Unbrick Kit mock IPC responses.
 */

import {
  AIOF_VMODESEL_NTSC,
  AIOF_VMODESEL_PAL,
  AIOF_VMODESEL_PAL_M,
  AVDD_NPALCTL0_ADDR,
  DEV_BIN_BYTE_SIZE,
  IFODEV2_BYTE_SIZE,
} from '@/constants';
import type {
  DualFamilySlotResolution,
  HardenSdUnbrickKitResponse,
  TvCalibrationBundleReport,
} from '@/models';
import { isTvStandardSlug, readBooleanProp, readStringProp } from '@/utils/typeGuards';

const WIDTH_16 = 16;
const NON_FF_TAIL = 288;
const MOCK_FRM_BYTES = 262_144;
const MOCK_MAIN_SIZE = 13_003_052;
const MOCK_SLOT_COUNT = 3;
const MOCK_FVU_SUM = 0x1a2b_3c4d;
const VMODESEL_REG_ADDR = 0x8800_0100;
const AVDD_PALM_VAL = 0x0001;
const AVDD_NTSC_VAL = 0x0000;
const MOCK_DIGADIAG_SHA = 'e5d3a7f24c1b890123456789abcdef0123456789abcdef0123456789abcdef01';
const MOCK_PANASD_SHA = 'a9c4e210887766554433221100ffeeddccbbaa99887766554433221100ffeedd';

export const MOCK_MAIN_BINARY_SIZE = MOCK_MAIN_SIZE;

export function buildCalibrationBundleReport(req: Record<string, unknown>): TvCalibrationBundleReport {
  const stdRaw = readStringProp(req, 'tvStandard') ?? 'pal_m';
  const tvStandard = isTvStandardSlug(stdRaw) ? stdRaw : 'pal_m';
  const vmodeVal =
    tvStandard === 'pal_m'
      ? AIOF_VMODESEL_PAL_M
      : tvStandard === 'pal'
        ? AIOF_VMODESEL_PAL
        : AIOF_VMODESEL_NTSC;

  return {
    profile: {
      preset: tvStandard === 'pal_m' ? 'br_pal_m_r4' : tvStandard === 'pal' ? 'eu_pal_r2' : 'us_ntsc_r1',
      tv_standard: tvStandard,
      chassis_family: tvStandard === 'pal_m' ? 'M6930LB' : 'M6930',
      dvd_region_code: tvStandard === 'pal_m' ? 4 : tvStandard === 'pal' ? 2 : 1,
      expected_oem_char: tvStandard === 'pal_m' ? 'M' : 'P',
      metadata_and_dev_oem_only: true,
      mmio_plan: [
        {
          addr: VMODESEL_REG_ADDR,
          width_bits: WIDTH_16,
          value: vmodeVal,
          register_name: 'AIOF_VMODESEL',
          note: `Encoder matrix target (${tvStandard.toUpperCase()})`,
        },
        {
          addr: AVDD_NPALCTL0_ADDR,
          width_bits: WIDTH_16,
          value: tvStandard === 'pal_m' ? AVDD_PALM_VAL : AVDD_NTSC_VAL,
          register_name: 'AVDD_NPALCTL0',
          note: 'Subcarrier / phase control',
        },
      ],
      notes: ['Only DEV OEM/Model metadata and MMIO preview calibrated safely.'],
    },
    dev_bin: {
      size_bytes: DEV_BIN_BYTE_SIZE,
      model: readStringProp(req, 'model') ?? 'EH55LB',
      oem: readStringProp(req, 'oem') ?? 'M',
      signature: 'PANASONIC_DEV_OK',
      md5: '0123456789abcdef0123456789abcdef',
      sha256: '44aa77112233445566778899aabbccddeeff00112233445566778899aabbccdd',
    },
    ifodev2_tail: {
      offset_range_hex: '0x600..0x800',
      byte_len: IFODEV2_BYTE_SIZE,
      non_ff_bytes: NON_FF_TAIL,
      prefix_hex: '454835354c420000',
      sha256: 'bb22cc33445566778899aabbccddeeff00112233445566778899aabbccddeeff',
      metadata_and_dev_oem_only: true,
    },
    metadata_and_dev_oem_only: true,
    main_region: {
      build_profile: '3410PQ',
      size_bytes: MOCK_MAIN_SIZE,
      va_hex: '0x5771ABFB',
      file_offset_hex: '0x71ABFB',
      state: 'BYPASS_COMPATIBLE',
      actual_hex: '8001f0fccbcb',
      sha256: 'cc33dd44556677889900aabbccddeeff00112233445566778899aabbccddeeff',
    },
    frm_toc: {
      layout: 'SD48_PLAINTEXT_TOC',
      entry_count: MOCK_SLOT_COUNT,
      all_list_sums_valid: true,
      entries: [],
      sha256: MOCK_DIGADIAG_SHA,
    },
    nvram_sa134_sa141: null,
    warnings: [],
  };
}

function makeSlotResolution(family: string, tag: string, slotIndex: number): DualFamilySlotResolution {
  return {
    hardware_family: family,
    tag_prefix: tag,
    slot_index: slotIndex,
    model_prefix_byte_hex: '0x4D',
    model_prefix_is_m: true,
    list_sum_valid: true,
    fvu_sum_valid: true,
    module_offset: MOCK_FRM_BYTES * slotIndex,
    module_size: MOCK_FRM_BYTES,
    fvu_checksum_u32: MOCK_FVU_SUM,
  };
}

export function buildHardenKitResponse(req: Record<string, unknown>): HardenSdUnbrickKitResponse {
  const outDir = readStringProp(req, 'outputDir') ?? 'firmware_and_images/sd_unbrick_hardened';
  const digPath = `${outDir}/DIGADIAG.FRM`;
  const panaPath = `${outDir}/PANA__SD.FRM`;
  const parity = {
    is_sd48_layout: true,
    m6930lb_prog061: makeSlotResolution('M6930LB', 'P061', 0),
    m6930lb_main: makeSlotResolution('M6930LB', 'MAIN', 1),
    m6930_prog061: makeSlotResolution('M6930', 'P061', 2),
    m6930_main: null,
    m6930lb_drv: null,
    m6930_drv: null,
    dual_parity_ok: true,
    issues: [],
  };
  const verification = {
    sd_root_dir: outDir,
    overall_status: 'VERIFIED_HARDENED_VER061',
    digadiag_present: true,
    digadiag_exact_size_ok: true,
    digadiag_dst_addr_hex: '0x90400000',
    digadiag_bgo_mode: 'BYPASS_CHECKS_ONLY',
    pana_sd_present: true,
    pana_sd_toc_entries: MOCK_SLOT_COUNT,
    pana_sd_list_sums_ok: true,
    pana_sd_prog061_mode: 'BYPASS_CHECKS_ONLY',
    pana_sd_main_build: '3410PQ',
    pana_sd_main_passthrough_mode: 'AV_PASSTHROUGH',
    pana_sd_main_hdd_dynamic: true,
    factory_unpatched_prog061_detected: false,
    dual_family_parity: parity,
    checks: [
      {
        id: 'hardened_ver061',
        severity: 'pass',
        title: 'Loader ver0.61 Dual-Family Hardened Kit Verified',
        detail: '262,144 B DIGADIAG.FRM + 48 B SD TOC verified across M6930/M6930LB.',
      },
    ],
  };

  return {
    kitReport: {
      output_dir: outDir,
      prog061_source: 'bundled_patched_prog061',
      main_source: '3410PQ_av_passthrough',
      drv_source: null,
      bgo_mode_applied: 'BYPASS_CHECKS_ONLY',
      loader_profile_applied: 'ver061',
      main_passthrough_applied: 'AV_PASSTHROUGH',
      hdd_capacity_applied: '5GB..2TB Dynamic',
      digadiag_path: digPath,
      digadiag_sha256: MOCK_DIGADIAG_SHA,
      pana_sd_path: panaPath,
      pana_sd_sha256: MOCK_PANASD_SHA,
      sha256sums_path: `${outDir}/SHA256SUMS.txt`,
      verification,
    },
    hardenedVerification: verification,
    dualFamilyParity: parity,
    factoryProg061Blocked: readBooleanProp(req, 'allowFactoryProg061') === false ? false : false,
  };
}
