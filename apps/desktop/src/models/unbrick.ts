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
  checks: SdUnbrickCheck[];
}

export interface SdUnbrickKitRequestDto {
  output_dir: string;
  pana_dvd_path?: string;
  prog061_path?: string;
  main_path?: string;
  drv_path?: string;
  bgo_mode?: 'auto_format' | 'bypass' | 'uart_monitor' | 'factory';
  main_passthrough_mode?: 'av_passthrough' | 'factory_original';
  include_pana_sd?: boolean;
  unlock_hdd_dynamic?: boolean;
  hdd_target_gb?: number | null;
  write_extracted_sources?: boolean;
}

export interface SdUnbrickKitReport {
  output_dir: string;
  prog061_source: string;
  main_source: string | null;
  drv_source: string | null;
  bgo_mode_applied: string;
  main_passthrough_applied: string | null;
  hdd_capacity_applied: string | null;
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
