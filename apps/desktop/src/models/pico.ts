/**
 * Raspberry Pi Pico (RP2040 / RP2350) P7501 Async UART (115200 8N1) Hardware Bench Models
 * Matches firmware/pico-p7501/p7501_types.h & p7501_pico_main.c
 */

export type ColdBreakinMode = 0 | 1 | 2 | 3;

export interface ColdBreakinModeInfo {
  mode: ColdBreakinMode;
  name: string;
  description: string;
  timing: string;
  recommended: boolean;
}

export type OcdCoreState = 'HALTED' | 'RUNNING' | 'RESET' | 'UNKNOWN' | 'DISCONNECTED';

export interface PicoDeviceStatus {
  connected: boolean;
  port: string;
  baudRate: number;
  vdd_volts: number;
  target_powered: boolean;
  breakin_mode: ColdBreakinMode;
  core_state: OcdCoreState;
  bus_freq_hz: number;
  firmware_version: string;
  chip_id: string; // e.g. "RP2040-B2"
  total_probes: number;
  hits_detected: number;
  open_bus_high: number;
  open_bus_low: number;
  ram_loader_injected: boolean;
  last_keepalive: string;
  wdt_active?: boolean;
  wdt_timeout_ms?: number;
  wdt_recoveries?: number;
  wdt_reboot_src?: string;
  wdt_fault_stage?: string;
  wdt_last_cmd?: string;
}

export type PicoBenchSectionFilter =
  | 'all'
  | 'diag'
  | 'bridge'
  | 'bgo'
  | 'davis'
  | 'gdb'
  | 'terminal';

export type LogLevel = 'info' | 'warn' | 'error' | 'success' | 'tx' | 'rx';

export interface SerialLogEntry {
  id: string;
  timestamp: string;
  level: LogLevel;
  source: 'HOST' | 'PICO' | 'TARGET';
  message: string;
  hexData?: string;
}

export interface InCircuitNvramEraseResult {
  success: boolean;
  elapsed_ms: number;
  voltage_before: number;
  voltage_after: number;
  ram_loader_addr: string;
  bytes_erased: number;
  verification_sha256: string;
  log_lines: string[];
}

export type PicoArchitecture = 'Rp2040' | 'Rp2350' | 'Bootloader' | 'Unknown';

export interface PicoDeviceInfo {
  port_name: string;
  architecture: PicoArchitecture;
  vid: number;
  pid: number;
  description: string;
}

export interface PicoStatusResponse {
  connected: boolean;
  port_name: string;
  baud_rate: number;
  vdd_voltage: number;
  target_powered: boolean;
  firmware_banner: string;
  message: string;
}

export interface GdbBridgeStatus {
  running: boolean;
  port_name: string;
  listen_addr: string;
  connected_clients: number;
  last_event: string;
}

export interface Am34RegisterState {
  d: [number, number, number, number];
  a: [number, number, number, number];
  mdr: number;
  psw: number;
  pc: number;
  sp: number;
  is_halted: boolean;
}

export interface GdbPacketTelemetryEvent {
  timestamp_ms: number;
  direction: string;
  command_type: string;
  raw_packet: string;
  checksum_valid: boolean;
}

export interface DigaModelProfile {
  model_id: string;
  series: string;
  flash_size_mb: number;
  flash_size_bytes: number;
  nvram_base_hex: string;
  sector_geometry: string;
  soc_name: string;
}

export interface FirstContactBackupResult {
  success: boolean;
  detected_model: string;
  nvram_bytes: number;
  nvram_sha256: string;
  double_read_verified: boolean;
  sram_bytes: number;
  ipl_bytes: number;
  log_lines: string[];
}

export * from './picoSsot';

export interface PincheckRow {
  pin_label: string;
  hiz: string;
  pull_down: string;
  pull_up: string;
  state: string;
}

export interface BusTransitionRow {
  signal: string;
  gpio: string;
  final_level?: number | null;
  transitions: number;
}

export interface DumpWordRow {
  address: string;
  words: string[];
  ascii: string;
}

export interface DiagnosticFinding {
  code: string;
  severity: 'info' | 'warning' | 'error';
  summary: string;
  detail: string;
}

export interface PicoDiagnosticResult {
  command_id: string;
  raw_command: string;
  success: boolean;
  elapsed_ms: number;
  summary: string;
  raw_output: string;
  vdd_volts?: number | null;
  detected_flash_mb?: number | null;
  xreset_rise_us?: string | null;
  pincheck_rows: PincheckRow[];
  transition_rows: BusTransitionRow[];
  dump_rows: DumpWordRow[];
  captured_bytes?: number | null;
  findings?: DiagnosticFinding[];
}

export interface FlashReport {
  success: boolean;
  target_drive: string;
  board_family: string;
  uf2_size_bytes: number;
  blocks_written: number;
  sha256_hash: string;
  message: string;
}

export interface ReactiveSerialEvent {
  id: string;
  timestamp: string;
  source: 'HOST' | 'PICO' | 'TARGET';
  level: LogLevel;
  message: string;
}

export type Sc0BridgeInputMode = 'text' | 'line' | 'hex' | 'rsp';

export interface Sc0BridgeStatus {
  running: boolean;
  port_name: string;
  baud_rate: number;
  boot_mode: string;
  bytes_rx: number;
  bytes_tx: number;
  last_event: string;
}

export interface Sc0BridgeDataEvent {
  stream: 'rx' | 'tx' | 'sys';
  text: string;
  hex: string;
  bytes: number[];
  timestamp: string;
}

export interface BridgeStreamLine extends Sc0BridgeDataEvent {
  id: string;
}

export type SerialConsoleState =
  | { status: 'idle' }
  | { status: 'sending'; command: string }
  | { status: 'error'; message: string };



