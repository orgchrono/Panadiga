import React, { useState } from 'react';
import { Cpu, Loader2 } from 'lucide-react';
import type { BootSequenceReport, MatshitaBootScenario } from '@/models';
import { Button } from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import { isMatshitaBootScenario } from '@/utils/typeGuards';
import { MatshitaErrorBanner } from './MatshitaErrorBanner';
import { MATSHITA_SELECT_CLS, type MatshitaAsyncSlice } from './matshitaTypes';

const BOOT_SCENARIOS: MatshitaBootScenario[] = [
  'healthy_with_disc',
  'unlock_disc_frm',
  'tray_cannot_close_80045300',
  'laser_apc_failure_80040904',
];

interface MatshitaBootSectionProps {
  bootState: MatshitaAsyncSlice<BootSequenceReport>;
  onRunBoot: (scenario: MatshitaBootScenario) => Promise<BootSequenceReport | null>;
}

export const MatshitaBootSection: React.FC<MatshitaBootSectionProps> = ({
  bootState,
  onRunBoot,
}) => {
  const { t } = useTranslation();
  const [scenario, setScenario] = useState<MatshitaBootScenario>('unlock_disc_frm');

  return (
    <section className="space-y-3 rounded-xl border border-slate-800 bg-slate-950/60 p-4">
      <div className="flex items-center gap-2 text-xs font-mono font-semibold text-slate-200">
        <Cpu className="w-3.5 h-3.5 text-emerald-400" />
        <span>{t('unbrick.matshita.bootTitle')}</span>
      </div>
      <div className="space-y-1.5">
        <label htmlFor="matshita-scenario" className="text-[11px] font-mono text-slate-400">
          {t('unbrick.matshita.scenarioLabel')}
        </label>
        <select
          id="matshita-scenario"
          className={MATSHITA_SELECT_CLS}
          value={scenario}
          onChange={e => {
            if (isMatshitaBootScenario(e.target.value)) {
              setScenario(e.target.value);
            }
          }}
        >
          {BOOT_SCENARIOS.length === 0 ? (
            <option disabled value="">
              -
            </option>
          ) : (
            BOOT_SCENARIOS.map(sc => (
              <option key={sc} value={sc}>
                {sc}
              </option>
            ))
          )}
        </select>
      </div>
      <Button
        type="button"
        size="sm"
        variant="outline"
        disabled={bootState.loading}
        onClick={() => void onRunBoot(scenario)}
        data-testid="matshita-run-boot-btn"
      >
        {bootState.loading && <Loader2 className="w-3.5 h-3.5 mr-1.5 animate-spin" />}
        {t('unbrick.matshita.runBootBtn')}
      </Button>
      <MatshitaErrorBanner message={bootState.error} />
      {bootState.data && (
        <div className="rounded-lg border border-slate-800 bg-slate-900/70 p-2.5 text-[11px] font-mono space-y-1">
          <div className="flex flex-wrap items-center justify-between gap-2 text-cyan-300">
            <span>{bootState.data.inquiry_id.trim()}</span>
            <span>
              {bootState.data.final_boot_code_hex}
              {bootState.data.jump_target_va_hex ? ` -> ${bootState.data.jump_target_va_hex}` : ''}
            </span>
          </div>
          <div className="max-h-28 overflow-y-auto space-y-0.5 text-slate-300">
            {bootState.data.steps.map(st => (
              <div key={st.step_index}>
                [{st.passed ? 'OK' : 'FAIL'}] {st.boot_fn_va} {st.boot_fn_name}:{' '}
                {st.boot_log_line}
              </div>
            ))}
          </div>
        </div>
      )}
    </section>
  );
};
