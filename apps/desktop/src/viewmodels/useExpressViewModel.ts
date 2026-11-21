/**
 * @file viewmodels/useExpressViewModel.ts
 * @brief MVVM ViewModel orchestrating Express Mode's 3 recovery missions & JIT P7501 safety gate.
 */

import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  PICO_DEFAULT_FALLBACK_PORT,
  SD_DEFAULT_VOLUME_MB,
  UPPER_NOR_CONFIRM_TOKEN,
} from '@/constants';
import { useToast } from '@/context/ToastContext';
import { useWorkbenchMode } from '@/context/WorkbenchModeContext';
import { translate } from '@/i18n/useTranslation';
import type {
  DigaModelProfile,
  Eh55BgoAuditReport,
  NorDumpProgressEvent,
  NorRepairExpressOutcome,
  NvramCleanProgressEvent,
  NvramCleanReport,
  PicoEh55NorDumpResponse,
  PicoPanamon1CleanNvramResponse,
  SdCardCreatorReport,
  SdFlashProgressEvent,
  SdTargetDriveInfo,
  ViewKey,
} from '@/models';
import {
  getDigaModelCatalog,
  listPicoSerialPorts,
} from '@/services/picoService';
import {
  picoEh55BgoAudit,
  picoEh55BgoRepair,
  picoEh55NorDump,
  picoPanamon1CleanNvram,
  subscribeNorDumpProgress,
  subscribeNvramCleanProgress,
} from '@/services/pipelineService';
import {
  analyzeFlashDump,
  cleanNvramSurgical,
} from '@/services/storageCryptoService';
import {
  emitNvramCleanedEvent,
  emitUnbrickCompletedEvent,
} from '@/services/systemService';
import {
  flashSdRecoveryCard,
  inspectSdCardTargets,
  subscribeSdFlashProgress,
} from '@/services/unbrickService';
import { logger } from '@/utils/logger';

const PROGRESS_DONE_DEFAULT = 8;

function formatErr(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

export function useExpressViewModel(onOpenEngineeringView: (view: ViewKey) => void) {
  const { showToast } = useToast();
  const { picoSafetySheetAcknowledged, acknowledgePicoSafetySheet } = useWorkbenchMode();

  // JIT P7501 safety gate (shown once per session before first electrical Pico action)
  const [safetySheetOpen, setSafetySheetOpen] = useState(false);
  const [pendingPicoAction, setPendingPicoAction] = useState<(() => Promise<void>) | null>(null);

  const runWithPicoSafetyGate = useCallback(
    (action: () => Promise<void>) => {
      if (picoSafetySheetAcknowledged) {
        void action();
        return;
      }
      setPendingPicoAction(() => action);
      setSafetySheetOpen(true);
    },
    [picoSafetySheetAcknowledged],
  );

  const confirmPicoSafetySheet = useCallback(() => {
    acknowledgePicoSafetySheet();
    setSafetySheetOpen(false);
    const pending = pendingPicoAction;
    setPendingPicoAction(null);
    if (pending) {
      void pending();
    }
  }, [acknowledgePicoSafetySheet, pendingPicoAction]);

  const cancelPicoSafetySheet = useCallback(() => {
    setSafetySheetOpen(false);
    setPendingPicoAction(null);
  }, []);

  // Mission 1: SD Recovery Card Creator
  const [modelCatalog, setModelCatalog] = useState<readonly DigaModelProfile[]>([]);
  const [selectedModel, setSelectedModel] = useState('DMR-EH55');
  const [includePanaSd, setIncludePanaSd] = useState(false);
  const [sdTargets, setSdTargets] = useState<readonly SdTargetDriveInfo[]>([]);
  const [sdTargetsLoading, setSdTargetsLoading] = useState(false);
  const [sdTargetMode, setSdTargetMode] = useState<'image' | 'physical'>('image');
  const [selectedDevicePath, setSelectedDevicePath] = useState('');
  const [sdConfirmToken, setSdConfirmToken] = useState('');
  const [sdFlashing, setSdFlashing] = useState(false);
  const [sdProgress, setSdProgress] = useState<SdFlashProgressEvent | null>(null);
  const [sdError, setSdError] = useState<string | null>(null);
  const [sdReport, setSdReport] = useState<SdCardCreatorReport | null>(null);

  // Mission 2: U99 Quick Rescue via Pico PANAMON1
  const [picoPorts, setPicoPorts] = useState<readonly string[]>([]);
  const [picoPortsLoading, setPicoPortsLoading] = useState(false);
  const [picoPortInput, setPicoPortInput] = useState(PICO_DEFAULT_FALLBACK_PORT);
  const [u99Cleaning, setU99Cleaning] = useState(false);
  const [u99Progress, setU99Progress] = useState<NvramCleanProgressEvent | null>(null);
  const [u99Error, setU99Error] = useState<string | null>(null);
  const [u99Report, setU99Report] = useState<PicoPanamon1CleanNvramResponse | null>(null);

  // Mission 3: Safe 8 MiB NOR Backup & Repair
  const [norSourceMode, setNorSourceMode] = useState<'local' | 'pico'>('local');
  const [norCapturing, setNorCapturing] = useState(false);
  const [norProgress, setNorProgress] = useState<NorDumpProgressEvent | null>(null);
  const [norDumpReport, setNorDumpReport] = useState<PicoEh55NorDumpResponse | null>(null);
  const [norAuditing, setNorAuditing] = useState(false);
  const [norAuditNvram, setNorAuditNvram] = useState<NvramCleanReport | null>(null);
  const [norAuditBgo, setNorAuditBgo] = useState<Eh55BgoAuditReport | null>(null);
  const [norPatching, setNorPatching] = useState(false);
  const [norRepairResult, setNorRepairResult] = useState<NorRepairExpressOutcome | null>(null);
  const [norError, setNorError] = useState<string | null>(null);

  const refreshSdTargets = useCallback(async () => {
    setSdTargetsLoading(true);
    try {
      const list = await inspectSdCardTargets();
      setSdTargets(list);
      const firstSafe = list.find(item => item.safe_sd_target);
      if (firstSafe && !selectedDevicePath) {
        setSelectedDevicePath(firstSafe.device_path);
      }
    } catch (err) {
      logger.warn('ExpressViewModel', 'Failed enumerating SD card targets', err);
    } finally {
      setSdTargetsLoading(false);
    }
  }, [selectedDevicePath]);

  const refreshPicoPorts = useCallback(async () => {
    setPicoPortsLoading(true);
    try {
      const ports = await listPicoSerialPorts();
      setPicoPorts(ports);
      if (ports.length > 0 && (!picoPortInput || picoPortInput === PICO_DEFAULT_FALLBACK_PORT)) {
        setPicoPortInput(ports[0]);
      }
    } catch (err) {
      logger.warn('ExpressViewModel', 'Failed enumerating Pico serial ports', err);
    } finally {
      setPicoPortsLoading(false);
    }
  }, [picoPortInput]);

  useEffect(() => {
    const controller = new AbortController();
    void Promise.resolve().then(async () => {
      try {
        const [catalog, targets, ports] = await Promise.all([
          getDigaModelCatalog(),
          inspectSdCardTargets(),
          listPicoSerialPorts(),
        ]);
        if (controller.signal.aborted) return;
        setModelCatalog(catalog.filter(item => item.model_id !== 'AUTO'));
        setSdTargets(targets);
        const firstSafe = targets.find(item => item.safe_sd_target);
        if (firstSafe) {
          setSelectedDevicePath(firstSafe.device_path);
        }
        setPicoPorts(ports);
        if (ports.length > 0) {
          setPicoPortInput(ports[0]);
        }
      } catch (err) {
        logger.warn('ExpressViewModel', 'Initial Express discovery failed', err);
      }
    });
    return () => controller.abort();
  }, []);

  const selectedSdDrive = useMemo(
    () => sdTargets.find(d => d.device_path === selectedDevicePath) ?? null,
    [sdTargets, selectedDevicePath],
  );

  const executeSdFlash = useCallback(async () => {
    setSdError(null);
    if (sdTargetMode === 'physical') {
      if (!selectedSdDrive) {
        setSdError(translate('errors.unbrick.sdTargetRequired'));
        return;
      }
      if (sdConfirmToken.trim() !== selectedSdDrive.expected_confirm_token) {
        setSdError(
          translate('errors.unbrick.sdConfirmRequired', {
            token: selectedSdDrive.expected_confirm_token,
          }),
        );
        return;
      }
    }
    setSdFlashing(true);
    setSdProgress(null);
    const unlisten = await subscribeSdFlashProgress(ev => setSdProgress(ev));
    try {
      const report = await flashSdRecoveryCard({
        ...(sdTargetMode === 'physical' && selectedSdDrive
          ? {
              devicePath: selectedSdDrive.device_path,
              confirmToken: sdConfirmToken.trim(),
            }
          : {}),
        model: selectedModel,
        volumeMb: SD_DEFAULT_VOLUME_MB,
        loaderProfile: 'ver061',
        includePanaSd,
      });
      setSdReport(report);
      emitUnbrickCompletedEvent(true);
      showToast({
        type: 'success',
        title: translate('errors.unbrick.sdFlashTag'),
        message: translate('errors.unbrick.sdFlashDone'),
      });
    } catch (err) {
      const msg = formatErr(err);
      setSdError(msg);
      showToast({ type: 'error', title: translate('errors.unbrick.sdFlashTag'), message: msg });
    } finally {
      unlisten();
      setSdFlashing(false);
    }
  }, [includePanaSd, sdConfirmToken, sdTargetMode, selectedModel, selectedSdDrive, showToast]);

  const executeU99QuickRescue = useCallback(() => {
    const cleanPort = picoPortInput.trim();
    if (!cleanPort) {
      setU99Error(translate('errors.pipelines.nvramPortRequired'));
      return;
    }
    setU99Error(null);
    runWithPicoSafetyGate(async () => {
      setU99Cleaning(true);
      setU99Error(null);
      setU99Progress({
        port: cleanPort,
        stage: 'connect_bridge',
        current: 0,
        total: PROGRESS_DONE_DEFAULT,
        detail: cleanPort,
      });
      const unlisten = await subscribeNvramCleanProgress(ev => setU99Progress(ev));
      try {
        const report = await picoPanamon1CleanNvram({ portName: cleanPort });
        setU99Report(report);
        emitNvramCleanedEvent(true);
        showToast({
          type: 'success',
          title: translate('errors.nvram.surgicalCleanBadgeTitle'),
          message: translate('errors.pipelines.panamon1CleanDone'),
        });
      } catch (err) {
        const msg = formatErr(err);
        setU99Error(msg);
        showToast({
          type: 'error',
          title: translate('errors.nvram.surgicalCleanFailedTitle'),
          message: msg,
        });
      } finally {
        unlisten();
        setU99Cleaning(false);
      }
    });
  }, [picoPortInput, runWithPicoSafetyGate, showToast]);

  const runLocalNorInspect = useCallback(async () => {
    setNorAuditing(true);
    setNorError(null);
    try {
      const nvram = await analyzeFlashDump();
      const bgo = await picoEh55BgoAudit({
        sourceMode: 'DUMP_FILE',
        norPath: nvram.input_file,
      });
      setNorAuditNvram(nvram);
      setNorAuditBgo(bgo);
    } catch (err) {
      const msg = formatErr(err);
      setNorError(msg);
    } finally {
      setNorAuditing(false);
    }
  }, []);

  const executeNorInspect = useCallback(() => {
    if (norSourceMode === 'local') {
      void runLocalNorInspect();
      return;
    }
    const cleanPort = picoPortInput.trim();
    if (!cleanPort) {
      setNorError(translate('errors.pipelines.nvramPortRequired'));
      return;
    }
    setNorError(null);
    runWithPicoSafetyGate(async () => {
      setNorCapturing(true);
      setNorAuditing(true);
      setNorProgress(null);
      const unlisten = await subscribeNorDumpProgress(ev => setNorProgress(ev));
      try {
        const dumpRes = await picoEh55NorDump({ portName: cleanPort, slice: 'full' });
        setNorDumpReport(dumpRes);
        const [nvram, bgo] = await Promise.all([
          analyzeFlashDump(),
          picoEh55BgoAudit({ sourceMode: 'DUMP_FILE', norPath: dumpRes.outputPath }),
        ]);
        setNorAuditNvram(nvram);
        setNorAuditBgo(bgo);
        showToast({ type: 'success', title: translate('errors.pipelines.norDumped') });
      } catch (err) {
        const msg = formatErr(err);
        setNorError(msg);
        showToast({ type: 'error', title: translate('errors.pipelines.norDumpFailed'), message: msg });
      } finally {
        unlisten();
        setNorCapturing(false);
        setNorAuditing(false);
      }
    });
  }, [norSourceMode, picoPortInput, runLocalNorInspect, runWithPicoSafetyGate, showToast]);

  const executeNorSafeRepair = useCallback(
    async (liveUpperNor = false) => {
      setNorPatching(true);
      setNorError(null);
      try {
        if (norAuditNvram && norAuditNvram.nvram_non_ff_bytes > 0) {
          const cleaned = await cleanNvramSurgical();
          setNorAuditNvram(cleaned);
          emitNvramCleanedEvent(true);
          setNorRepairResult({
            mode: 'nvram_clean',
            summaryPath: cleaned.output_file,
            sha256: cleaned.clean_sha256,
          });
          showToast({
            type: 'success',
            title: translate('errors.nvram.surgicalCleanTitle'),
            message: translate('errors.nvram.surgicalCleanMsg'),
          });
          return;
        }
        const targetNor = norDumpReport?.outputPath ?? norAuditNvram?.input_file;
        const bgoRes = await picoEh55BgoRepair({
          norPath: targetNor,
          dryRunOnly: !liveUpperNor,
          ...(liveUpperNor
            ? { portName: picoPortInput.trim(), confirmToken: UPPER_NOR_CONFIRM_TOKEN }
            : {}),
        });
        setNorAuditBgo(bgoRes.plan.post_audit);
        setNorRepairResult({
          mode: liveUpperNor ? 'bgo_live' : 'bgo_cow',
          summaryPath:
            bgoRes.liveReport?.backup_path ??
            bgoRes.cowOutPath ??
            bgoRes.outPlanPath ??
            targetNor ??
            '',
          sha256: bgoRes.liveReport?.post_bgo_sha256 ?? bgoRes.plan.expected_post_sha256,
        });
        showToast({
          type: 'success',
          title: translate(
            liveUpperNor ? 'errors.pipelines.bgoLiveApplied' : 'errors.pipelines.bgoDryRunReady',
          ),
        });
      } catch (err) {
        const msg = formatErr(err);
        setNorError(msg);
        showToast({ type: 'error', title: translate('errors.pipelines.bgoRepairFailed'), message: msg });
      } finally {
        setNorPatching(false);
      }
    },
    [norAuditNvram, norDumpReport, picoPortInput, showToast],
  );

  return {
    safetySheetOpen,
    confirmPicoSafetySheet,
    cancelPicoSafetySheet,
    openInEngineering: onOpenEngineeringView,
    // Mission 1
    modelCatalog,
    selectedModel,
    setSelectedModel,
    includePanaSd,
    setIncludePanaSd,
    sdTargets,
    sdTargetsLoading,
    sdTargetMode,
    setSdTargetMode,
    selectedDevicePath,
    setSelectedDevicePath,
    selectedSdDrive,
    sdConfirmToken,
    setSdConfirmToken,
    sdFlashing,
    sdProgress,
    sdError,
    sdReport,
    refreshSdTargets,
    executeSdFlash,
    // Mission 2
    picoPorts,
    picoPortsLoading,
    picoPortInput,
    setPicoPortInput,
    refreshPicoPorts,
    u99Cleaning,
    u99Progress,
    u99Error,
    u99Report,
    executeU99QuickRescue,
    // Mission 3
    norSourceMode,
    setNorSourceMode,
    norCapturing,
    norProgress,
    norDumpReport,
    norAuditing,
    norAuditNvram,
    norAuditBgo,
    norPatching,
    norRepairResult,
    norError,
    executeNorInspect,
    executeNorSafeRepair,
  };
}
