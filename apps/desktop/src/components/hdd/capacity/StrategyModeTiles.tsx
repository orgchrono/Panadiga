import React from 'react';
import { HardDrive, ShieldCheck, Sparkles } from 'lucide-react';
import { useTranslation } from '@/i18n/useTranslation';
import type { PatchMode } from '@/models';

interface StrategyModeTilesProps {
  readonly mode: PatchMode;
  readonly onModeChange: (nextMode: PatchMode) => void;
}

export const StrategyModeTiles: React.FC<StrategyModeTilesProps> = ({ mode, onModeChange }) => {
  const { t } = useTranslation();

  return (
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
  );
};
