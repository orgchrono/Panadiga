/**
 * Matshita R6-MV4 Drive Emulator & Unlock FRM Container Models
 */

export type MatshitaBootScenario =
  | 'healthy_with_disc'
  | 'unlock_disc_frm'
  | 'tray_cannot_close_80045300'
  | 'laser_apc_failure_80040904';

export interface BootSequenceStep {
  step_index: number;
  boot_fn_va: string;
  boot_fn_name: string;
  cdb_hex: string;
  sense_hex: string;
  boot_log_line: string;
  passed: boolean;
}

export interface BootSequenceReport {
  scenario: MatshitaBootScenario;
  inquiry_id: string;
  steps: BootSequenceStep[];
  uart_boot_transcript: string[];
  startup_succeeded: boolean;
  unlock_media_verified: boolean;
  jump_target_va_hex: string | null;
  final_boot_code_hex: string;
}

export interface WriteInjectionRecord {
  seq: number;
  source: string;
  cdb0_or_cmd: string;
  mode: number;
  target_va_or_lba: number;
  len_bytes: number;
  payload_sha256: string;
  status: string;
  note: string;
}

export interface MatshitaEmulatorSnapshot {
  inquiry_vendor: string;
  inquiry_product: string;
  inquiry_revision: string;
  tray_closed: boolean;
  media_present: boolean;
  sram_0cd8_auth: number;
  sram_0d78_s211: number;
  sram_0242_ed_mode: number;
  mmode: number;
  slog_enabled: boolean;
  slog_entries: string[];
  buffer_len: number;
  buffer_sha256: string;
  disc_lba_sectors: number;
  history_count: number;
  injections: WriteInjectionRecord[];
}

export interface MatshitaInjectWriteResult {
  record: WriteInjectionRecord;
  snapshot: MatshitaEmulatorSnapshot;
}

export interface SerialCliResponse {
  command: string;
  authorized_flag_cd8: number;
  s211_flag_d78: number;
  slog_enabled: boolean;
  output_lines: string[];
  status: string;
}

export interface FrmUnlockVerificationSummary {
  container_kind: string;
  container_bytes: number;
  sha256: string;
  entry_count: number;
  bootloader_jump_va_hex: string;
  extrhead_valid: boolean | null;
  verified: boolean;
  notes: string[];
}

export interface PackUnlockFrmResult {
  verification: FrmUnlockVerificationSummary;
  output_path: string | null;
  container_bytes: number;
}
