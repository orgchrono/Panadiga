import React from 'react';
import { Cpu, HardDrive, ShieldCheck, Sparkles, Unlock } from 'lucide-react';
import {
  HDD_DEFAULT_CAPACITY_GB,
  HDD_MAX_CAPACITY_GB,
  HDD_MIN_CAPACITY_GB,
  HEX32_PAD_WIDTH,
  ONE_TERABYTE_GB,
  QUICK_CAPACITIES_GB,
  SECTORS_PER_MEIHDFS_BLOCK,
} from '@/constants';
import type { TranslationKey } from '@/i18n/translations';
import { useTranslation } from '@/i18n/useTranslation';
import type { CapacityPassthroughMode, PatchMode } from '@/models';

const QUICK_CAPACITIES: readonly number[] = QUICK_CAPACITIES_GB;

const PASSTHROUGH_OPTIONS: ReadonlyArray<{
  readonly id: CapacityPassthroughMode;
  readonly labelKey: TranslationKey;
}> = [
  { id: 'unchanged', labelKey: 'hdd.capacity.strategy.optUnchanged' },
  { id: 'av_passthrough', labelKey: 'hdd.capacity.strategy.optAvPassthrough' },
  { id: 'factory_original', labelKey: 'hdd.capacity.strategy.optFactoryOriginal' },
];

export interface CapacityStrategyPanelProps {
  mode: PatchMode;
  onModeChange: (nextMode: PatchMode) => void;
  targetGb: number;
  onTargetGbChange: (gb: number) => void;
  limitInDynamicMode: boolean;
  onLimitInDynamicChange: (checked: boolean) => void;
  passthroughMode?: CapacityPassthroughMode;
  onPassthroughModeChange?: (mode: CapacityPassthroughMode) => void;
  previewBlocks: number;
  patching: boolean;
  onApply: () => void;
}

export const CapacityStrategyPanel: React.FC<CapacityStrategyPanelProps> = ({
  mode,
  onModeChange,
  targetGb,
  onTargetGbChange,
  limitInDynamicMode,
  onLimitInDynamicChange,
  passthroughMode = 'unchanged',
  onPassthroughModeChange,
  previewBlocks,
  patching,
  onApply,
}) => {
  const { t } = useTranslation();

  return (
    <div className="p-5 rounded-xl bg-slate-950 border border-slate-800 space-y-4">
      <div className="flex items-center gap-2 text-xs font-bold text-slate-200">
        <Cpu className="w-4 h-4 text-cyan-400" />
        <span>{t('hdd.capacity.strategy.heading')}</span>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-3 text-xs">
        <button
          type="button"
          onClick={() => onModeChange('dynamic')}
          className={`p-3.5 rounded-xl border text-left transition-all ${
            mode === 'dynamic'
              ? 'bg-emerald-500/10 border-emerald-500/60 text-emerald-200 shadow-lg'
              : 'bg-bench-900 border-slate-800 text-slate-400 hover:border-slate-700'
          }`}
        >
          <div className="font-bold flex items-center gap-1.5">
            <Sparkles className="w-3.5 h-3.5 text-emerald-400" />
            <span>{t('hdd.capacity.strategy.dynamicTitle')}</span>
          </div>
          <p className="text-[11px] text-slate-400 mt-1">
            {t('hdd.capacity.strategy.dynamicDescPrefix')}{' '}
            <code>bls +12</code> {t('hdd.capacity.strategy.dynamicDescWith')}{' '}
            <code>bra +12</code> (<code>0xCA</code>){' '}
            {t('hdd.capacity.strategy.dynamicDescSuffix')}
          </p>
        </button>

        <button
          type="button"
          onClick={() => onModeChange('cap')}
          className={`p-3.5 rounded-xl border text-left transition-all ${
            mode === 'cap'
              ? 'bg-cyan-500/10 border-cyan-500/60 text-cyan-200 shadow-lg'
              : 'bg-bench-900 border-slate-800 text-slate-400 hover:border-slate-700'
          }`}
        >
          <div className="font-bold flex items-center gap-1.5">
            <HardDrive className="w-3.5 h-3.5 text-cyan-400" />
            <span>{t('hdd.capacity.strategy.capTitle')}</span>
          </div>
          <p className="text-[11px] text-slate-400 mt-1">
            {t('hdd.capacity.strategy.capDescPrefix')}
            <code>0xC6</code>
            {t('hdd.capacity.strategy.capDescMid')}{' '}
            <code>MaxProductLBA</code>{' '}
            {t('hdd.capacity.strategy.capDescOfficial')}
          </p>
        </button>

        <button
          type="button"
          onClick={() => onModeChange('factory_200gb')}
          className={`p-3.5 rounded-xl border text-left transition-all ${
            mode === 'factory_200gb'
              ? 'bg-amber-500/10 border-amber-500/60 text-amber-200 shadow-lg'
              : 'bg-bench-900 border-slate-800 text-slate-400 hover:border-slate-700'
          }`}
        >
          <div className="font-bold flex items-center gap-1.5">
            <ShieldCheck className="w-3.5 h-3.5 text-amber-400" />
            <span>{t('hdd.capacity.strategy.factoryTitle')}</span>
          </div>
          <p className="text-[11px] text-slate-400 mt-1">
            {t('hdd.capacity.strategy.factoryDesc')}
          </p>
        </button>
      </div>

      {mode !== 'factory_200gb' && (
        <div className="space-y-3 pt-2 border-t border-slate-800/80">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <span className="text-[11px] text-slate-400 font-semibold uppercase">
              {mode === 'dynamic'
                ? t('hdd.capacity.strategy.dynamicOptionalPrompt')
                : t('hdd.capacity.strategy.capSelectPrompt')}
            </span>
            {mode === 'dynamic' && (
              <label className="flex items-center gap-2 text-[11px] text-cyan-300 cursor-pointer">
                <input
                  type="checkbox"
                  checked={limitInDynamicMode}
                  onChange={(e) => onLimitInDynamicChange(e.target.checked)}
                  className="rounded border-slate-700 bg-slate-900"
                />
                <span>{t('hdd.capacity.strategy.limitInDynamicLabel')}</span>
              </label>
            )}
          </div>

          {(mode === 'cap' || limitInDynamicMode) && (
            <div className="flex flex-wrap items-center gap-2">
              {QUICK_CAPACITIES.length === 0 ? (
                <span className="text-[11px] text-slate-500">
                  {t('hdd.capacity.strategy.emptyPresets')}
                </span>
              ) : (
                QUICK_CAPACITIES.map((gb) => (
                  <button
                    key={gb}
                    type="button"
                    onClick={() => onTargetGbChange(gb)}
                    className={`px-3 py-1.5 rounded-lg text-xs font-bold border transition-colors ${
                      targetGb === gb
                        ? 'bg-cyan-500 text-slate-950 border-cyan-400'
                        : 'bg-bench-900 text-slate-300 border-slate-800 hover:border-slate-700'
                    }`}
                  >
                    {gb >= ONE_TERABYTE_GB
                      ? t('hdd.capacity.strategy.tbLabel', { tb: gb / ONE_TERABYTE_GB, gb })
                      : t('hdd.capacity.strategy.gbLabel', { gb })}
                    {gb === HDD_DEFAULT_CAPACITY_GB ? ' ★' : ''}
                  </button>
                ))
              )}

              <div className="flex items-center gap-1.5 ml-auto">
                <label htmlFor="cap-custom-gb" className="text-[11px] text-slate-400">
                  {t('hdd.capacity.strategy.customGbLabel')}
                </label>
                <input
                  id="cap-custom-gb"
                  type="number"
                  min={HDD_MIN_CAPACITY_GB}
                  max={HDD_MAX_CAPACITY_GB}
                  value={targetGb}
                  onChange={(e) =>
                    onTargetGbChange(
                      Math.max(
                        HDD_MIN_CAPACITY_GB,
                        Math.min(
                          HDD_MAX_CAPACITY_GB,
                          Number(e.target.value) || HDD_DEFAULT_CAPACITY_GB,
                        ),
                      ),
                    )
                  }
                  className="w-24 px-2.5 py-1.5 rounded-lg bg-slate-900 border border-slate-700 text-xs text-cyan-300 focus:outline-none focus:border-cyan-400"
                />
              </div>
            </div>
          )}
        </div>
      )}

      {onPassthroughModeChange && (
        <div className="space-y-2 pt-2 border-t border-slate-800/80 text-xs">
          <span className="text-[11px] text-slate-400 font-semibold uppercase block">
            {t('hdd.capacity.strategy.passthroughHeading')}
          </span>
          <div className="flex flex-wrap gap-2">
            {PASSTHROUGH_OPTIONS.map((opt) => (
              <button
                key={opt.id}
                type="button"
                onClick={() => onPassthroughModeChange(opt.id)}
                className={`px-3 py-1.5 rounded-lg text-[11px] font-bold border transition-colors ${
                  passthroughMode === opt.id
                    ? 'bg-emerald-500/20 text-emerald-200 border-emerald-500/60'
                    : 'bg-bench-900 text-slate-400 border-slate-800 hover:border-slate-700'
                }`}
              >
                {t(opt.labelKey)}
              </button>
            ))}
          </div>
        </div>
      )}

      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 pt-3 border-t border-slate-800">
        <div className="text-[11px] text-slate-300 space-y-0.5">
          <div>
            {t('hdd.capacity.strategy.calcCapLabel')}{' '}
            <code className="text-emerald-300 font-bold">
              0x{previewBlocks.toString(16).toUpperCase().padStart(HEX32_PAD_WIDTH, '0')}
            </code>{' '}
            {t('hdd.capacity.strategy.blocksUnit', { blocks: previewBlocks.toLocaleString() })}
          </div>
          <div className="text-slate-400">
            {t('hdd.capacity.strategy.lba48SectorsLabel')}{' '}
            <span className="text-cyan-300">
              {(previewBlocks * SECTORS_PER_MEIHDFS_BLOCK).toLocaleString()}
            </span>{' '}
            {t('hdd.capacity.strategy.lba48SummarySuffix')}
          </div>
        </div>

        <button
          type="button"
          onClick={onApply}
          disabled={patching}
          className="px-5 py-2.5 rounded-xl bg-emerald-500 hover:bg-emerald-400 text-slate-950 font-bold text-xs flex items-center gap-2 transition-all disabled:opacity-50 shrink-0"
        >
          <Unlock className="w-4 h-4" />
          <span>
            {patching ? t('hdd.capacity.strategy.patchingBtn') : t('hdd.capacity.strategy.applyBtn')}
          </span>
        </button>
      </div>
    </div>
  );
};
