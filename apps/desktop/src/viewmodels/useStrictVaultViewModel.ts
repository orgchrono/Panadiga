/**
 * @file viewmodels/useStrictVaultViewModel.ts
 * @brief MVVM ViewModel orchestrating the 6-step strict deterministic Express workflow:
 *        Model/JSON Registry -> Clean-Room SD/DVD (ISO 9660 1x) -> Pico BOOTSEL ->
 *        Segregated Vault (raw/ vs frm/ vs recompiled/) -> Mandatory Mn103Vm Pre-Write Gate -> Composed Write.
 */

import { useCallback, useEffect, useState } from 'react';
import { useToast } from '@/context/ToastContext';
import { translate } from '@/i18n/useTranslation';
import type {
  ComposedHddFlashReport,
  DvdBurnProgressEvent,
  DvdCreatorReport,
  FlashReport,
  OpticalBurnerDriveInfo,
  PicoUf2Variant,
  PreWriteGateReport,
  ResolvedModelProfile,
  SafeTargetDiskInfo,
  SdLoaderProfile,
  StrictWorkflowStep,
  VaultArtifactRecord,
  VaultBucket,
  VaultSessionManifest,
} from '@/models';
import { detectPicoBootselDrives, flashPicoFirmwareUf2 } from '@/services/picoService';
import { inspectSafeTargetDisks } from '@/services/pipelineService';
import {
  createDvdRecoveryDisc,
  flashComposedHddSafe,
  getResolvedModelProfile,
  initVaultSession,
  ingestVaultFile,
  inspectOpticalBurners,
  markVaultWorkflowStep,
  runPreWriteEmuGate,
  subscribeDvdBurnProgress,
} from '@/services/vaultService';
import { logger } from '@/utils/logger';

const DEFAULT_MODEL_ID = 'DMR-EH55';
const DEFAULT_DVD_LABEL = 'PANADIGA_REC';
const DEFAULT_INGEST_SAMPLE = 'ic5001_nor_8mb.bin';
const SECTOR_COUNT_ST3250 = 488_397_168;

function formatErr(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

function appendUniqueStep(
  steps: readonly StrictWorkflowStep[],
  next: StrictWorkflowStep,
): readonly StrictWorkflowStep[] {
  return steps.includes(next) ? steps : [...steps, next];
}

function appendArtifactRecord(
  session: VaultSessionManifest,
  rec: VaultArtifactRecord,
): VaultSessionManifest {
  const filterByName = (list: readonly VaultArtifactRecord[]) =>
    list.filter(item => item.filename !== rec.filename);
  const rawArtifacts = rec.bucket === 'raw' ? [...filterByName(session.rawArtifacts), rec] : session.rawArtifacts;
  const frmArtifacts = rec.bucket === 'frm' ? [...filterByName(session.frmArtifacts), rec] : session.frmArtifacts;
  const recompiledArtifacts =
    rec.bucket === 'recompiled' ? [...filterByName(session.recompiledArtifacts), rec] : session.recompiledArtifacts;
  return {
    ...session,
    rawArtifacts,
    frmArtifacts,
    recompiledArtifacts,
    completedSteps: appendUniqueStep(session.completedSteps, 'capture_and_backup'),
    currentStep: 'organize_and_recompile',
  };
}

export function useStrictVaultViewModel(selectedModel: string) {
  const { showToast } = useToast();
  const effectiveModel = selectedModel.trim() || DEFAULT_MODEL_ID;

  const [loaderProfile, setLoaderProfile] = useState<SdLoaderProfile>('ver061');
  const [resolvedProfile, setResolvedProfile] = useState<ResolvedModelProfile | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  const [activeSession, setActiveSession] = useState<VaultSessionManifest | null>(null);
  const [sessionBusy, setSessionBusy] = useState(false);
  const [sessionError, setSessionError] = useState<string | null>(null);

  // Step 2: Optical DVD ISO 9660 (1x)
  const [opticalBurners, setOpticalBurners] = useState<readonly OpticalBurnerDriveInfo[]>([]);
  const [dvdTargetMode, setDvdTargetMode] = useState<'iso' | 'burner'>('iso');
  const [selectedBurnerId, setSelectedBurnerId] = useState('');
  const [dvdConfirmToken, setDvdConfirmToken] = useState('');
  const [dvdBusy, setDvdBusy] = useState(false);
  const [dvdProgress, setDvdProgress] = useState<DvdBurnProgressEvent | null>(null);
  const [dvdReport, setDvdReport] = useState<DvdCreatorReport | null>(null);
  const [dvdError, setDvdError] = useState<string | null>(null);

  // Step 3: Pico BOOTSEL UF2
  const [bootselDrives, setBootselDrives] = useState<readonly string[]>([]);
  const [uf2Variant, setUf2Variant] = useState<PicoUf2Variant>('rp2040');
  const [uf2Busy, setUf2Busy] = useState(false);
  const [uf2Report, setUf2Report] = useState<FlashReport | null>(null);
  const [uf2Error, setUf2Error] = useState<string | null>(null);

  // Step 4: Segregated Vault Ingest
  const [ingestPathInput, setIngestPathInput] = useState(DEFAULT_INGEST_SAMPLE);
  const [ingestBucketOverride, setIngestBucketOverride] = useState<'auto' | VaultBucket>('auto');
  const [ingestBusy, setIngestBusy] = useState(false);
  const [ingestError, setIngestError] = useState<string | null>(null);

  // Step 5 & 6: Mandatory Mn103Vm Gate & Streaming Composed HDD Write
  const [emuGateBusy, setEmuGateBusy] = useState(false);
  const [emuGateReport, setEmuGateReport] = useState<PreWriteGateReport | null>(null);
  const [emuGateError, setEmuGateError] = useState<string | null>(null);
  const [hddTargets, setHddTargets] = useState<readonly SafeTargetDiskInfo[]>([]);
  const [hddDevicePath, setHddDevicePath] = useState('\\\\.\\PhysicalDrive4');
  const [hddConfirmToken, setHddConfirmToken] = useState('');
  const [hddBusy, setHddBusy] = useState(false);
  const [hddReport, setHddReport] = useState<ComposedHddFlashReport | null>(null);
  const [hddError, setHddError] = useState<string | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    void Promise.resolve().then(async () => {
      setIsLoading(true);
      try {
        const [profile, burners, disks, drives] = await Promise.all([
          getResolvedModelProfile(effectiveModel),
          inspectOpticalBurners(),
          inspectSafeTargetDisks(),
          detectPicoBootselDrives(),
        ]);
        if (controller.signal.aborted) return;
        setResolvedProfile(profile);
        setOpticalBurners(burners);
        if (burners.length > 0 && !selectedBurnerId) {
          setSelectedBurnerId(burners[0].deviceId);
        }
        setHddTargets(disks);
        const safeDisk = disks.find(d => d.safe_to_flash);
        if (safeDisk) {
          setHddDevicePath(safeDisk.device_path);
        }
        setBootselDrives(drives);
      } catch (err) {
        logger.warn('StrictVaultViewModel', 'Relational hardware profile load warning', err);
      } finally {
        if (!controller.signal.aborted) {
          setIsLoading(false);
        }
      }
    });
    return () => controller.abort();
  }, [effectiveModel, selectedBurnerId]);

  const initializeVaultSession = useCallback(async () => {
    setSessionBusy(true);
    setSessionError(null);
    try {
      const manifest = await initVaultSession({ modelId: effectiveModel, loaderProfile });
      setActiveSession(manifest);
      setEmuGateReport(manifest.lastEmuGate);
      showToast({
        type: 'success',
        title: translate('express.vault.sessionReady'),
        message: manifest.sessionId,
      });
    } catch (err) {
      setSessionError(formatErr(err));
    } finally {
      setSessionBusy(false);
    }
  }, [effectiveModel, loaderProfile, showToast]);

  const recordStepCompleted = useCallback((step: StrictWorkflowStep, gate?: PreWriteGateReport) => {
    setActiveSession(prev => {
      if (!prev) return prev;
      return {
        ...prev,
        completedSteps: appendUniqueStep(prev.completedSteps, step),
        ...(gate ? { lastEmuGate: gate } : {}),
      };
    });
  }, []);

  const refreshBurners = useCallback(async () => {
    const list = await inspectOpticalBurners();
    setOpticalBurners(list);
    if (list.length > 0 && !selectedBurnerId) {
      setSelectedBurnerId(list[0].deviceId);
    }
  }, [selectedBurnerId]);

  const executeDvdCreate = useCallback(async () => {
    setDvdError(null);
    if (dvdTargetMode === 'burner' && !selectedBurnerId.trim()) {
      setDvdError(translate('express.dvd.noBurnerSelected'));
      return;
    }
    setDvdBusy(true);
    setDvdProgress(null);
    const unlisten = await subscribeDvdBurnProgress(ev => setDvdProgress(ev));
    try {
      const rep = await createDvdRecoveryDisc({
        ...(dvdTargetMode === 'burner'
          ? { burnerId: selectedBurnerId.trim(), confirmToken: dvdConfirmToken.trim() }
          : {}),
        model: effectiveModel,
        volumeLabel: DEFAULT_DVD_LABEL,
        loaderProfile,
        cleanroomOnly: true,
        ...(activeSession ? { sessionId: activeSession.sessionId } : {}),
      });
      setDvdReport(rep);
      setEmuGateReport(rep.emuGate);
      recordStepCompleted('build_cleanroom_sd', rep.emuGate);
      showToast({ type: 'success', title: translate('express.dvd.successTitle') });
    } catch (err) {
      setDvdError(formatErr(err));
    } finally {
      unlisten();
      setDvdBusy(false);
    }
  }, [
    activeSession,
    dvdConfirmToken,
    dvdTargetMode,
    effectiveModel,
    loaderProfile,
    recordStepCompleted,
    selectedBurnerId,
    showToast,
  ]);

  const detectBootsel = useCallback(async () => {
    const found = await detectPicoBootselDrives();
    setBootselDrives(found);
  }, []);

  const flashPicoUf2 = useCallback(async () => {
    setUf2Busy(true);
    setUf2Error(null);
    try {
      const rep = await flashPicoFirmwareUf2(uf2Variant);
      setUf2Report(rep);
      if (activeSession) {
        await markVaultWorkflowStep(activeSession.sessionId, 'setup_pico_probe');
      }
      recordStepCompleted('setup_pico_probe');
    } catch (err) {
      setUf2Error(formatErr(err));
    } finally {
      setUf2Busy(false);
    }
  }, [activeSession, recordStepCompleted, uf2Variant]);

  const ingestIntoVault = useCallback(async () => {
    const cleanPath = ingestPathInput.trim();
    if (!cleanPath) {
      setIngestError(translate('express.vault.pathRequired'));
      return;
    }
    const targetSession = activeSession ?? (await initVaultSession({ modelId: effectiveModel, loaderProfile }));
    if (!activeSession) {
      setActiveSession(targetSession);
    }
    setIngestBusy(true);
    setIngestError(null);
    try {
      const rec = await ingestVaultFile({
        sessionId: targetSession.sessionId,
        sourcePath: cleanPath,
        ...(ingestBucketOverride !== 'auto' ? { bucketOverride: ingestBucketOverride } : {}),
        origin: 'express_vault_ingest',
      });
      setActiveSession(prev => appendArtifactRecord(prev ?? targetSession, rec));
    } catch (err) {
      setIngestError(formatErr(err));
    } finally {
      setIngestBusy(false);
    }
  }, [activeSession, effectiveModel, ingestBucketOverride, ingestPathInput, loaderProfile]);

  const executeMandatoryEmuGate = useCallback(async () => {
    setEmuGateBusy(true);
    setEmuGateError(null);
    try {
      const gate = await runPreWriteEmuGate({
        ...(activeSession ? { sessionId: activeSession.sessionId } : {}),
        targetKind: 'composed_hdd_stream',
        loaderProfile,
        totalSectors: SECTOR_COUNT_ST3250,
        expectPassthrough: true,
        expectCapacityUnlock: true,
      });
      setEmuGateReport(gate);
      recordStepCompleted('organize_and_recompile', gate);
      showToast({ type: 'success', title: translate('express.gate.passedTitle') });
    } catch (err) {
      setEmuGateError(formatErr(err));
    } finally {
      setEmuGateBusy(false);
    }
  }, [activeSession, loaderProfile, recordStepCompleted, showToast]);

  const executeComposedHddFlash = useCallback(async () => {
    if (!emuGateReport?.passed) {
      setHddError(translate('express.gate.lockedAlert'));
      return;
    }
    setHddBusy(true);
    setHddError(null);
    try {
      const rep = await flashComposedHddSafe({
        devicePath: hddDevicePath.trim(),
        confirmDiskModelToken: hddConfirmToken.trim(),
        applyDynamicCapacityUnlock: true,
        applyAvPassthrough: true,
        ...(activeSession ? { sessionId: activeSession.sessionId } : {}),
      });
      setHddReport(rep);
      setEmuGateReport(rep.emuGate);
      recordStepCompleted('emulate_and_flash', rep.emuGate);
      showToast({ type: 'success', title: translate('express.gate.hddWriteDone') });
    } catch (err) {
      setHddError(formatErr(err));
    } finally {
      setHddBusy(false);
    }
  }, [activeSession, emuGateReport, hddConfirmToken, hddDevicePath, recordStepCompleted, showToast]);

  return {
    loaderProfile,
    setLoaderProfile,
    resolvedProfile,
    isLoading,
    activeSession,
    sessionBusy,
    sessionError,
    initializeVaultSession,
    recordStepCompleted,
    // Step 2 Optical DVD
    opticalBurners,
    dvdTargetMode,
    setDvdTargetMode,
    selectedBurnerId,
    setSelectedBurnerId,
    dvdConfirmToken,
    setDvdConfirmToken,
    dvdBusy,
    dvdProgress,
    dvdReport,
    dvdError,
    refreshBurners,
    executeDvdCreate,
    // Step 3 Pico BOOTSEL
    bootselDrives,
    uf2Variant,
    setUf2Variant,
    uf2Busy,
    uf2Report,
    uf2Error,
    detectBootsel,
    flashPicoUf2,
    // Step 4 Segregated Vault
    ingestPathInput,
    setIngestPathInput,
    ingestBucketOverride,
    setIngestBucketOverride,
    ingestBusy,
    ingestError,
    ingestIntoVault,
    // Step 5 & 6 Gate + HDD Streaming
    emuGateBusy,
    emuGateReport,
    emuGateError,
    executeMandatoryEmuGate,
    hddTargets,
    hddDevicePath,
    setHddDevicePath,
    hddConfirmToken,
    setHddConfirmToken,
    hddBusy,
    hddReport,
    hddError,
    executeComposedHddFlash,
  };
}
