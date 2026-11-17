import React from 'react';
import type { BreakInModeSpec, ColdBreakinMode } from '@/models';
import { Badge, Card } from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import { Sliders } from 'lucide-react';

export interface ColdBreakinSelectorProps {
  currentMode: ColdBreakinMode;
  onSelectMode: (mode: ColdBreakinMode) => void;
  breakinModes: BreakInModeSpec[];
}

/**
 * Presentation Component: Selects Cold Break-In timing profile for intercepting CPU boot vector
 * driven directly from Rust SSOT (`ssot.breakin_modes`).
 */
export const ColdBreakinSelector: React.FC<ColdBreakinSelectorProps> = ({
  currentMode,
  onSelectMode,
  breakinModes,
}) => {
  const { t } = useTranslation();
  return (
    <Card className="p-5 shadow-xl space-y-4">
      <div>
        <h3
          data-testid="pico-breakin-title"
          className="text-xs font-semibold uppercase tracking-wider text-slate-300 flex items-center gap-2"
        >
          <Sliders className="w-4 h-4 text-cyan-400" />
          <span>{t('pico.breakin.title')}</span>
        </h3>
        <p className="text-[11px] text-slate-500 font-sans mt-0.5">
          {t('pico.breakin.subtitle')}
        </p>
      </div>

      <fieldset className="space-y-2.5 border-0 p-0 m-0">
        <legend className="sr-only">{t('pico.breakin.groupAria')}</legend>
        {breakinModes.length === 0 ? (
          <div className="text-slate-500 text-xs p-3 font-mono">{t('pico.breakin.empty')}</div>
        ) : (
          breakinModes.map((m) => {
            const isSelected = currentMode === m.mode;
            return (
              <label
                key={m.mode}
                className={`block p-3.5 rounded-xl border cursor-pointer transition-all focus-within:ring-2 focus-within:ring-cyan-400 focus-within:ring-offset-2 focus-within:ring-offset-bench-900 ${
                  isSelected
                    ? 'bg-cyan-500/10 border-cyan-500/50 shadow-md shadow-cyan-950/20'
                    : 'bg-slate-950 border-slate-800 hover:border-slate-700'
                }`}
              >
                <input
                  type="radio"
                  name="cold-breakin-mode"
                  value={m.mode}
                  checked={isSelected}
                  onChange={() => onSelectMode(m.mode)}
                  className="sr-only"
                />
                <div className="flex items-center justify-between mb-1 gap-2">
                  <span className={`text-xs font-bold ${isSelected ? 'text-cyan-400' : 'text-slate-200'}`}>
                    {m.label}
                  </span>
                  {m.mode === 2 && <Badge variant="success">{m.badge}</Badge>}
                </div>
                <p className="text-[11px] text-slate-400 font-sans leading-relaxed">
                  {m.description}
                </p>
                <span className="text-[10px] text-slate-500 block mt-2 font-mono">
                  {m.signal_diagram}
                </span>
              </label>
            );
          })
        )}
      </fieldset>
    </Card>
  );
};
