import React from 'react';
import { CheckCircle2, Lock, ShieldCheck } from 'lucide-react';
import { Badge, EmptyState } from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import type { PreWriteGateReport, StrictWorkflowStep, VaultSessionManifest } from '@/models';

const ORDERED_STEPS: readonly StrictWorkflowStep[] = [
  'select_model',
  'build_cleanroom_sd',
  'setup_pico_probe',
  'capture_and_backup',
  'organize_and_recompile',
  'emulate_and_flash',
];

export interface StrictWorkflowStepperProps {
  readonly activeSession: VaultSessionManifest | null;
  readonly emuGateReport: PreWriteGateReport | null;
  readonly onJumpToVault: () => void;
}

export const StrictWorkflowStepper: React.FC<StrictWorkflowStepperProps> = ({
  activeSession,
  emuGateReport,
  onJumpToVault,
}) => {
  const { t } = useTranslation();
  const completed = activeSession?.completedSteps ?? [];
  const current = activeSession?.currentStep ?? 'select_model';
  const gatePassed = emuGateReport?.passed ?? activeSession?.lastEmuGate?.passed ?? false;

  return (
    <section
      data-testid="strict-workflow-stepper"
      className="rounded-2xl border border-slate-800 bg-bench-900/85 p-3.5 space-y-2.5 shadow-md"
    >
      <div className="flex flex-wrap items-center justify-between gap-2">
        <span className="text-xs font-mono font-bold text-cyan-300">
          {t('express.stepper.heading')}
        </span>
        <div className="flex flex-wrap items-center gap-2">
          <Badge variant="secondary" className="font-mono text-[10px]">
            {activeSession ? activeSession.sessionId : t('express.stepper.noSessionYet')}
          </Badge>
          <Badge
            data-testid="strict-gate-badge"
            variant={gatePassed ? 'success' : 'warning'}
            className="gap-1 font-mono text-[10px]"
          >
            {gatePassed ? <ShieldCheck className="w-3 h-3" /> : <Lock className="w-3 h-3" />}
            <span>{gatePassed ? t('express.stepper.gatePass') : t('express.stepper.gateLocked')}</span>
          </Badge>
        </div>
      </div>

      <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-2">
        {ORDERED_STEPS.length === 0 ? (
          <EmptyState compact title={t('express.stepper.noSessionYet')} />
        ) : (
          ORDERED_STEPS.map((step, idx) => {
            const isDone = completed.includes(step);
            const isCurrent = current === step && !isDone;
            const borderCls = isDone
              ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-200'
              : isCurrent
                ? 'border-cyan-500/50 bg-cyan-500/10 text-cyan-200'
                : 'border-slate-800 bg-slate-950/70 text-slate-400';
            return (
              <button
                key={step}
                type="button"
                onClick={onJumpToVault}
                data-testid={`strict-step-${idx + 1}`}
                className={`rounded-lg border px-2.5 py-2 text-left transition-colors hover:border-cyan-400/50 ${borderCls}`}
              >
                <div className="flex items-center justify-between text-[10px] font-mono">
                  <span>0{idx + 1}</span>
                  {isDone && <CheckCircle2 className="w-3 h-3 text-emerald-400" />}
                </div>
                <div className="mt-0.5 text-[11px] font-medium truncate">
                  {t(`express.stepper.steps.${step}`)}
                </div>
              </button>
            );
          })
        )}
      </div>
    </section>
  );
};
