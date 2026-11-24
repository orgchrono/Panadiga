/**
 * @file models/pipelines.ts
 * @brief Shared pipeline UI primitives plus Region/TV Calibration, Safe HDD Flash, and TBC/SD Hardening DTOs.
 */

import type { LbsRegionPresetSlug, TvStandardSlug } from '@/constants';
import type { DecryptedFrmTocEntry } from './crypto';
import type { NvramCleanReport } from './nvram';
import type {
  DualFamilyParityReport,
  MainPassthroughMode,
  MainPassthroughReport,
  SdMediaVerificationReport,
  SdUnbrickKitReport,
} from './unbrick';

export * from './benchPipelines';
export * from './hddFlashModels';

export type PipelineStepStatus = 'pending' | 'running' | 'verified' | 'blocked' | 'failed';
export type SafetySeverity = 'critical' | 'warning' | 'info';

export interface PipelineStepDescriptor {
  readonly id: string;
  readonly label: string;
  readonly description?: string;
  readonly status: PipelineStepStatus;
}

export interface PipelineProgressState {
  readonly current: number;
  readonly total: number;
  readonly stageLabel: string;
  readonly activeSa?: number;
  readonly bytesPerSec?: number;
  readonly etaSeconds?: number;
}

export interface PreflightCheckItem {
  readonly id: string;
  readonly title: string;
  readonly detail: string;
  readonly severity: SafetySeverity;
  readonly lockedReason?: string;
}

export interface PipelinePreflightAndGateProps {
  readonly checkedPreflightIds: readonly string[];
  readonly togglePreflight: (id: string) => void;
  readonly preflightChecks: readonly PreflightCheckItem[];
  readonly preflightReady: boolean;
  readonly confirmTokenInput: string;
  readonly setConfirmTokenInput: (v: string) => void;
  readonly expectedConfirmToken: string;
}

export interface HardwareRegisterSnapshot {
  readonly id: string;
  readonly name: string;
  readonly addressHex: string;
  readonly valueHex: string;
  readonly widthBits: number;
  readonly statusLabel: string;
  readonly severity: SafetySeverity | 'ok';
  readonly flagBreakdown?: string;
  readonly note?: string;
}

export interface MmioWriteOp {
  readonly addr: number;
  readonly width_bits: number;
  readonly value: number;
  readonly register_name: string;
  readonly note: string;
}

export interface TvCalibrationProfile {
  readonly preset: LbsRegionPresetSlug;
  readonly tv_standard: TvStandardSlug;
  readonly chassis_family: string;
  readonly dvd_region_code: number | null;
  readonly expected_oem_char: string | null;
  readonly metadata_and_dev_oem_only: boolean;
  readonly mmio_plan: readonly MmioWriteOp[];
  readonly notes: readonly string[];
}

export interface DevBinReport {
  readonly size_bytes: number;
  readonly model: string;
  readonly oem: string;
  readonly signature: string;
  readonly md5: string;
  readonly sha256: string;
}

export interface Ifodev2TailMetadata {
  readonly offset_range_hex: string;
  readonly byte_len: number;
  readonly non_ff_bytes: number;
  readonly prefix_hex: string;
  readonly sha256: string;
  readonly metadata_and_dev_oem_only: boolean;
}

export interface MainRegionCompatReport {
  readonly build_profile: string;
  readonly size_bytes: number;
  readonly va_hex: string;
  readonly file_offset_hex: string;
  readonly state: string;
  readonly actual_hex: string;
  readonly sha256: string;
}

export interface FrmRetargetReport {
  readonly layout: string;
  readonly total_bytes: number;
  readonly updated_entries: number;
  readonly entries: readonly DecryptedFrmTocEntry[];
  readonly sha256: string;
}

export interface FrmTocInspectionSummary {
  readonly layout: string;
  readonly entry_count: number;
  readonly all_list_sums_valid: boolean;
  readonly entries: readonly DecryptedFrmTocEntry[];
  readonly sha256: string;
}

export interface TvCalibrationBundleReport {
  readonly profile: TvCalibrationProfile;
  readonly dev_bin: DevBinReport | null;
  readonly ifodev2_tail: Ifodev2TailMetadata | null;
  readonly metadata_and_dev_oem_only: boolean;
  readonly main_region: MainRegionCompatReport | null;
  readonly frm_toc: FrmTocInspectionSummary | null;
  readonly nvram_sa134_sa141: NvramCleanReport | null;
  readonly warnings: readonly string[];
}

export interface InspectRegionCalibrationRequest {
  readonly preset?: string;
  readonly tvStandard?: string;
  readonly devBinPath?: string;
  readonly mainPath?: string;
  readonly frmPath?: string;
  readonly nvramPath?: string;
}

export interface ApplyRegionCalibrationRequest {
  readonly preset?: string;
  readonly tvStandard?: string;
  readonly outputDir: string;
  readonly devBinPath?: string;
  readonly model?: string;
  readonly oem?: string;
  readonly mainPath?: string;
  readonly mainCompatMode?: string;
  readonly frmPath?: string;
  readonly hwFamily?: string;
  readonly modelGroup?: string;
  readonly targetFlag?: number;
  readonly confirmToken: string;
}

export interface ApplyRegionCalibrationResponse {
  readonly outputDir: string;
  readonly metadataAndDevOemOnly: boolean;
  readonly devBinOutputPath: string | null;
  readonly devBinReport: DevBinReport | null;
  readonly mainOutputPath: string | null;
  readonly mainReport: MainRegionCompatReport | null;
  readonly frmOutputPath: string | null;
  readonly frmReport: FrmRetargetReport | null;
  readonly bundleAudit: TvCalibrationBundleReport;
}

export type TbcApplianceMode = 'digadiag_ram_tbc' | 'experimental_sd_ram_chainload';

export interface VerifiedMainPassthroughSiteRef {
  readonly name: string;
  readonly va_hex: string;
  readonly file_offset_hex: string;
  readonly length_bytes: number;
  readonly note: string;
}

export interface TbcAppliancePlan {
  readonly mode: TbcApplianceMode;
  readonly tv_standard: TvStandardSlug;
  readonly requires_experimental_flag: boolean;
  readonly warnings: readonly string[];
  readonly steps: readonly string[];
  readonly mmio_ops: readonly MmioWriteOp[];
  readonly nor_sa0_to_sa3_untouched: boolean;
  readonly verified_main_passthrough_sites: readonly VerifiedMainPassthroughSiteRef[];
}

export interface ApplyMainTbcBypassesRequest {
  readonly inputMainPath?: string;
  readonly outputMainPath?: string;
  readonly passthroughMode?: MainPassthroughMode;
  readonly applianceMode?: TbcApplianceMode;
  readonly tvStandard?: TvStandardSlug;
  readonly experimentalRamChainload?: boolean;
  readonly confirmToken: string;
}

export interface ApplyMainTbcBypassesResponse {
  readonly appliancePlan: TbcAppliancePlan;
  readonly mainPatchReport: MainPassthroughReport | null;
  readonly cowOutputPath: string | null;
}

export interface HardenSdUnbrickKitRequest {
  readonly outputDir: string;
  readonly panaDvdPath?: string;
  readonly prog061Path?: string;
  readonly mainPath?: string;
  readonly drvPath?: string;
  readonly mainPassthroughMode?: MainPassthroughMode;
  readonly unlockHddDynamic?: boolean;
  readonly hddTargetGb?: number;
  readonly expectedMainBuild?: string;
  readonly allowTargetMismatch?: boolean;
  readonly allowFactoryProg061?: boolean;
  readonly writeExtractedSources?: boolean;
}

export interface HardenSdUnbrickKitResponse {
  readonly kitReport: SdUnbrickKitReport;
  readonly hardenedVerification: SdMediaVerificationReport;
  readonly dualFamilyParity: DualFamilyParityReport | null;
  readonly factoryProg061Blocked: boolean;
}
