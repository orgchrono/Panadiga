/**
 * TypeScript domain models matching `crates/panadiga-core/src/re/unbrick_*` & Tauri IPC DTOs.
 */

export interface Prog061BgoSiteStatus {
  va_hex: string;
  file_offset_hex: string;
  state: string;
  bytes_hex: string;
}

export interface Prog061BgoInspection {
  raw_size: number;
  raw_sha256: string;
  detected_mode: string;
  bgo_sum_site: Prog061BgoSiteStatus;
  devinfo_site: Prog061BgoSiteStatus;
  bgo_init_hook_site: Prog061BgoSiteStatus;
}

export interface MainPassthroughSiteReport {
  name: string;
  description: string;
  va_hex: string;
  file_offset_hex: string;
  length: number;
  state: string;
  current_bytes_hex: string;
  rsp_passthrough_frame: string;
  rsp_factory_frame: string;
}

export interface MainPassthroughReport {
  file_size: number;
  expected_size_match: boolean;
  detected_mode: string;
  sites: MainPassthroughSiteReport[];
  output_path: string | null;
  applied_mode: string | null;
}

export interface MainPassthroughRspPlan {
  handshake_hex: string;
  nak_byte_hex: string;
  serial_state_base_va: string;
  serial_callback_va: string;
  isr_ctx_ptr_va: string;
  isr_saved_pc_offset: string;
  passthrough_frames: string[];
  factory_restore_frames: string[];
  callback_write_frame: string | null;
  pico_cli_shortcuts: string[];
}

export interface SdUnbrickCheck {
  id: string;
  severity: 'pass' | 'warn' | 'fail' | string;
  title: string;
  detail: string;
}

export interface MbrPartitionRecord {
  slot: number;
  boot_flag: number;
  partition_type: number;
  type_name: string;
  lba_start: number;
  sector_count: number;
}

export interface FatBpbReport {
  oem_name: string;
  bytes_per_sector: number;
  sectors_per_cluster: number;
  cluster_size_bytes: number;
  reserved_sectors: number;
  num_fats: number;
  root_entry_count: number;
  total_sectors: number;
  media_descriptor: number;
  sectors_per_fat: number;
  hidden_sectors: number;
  fat_type_label: string;
  volume_label: string;
  data_clusters: number;
  computed_fat_kind: string;
}

export interface RootDirFileReport {
  name_8_3: string;
  attr: number;
  hidden: boolean;
  system: boolean;
  read_only: boolean;
  first_cluster: number;
  file_size: number;
  has_preceding_lfn: boolean;
  chain_clusters: number[];
  chain_contiguous: boolean;
  chain_complete: boolean;
}

export interface SdDiskInspectionReport {
  target: string;
  partition_scheme: string;
  mbr_partitions: MbrPartitionRecord[];
  active_partition_lba?: number | null;
  bpb?: FatBpbReport | null;
  digadiag_entry?: RootDirFileReport | null;
  pana_sd_entry?: RootDirFileReport | null;
  root_files_sample: string[];
  digadiag_payload_verified: boolean;
  overall_status: string;
  checks: SdUnbrickCheck[];
}

export interface DualFamilySlotResolution {
  hardware_family: string;
  tag_prefix: string;
  slot_index: number;
  model_prefix_byte_hex: string;
  model_prefix_is_m: boolean;
  list_sum_valid: boolean;
  fvu_sum_valid: boolean;
  module_offset: number;
  module_size: number;
  fvu_checksum_u32: number;
}

export interface DualFamilyParityReport {
  is_sd48_layout: boolean;
  m6930lb_prog061: DualFamilySlotResolution | null;
  m6930lb_main: DualFamilySlotResolution | null;
  m6930_prog061: DualFamilySlotResolution | null;
  m6930_main: DualFamilySlotResolution | null;
  m6930lb_drv: DualFamilySlotResolution | null;
  m6930_drv: DualFamilySlotResolution | null;
  dual_parity_ok: boolean;
  issues: string[];
}

export interface SdMediaVerificationReport {
  sd_root_dir: string;
  overall_status: string;
  digadiag_present: boolean;
  digadiag_exact_size_ok: boolean;
  digadiag_dst_addr_hex: string | null;
  digadiag_bgo_mode: string | null;
  pana_sd_present: boolean;
  pana_sd_toc_entries: number;
  pana_sd_list_sums_ok: boolean;
  pana_sd_prog061_mode: string | null;
  pana_sd_main_build: string | null;
  pana_sd_main_passthrough_mode: string | null;
  pana_sd_main_hdd_dynamic: boolean | null;
  digadiag_bgo_geometry?: string | null;
  pana_sd_prog061_bgo_geometry?: string | null;
  pana_sd_main_hdd_capacity_label?: string | null;
  pana_sd_prog061_sata_bridge_fix?: string | null;
  pana_sd_prog061_stage2_boot?: string | null;
  pana_sd_main_sata_bridge_fix?: string | null;
  factory_unpatched_prog061_detected?: boolean;
  dual_family_parity?: DualFamilyParityReport | null;
  checks: SdUnbrickCheck[];
}

export type SdLoaderProfile = 'ver061' | 'ver063';

export interface SdUnbrickKitRequestDto {
  output_dir: string;
  pana_dvd_path?: string;
  prog061_path?: string;
  main_path?: string;
  drv_path?: string;
  bgo_mode?: 'auto_format' | 'bypass' | 'uart_monitor' | 'factory';
  allow_unsafe_auto_format?: boolean;
  main_passthrough_mode?: 'av_passthrough' | 'factory_original';
  loader_profile?: SdLoaderProfile;
  include_pana_sd?: boolean;
  unlock_hdd_dynamic?: boolean;
  hdd_target_gb?: number | null;
  write_extracted_sources?: boolean;
  expected_main_build?: string;
  allow_target_mismatch?: boolean;
  sata_bridge_reset_fix?: boolean;
  sata_bridge_stage2_boot?: boolean;
}

export interface SdUnbrickKitReport {
  output_dir: string;
  prog061_source: string;
  main_source: string | null;
  drv_source: string | null;
  bgo_mode_applied: string;
  loader_profile_applied?: string;
  main_passthrough_applied: string | null;
  hdd_capacity_applied: string | null;
  sata_bridge_reset_fix_applied?: boolean;
  sata_bridge_stage2_boot_applied?: boolean;
  digadiag_path: string;
  digadiag_sha256: string;
  pana_sd_path: string | null;
  pana_sd_sha256: string | null;
  sha256sums_path: string;
  verification: SdMediaVerificationReport;
}

export interface ExtractedFrmModuleReport {
  name: string;
  offset_hex: string;
  encrypted_size: number;
  decrypted_path: string;
  decrypted_sha256: string;
  unpacked_path: string | null;
  unpacked_size: number | null;
  dst_addr_hex: string | null;
  checksum_valid: boolean | null;
}

export interface FrmModuleExtractionReport {
  source_frm_path: string;
  container_format: string;
  output_dir: string;
  modules: ExtractedFrmModuleReport[];
}

import type { PreWriteGateReport } from './vault';

export interface UnbrickHardwareFact {
  topic: string;
  verdict: string;
  evidence: string;
}

export interface UnbrickWorkspaceReport {
  workspace_root: string;
  discovered_pana_dvd: string | null;
  discovered_prog061: string | null;
  discovered_main_3410pq: string | null;
  discovered_drv: string | null;
  discovered_sd_dir: string | null;
  prog061_bgo: Prog061BgoInspection | null;
  main_passthrough: MainPassthroughReport | null;
  main_capacity: {
    patch_site_va_hex: string;
    is_dynamic_unclamped: boolean;
    effective_default_limit_gb: number;
  } | null;
  rsp_plan: MainPassthroughRspPlan;
  sd_verification: SdMediaVerificationReport | null;
  hardware_facts: UnbrickHardwareFact[];
  digadiag_vs_pana_notes: string[];
}

export interface NorDumperStubResult {
  start_addr: string;
  total_len_bytes: number;
  block_size: number;
  load_addr: string;
  raw_stub_bytes: number;
  wrapped_digadiag: boolean;
  output_bytes: number;
  written_path: string | null;
  stub_hex: string;
}

export * from './matshita';

export type BgoMode = 'auto_format' | 'bypass' | 'uart_monitor' | 'factory';
export type MainPassthroughMode = 'av_passthrough' | 'factory_original';
export type HddCapacityTargetOption = 'dynamic' | '250' | '400' | '300' | '200' | '160' | '120' | '80';

export type SdCreatorStage =
  | 'synthesize'
  | 'lockout_verify'
  | 'delete_layout'
  | 'write_partition'
  | 'readback_verify'
  | 'inspect_fat16';

export interface SdFlashProgressEvent {
  readonly target: string;
  readonly stage: SdCreatorStage;
  readonly bytesDone: number;
  readonly bytesTotal: number;
  readonly detail: string;
}

export interface SdCardAdvisory {
  readonly is_sdxc_warning: boolean;
  readonly card_class: string;
  readonly recommendation: string;
  readonly detail: string;
}

export interface SdTargetDriveInfo {
  readonly device_path: string;
  readonly display_name: string;
  readonly size_bytes: number;
  readonly size_gb: number;
  readonly drive_role: string;
  readonly bus_type: string;
  readonly removable: boolean;
  readonly safe_sd_target: boolean;
  readonly expected_confirm_token: string;
  readonly advisory: SdCardAdvisory;
  readonly block_reason: string | null;
}

export interface Fat16PlacedFileReport {
  readonly name_8_3: string;
  readonly size_bytes: number;
  readonly first_cluster: number;
  readonly sector_count: number;
  readonly sha256: string;
}

export interface SdCardCreatorReport {
  readonly target: string;
  readonly target_kind: string;
  readonly model: string;
  readonly volume_mb: number;
  readonly cleanroom_only: boolean;
  readonly bytes_written: number;
  readonly image_sha256: string;
  readonly advisory: SdCardAdvisory;
  readonly files: readonly Fat16PlacedFileReport[];
  readonly emu_gate: PreWriteGateReport;
  readonly readback_verified: boolean;
  readonly inspection: SdDiskInspectionReport;
  readonly next_steps: readonly string[];
}

export interface FlashSdRecoveryCardRequest {
  readonly devicePath?: string;
  readonly confirmToken?: string;
  readonly outputImagePath?: string;
  readonly model?: string;
  readonly volumeMb?: number;
  readonly loaderProfile?: SdLoaderProfile;
  readonly digadiagPath?: string;
  readonly panaSdPath?: string;
  readonly panaDvdPath?: string;
  readonly includePanaSd?: boolean;
  readonly cleanroomOnly?: boolean;
  readonly sessionId?: string;
}
