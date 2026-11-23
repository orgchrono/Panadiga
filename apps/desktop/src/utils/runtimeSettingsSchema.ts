/**
 * @file utils/runtimeSettingsSchema.ts
 * @brief Zod runtime validation schema & safe normalizer for canonical RuntimeSettings.
 */

import { z } from 'zod';
import {
  ALLOWED_NOR_DUMPER_BLOCK_SIZES,
  ALLOWED_RUNTIME_BAUD_RATES,
  DEFAULT_RUNTIME_SETTINGS,
  RUNTIME_LOG_LEVEL_OPTIONS,
  RUNTIME_SETTINGS_BOUNDS,
  SAFE_PICO_IR_GPIOS,
} from '@/constants';
import type { RuntimeSettings } from '@/models';

const gdbSocketPattern = /^[^\s:]+:\d{1,5}$/;

export const runtimeSettingsSchema: z.ZodType<RuntimeSettings> = z.object({
  serial: z.object({
    default_baud_rate: z
      .number()
      .int()
      .refine((v) => ALLOWED_RUNTIME_BAUD_RATES.includes(v)),
    gdb_listen_addr: z.string().trim().regex(gdbSocketPattern),
    diagnostic_timeout_ms: z
      .number()
      .int()
      .min(RUNTIME_SETTINGS_BOUNDS.diagTimeoutMinMs)
      .max(RUNTIME_SETTINGS_BOUNDS.diagTimeoutMaxMs),
    telemetry_poll_interval_ms: z
      .number()
      .int()
      .min(RUNTIME_SETTINGS_BOUNDS.telemetryPollMinMs)
      .max(RUNTIME_SETTINGS_BOUNDS.telemetryPollMaxMs),
  }),
  storage: z.object({
    hdd_default_capacity_gb: z
      .number()
      .int()
      .min(RUNTIME_SETTINGS_BOUNDS.hddCapacityMinGb)
      .max(RUNTIME_SETTINGS_BOUNDS.hddCapacityMaxGb),
    nor_dumper_block_size: z
      .number()
      .int()
      .refine((v) => ALLOWED_NOR_DUMPER_BLOCK_SIZES.includes(v)),
    hex_page_bytes: z
      .number()
      .int()
      .min(RUNTIME_SETTINGS_BOUNDS.hexPageMinBytes)
      .max(RUNTIME_SETTINGS_BOUNDS.hexPageMaxBytes)
      .refine((v) => v % RUNTIME_SETTINGS_BOUNDS.hexPageStepBytes === 0),
    forensic_keywords: z
      .array(z.string().trim().min(1))
      .min(1)
      .max(RUNTIME_SETTINGS_BOUNDS.maxForensicKeywords),
  }),
  ir: z.object({
    default_ir_pico_gpio: z
      .number()
      .int()
      .min(RUNTIME_SETTINGS_BOUNDS.safeIrGpioMin)
      .max(RUNTIME_SETTINGS_BOUNDS.safeIrGpioMax)
      .refine((pin) => SAFE_PICO_IR_GPIOS.includes(pin), {
        message: 'Pico IR TX GPIO must be in the safe GP6..GP9 or GP16..GP22 set',
      }),
    ir_catalog_search_limit: z
      .number()
      .int()
      .min(RUNTIME_SETTINGS_BOUNDS.irSearchLimitMin)
      .max(RUNTIME_SETTINGS_BOUNDS.irSearchLimitMax),
  }),
  workbench: z.object({
    default_split_left_pct: z
      .number()
      .int()
      .min(RUNTIME_SETTINGS_BOUNDS.defaultSplitMinPct)
      .max(RUNTIME_SETTINGS_BOUNDS.defaultSplitMaxPct),
    manuals_split_initial_pct: z
      .number()
      .int()
      .min(RUNTIME_SETTINGS_BOUNDS.manualsSplitMinPct)
      .max(RUNTIME_SETTINGS_BOUNDS.manualsSplitMaxPct),
    hex_split_initial_pct: z
      .number()
      .int()
      .min(RUNTIME_SETTINGS_BOUNDS.hexSplitMinPct)
      .max(RUNTIME_SETTINGS_BOUNDS.hexSplitMaxPct),
    log_level: z.enum(RUNTIME_LOG_LEVEL_OPTIONS),
  }),
});

/** Validates unknown payload via Zod and returns canonical defaults on mismatch. */
export function parseRuntimeSettings(raw: unknown): RuntimeSettings {
  const result = runtimeSettingsSchema.safeParse(raw);
  return result.success ? result.data : DEFAULT_RUNTIME_SETTINGS;
}
