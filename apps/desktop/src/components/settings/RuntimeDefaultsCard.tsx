import React from 'react';
import { RotateCcw, Save, ShieldAlert, Sliders } from 'lucide-react';
import {
  Alert,
  AlertDescription,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  Input,
  Select,
} from '@/components/ui';
import {
  ALLOWED_NOR_DUMPER_BLOCK_SIZES,
  ALLOWED_RUNTIME_BAUD_RATES,
  RUNTIME_LOG_LEVEL_OPTIONS,
  RUNTIME_SETTINGS_BOUNDS,
} from '@/constants';
import type { TranslationKey } from '@/i18n/translations';
import { useTranslation } from '@/i18n/useTranslation';
import type {
  IrRuntimeSettings,
  RuntimeLogLevel,
  RuntimeSettings,
  SerialRuntimeSettings,
  StorageRuntimeSettings,
  WorkbenchRuntimeSettings,
} from '@/models';
import { oneOfGuard } from '@/utils/typeGuards';

const isRuntimeLogLevel = oneOfGuard(RUNTIME_LOG_LEVEL_OPTIONS);

interface NumberFieldSpec {
  readonly id: string;
  readonly labelKey: TranslationKey;
  readonly min: number;
  readonly max: number;
  readonly step?: number;
  readonly getValue: (settings: RuntimeSettings) => number;
  readonly applyValue: (
    next: number,
    handlers: FieldUpdateHandlers,
  ) => void;
}

interface FieldUpdateHandlers {
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

const SERIAL_NUMERIC_FIELDS: readonly NumberFieldSpec[] = [
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

const STORAGE_NUMERIC_FIELDS: readonly NumberFieldSpec[] = [
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

const IR_AND_LAYOUT_FIELDS: readonly NumberFieldSpec[] = [
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

export interface RuntimeDefaultsCardProps extends FieldUpdateHandlers {
  readonly draft: RuntimeSettings;
  readonly keywordsText: string;
  readonly onChangeKeywordsText: (val: string) => void;
  readonly isLoading: boolean;
  readonly saving: boolean;
  readonly error: string | null;
  readonly successMessage: string | null;
  readonly isDirty: boolean;
  readonly onPersistDefaults: () => void;
  readonly onResetDefaults: () => void;
}

export const RuntimeDefaultsCard: React.FC<RuntimeDefaultsCardProps> = ({
  draft,
  keywordsText,
  onChangeKeywordsText,
  isLoading,
  saving,
  error,
  successMessage,
  isDirty,
  onUpdateSerial,
  onUpdateStorage,
  onUpdateIr,
  onUpdateWorkbench,
  onPersistDefaults,
  onResetDefaults,
}) => {
  const { t } = useTranslation();
  const handlers: FieldUpdateHandlers = {
    onUpdateSerial,
    onUpdateStorage,
    onUpdateIr,
    onUpdateWorkbench,
  };

  const renderNumberField = (spec: NumberFieldSpec) => (
    <div key={spec.id}>
      <label htmlFor={spec.id} className="text-[11px] text-slate-400 block mb-1">
        {t(spec.labelKey)}
      </label>
      <Input
        id={spec.id}
        type="number"
        min={spec.min}
        max={spec.max}
        step={spec.step ?? 1}
        value={spec.getValue(draft)}
        onChange={(e) => {
          const parsed = Number.parseInt(e.target.value, 10);
          if (Number.isFinite(parsed)) {
            spec.applyValue(parsed, handlers);
          }
        }}
      />
    </div>
  );

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Sliders className="w-4 h-4 text-cyan-400" />
          <span>{t('settings.runtime.cardTitle')}</span>
        </CardTitle>
        <CardDescription>{t('settings.runtime.cardDesc')}</CardDescription>
      </CardHeader>

      <CardContent className="space-y-5 text-xs">
        <Alert variant="warning">
          <ShieldAlert className="w-4 h-4" />
          <AlertDescription>{t('settings.runtime.lockedShieldBanner')}</AlertDescription>
        </Alert>

        {error && (
          <Alert variant="destructive">
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}

        {isLoading ? (
          <div className="space-y-2" aria-busy="true">
            <div className="text-slate-400">{t('settings.runtime.loadingLabel')}</div>
            <div className="h-24 w-full rounded-lg bg-slate-800/60 animate-pulse" />
          </div>
        ) : (
          <div className="space-y-5">
            <div className="space-y-2">
              <div className="text-[11px] font-bold uppercase text-cyan-300">
                {t('settings.runtime.sectionSerial')}
              </div>
              <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3">
                <div>
                  <label htmlFor="rt-baud" className="text-[11px] text-slate-400 block mb-1">
                    {t('settings.runtime.baudLabel')}
                  </label>
                  <Select
                    id="rt-baud"
                    value={String(draft.serial.default_baud_rate)}
                    onChange={(e) => {
                      const next = Number.parseInt(e.target.value, 10);
                      if (Number.isFinite(next)) {
                        onUpdateSerial('default_baud_rate', next);
                      }
                    }}
                  >
                    {ALLOWED_RUNTIME_BAUD_RATES.length === 0 ? null : ALLOWED_RUNTIME_BAUD_RATES.map((baud) => (
                      <option key={baud} value={String(baud)}>
                        {baud}
                      </option>
                    ))}
                  </Select>
                </div>

                <div>
                  <label htmlFor="rt-gdb-addr" className="text-[11px] text-slate-400 block mb-1">
                    {t('settings.runtime.gdbAddrLabel')}
                  </label>
                  <Input
                    id="rt-gdb-addr"
                    value={draft.serial.gdb_listen_addr}
                    onChange={(e) => onUpdateSerial('gdb_listen_addr', e.target.value)}
                  />
                </div>

                {SERIAL_NUMERIC_FIELDS.length === 0 ? null : SERIAL_NUMERIC_FIELDS.map(renderNumberField)}
              </div>
            </div>

            <div className="space-y-2 pt-3 border-t border-slate-800">
              <div className="text-[11px] font-bold uppercase text-emerald-300">
                {t('settings.runtime.sectionStorage')}
              </div>
              <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
                {STORAGE_NUMERIC_FIELDS.length === 0 ? null : STORAGE_NUMERIC_FIELDS.map(renderNumberField)}
                <div>
                  <label htmlFor="rt-nor-blk" className="text-[11px] text-slate-400 block mb-1">
                    {t('settings.runtime.norBlockLabel')}
                  </label>
                  <Select
                    id="rt-nor-blk"
                    value={String(draft.storage.nor_dumper_block_size)}
                    onChange={(e) => {
                      const next = Number.parseInt(e.target.value, 10);
                      if (Number.isFinite(next)) {
                        onUpdateStorage('nor_dumper_block_size', next);
                      }
                    }}
                  >
                    {ALLOWED_NOR_DUMPER_BLOCK_SIZES.length === 0 ? null : ALLOWED_NOR_DUMPER_BLOCK_SIZES.map((blk) => (
                      <option key={blk} value={String(blk)}>
                        {blk}
                      </option>
                    ))}
                  </Select>
                </div>
              </div>
              <div>
                <label htmlFor="rt-keywords" className="text-[11px] text-slate-400 block mb-1">
                  {t('settings.runtime.forensicKeywordsLabel')}
                </label>
                <Input
                  id="rt-keywords"
                  value={keywordsText}
                  onChange={(e) => onChangeKeywordsText(e.target.value)}
                />
              </div>
            </div>

            <div className="space-y-2 pt-3 border-t border-slate-800">
              <div className="text-[11px] font-bold uppercase text-amber-300">
                {t('settings.runtime.sectionIrLayout')}
              </div>
              <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
                {IR_AND_LAYOUT_FIELDS.length === 0 ? null : IR_AND_LAYOUT_FIELDS.map(renderNumberField)}
                <div>
                  <label htmlFor="rt-log-level" className="text-[11px] text-slate-400 block mb-1">
                    {t('settings.runtime.logLevelLabel')}
                  </label>
                  <Select
                    id="rt-log-level"
                    value={draft.workbench.log_level}
                    onChange={(e) => {
                      if (isRuntimeLogLevel(e.target.value)) {
                        const lvl: RuntimeLogLevel = e.target.value;
                        onUpdateWorkbench('log_level', lvl);
                      }
                    }}
                  >
                    {RUNTIME_LOG_LEVEL_OPTIONS.length === 0 ? null : RUNTIME_LOG_LEVEL_OPTIONS.map((lvl) => (
                      <option key={lvl} value={lvl}>
                        {lvl.toUpperCase()}
                      </option>
                    ))}
                  </Select>
                </div>
              </div>
            </div>

            <div className="flex flex-wrap items-center gap-2 pt-3 border-t border-slate-800">
              <Button
                type="button"
                size="sm"
                disabled={saving || !isDirty}
                onClick={onPersistDefaults}
                data-testid="runtime-settings-save-btn"
                className="gap-1.5"
              >
                <Save className="w-3.5 h-3.5" />
                <span>{t('settings.runtime.saveBtn')}</span>
              </Button>
              <Button
                type="button"
                size="sm"
                variant="outline"
                disabled={saving}
                onClick={onResetDefaults}
                data-testid="runtime-settings-reset-btn"
                className="gap-1.5"
              >
                <RotateCcw className="w-3.5 h-3.5" />
                <span>{t('settings.runtime.resetBtn')}</span>
              </Button>
            </div>
            {successMessage && (
              <output className="block">
                <Alert variant="success">
                  <AlertDescription>{successMessage}</AlertDescription>
                </Alert>
              </output>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  );
};
