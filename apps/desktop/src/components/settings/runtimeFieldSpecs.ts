import type { TranslationKey } from '@/i18n/translations';
import type {
  IrRuntimeSettings,
  RuntimeSettings,
  SerialRuntimeSettings,
  StorageRuntimeSettings,
  WorkbenchRuntimeSettings,
} from '@/models';
import { RUNTIME_SETTINGS_BOUNDS } from '@/constants';

export interface FieldUpdateHandlers {
  readonly onUpdateSerial: <K extends keyof SerialRuntimeSettings>(
    key: K,
    value: SerialRuntimeSettings[K],
  ) => void;
  readonly onUpdateStorage: <K extends keyof StorageRuntimeSettings>(
    key: K,
    value: StorageRuntimeSettings[K],
  ) => void;
  readonly onUpdateIr: <K extends keyof IrRuntimeSettings>(
    key: K,
    value: IrRuntimeSettings[K],
  ) => void;
  readonly onUpdateWorkbench: <K extends keyof WorkbenchRuntimeSettings>(
    key: K,
    value: WorkbenchRuntimeSettings[K],
  ) => void;
}

export interface NumberFieldSpec {
  readonly id: string;
  readonly labelKey: TranslationKey;
  readonly min: number;
  readonly max: number;
  readonly step?: number;
  readonly getValue: (settings: RuntimeSettings) => number;
  readonly applyValue: (next: number, handlers: FieldUpdateHandlers) => void;
}

export const SERIAL_NUMERIC_FIELDS: readonly NumberFieldSpec[] = [
  {
    id: 'rt-diag-timeout',
    labelKey: 'settings.runtime.diagTimeoutLabel',
    min: RUNTIME_SETTINGS_BOUNDS.diagTimeoutMinMs,
    max: RUNTIME_SETTINGS_BOUNDS.diagTimeoutMaxMs,
    getValue: (s) => s.serial.diagnostic_timeout_ms,
    applyValue: (v, h) => h.onUpdateSerial('diagnostic_timeout_ms', v),
  },
  {
    id: 'rt-poll-interval',
    labelKey: 'settings.runtime.pollIntervalLabel',
    min: RUNTIME_SETTINGS_BOUNDS.telemetryPollMinMs,
    max: RUNTIME_SETTINGS_BOUNDS.telemetryPollMaxMs,
    getValue: (s) => s.serial.telemetry_poll_interval_ms,
    applyValue: (v, h) => h.onUpdateSerial('telemetry_poll_interval_ms', v),
  },
];

export const STORAGE_NUMERIC_FIELDS: readonly NumberFieldSpec[] = [
  {
    id: 'rt-hdd-cap',
    labelKey: 'settings.runtime.hddCapacityLabel',
    min: RUNTIME_SETTINGS_BOUNDS.hddCapacityMinGb,
    max: RUNTIME_SETTINGS_BOUNDS.hddCapacityMaxGb,
    getValue: (s) => s.storage.hdd_default_capacity_gb,
    applyValue: (v, h) => h.onUpdateStorage('hdd_default_capacity_gb', v),
  },
  {
    id: 'rt-hex-page',
    labelKey: 'settings.runtime.hexPageLabel',
    min: RUNTIME_SETTINGS_BOUNDS.hexPageMinBytes,
    max: RUNTIME_SETTINGS_BOUNDS.hexPageMaxBytes,
    step: RUNTIME_SETTINGS_BOUNDS.hexPageStepBytes,
    getValue: (s) => s.storage.hex_page_bytes,
    applyValue: (v, h) => h.onUpdateStorage('hex_page_bytes', v),
  },
];

export const IR_AND_LAYOUT_FIELDS: readonly NumberFieldSpec[] = [
  {
    id: 'rt-ir-gpio',
    labelKey: 'settings.runtime.irGpioLabel',
    min: RUNTIME_SETTINGS_BOUNDS.safeIrGpioMin,
    max: RUNTIME_SETTINGS_BOUNDS.safeIrGpioMax,
    getValue: (s) => s.ir.default_ir_pico_gpio,
    applyValue: (v, h) => h.onUpdateIr('default_ir_pico_gpio', v),
  },
  {
    id: 'rt-ir-limit',
    labelKey: 'settings.runtime.irLimitLabel',
    min: RUNTIME_SETTINGS_BOUNDS.irSearchLimitMin,
    max: RUNTIME_SETTINGS_BOUNDS.irSearchLimitMax,
    getValue: (s) => s.ir.ir_catalog_search_limit,
    applyValue: (v, h) => h.onUpdateIr('ir_catalog_search_limit', v),
  },
  {
    id: 'rt-split-default',
    labelKey: 'settings.runtime.splitLeftLabel',
    min: RUNTIME_SETTINGS_BOUNDS.defaultSplitMinPct,
    max: RUNTIME_SETTINGS_BOUNDS.defaultSplitMaxPct,
    getValue: (s) => s.workbench.default_split_left_pct,
    applyValue: (v, h) => h.onUpdateWorkbench('default_split_left_pct', v),
  },
  {
    id: 'rt-split-manuals',
    labelKey: 'settings.runtime.manualsSplitLabel',
    min: RUNTIME_SETTINGS_BOUNDS.manualsSplitMinPct,
    max: RUNTIME_SETTINGS_BOUNDS.manualsSplitMaxPct,
    getValue: (s) => s.workbench.manuals_split_initial_pct,
    applyValue: (v, h) => h.onUpdateWorkbench('manuals_split_initial_pct', v),
  },
  {
    id: 'rt-split-hex',
    labelKey: 'settings.runtime.hexSplitLabel',
    min: RUNTIME_SETTINGS_BOUNDS.hexSplitMinPct,
    max: RUNTIME_SETTINGS_BOUNDS.hexSplitMaxPct,
    getValue: (s) => s.workbench.hex_split_initial_pct,
    applyValue: (v, h) => h.onUpdateWorkbench('hex_split_initial_pct', v),
  },
];
