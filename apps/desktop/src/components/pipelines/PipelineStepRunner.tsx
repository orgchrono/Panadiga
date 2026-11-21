import React from 'react';
import { CheckCircle2, Loader2, Lock, RotateCcw, XCircle } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle, Badge, Button, EmptyState } from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import type { PipelineProgressState, PipelineStepDescriptor, PipelineStepStatus } from '@/models';
import { cn } from '@/lib/utils';

const PROGRESS_MAX_PERCENT = 100;
const BYTES_PER_KIB = 1024;

export interface PipelineStepRunnerProps {
  readonly title: string;
  readonly subtitle?: string;
  readonly steps: readonly PipelineStepDescriptor[];
  readonly activeStepId?: string | null;
  readonly isBusy: boolean;
  readonly error?: string | null;
  readonly successMessage?: string | null;
  readonly progress?: PipelineProgressState | null;
  readonly logs?: readonly string[];
  readonly onReset?: () => void;
}

const STEP_STYLE_MAP: Readonly<Record<PipelineStepStatus, string>> = {
  pending: 'bg-slate-950/70 border-slate-800 text-slate-400',
  running: 'bg-cyan-500/10 border-cyan-500/50 text-cyan-200',
  verified: 'bg-emerald-500/10 border-emerald-500/40 text-emerald-200',
  blocked: 'bg-amber-500/10 border-amber-500/40 text-amber-200',
  failed: 'bg-rose-500/10 border-rose-500/50 text-rose-200',
};

function renderStepStatusIcon(status: PipelineStepStatus): React.ReactNode {
  if (status === 'running') return <Loader2 className="w-3.5 h-3.5 animate-spin text-cyan-400 shrink-0" />;
  if (status === 'verified') return <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400 shrink-0" />;
  if (status === 'blocked') return <Lock className="w-3.5 h-3.5 text-amber-400 shrink-0" />;
  if (status === 'failed') return <XCircle className="w-3.5 h-3.5 text-rose-400 shrink-0" />;
  return <span className="w-2.5 h-2.5 rounded-full bg-slate-600 shrink-0" />;
}

interface KeyedLogLine {
  readonly key: string;
  readonly text: string;
}

function toUniqueLogEntries(lines: readonly string[]): readonly KeyedLogLine[] {
  return lines.reduce<readonly KeyedLogLine[]>((acc, text) => {
    const priorCount = acc.filter(entry => entry.text === text).length;
    return [...acc, { key: `${text}::${priorCount}`, text }];
  }, []);
}

export const PipelineStepRunner: React.FC<PipelineStepRunnerProps> = ({
  title,
  subtitle,
  steps,
  activeStepId,
  isBusy,
  error,
  successMessage,
  progress,
  logs = [],
  onReset,
}) => {
  const { t } = useTranslation();
  const pct =
    progress && progress.total > 0
      ? Math.min(
          PROGRESS_MAX_PERCENT,
          Math.max(0, Math.round((progress.current / progress.total) * PROGRESS_MAX_PERCENT)),
        )
      : 0;
  const keyedLogs = toUniqueLogEntries(logs);

  return (
    <div className="rounded-xl bg-bench-900 border border-slate-800 p-4 space-y-3 font-mono">
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-slate-800 pb-2.5">
        <div>
          <h4 className="text-xs font-bold text-slate-100">{title}</h4>
          {subtitle && <p className="text-[11px] text-slate-400 font-sans mt-0.5">{subtitle}</p>}
        </div>
        <div className="flex items-center gap-2">
          <Badge variant={error ? 'destructive' : isBusy ? 'default' : successMessage ? 'success' : 'secondary'}>
            {error
              ? t('pipelines.primitives.stepRunner.failed')
              : isBusy
                ? t('pipelines.primitives.stepRunner.running')
                : successMessage
                  ? t('pipelines.primitives.stepRunner.completed')
                  : t('pipelines.primitives.stepRunner.pending')}
          </Badge>
          {onReset && (
            <Button type="button" size="sm" variant="outline" disabled={isBusy} onClick={onReset} className="h-7 px-2 text-[11px] gap-1">
              <RotateCcw className="w-3 h-3" />
              <span>{t('pipelines.primitives.stepRunner.resetBtn')}</span>
            </Button>
          )}
        </div>
      </div>

      {steps.length === 0 ? (
        <EmptyState compact title={t('pipelines.primitives.stepRunner.emptySteps')} />
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-2">
          {steps.map(step => (
            <div
              key={step.id}
              data-step-id={step.id}
              data-step-status={step.status}
              className={cn(
                'rounded-lg border px-3 py-2 space-y-1 transition-colors',
                STEP_STYLE_MAP[step.status],
                activeStepId === step.id && 'ring-1 ring-cyan-400',
              )}
            >
              <div className="flex items-center gap-1.5 text-[11px] font-bold">
                {renderStepStatusIcon(step.status)}
                <span className="truncate">{step.label}</span>
              </div>
              {step.description && <p className="text-[10px] opacity-80 font-sans truncate">{step.description}</p>}
            </div>
          ))}
        </div>
      )}

      {progress && (
        <div className="space-y-1">
          <div className="flex flex-wrap items-center justify-between gap-2 text-[11px] text-cyan-300">
            <span className="truncate">{progress.stageLabel}</span>
            <div className="flex items-center gap-2 shrink-0">
              {typeof progress.activeSa === 'number' && (
                <Badge variant="outline" className="h-4 px-1.5 text-[10px] border-cyan-500/40 text-cyan-200">
                  SA{progress.activeSa}
                </Badge>
              )}
              {typeof progress.bytesPerSec === 'number' && progress.bytesPerSec > 0 && (
                <span className="text-[10px] text-slate-300">
                  {(progress.bytesPerSec / BYTES_PER_KIB).toFixed(1)} KiB/s
                </span>
              )}
              {typeof progress.etaSeconds === 'number' && (
                <span className="text-[10px] text-slate-400">ETA {progress.etaSeconds}s</span>
              )}
              <span>{pct}%</span>
            </div>
          </div>
          <progress
            aria-label={t('pipelines.primitives.stepRunner.progressAria')}
            value={pct}
            max={PROGRESS_MAX_PERCENT}
            className="w-full h-2 rounded-full overflow-hidden accent-cyan-400 bg-slate-950"
          />
        </div>
      )}

      {error && (
        <Alert variant="destructive">
          <AlertTitle>{t('pipelines.primitives.stepRunner.failed')}</AlertTitle>
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      {successMessage && !error && (
        <Alert variant="success">
          <AlertTitle>{t('pipelines.primitives.stepRunner.completed')}</AlertTitle>
          <AlertDescription>{successMessage}</AlertDescription>
        </Alert>
      )}

      {keyedLogs.length > 0 && (
        <div className="rounded-lg bg-slate-950 border border-slate-800 p-2.5 max-h-32 overflow-y-auto space-y-0.5 text-[10px] text-slate-300">
          <div className="text-slate-500 uppercase font-bold mb-1">
            {t('pipelines.primitives.stepRunner.logsLabel')}
          </div>
          {keyedLogs.map(entry => (
            <div key={entry.key} className="break-all">
              {entry.text}
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
