import React from 'react';
import { Download, Layers } from 'lucide-react';
import { PipelineCardHeader, PipelineStepRunner, Sha256ArtifactCard } from '@/components/pipelines';
import { useTranslation } from '@/i18n/useTranslation';
import {
  Button,
  Card,
  CardContent,
  Input,
  Select,
} from '@/components/ui';
import type {
  BgoAuditSourceMode,
  Eh55BgoAuditReport,
  NorSliceKind,
  PicoEh55BgoRepairResponse,
  PicoEh55NorDumpResponse,
  PipelinePreflightAndGateProps,
  PipelineProgressState,
  PipelineStepDescriptor,
} from '@/models';
import { isNorSliceKind } from '@/utils/typeGuards';
import { BgoL2pAuditSection, type BgoSectorMapStateProps } from './BgoL2pAuditSection';
import { BgoRepairSection } from './BgoRepairSection';

export interface BgoBenchVm extends PipelinePreflightAndGateProps, BgoSectorMapStateProps {
  readonly portName: string;
  readonly setPortName: (v: string) => void;
  readonly slice: NorSliceKind;
  readonly setSlice: (s: NorSliceKind) => void;
  readonly dumpOutPath: string;
  readonly setDumpOutPath: (v: string) => void;
  readonly auditSourceMode: BgoAuditSourceMode;
  readonly setAuditSourceMode: (m: BgoAuditSourceMode) => void;
  readonly norPath: string;
  readonly setNorPath: (v: string) => void;
  readonly backupDir: string;
  readonly setBackupDir: (v: string) => void;
  readonly enableBootTrace: boolean;
  readonly setEnableBootTrace: (v: boolean) => void;
  readonly dumpReport: PicoEh55NorDumpResponse | null;
  readonly dumpBusy: boolean;
  readonly dumpError: string | null;
  readonly dumpProgress: PipelineProgressState | null;
  readonly dumpLogs: readonly string[];
  readonly runNorDump: () => Promise<PicoEh55NorDumpResponse | null>;
  readonly auditReport: Eh55BgoAuditReport | null;
  readonly auditBusy: boolean;
  readonly auditError: string | null;
  readonly runBgoAudit: () => Promise<Eh55BgoAuditReport | null>;
  readonly dryRunResponse: PicoEh55BgoRepairResponse | null;
  readonly dryRunBusy: boolean;
  readonly dryRunError: string | null;
  readonly runDryRunPlan: () => Promise<PicoEh55BgoRepairResponse | null>;
  readonly liveResponse: PicoEh55BgoRepairResponse | null;
  readonly liveBusy: boolean;
  readonly liveError: string | null;
  readonly liveSuccess: string | null;
  readonly liveProgress: PipelineProgressState | null;
  readonly liveLogs: readonly string[];
  readonly runLiveRepair: () => Promise<PicoEh55BgoRepairResponse | null>;
  readonly eraseBlockedSas: readonly number[];
  readonly steps: readonly PipelineStepDescriptor[];
  readonly resetAll: () => void;
}

export const BgoFtlBenchPanel: React.FC<{ readonly vm: BgoBenchVm }> = ({ vm }) => {
  const { t } = useTranslation();
  const portMissing = !vm.portName.trim();
  const dumpOutMissing = !vm.dumpOutPath.trim();
  const backupMissing = !vm.backupDir.trim();
  const activeError = vm.liveError ?? vm.dryRunError ?? vm.auditError ?? vm.dumpError;
  const activeLogs = vm.liveLogs.length > 0 ? vm.liveLogs : vm.dumpLogs;

  return (
    <Card className="bg-bench-900 border-slate-800 space-y-4">
      <PipelineCardHeader
        icon={<Layers className="w-4 h-4 text-indigo-400" />}
        title={t('pipelines.bgoBench.title')}
        subtitle={t('pipelines.bgoBench.subtitle')}
        badge={t('pipelines.bgoBench.sa0Sa3LockedBadge')}
        badgeVariant="warning"
      />

      <CardContent className="space-y-4 font-mono">
        <PipelineStepRunner
          title={t('pipelines.bgoBench.title')}
          steps={vm.steps}
          isBusy={vm.dumpBusy || vm.auditBusy || vm.dryRunBusy || vm.liveBusy}
          error={activeError}
          successMessage={vm.liveSuccess}
          progress={vm.liveProgress ?? vm.dumpProgress}
          logs={activeLogs}
          onReset={vm.resetAll}
        />

        <div className="rounded-xl bg-slate-950 border border-slate-800 p-4 space-y-3 text-xs">
          <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
            <div className="space-y-1">
              <label htmlFor="bgo-port-input" className="text-slate-400 block">
                {t('pipelines.bgoBench.portLabel')}
              </label>
              <Input
                id="bgo-port-input"
                data-testid="bgo-port-input"
                type="text"
                value={vm.portName}
                aria-invalid={portMissing}
                onChange={e => vm.setPortName(e.target.value)}
              />
              {portMissing && (
                <span className="text-[11px] text-rose-400">{t('pipelines.bgoBench.portRequired')}</span>
              )}
            </div>
            <div className="space-y-1">
              <label htmlFor="bgo-slice-select" className="text-slate-400 block">
                {t('pipelines.bgoBench.sliceLabel')}
              </label>
              <Select
                id="bgo-slice-select"
                data-testid="bgo-slice-select"
                value={vm.slice}
                onChange={e => {
                  if (isNorSliceKind(e.target.value)) vm.setSlice(e.target.value);
                }}
              >
                <option value="bgo">{t('pipelines.bgoBench.sliceBgo')}</option>
                <option value="full">{t('pipelines.bgoBench.sliceFull')}</option>
                <option value="nvram">{t('pipelines.bgoBench.sliceNvram')}</option>
                <option value="loader">{t('pipelines.bgoBench.sliceLoader')}</option>
              </Select>
            </div>
            <div className="space-y-1">
              <label htmlFor="bgo-dump-out" className="text-slate-400 block">
                {t('pipelines.bgoBench.dumpOutPath')}
              </label>
              <Input
                id="bgo-dump-out"
                data-testid="bgo-dump-out"
                type="text"
                value={vm.dumpOutPath}
                aria-invalid={dumpOutMissing}
                onChange={e => vm.setDumpOutPath(e.target.value)}
              />
              {dumpOutMissing && (
                <span className="text-[11px] text-rose-400">{t('pipelines.bgoBench.dumpOutRequired')}</span>
              )}
            </div>
          </div>

          <Button
            type="button"
            size="sm"
            data-testid="bgo-nor-dump-btn"
            disabled={vm.dumpBusy || portMissing || dumpOutMissing}
            onClick={() => void vm.runNorDump()}
            className="gap-1.5"
          >
            <Download className="w-3.5 h-3.5" />
            <span>{t('pipelines.bgoBench.dumpBtn')}</span>
          </Button>

          {vm.dumpReport && (
            <Sha256ArtifactCard
              outputPath={vm.dumpReport.outputPath}
              outputSha256={vm.dumpReport.sha256}
              bytesProcessed={vm.dumpReport.totalLen}
              verified
            />
          )}
        </div>

        <BgoL2pAuditSection {...vm} />

        <BgoRepairSection
          portMissing={portMissing}
          backupMissing={backupMissing}
          dryRunBusy={vm.dryRunBusy}
          liveBusy={vm.liveBusy}
          dryRunResponse={vm.dryRunResponse}
          liveResponse={vm.liveResponse}
          eraseBlockedSas={vm.eraseBlockedSas}
          backupDir={vm.backupDir}
          setBackupDir={vm.setBackupDir}
          enableBootTrace={vm.enableBootTrace}
          setEnableBootTrace={vm.setEnableBootTrace}
          preflightChecks={vm.preflightChecks}
          checkedPreflightIds={vm.checkedPreflightIds}
          togglePreflight={vm.togglePreflight}
          preflightReady={vm.preflightReady}
          expectedConfirmToken={vm.expectedConfirmToken}
          confirmTokenInput={vm.confirmTokenInput}
          setConfirmTokenInput={vm.setConfirmTokenInput}
          runDryRunPlan={vm.runDryRunPlan}
          runLiveRepair={vm.runLiveRepair}
        />
      </CardContent>
    </Card>
  );
};
