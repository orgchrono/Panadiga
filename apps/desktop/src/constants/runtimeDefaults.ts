/**
 * @file constants/runtimeDefaults.ts
 * @brief SSOT bounded limits and default runtime operational settings mirroring `panadiga-core::settings`.
 */

import type { RuntimeLogLevel, RuntimeSettings } from '@/models';
import {
  GDB_DEFAULT_LISTEN_ADDR,
  IR_CATALOG_DEFAULT_SEARCH_LIMIT,
  PICO_DEFAULT_BAUD_RATE,
  PICO_DIAGNOSTIC_DEFAULT_TIMEOUT_MS,
} from './hardwareSpecs/physicalConstants';
import { NOR_DUMPER_DEFAULT_BLOCK_SIZE } from './hardwareSpecs/catalogTokens';
import { HDD_DEFAULT_CAPACITY_GB } from './hddCapacity';
import {
  DEFAULT_IR_PICO_GPIO,
  DEFAULT_SPLIT_LEFT_PCT,
  HEX_PAGE_BYTES,
  HEX_SPLIT_INITIAL_PCT,
  MANUALS_SPLIT_INITIAL_PCT,
} from './uiLayout';

export const ALLOWED_RUNTIME_BAUD_RATES: readonly number[] = Object.freeze([
  9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600,
]);

export const ALLOWED_NOR_DUMPER_BLOCK_SIZES: readonly number[] = Object.freeze([
  512, 1024, 2048, 4096, 8192,
]);

export const RUNTIME_LOG_LEVEL_OPTIONS: readonly RuntimeLogLevel[] = [
  'off',
  'error',
  'warn',
  'info',
  'debug',
  'trace',
] as const;

export const RUNTIME_SETTINGS_BOUNDS = Object.freeze({
  diagTimeoutMinMs: 500,
  diagTimeoutMaxMs: 60000,
  telemetryPollMinMs: 200,
  telemetryPollMaxMs: 30000,
  telemetryPollDefaultMs: 1000,
  hddCapacityMinGb: 80,
  hddCapacityMaxGb: 2000,
  hexPageMinBytes: 128,
  hexPageMaxBytes: 4096,
  hexPageStepBytes: 16,
  maxForensicKeywords: 32,
  safeIrGpioMin: 6,
  safeIrGpioMax: 28,
  irSearchLimitMin: 10,
  irSearchLimitMax: 500,
  defaultSplitMinPct: 24,
  defaultSplitMaxPct: 74,
  manualsSplitMinPct: 42,
  manualsSplitMaxPct: 76,
  hexSplitMinPct: 22,
  hexSplitMaxPct: 55,
});

export const DEFAULT_FORENSIC_KEYWORDS_LIST: readonly string[] = Object.freeze([
  'panasonic',
  'diga',
  'firmware',
  'boot',
  'dvd',
  'version',
  'error',
  'system',
  'copyright',
  'spansion',
  'mei',
  'matsushita',
  'dmr',
  'mn103s',
  'p7501',
  'vep79138',
  'am34',
  'meihdfs',
  'u99',
]);

export const DEFAULT_RUNTIME_SETTINGS: RuntimeSettings = Object.freeze({
  serial: Object.freeze({
    default_baud_rate: PICO_DEFAULT_BAUD_RATE,
    gdb_listen_addr: GDB_DEFAULT_LISTEN_ADDR,
    diagnostic_timeout_ms: PICO_DIAGNOSTIC_DEFAULT_TIMEOUT_MS,
    telemetry_poll_interval_ms: RUNTIME_SETTINGS_BOUNDS.telemetryPollDefaultMs,
  }),
  storage: Object.freeze({
    hdd_default_capacity_gb: HDD_DEFAULT_CAPACITY_GB,
    nor_dumper_block_size: NOR_DUMPER_DEFAULT_BLOCK_SIZE,
    hex_page_bytes: HEX_PAGE_BYTES,
    forensic_keywords: DEFAULT_FORENSIC_KEYWORDS_LIST,
  }),
  ir: Object.freeze({
    default_ir_pico_gpio: DEFAULT_IR_PICO_GPIO,
    ir_catalog_search_limit: IR_CATALOG_DEFAULT_SEARCH_LIMIT,
  }),
  workbench: Object.freeze({
    default_split_left_pct: DEFAULT_SPLIT_LEFT_PCT,
    manuals_split_initial_pct: MANUALS_SPLIT_INITIAL_PCT,
    hex_split_initial_pct: HEX_SPLIT_INITIAL_PCT,
    log_level: 'info',
  }),
});
