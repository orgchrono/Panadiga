import { useCallback, useMemo, useState } from 'react';
import {
  DEFAULT_BGO_BOOTTRACE_MS,
  SPANSION_BGO_FIRST_SA,
  UPPER_NOR_CONFIRM_TOKEN,
} from '@/constants';
import { useToast } from '@/context/ToastContext';
import { translate } from '@/i18n/useTranslation';
import type {
  BgoAuditSourceMode,
  Eh55BgoAuditReport,
  FlashSectorInfo,
  NorSliceKind,
  PicoEh55BgoRepairResponse,
  PicoEh55NorDumpResponse,
  PipelineStepDescriptor,
  PreflightCheckItem,
  StreamProgressTelemetryFields,
} from '@/models';
import {
  picoEh55BgoAudit,
  picoEh55BgoRepair,
  picoEh55NorDump,
  subscribeBgoRepairProgress,
  subscribeNorDumpProgress,
} from '@/services/pipelineService';
import { buildBgo142SectorMap, computeF2Integrity } from './benchHelpers';
import { usePreflightSelection } from './usePipelineHelpers';
import { usePipelineRunner } from './usePipelineRunner';

export interface CowBackupSession {
  readonly path: string;
  readonly sha256: string;
}

export type BgoSectorViewFilter = 'all' | 'bgo' | 'bootloader' | 'nvram';

type RunnerProgressCallback = Parameters<
  NonNullable<Parameters<typeof usePipelineRunner>[0]['subscribeProgress']>
>[0];

function emitBenchProgress(
  ev: StreamProgressTelemetryFields & { readonly current: number; readonly total: number },
  stageLabel: string,
  logLine: string,
  setSelectedSa: (sa: number) => void,
  onProgress: RunnerProgressCallback,
): void {
  if (typeof ev.activeSa === 'number') setSelectedSa(ev.activeSa);
  onProgress(
    {
      current: ev.current,
      total: Math.max(1, ev.total),
      stageLabel,
      activeSa: ev.activeSa,
      bytesPerSec: ev.bytesPerSec,
      etaSeconds: ev.etaSeconds,
    },
    logLine,
  );
}

export function useBgoBenchViewModel(defaultPort?: string | null) {
  const { showToast } = useToast();
  const [portOverride, setPortOverride] = useState<string | null>(null);
  const [slice, setSlice] = useState<NorSliceKind>('bgo');
  const [dumpOutPath, setDumpOutPath] = useState('unbrick_out/eh55_nor_bgo_cow.bin');
  const [auditSourceMode, setAuditSourceMode] = useState<BgoAuditSourceMode>('DUMP_FILE');
  const [norPath, setNorPath] = useState('unbrick_out/eh55_nor_8mb.bin');
  const [backupDir, setBackupDir] = useState('unbrick_out/bgo_pre_repair_backup');
  const [enableBootTrace, setEnableBootTrace] = useState(true);
  const [cowSession, setCowSession] = useState<CowBackupSession | null>(null);
  const { checkedPreflightIds, togglePreflight } = usePreflightSelection();
  const [confirmTokenInput, setConfirmTokenInput] = useState('');
  const [sectorFilter, setSectorFilter] = useState<BgoSectorViewFilter>('bgo');
  const [selectedSa, setSelectedSa] = useState<number>(SPANSION_BGO_FIRST_SA);

  const portName = portOverride ?? (defaultPort || '');

  const subscribeNor = useCallback(
    (onProgress: RunnerProgressCallback) =>
      subscribeNorDumpProgress(ev =>
        emitBenchProgress(
          ev,
          `${ev.stage} (${ev.port})`,
          `[${ev.port}] ${ev.stage} ${ev.current}/${ev.total}`,
          setSelectedSa,
          onProgress,
        ),
      ),
    [],
  );

  const norDumpRunner = usePipelineRunner<void, PicoEh55NorDumpResponse>({
    subscribeProgress: subscribeNor,
    validateFn: () => {
      if (!portName.trim()) return translate('pipelines.bgoBench.portRequired');
      if (!dumpOutPath.trim()) return translate('pipelines.bgoBench.dumpOutRequired');
      return null;
    },
    executeFn: async () => {
      const res = await picoEh55NorDump({
        portName: portName.trim(),
        slice,
        outputPath: dumpOutPath.trim(),
      });
      setCowSession({ path: res.outputPath, sha256: res.sha256 });
      setNorPath(res.outputPath);
      showToast({ type: 'success', title: translate('errors.pipelines.norDumped') });
      return res;
    },
    formatSuccess: res => res.sha256,
  });

  const auditRunner = usePipelineRunner<void, Eh55BgoAuditReport>({
    validateFn: () => {
      if (auditSourceMode === 'DUMP_FILE' && !norPath.trim()) {
        return translate('pipelines.bgoBench.norPathRequired');
      }
      if (auditSourceMode === 'LIVE_PICO' && !portName.trim()) {
        return translate('pipelines.bgoBench.portRequired');
      }
      return null;
    },
    executeFn: () =>
      picoEh55BgoAudit({
        sourceMode: auditSourceMode,
        norPath: auditSourceMode === 'DUMP_FILE' ? norPath.trim() : undefined,
        portName: auditSourceMode === 'LIVE_PICO' ? portName.trim() : undefined,
      }),
    formatSuccess: rep => `${rep.total_sectors}`,
  });

  const dryRunRunner = usePipelineRunner<void, PicoEh55BgoRepairResponse>({
    executeFn: async () => {
      const res = await picoEh55BgoRepair({
        norPath: norPath.trim() || cowSession?.path,
        dryRunOnly: true,
        portName: portName.trim() || undefined,
      });
      showToast({ type: 'info', title: translate('errors.pipelines.bgoDryRunReady') });
      return res;
    },
    formatSuccess: res => res.outPlanPath ?? res.plan.expected_post_sha256,
  });

  const subscribeRepair = useCallback(
    (onProgress: RunnerProgressCallback) =>
      subscribeBgoRepairProgress(ev =>
        emitBenchProgress(
          ev,
          `${ev.stage}: ${ev.detail}`,
          `[${ev.stage}] ${ev.detail}`,
          setSelectedSa,
          onProgress,
        ),
      ),
    [],
  );

  const liveRepairRunner = usePipelineRunner<void, PicoEh55BgoRepairResponse>({
    subscribeProgress: subscribeRepair,
    validateFn: () => {
      if (!portName.trim()) return translate('pipelines.bgoBench.portRequired');
      if (!backupDir.trim()) return translate('pipelines.bgoBench.backupDirRequired');
      if (!cowSession) return translate('pipelines.bgoBench.preflight.cowBackupLockedReason');
      return null;
    },
    executeFn: async () => {
      const res = await picoEh55BgoRepair({
        norPath: cowSession?.path ?? (norPath.trim() || undefined),
        dryRunOnly: false,
        portName: portName.trim(),
        backupDir: backupDir.trim(),
        confirmToken: confirmTokenInput.trim(),
        runBootTraceMs: enableBootTrace ? DEFAULT_BGO_BOOTTRACE_MS : undefined,
      });
      showToast({ type: 'success', title: translate('errors.pipelines.bgoLiveApplied') });
      return res;
    },
    formatSuccess: res => res.liveReport?.post_bgo_sha256 ?? res.plan.expected_post_sha256,
  });

  const preflightChecks = useMemo<readonly PreflightCheckItem[]>(
    () => [
      {
        id: 'cowBackupSession',
        title: translate('pipelines.bgoBench.preflight.cowBackupSession'),
        detail: cowSession ? `${cowSession.path} (${cowSession.sha256.slice(0, 16)})` : translate('pipelines.bgoBench.preflight.cowBackupLockedReason'),
        severity: 'critical',
        lockedReason: cowSession ? undefined : translate('pipelines.bgoBench.preflight.cowBackupLockedReason'),
      },
      {
        id: 'upperNorUnlock',
        title: translate('pipelines.bgoBench.preflight.upperNorUnlock'),
        detail: translate('pipelines.bgoBench.preflight.upperNorUnlockDetail'),
        severity: 'critical',
      },
      {
        id: 'noBootInterrupt',
        title: translate('pipelines.bgoBench.preflight.noBootInterrupt'),
        detail: translate('pipelines.bgoBench.preflight.noBootInterruptDetail'),
        severity: 'warning',
      },
    ],
    [cowSession],
  );

  const eraseBlockedSas = useMemo<readonly number[]>(() => {
    const plan = liveRepairRunner.data?.plan ?? dryRunRunner.data?.plan;
    if (!plan) return [];
    const writeBlocked = plan.ops
      .filter((op): op is Extract<typeof op, { op: 'write_bytes' }> => op.op === 'write_bytes' && op.requires_erase_first)
      .map(op => op.sa);
    return [...new Set([...plan.erased_sas, ...writeBlocked])];
  }, [dryRunRunner.data?.plan, liveRepairRunner.data?.plan]);

  const preflightReady =
    Boolean(cowSession) &&
    Boolean(portName.trim()) &&
    Boolean(backupDir.trim()) &&
    preflightChecks.every(c => checkedPreflightIds.includes(c.id));

  const sectors = useMemo<readonly FlashSectorInfo[]>(
    () => buildBgo142SectorMap(auditRunner.data),
    [auditRunner.data],
  );
  const selectedSector = useMemo(
    () => sectors.find(sec => sec.index === selectedSa) ?? sectors[0] ?? null,
    [sectors, selectedSa],
  );
  const f2Integrity = useMemo(
    () => computeF2Integrity(auditRunner.data?.p2_present_ids ?? []),
    [auditRunner.data?.p2_present_ids],
  );

  const steps = useMemo<readonly PipelineStepDescriptor[]>(
    () => [
      { id: 'cowDump', label: translate('pipelines.bgoBench.steps.cowDump'), status: norDumpRunner.isBusy ? 'running' : norDumpRunner.error ? 'failed' : cowSession ? 'verified' : 'pending' },
      { id: 'bgoAudit', label: translate('pipelines.bgoBench.steps.bgoAudit'), status: auditRunner.isBusy ? 'running' : auditRunner.error ? 'failed' : auditRunner.data ? 'verified' : 'pending' },
      { id: 'dryRunPlan', label: translate('pipelines.bgoBench.steps.dryRunPlan'), status: dryRunRunner.isBusy ? 'running' : dryRunRunner.error ? 'failed' : dryRunRunner.data ? (eraseBlockedSas.length > 0 ? 'blocked' : 'verified') : 'pending' },
      { id: 'liveRepair', label: translate('pipelines.bgoBench.steps.liveRepair'), status: liveRepairRunner.isBusy ? 'running' : liveRepairRunner.error ? 'failed' : liveRepairRunner.data?.liveReport ? 'verified' : preflightReady ? 'pending' : 'blocked' },
    ],
    [auditRunner.data, auditRunner.error, auditRunner.isBusy, cowSession, dryRunRunner.data, dryRunRunner.error, dryRunRunner.isBusy, eraseBlockedSas.length, liveRepairRunner.data, liveRepairRunner.error, liveRepairRunner.isBusy, norDumpRunner.error, norDumpRunner.isBusy, preflightReady],
  );

  return {
    portName, setPortName: setPortOverride, slice, setSlice, dumpOutPath, setDumpOutPath,
    auditSourceMode, setAuditSourceMode, norPath, setNorPath, backupDir, setBackupDir,
    enableBootTrace, setEnableBootTrace, cowSession,
    dumpReport: norDumpRunner.data, dumpBusy: norDumpRunner.isBusy, dumpError: norDumpRunner.error,
    dumpProgress: norDumpRunner.progress, dumpLogs: norDumpRunner.logs, runNorDump: () => norDumpRunner.execute(),
    auditReport: auditRunner.data, auditBusy: auditRunner.isBusy, auditError: auditRunner.error, runBgoAudit: () => auditRunner.execute(),
    dryRunResponse: dryRunRunner.data, dryRunBusy: dryRunRunner.isBusy, dryRunError: dryRunRunner.error, runDryRunPlan: () => dryRunRunner.execute(),
    liveResponse: liveRepairRunner.data, liveBusy: liveRepairRunner.isBusy, liveError: liveRepairRunner.error,
    liveSuccess: liveRepairRunner.successMessage, liveProgress: liveRepairRunner.progress, liveLogs: liveRepairRunner.logs,
    runLiveRepair: () => liveRepairRunner.execute(),
    checkedPreflightIds, togglePreflight, preflightChecks, preflightReady,
    confirmTokenInput, setConfirmTokenInput, expectedConfirmToken: UPPER_NOR_CONFIRM_TOKEN,
    eraseBlockedSas, sectors, sectorFilter, setSectorFilter, selectedSector,
    onSelectSector: (sec: FlashSectorInfo) => setSelectedSa(sec.index),
    f2Integrity, steps,
    resetAll: () => {
      norDumpRunner.reset();
      auditRunner.reset();
      dryRunRunner.reset();
      liveRepairRunner.reset();
      setConfirmTokenInput('');
    },
  };
}
