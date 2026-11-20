/**
 * @file constants/regionPresets.ts
 * @brief SSOT presets and MMIO descriptors for TV/Region calibration, MAIN 3410PQ TBC bypasses, and DAVIS IDE probing.
 */

export type RegionPresetLabelKey =
  | 'pipelines.calibration.presets.brPalM.label'
  | 'pipelines.calibration.presets.usNtsc.label'
  | 'pipelines.calibration.presets.euPal.label'
  | 'pipelines.calibration.presets.custom.label';

export type RegionPresetDescKey =
  | 'pipelines.calibration.presets.brPalM.desc'
  | 'pipelines.calibration.presets.usNtsc.desc'
  | 'pipelines.calibration.presets.euPal.desc'
  | 'pipelines.calibration.presets.custom.desc';

export type MainBypassLabelKey =
  | 'pipelines.tbcSd.bypasses.fpcStatus.label'
  | 'pipelines.tbcSd.bypasses.oddBranch.label'
  | 'pipelines.tbcSd.bypasses.oddInit.label'
  | 'pipelines.tbcSd.bypasses.hddCmd.label'
  | 'pipelines.tbcSd.bypasses.fmuDevinfo.label';

export type MainBypassDescKey =
  | 'pipelines.tbcSd.bypasses.fpcStatus.desc'
  | 'pipelines.tbcSd.bypasses.oddBranch.desc'
  | 'pipelines.tbcSd.bypasses.oddInit.desc'
  | 'pipelines.tbcSd.bypasses.hddCmd.desc'
  | 'pipelines.tbcSd.bypasses.fmuDevinfo.desc';

export type TvStandardSlug = 'ntsc' | 'pal_m' | 'pal' | 'secam';
export type LbsRegionPresetSlug = 'br_pal_m_r4' | 'us_ntsc_r1' | 'eu_pal_r2' | 'custom_inspected';
export type MainBypassId =
  | 'fpc_status_ctrl'
  | 'odd_state_branch'
  | 'odd_init_status'
  | 'hdd_cmd'
  | 'fmu_devinfo';

export const REGION_CALIBRATION_CONFIRM_TOKEN = 'CALIBRATE_REGION_COW';
export const TBC_BYPASS_CONFIRM_TOKEN = 'APPLY_MAIN_TBC_COW';
export const UPPER_NOR_CONFIRM_TOKEN = 'CONFIRM_UPPER_NOR';
export const HDD_CONFIRM_TOKEN_PREFIX = 'CONFIRM-';
export const SD_FAT16_MAX_BYTES = 2 * 1024 * 1024 * 1024;
export const IFODEV2_BYTE_SIZE = 2048;
export const DEV_BIN_BYTE_SIZE = 512;
export const VER061_DIGADIAG_RAW_BYTES = 262_144;
export const SD48_TOC_STRIDE_BYTES = 48;
export const SD48_MODEL_PREFIX_M_HEX = '0x4D';
export const LOADER_VER061_TIMER_MCU_RANGE = '0x440340b9..4101';
export const FACTORY_PROG061_DATE_TAG = '[05/11/01]';
export const DEFAULT_BGO_BOOTTRACE_MS = 15_000;

export const AIOF_MMIO_BASE = 0x8800_0000;
export const AIOF_VMODESEL_NTSC = 0x8020;
export const AIOF_VMODESEL_PAL_M = 0x8021;
export const AIOF_VMODESEL_PAL = 0x8027;
export const AIOF_FS0_STANDARD = 0x40f0;
export const AIOF_YC3_MVAGC_TBC = 0x0020;
export const AVDD_NPALCTL0_ADDR = 0x8980_0624;

export interface RegionCalibrationPresetSpec {
  readonly id: LbsRegionPresetSlug;
  readonly labelKey: RegionPresetLabelKey;
  readonly descKey: RegionPresetDescKey;
  readonly tvStandard: TvStandardSlug;
  readonly dvdRegionCode: number | null;
  readonly chassisFamily: string;
  readonly defaultModelCode: string;
  readonly defaultOemChar: string;
  readonly byteMutationSupport: 'METADATA_AND_DEV_OEM_ONLY';
}

export const REGION_CALIBRATION_PRESETS: readonly RegionCalibrationPresetSpec[] = Object.freeze([
  {
    id: 'br_pal_m_r4',
    labelKey: 'pipelines.calibration.presets.brPalM.label',
    descKey: 'pipelines.calibration.presets.brPalM.desc',
    tvStandard: 'pal_m',
    dvdRegionCode: 4,
    chassisFamily: 'M6930LB',
    defaultModelCode: 'EH55LB',
    defaultOemChar: 'M',
    byteMutationSupport: 'METADATA_AND_DEV_OEM_ONLY',
  },
  {
    id: 'us_ntsc_r1',
    labelKey: 'pipelines.calibration.presets.usNtsc.label',
    descKey: 'pipelines.calibration.presets.usNtsc.desc',
    tvStandard: 'ntsc',
    dvdRegionCode: 1,
    chassisFamily: 'M6930',
    defaultModelCode: 'EH55P',
    defaultOemChar: 'P',
    byteMutationSupport: 'METADATA_AND_DEV_OEM_ONLY',
  },
  {
    id: 'eu_pal_r2',
    labelKey: 'pipelines.calibration.presets.euPal.label',
    descKey: 'pipelines.calibration.presets.euPal.desc',
    tvStandard: 'pal',
    dvdRegionCode: 2,
    chassisFamily: 'M6930',
    defaultModelCode: 'EH55EG',
    defaultOemChar: 'E',
    byteMutationSupport: 'METADATA_AND_DEV_OEM_ONLY',
  },
  {
    id: 'custom_inspected',
    labelKey: 'pipelines.calibration.presets.custom.label',
    descKey: 'pipelines.calibration.presets.custom.desc',
    tvStandard: 'ntsc',
    dvdRegionCode: null,
    chassisFamily: 'M6930',
    defaultModelCode: 'EH55',
    defaultOemChar: 'P',
    byteMutationSupport: 'METADATA_AND_DEV_OEM_ONLY',
  },
]);

export interface MainBypassDescriptor {
  readonly id: MainBypassId;
  readonly vaHex: string;
  readonly offsetHex: string;
  readonly origHex: string;
  readonly patchedHex: string;
  readonly labelKey: MainBypassLabelKey;
  readonly descKey: MainBypassDescKey;
  readonly standaloneDefault: boolean;
}

export const MAIN_TBC_BYPASS_SPECS: readonly MainBypassDescriptor[] = Object.freeze([
  {
    id: 'fpc_status_ctrl',
    vaHex: '0x5771ABFB',
    offsetHex: '0x71ABFB',
    origHex: 'cfe0f8fed082',
    patchedHex: '8001f0fccbcb',
    labelKey: 'pipelines.tbcSd.bypasses.fpcStatus.label',
    descKey: 'pipelines.tbcSd.bypasses.fpcStatus.desc',
    standaloneDefault: true,
  },
  {
    id: 'odd_state_branch',
    vaHex: '0x574F7B8C',
    offsetHex: '0x4F7B8C',
    origHex: 'bf03c110',
    patchedHex: 'ccd000cb',
    labelKey: 'pipelines.tbcSd.bypasses.oddBranch.label',
    descKey: 'pipelines.tbcSd.bypasses.oddBranch.desc',
    standaloneDefault: true,
  },
  {
    id: 'odd_init_status',
    vaHex: '0x574F7C5C',
    offsetHex: '0x4F7C5C',
    origHex: 'ddcd3c00000000',
    patchedHex: '8000cbcbcbcbcb',
    labelKey: 'pipelines.tbcSd.bypasses.oddInit.label',
    descKey: 'pipelines.tbcSd.bypasses.oddInit.desc',
    standaloneDefault: true,
  },
  {
    id: 'hdd_cmd',
    vaHex: '0x574DD1FC',
    offsetHex: '0x4DD1FC',
    origHex: 'c6',
    patchedHex: 'ca',
    labelKey: 'pipelines.tbcSd.bypasses.hddCmd.label',
    descKey: 'pipelines.tbcSd.bypasses.hddCmd.desc',
    standaloneDefault: true,
  },
  {
    id: 'fmu_devinfo',
    vaHex: '0x57498738',
    offsetHex: '0x498738',
    origHex: 'deviceinfo_strict',
    patchedHex: 'deviceinfo_bypass',
    labelKey: 'pipelines.tbcSd.bypasses.fmuDevinfo.label',
    descKey: 'pipelines.tbcSd.bypasses.fmuDevinfo.desc',
    standaloneDefault: true,
  },
]);

export const DAVIS_MMIO_SPECS = Object.freeze({
  ch0BaseHex: '0x89000500',
  ch1BaseHex: '0x89000D00',
  ch0StatusHex: '0x8900053C',
  ch1StatusHex: '0x89000D3C',
  ch0AltStatusHex: '0x89000558',
  ch1AltStatusHex: '0x89000D58',
  ch0DriveHeadHex: '0x89000538',
  ch1DriveHeadHex: '0x89000D38',
  sysStatusHex: '0xC0001008',
  wdbcHex: '0xC0001000',
  lethalRegHex: '0xC000100C',
  errAtapi40dHex: '0x8000040D',
  errTimeout32sHex: '0x8000040F',
  ataIdentifyCmdHex: '0xEC',
  atapiIdentifyCmdHex: '0xA1',
  atapiLbaMid: 0x14,
  atapiLbaHigh: 0xeb,
  statusBsyBit: 0x80,
  statusDrdyBit: 0x40,
  statusDrqBit: 0x08,
  statusErrBit: 0x01,
  driveHeadSlaveBit: 0x10,
});
