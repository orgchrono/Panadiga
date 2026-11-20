import React from 'react';
import { ShieldAlert, Wrench } from 'lucide-react';
import {
  SafetyConfirmGate,
  SafetyPreflightCard,
  Sha256ArtifactCard,
} from '@/components/pipelines';
import {
  Alert,
  AlertDescription,
  AlertTitle,
  Button,
  EmptyState,
  Input,
} from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import type { PicoEh55BgoRepairResponse, PreflightCheckItem } from '@/models';

export interface BgoRepairSectionProps {
  readonly portMissing: boolean;
  readonly backupMissing: boolean;
  readonly dryRunBusy: boolean;
  readonly liveBusy: boolean;
  readonly dryRunResponse: PicoEh55BgoRepairResponse | null;
  readonly liveResponse: PicoEh55BgoRepairResponse | null;
  readonly eraseBlockedSas: readonly number[];
  readonly backupDir: string;
  readonly setBackupDir: (v: string) => void;
  readonly enableBootTrace: boolean;
  readonly setEnableBootTrace: (v: boolean) => void;
  readonly preflightChecks: readonly PreflightCheckItem[];
  readonly checkedPreflightIds: readonly string[];
  readonly togglePreflight: (id: string) => void;
  readonly preflightReady: boolean;
  readonly expectedConfirmToken: string;
  readonly confirmTokenInput: string;
  readonly setConfirmTokenInput: (v: string) => void;
  readonly runDryRunPlan: () => Promise<PicoEh55BgoRepairResponse | null>;
  readonly runLiveRepair: () => Promise<PicoEh55BgoRepairResponse | null>;
}

export const BgoRepairSection: React.FC<BgoRepairSectionProps> = ({
  portMissing,
  backupMissing,
  dryRunBusy,
  liveBusy,
  dryRunResponse,
  liveResponse,
  eraseBlockedSas,
  backupDir,
  setBackupDir,
  enableBootTrace,
  setEnableBootTrace,
  preflightChecks,
  checkedPreflightIds,
  togglePreflight,
  preflightReady,
  expectedConfirmToken,
  confirmTokenInput,
  setConfirmTokenInput,
  runDryRunPlan,
  runLiveRepair,
}) => {
  const { t } = useTranslation();
  const plan = liveResponse?.plan ?? dryRunResponse?.plan ?? null;
  const liveReport = liveResponse?.liveReport;

  return (
    <div className="rounded-xl bg-slate-950 border border-slate-800 p-4 space-y-3 text-xs">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h4 className="font-bold text-slate-100">{t('pipelines.bgoBench.repairTitle')}</h4>
        <Button
          type="button"
          size="sm"
          variant="outline"
          data-testid="bgo-dry-run-btn"
          disabled={dryRunBusy}
          onClick={() => void runDryRunPlan()}
          className="gap-1.5"
        >
          <Wrench className="w-3.5 h-3.5" />
          <span>{t('pipelines.bgoBench.dryRunBtn')}</span>
        </Button>
      </div>

      {!plan ? (
        <EmptyState compact title={t('pipelines.bgoBench.opsEmpty')} />
      ) : eraseBlockedSas.length > 0 ? (
        <Alert variant="destructive">
          <ShieldAlert className="h-4 w-4" />
          <AlertDescription>
            {t('pipelines.bgoBench.eraseRequiredWarning', {
              count: eraseBlockedSas.length,
              sas: eraseBlockedSas.map(s => `SA${s}`).join(', '),
            })}
          </AlertDescription>
        </Alert>
      ) : (
        <Alert variant="success">
          <AlertDescription>{t('pipelines.bgoBench.oneToZeroOk')}</AlertDescription>
        </Alert>
      )}

      <div className="grid grid-cols-1 md:grid-cols-2 gap-3 items-end">
        <div className="space-y-1">
          <label htmlFor="bgo-backup-dir" className="text-slate-400 block">
            {t('pipelines.bgoBench.backupDirLabel')}
          </label>
          <Input
            id="bgo-backup-dir"
            type="text"
            value={backupDir}
            aria-invalid={backupMissing}
            onChange={e => setBackupDir(e.target.value)}
          />
          {backupMissing && (
            <span className="text-[11px] text-rose-400">{t('pipelines.bgoBench.backupDirRequired')}</span>
          )}
        </div>
        <label className="flex items-center gap-2 text-slate-300 pb-2 cursor-pointer">
          <input
            type="checkbox"
            checked={enableBootTrace}
            onChange={e => setEnableBootTrace(e.target.checked)}
          />
          <span>{t('pipelines.bgoBench.bootTraceToggle')}</span>
        </label>
      </div>

      <SafetyPreflightCard
        checks={preflightChecks}
        checkedIds={checkedPreflightIds}
        onToggleCheck={togglePreflight}
        severity="critical"
      />

      <SafetyConfirmGate
        expectedToken={expectedConfirmToken}
        typedValue={confirmTokenInput}
        onChange={setConfirmTokenInput}
        onConfirm={() => void runLiveRepair()}
        isBusy={liveBusy}
        preflightReady={preflightReady && !portMissing && !backupMissing && eraseBlockedSas.length === 0}
        actionLabel={t('pipelines.bgoBench.liveRepairBtn')}
        inputTestId="bgo-confirm-input"
        submitTestId="bgo-live-repair-btn"
      />

      {liveReport && liveResponse && (
        <div className="space-y-2">
          <Sha256ArtifactCard
            outputPath={liveReport.backup_path}
            outputSha256={liveReport.post_bgo_sha256}
            expectedSha256={liveResponse.plan.expected_post_sha256}
            bytesProcessed={liveReport.written_bytes}
          />
          {liveReport.boot_trace_output && (
            <Alert variant={liveReport.boot_normal_ok ? 'success' : 'warning'}>
              <AlertTitle>{t('pipelines.bgoBench.bootTraceTitle')}</AlertTitle>
              <AlertDescription className="whitespace-pre-wrap">
                {liveReport.boot_trace_output}
              </AlertDescription>
            </Alert>
          )}
        </div>
      )}
    </div>
  );
};
