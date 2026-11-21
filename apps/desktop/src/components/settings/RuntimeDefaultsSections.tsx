import React from 'react';
import { Input, Select } from '@/components/ui';
import {
  ALLOWED_NOR_DUMPER_BLOCK_SIZES,
  ALLOWED_RUNTIME_BAUD_RATES,
  RUNTIME_LOG_LEVEL_OPTIONS,
} from '@/constants';
import { useTranslation } from '@/i18n/useTranslation';
import type { RuntimeLogLevel, RuntimeSettings } from '@/models';
import { oneOfGuard } from '@/utils/typeGuards';
import {
  IR_AND_LAYOUT_FIELDS,
  SERIAL_NUMERIC_FIELDS,
  STORAGE_NUMERIC_FIELDS,
  type FieldUpdateHandlers,
  type NumberFieldSpec,
} from './runtimeFieldSpecs';

export type { FieldUpdateHandlers };

const isRuntimeLogLevel = oneOfGuard(RUNTIME_LOG_LEVEL_OPTIONS);

const RuntimeNumberField: React.FC<{
  readonly spec: NumberFieldSpec;
  readonly draft: RuntimeSettings;
  readonly handlers: FieldUpdateHandlers;
}> = ({ spec, draft, handlers }) => {
  const { t } = useTranslation();
  return (
    <div>
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
};

export const RuntimeSerialSection: React.FC<{
  readonly draft: RuntimeSettings;
  readonly handlers: FieldUpdateHandlers;
}> = ({ draft, handlers }) => {
  const { t } = useTranslation();
  return (
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
                handlers.onUpdateSerial('default_baud_rate', next);
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
            onChange={(e) => handlers.onUpdateSerial('gdb_listen_addr', e.target.value)}
          />
        </div>
        {SERIAL_NUMERIC_FIELDS.length === 0
          ? null
          : SERIAL_NUMERIC_FIELDS.map((spec) => (
              <RuntimeNumberField key={spec.id} spec={spec} draft={draft} handlers={handlers} />
            ))}
      </div>
    </div>
  );
};

export const RuntimeStorageSection: React.FC<{
  readonly draft: RuntimeSettings;
  readonly keywordsText: string;
  readonly onChangeKeywordsText: (val: string) => void;
  readonly handlers: FieldUpdateHandlers;
}> = ({ draft, keywordsText, onChangeKeywordsText, handlers }) => {
  const { t } = useTranslation();
  return (
    <div className="space-y-2 pt-3 border-t border-slate-800">
      <div className="text-[11px] font-bold uppercase text-emerald-300">
        {t('settings.runtime.sectionStorage')}
      </div>
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
        {STORAGE_NUMERIC_FIELDS.length === 0
          ? null
          : STORAGE_NUMERIC_FIELDS.map((spec) => (
              <RuntimeNumberField key={spec.id} spec={spec} draft={draft} handlers={handlers} />
            ))}
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
                handlers.onUpdateStorage('nor_dumper_block_size', next);
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
  );
};

export const RuntimeIrLayoutSection: React.FC<{
  readonly draft: RuntimeSettings;
  readonly handlers: FieldUpdateHandlers;
}> = ({ draft, handlers }) => {
  const { t } = useTranslation();
  return (
    <div className="space-y-2 pt-3 border-t border-slate-800">
      <div className="text-[11px] font-bold uppercase text-amber-300">
        {t('settings.runtime.sectionIrLayout')}
      </div>
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
        {IR_AND_LAYOUT_FIELDS.length === 0
          ? null
          : IR_AND_LAYOUT_FIELDS.map((spec) => (
              <RuntimeNumberField key={spec.id} spec={spec} draft={draft} handlers={handlers} />
            ))}
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
                handlers.onUpdateWorkbench('log_level', lvl);
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
  );
};
