/**
 * @file viewmodels/useExpressViewModel.ts
 * @brief MVVM ViewModel orchestrating Express Mode's 3 recovery missions & JIT P7501 safety gate.
 */

import { useCallback, useEffect, useMemo, useState } from 'react';
import { PICO_DEFAULT_FALLBACK_PORT } from '@/constants';
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
  fetchExpressPicoPorts,
  fetchExpressSdTargets,
  fetchInitialExpressDiscovery,
  formatExpressError,
  pickDefaultSafeSdPath,
  runLocalNorInspectMission,
  runNorRepairMission,
  runPicoNorInspectMission,
  runSdFlashMission,
  runU99RescueMission,
  type ShowToastFn,
  validateSdFlashSelection,
} from './expressMissionLogic';

type ExpressToastTitleKey = Parameters<typeof translate>[0];

async function executeMissionWithGuard(
  setError: (msg: string | null) => void,
  errorTitleKey: ExpressToastTitleKey | null,
  showToast: ShowToastFn,
  task: () => Promise<void>,
): Promise<void> {
  setError(null);
  try {
    await task();
  } catch (err) {
    const msg = formatExpressError(err);
    setError(msg);
    if (errorTitleKey) showToast({ type: 'error', title: translate(errorTitleKey), message: msg });
  }
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
    const list = await fetchExpressSdTargets();
    if (list) {
      setSdTargets(list);
      const firstSafe = pickDefaultSafeSdPath(list);
      if (firstSafe && !selectedDevicePath) setSelectedDevicePath(firstSafe);
    }
    setSdTargetsLoading(false);
  }, [selectedDevicePath]);

  const refreshPicoPorts = useCallback(async () => {
    setPicoPortsLoading(true);
    const ports = await fetchExpressPicoPorts();
    if (ports) {
      setPicoPorts(ports);
      if (ports.length > 0 && (!picoPortInput || picoPortInput === PICO_DEFAULT_FALLBACK_PORT)) {
        setPicoPortInput(ports[0]);
      }
    }
    setPicoPortsLoading(false);
  }, [picoPortInput]);

  useEffect(() => {
    const controller = new AbortController();
    void fetchInitialExpressDiscovery().then(res => {
      if (controller.signal.aborted || !res) return;
      setModelCatalog(res.models);
      setSdTargets(res.targets);
      const firstSafe = pickDefaultSafeSdPath(res.targets);
      if (firstSafe) setSelectedDevicePath(firstSafe);
      setPicoPorts(res.ports);
      if (res.ports.length > 0) setPicoPortInput(res.ports[0]);
    });
    return () => controller.abort();
  }, []);

  const selectedSdDrive = useMemo(
    () => sdTargets.find(d => d.device_path === selectedDevicePath) ?? null,
    [sdTargets, selectedDevicePath],
  );

  const executeSdFlash = useCallback(async () => {
    setSdError(null);
    const validationError = validateSdFlashSelection(sdTargetMode, selectedSdDrive, sdConfirmToken);
    if (validationError) {
      setSdError(validationError);
      return;
    }
    setSdFlashing(true);
    setSdProgress(null);
    await executeMissionWithGuard(setSdError, 'errors.unbrick.sdFlashTag', showToast, () =>
      runSdFlashMission({
        sdTargetMode,
        selectedSdDrive,
        sdConfirmToken,
        selectedModel,
        includePanaSd,
        onProgress: setSdProgress,
        onReport: setSdReport,
        showToast,
      }),
    );
    setSdFlashing(false);
  }, [includePanaSd, sdConfirmToken, sdTargetMode, selectedModel, selectedSdDrive, showToast]);

  const executeU99QuickRescue = useCallback(() => {
    const cleanPort = picoPortInput.trim();
    if (!cleanPort) { setU99Error(translate('errors.pipelines.nvramPortRequired')); return; }
    setU99Error(null);
    runWithPicoSafetyGate(async () => {
      setU99Cleaning(true);
      await executeMissionWithGuard(setU99Error, 'errors.nvram.surgicalCleanFailedTitle', showToast, () =>
        runU99RescueMission({ cleanPort, onProgress: setU99Progress, onReport: setU99Report, showToast }),
      );
      setU99Cleaning(false);
    });
  }, [picoPortInput, runWithPicoSafetyGate, showToast]);

  const runLocalNorInspect = useCallback(async () => {
    setNorAuditing(true);
    await executeMissionWithGuard(setNorError, null, showToast, () =>
      runLocalNorInspectMission({
        onAudit: (nvram, bgo) => { setNorAuditNvram(nvram); setNorAuditBgo(bgo); },
      }),
    );
    setNorAuditing(false);
  }, [showToast]);

  const executeNorInspect = useCallback(() => {
    if (norSourceMode === 'local') { void runLocalNorInspect(); return; }
    const cleanPort = picoPortInput.trim();
    if (!cleanPort) { setNorError(translate('errors.pipelines.nvramPortRequired')); return; }
    setNorError(null);
    runWithPicoSafetyGate(async () => {
      setNorCapturing(true); setNorAuditing(true); setNorProgress(null);
      await executeMissionWithGuard(setNorError, 'errors.pipelines.norDumpFailed', showToast, () =>
        runPicoNorInspectMission({
          cleanPort, onProgress: setNorProgress, onDumpReport: setNorDumpReport,
          onAudit: (nvram, bgo) => { setNorAuditNvram(nvram); setNorAuditBgo(bgo); }, showToast,
        }),
      );
      setNorCapturing(false); setNorAuditing(false);
    });
  }, [norSourceMode, picoPortInput, runLocalNorInspect, runWithPicoSafetyGate, showToast]);

  const executeNorSafeRepair = useCallback(
    async (liveUpperNor = false) => {
      setNorPatching(true);
      await executeMissionWithGuard(
        setNorError,
        'errors.pipelines.bgoRepairFailed',
        showToast,
        () =>
          runNorRepairMission({
            liveUpperNor,
            portName: picoPortInput,
            norAuditNvram,
            norDumpReport,
            onNvramUpdated: setNorAuditNvram,
            onBgoUpdated: setNorAuditBgo,
            onResult: setNorRepairResult,
            showToast,
          }),
      );
      setNorPatching(false);
    },
    [norAuditNvram, norDumpReport, picoPortInput, showToast],
  );

  return {
    safetySheetOpen, confirmPicoSafetySheet, cancelPicoSafetySheet, openInEngineering: onOpenEngineeringView,
    // Mission 1
    modelCatalog, selectedModel, setSelectedModel, includePanaSd, setIncludePanaSd,
    sdTargets, sdTargetsLoading, sdTargetMode, setSdTargetMode, selectedDevicePath, setSelectedDevicePath,
    selectedSdDrive, sdConfirmToken, setSdConfirmToken, sdFlashing, sdProgress, sdError, sdReport,
    refreshSdTargets, executeSdFlash,
    // Mission 2
    picoPorts, picoPortsLoading, picoPortInput, setPicoPortInput, refreshPicoPorts,
    u99Cleaning, u99Progress, u99Error, u99Report, executeU99QuickRescue,
    // Mission 3
    norSourceMode, setNorSourceMode, norCapturing, norProgress, norDumpReport, norAuditing,
    norAuditNvram, norAuditBgo, norPatching, norRepairResult, norError,
    executeNorInspect, executeNorSafeRepair,
  };
}
