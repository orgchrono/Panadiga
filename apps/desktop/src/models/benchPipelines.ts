/**
 * @file models/benchPipelines.ts
 * @brief Domain models for Live Pico NOR Dump, 45-Sector BGO FTL Audit/Repair, and DAVIS CH0/CH1 IDE Probing.
 */

import type { PreWriteGateReport } from './vault';

export interface BridgeExitSummary {
  readonly rx_bytes: number;
  readonly tx_bytes: number;
  readonly ferr: number;
  readonly ctrl_edges: number;
  readonly overflow: boolean;
  readonly raw_line: string;
}

export type NorSliceKind = 'full' | 'bgo' | 'nvram' | 'loader' | 'custom';
export type BgoAuditSourceMode = 'DUMP_FILE' | 'LIVE_PICO';
export type Eh55BgoSectorKind =
  | 'Spare'
  | 'Partition1Active'
  | 'Partition1Migrating'
  | 'Partition2'
  | 'Corrupt';
export type BgoApplyStage =
  | 'pre_read_verify'
  | 'backup_write'
  | 'unlock_upper_nor'
  | 'erase_and_program'
  | 'lock_upper_nor'
  | 'post_read_verify'
  | 'boot_trace'
  | 'complete';

export interface PicoEh55NorDumpRequest {
  readonly portName: string;
  readonly slice?: NorSliceKind;
  readonly customStartVaHex?: string;
  readonly customLenHex?: string;
  readonly outputPath?: string;
}

export interface PicoPanamon1CleanNvramRequest {
  readonly portName: string;
  readonly backupDir?: string;
  readonly sessionId?: string;
}

export interface NvramCleanProgressEvent {
  readonly port: string;
  readonly stage: string;
  readonly current: number;
  readonly total: number;
  readonly detail: string;
}

export interface PicoPanamon1CleanNvramResponse {
  readonly backupPath: string;
  readonly sha256Before: string;
  readonly sha256After: string;
  readonly erasedSectors: number;
  readonly verifiedAllFf: boolean;
  readonly bytesErased: number;
  readonly emuGate?: PreWriteGateReport;
  readonly nextStep: string;
}

export interface NorRepairExpressOutcome {
  readonly mode: 'nvram_clean' | 'bgo_cow' | 'bgo_live';
  readonly summaryPath: string;
  readonly sha256: string;
}

export interface StreamProgressTelemetryFields {
  readonly activeSa?: number;
  readonly bytesDone?: number;
  readonly bytesTotal?: number;
  readonly bytesPerSec?: number;
  readonly etaSeconds?: number;
}

export interface NorDumpProgressEvent extends StreamProgressTelemetryFields {
  readonly port: string;
  readonly stage: string;
  readonly current: number;
  readonly total: number;
}

export interface NorSliceCaptureReport {
  readonly start_va: number;
  readonly len: number;
  readonly sha256: string;
  readonly bridge_summary: BridgeExitSummary | null;
  readonly checkpoint_path: string | null;
  readonly resumed_from_checkpoint: boolean;
}

export interface PicoEh55NorDumpResponse {
  readonly outputPath: string;
  readonly startVaHex: string;
  readonly totalLen: number;
  readonly sha256: string;
  readonly slices: readonly NorSliceCaptureReport[];
}

export interface PicoEh55BgoAuditRequest {
  readonly sourceMode: BgoAuditSourceMode;
  readonly norPath?: string;
  readonly portName?: string;
}

export interface Eh55BgoSectorReport {
  readonly sa: number;
  readonly flash_offset: number;
  readonly phys_va: number;
  readonly sector_id: number;
  readonly kind: Eh55BgoSectorKind;
  readonly migration_idx: number | null;
  readonly migration_target: number;
  readonly zero_marker_ok: boolean;
}

export interface Eh55BgoAuditReport {
  readonly healthy: boolean;
  readonly total_sectors: number;
  readonly p1_active_count: number;
  readonly p1_migrating_count: number;
  readonly p2_present_ids: readonly number[];
  readonly p2_missing_ids: readonly number[];
  readonly spare_sas: readonly number[];
  readonly corrupt_sas: readonly number[];
  readonly active_l2p_pages: number;
  readonly l2p_tag_mismatches: number;
  readonly fat_mirror_mismatches: number;
  readonly nfrom_file_count: number;
  readonly ifodev2_model: string | null;
  readonly sectors: readonly Eh55BgoSectorReport[];
}

export type BgoRepairOp =
  | { readonly op: 'erase_sector'; readonly sa: number; readonly phys_va: number }
  | {
      readonly op: 'write_bytes';
      readonly sa: number;
      readonly phys_va: number;
      readonly data: readonly number[];
      readonly requires_erase_first: boolean;
    };

export interface BgoRepairPlan {
  readonly bgo_window_start_va: number;
  readonly bgo_window_end_va: number;
  readonly target_sectors: readonly number[];
  readonly source_nor_sha256: string;
  readonly source_bgo_sha256: string;
  readonly expected_post_sha256: string;
  readonly expected_post_nor_sha256: string;
  readonly missing_f2_ids: readonly number[];
  readonly assigned_spare_sas: readonly number[];
  readonly erased_sas: readonly number[];
  readonly total_write_bytes: number;
  readonly ops: readonly BgoRepairOp[];
  readonly post_audit: Eh55BgoAuditReport;
}

export interface BgoRepairProgressEvent extends StreamProgressTelemetryFields {
  readonly stage: BgoApplyStage;
  readonly current: number;
  readonly total: number;
  readonly detail: string;
}

export interface BgoLiveApplyReport {
  readonly backup_path: string;
  readonly backup_sha256: string;
  readonly pre_bgo_sha256: string;
  readonly post_bgo_sha256: string;
  readonly erased_sectors: number;
  readonly written_bytes: number;
  readonly boot_trace_output: string | null;
  readonly boot_normal_ok: boolean | null;
}

export interface PicoEh55BgoRepairRequest {
  readonly norPath?: string;
  readonly planPath?: string;
  readonly referenceNorPath?: string;
  readonly outPlanPath?: string;
  readonly cowOutPath?: string;
  readonly dryRunOnly: boolean;
  readonly portName?: string;
  readonly backupDir?: string;
  readonly confirmToken?: string;
  readonly runBootTraceMs?: number;
}

export interface PicoEh55BgoRepairResponse {
  readonly dryRunOnly: boolean;
  readonly sa0ToSa3LockedE05: boolean;
  readonly plan: BgoRepairPlan;
  readonly outPlanPath: string | null;
  readonly cowOutPath: string | null;
  readonly liveReport: BgoLiveApplyReport | null;
}

export type DavisChannelId = 'ch0' | 'ch1';
export type DavisDriveSelector = 'master' | 'slave';
export type DavisHangClassification =
  | 'floating_bus'
  | 'reset_stuck_or_unclocked'
  | 'bsy_iordy_hang'
  | 'cs_jumper_or_slave_contention'
  | 'atapi_signature_detected'
  | 'ata_ready'
  | 'unknown';

export interface DavisTaskfileSnapshot {
  readonly data: number;
  readonly err: number;
  readonly seccnt: number;
  readonly lbalo: number;
  readonly lbamid: number;
  readonly lbahi: number;
  readonly devsel: number;
  readonly status: number;
  readonly altstat: number;
  readonly sentinel: number;
}

export interface AtaIdentifyInfo {
  readonly word0: number;
  readonly is_atapi: boolean;
  readonly model: string;
  readonly serial_number: string;
  readonly firmware_revision: string;
  readonly lba28_sectors: number;
  readonly lba48_supported: boolean;
  readonly lba48_sectors: number;
  readonly effective_sectors: number;
  readonly capacity_bytes: number;
  readonly mirror_main_lba: number | null;
  readonly word63_mwdma: number;
  readonly mwdma2_supported: boolean;
  readonly mwdma_active_mode: number | null;
  readonly word88_udma: number;
  readonly udma_supported_mask: number;
  readonly udma_active_mode: number | null;
}

export interface DavisProbeReport {
  readonly c0001008_before: number;
  readonly c0001008_after: number;
  readonly fe_released: boolean;
  readonly ch0_taskfile: DavisTaskfileSnapshot;
  readonly ch1_raw: DavisTaskfileSnapshot;
  readonly ch1_master: DavisTaskfileSnapshot;
  readonly ch1_slave: DavisTaskfileSnapshot;
  readonly ch1_2b_reset: DavisTaskfileSnapshot | null;
  readonly ch1_2c_reset: DavisTaskfileSnapshot | null;
  readonly fun_cd4d_rc: number;
  readonly ch1_struct_hex: string;
  readonly identify_sentinel: number | null;
  readonly identify_post_altstat: number | null;
  readonly identify_info: AtaIdentifyInfo | null;
  readonly classification: DavisHangClassification;
  readonly step_logs: readonly string[];
}

export interface PicoProbeDavisIdeRequest {
  readonly portName: string;
  readonly drive?: DavisDriveSelector;
  readonly bootMode?: string;
  readonly bootTimeoutMs?: number;
}

export interface PicoProbeDavisIdeResponse {
  readonly port: string;
  readonly driveSelHex: string;
  readonly lethal0xc000100cBlocked: boolean;
  readonly report: DavisProbeReport;
}
