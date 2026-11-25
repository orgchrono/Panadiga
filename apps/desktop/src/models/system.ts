/**
 * System Architecture & Local Workspace Status Models
 */

export type {
  AccessibilitySettings,
  FontScaleMode,
  PipelineProgressState as WorkbenchPipelineState,
  ToastItem as ToastMessage,
  WorkbenchExperienceMode,
  WorkbenchTheme,
} from '@/types/workbench';

export type NavigationTabId =
  | 'dashboard'
  | 'nvram'
  | 'hdd'
  | 'pico'
  | 'crypto'
  | 'manuals'
  | 'hex'
  | 'rag';

export type ViewKey =
  | 'dashboard'
  | 'nvram'
  | 'hdd'
  | 'picobench'
  | 'ssotstudio'
  | 'siliconexplorer'
  | 'frmcrypto'
  | 'unbrick'
  | 'manuals'
  | 'hexinspector'
  | 'ircommander'
  | 'settings';

export const ExecutionStatus = {
  IDLE: 'IDLE',
  RUNNING: 'RUNNING',
  SUCCESS: 'SUCCESS',
  ERROR: 'ERROR',
} as const;

export type ExecutionStatus = (typeof ExecutionStatus)[keyof typeof ExecutionStatus];

export interface BinaryFileStatus {
  id: string;
  name: string;
  path: string;
  exists: boolean;
  size_bytes?: number;
  formatted_size?: string;
  expected_role: string;
  status: 'ready' | 'missing' | 'warning';
}

export interface SystemOverview {
  product_model: string;
  soc_processor: string;
  core_architecture: string;
  os_system: string;
  flash_rom: string;
  hdd_specs: string;
  optical_drive: string;
  debug_port: string;
  error_code: string;
  error_description: string;
  firmware_version: string;
  binaries: BinaryFileStatus[];
}

export type StatusVariant = 'neutral' | 'success' | 'warning' | 'error' | 'info';
export type BadgeVariant = 'success' | 'warning' | 'destructive' | 'secondary';

export type RuntimeLogLevel = 'off' | 'error' | 'warn' | 'info' | 'debug' | 'trace';

export interface SerialRuntimeSettings {
  readonly default_baud_rate: number;
  readonly gdb_listen_addr: string;
  readonly diagnostic_timeout_ms: number;
  readonly telemetry_poll_interval_ms: number;
}

export interface StorageRuntimeSettings {
  readonly hdd_default_capacity_gb: number;
  readonly nor_dumper_block_size: number;
  readonly hex_page_bytes: number;
  readonly forensic_keywords: readonly string[];
}

export interface IrRuntimeSettings {
  readonly default_ir_pico_gpio: number;
  readonly ir_catalog_search_limit: number;
}

export interface WorkbenchRuntimeSettings {
  readonly default_split_left_pct: number;
  readonly manuals_split_initial_pct: number;
  readonly hex_split_initial_pct: number;
  readonly log_level: RuntimeLogLevel;
}

export interface RuntimeSettings {
  readonly serial: SerialRuntimeSettings;
  readonly storage: StorageRuntimeSettings;
  readonly ir: IrRuntimeSettings;
  readonly workbench: WorkbenchRuntimeSettings;
}

export interface LauncherInstallReport {
  readonly platform: string;
  readonly desktop_entry_path: string;
  readonly icon_path?: string | null;
  readonly shortcut_path?: string | null;
  readonly created: boolean;
  readonly message: string;
}
