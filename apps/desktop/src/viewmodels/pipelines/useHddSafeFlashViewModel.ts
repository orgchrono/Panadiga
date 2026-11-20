import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  BUS_TYPE_NVME_ID,
  HDD_BENCH_5GB_CAPACITY,
  HDD_DEFAULT_CAPACITY_GB,
  HDD_MAX_CAPACITY_GB,
  HDD_MAX_EXTENDED_4TB_GB,
  HDD_MIN_REGULAR_MEIHDFS_GB,
} from '@/constants';
import { useToast } from '@/context/ToastContext';
import { translate } from '@/i18n/useTranslation';
import type {
  Eh55HddFlashReport,
  PipelineStepDescriptor,
  PreflightCheckItem,
  SafeTargetDiskInfo,
} from '@/models';
import {
  flashEh55HddSafe,
  inspectSafeTargetDisks,
  subscribeHddFlashProgress,
} from '@/services/pipelineService';
import { isValidEh55CapacityGb, previewBlocks2Kb } from '@/utils/hddCapacity';
import { getExpectedDriveConfirmToken } from '../useHddWriterViewModel';
import { useDigestCopy, usePreflightSelection } from './usePipelineHelpers';
import { usePipelineRunner } from './usePipelineRunner';

const DEFAULT_EH55_EXPECTED_MAX_GB = 260;

export function isDiskPermanentlyBlocked(disk: SafeTargetDiskInfo): boolean {
  return (
    !disk.safe_to_flash ||
    disk.is_nvme_blocked ||
    disk.is_os_blocked ||
    disk.bus_type_id === BUS_TYPE_NVME_ID ||
    disk.device_path.toLowerCase().includes('physicaldrive0')
  );
}

export function useHddSafeFlashViewModel() {
  const { showToast } = useToast();
  const [targetGb, setTargetGb] = useState<number>(HDD_DEFAULT_CAPACITY_GB);
  const [allowExtended4tb, setAllowExtended4tb] = useState(false);
  const [disks, setDisks] = useState<readonly SafeTargetDiskInfo[]>([]);
  const [scanningDisks, setScanningDisks] = useState(false);
  const [scanError, setScanError] = useState<string | null>(null);
  const [selectedDevicePath, setSelectedDevicePath] = useState<string | null>(null);
  const [imagePath, setImagePath] = useState('');
  const [backupDir, setBackupDir] = useState('unbrick_out/hdd_64k_backup');
  const { checkedPreflightIds, togglePreflight } = usePreflightSelection();
  const [confirmTokenInput, setConfirmTokenInput] = useState('');
  const { copiedHash, copyDigest } = useDigestCopy();

  const loadDisks = useCallback(async (signal?: AbortSignal) => {
    setScanningDisks(true);
    setScanError(null);
    try {
      const list = await inspectSafeTargetDisks();
      if (signal?.aborted) return;
      setDisks(list);
      const firstSafe = list.find(d => !isDiskPermanentlyBlocked(d));
      setSelectedDevicePath(prev =>
        prev && list.some(d => d.device_path === prev && !isDiskPermanentlyBlocked(d))
          ? prev
          : firstSafe?.device_path ?? null,
      );
    } catch (err) {
      if (signal?.aborted) return;
      setScanError(err instanceof Error ? err.message : String(err));
    } finally {
      if (!signal?.aborted) setScanningDisks(false);
    }
  }, []);

  useEffect(() => {
    const controller = new AbortController();
    void Promise.resolve().then(() => {
      if (!controller.signal.aborted) {
        void loadDisks(controller.signal);
      }
    });
    return () => controller.abort();
  }, [loadDisks]);

  const selectedDisk = useMemo(
    () => disks.find(d => d.device_path === selectedDevicePath && !isDiskPermanentlyBlocked(d)) ?? null,
    [disks, selectedDevicePath],
  );

  const selectSafeDisk = useCallback((disk: SafeTargetDiskInfo) => {
    if (!isDiskPermanentlyBlocked(disk)) {
      setSelectedDevicePath(disk.device_path);
      setConfirmTokenInput('');
    }
  }, []);

  const capacityValid = isValidEh55CapacityGb(targetGb, allowExtended4tb);
  const maxAllowedGb = allowExtended4tb ? HDD_MAX_EXTENDED_4TB_GB : HDD_MAX_CAPACITY_GB;
  const previewBlocks = previewBlocks2Kb(targetGb);
  const expectedConfirmToken = selectedDisk ? getExpectedDriveConfirmToken(selectedDisk.device_path) : '';

  const subscribeFlash = useCallback(
    (onProgress: Parameters<NonNullable<Parameters<typeof usePipelineRunner>[0]['subscribeProgress']>>[0]) =>
      subscribeHddFlashProgress(ev =>
        onProgress(
          { current: ev.bytesDone, total: Math.max(1, ev.bytesTotal), stageLabel: `${ev.stage}: ${ev.detail}` },
          `[${ev.stage}] ${ev.detail}`,
        ),
      ),
    [],
  );

  const flashRunner = usePipelineRunner<void, Eh55HddFlashReport>({
    subscribeProgress: subscribeFlash,
    validateFn: () => {
      if (!capacityValid) return translate('pipelines.hddFlash.invalidCapacity', { max: maxAllowedGb });
      if (!selectedDisk) return translate('pipelines.hddFlash.inputs.selectTargetPrompt');
      if (!backupDir.trim()) return translate('pipelines.hddFlash.inputs.backupDirRequired');
      return null;
    },
    executeFn: async () => {
      const res = await flashEh55HddSafe({
        devicePath: selectedDisk?.device_path ?? '',
        confirmDiskModelToken: confirmTokenInput.trim(),
        imagePath: imagePath.trim() || undefined,
        backupDir: backupDir.trim(),
        expectedMinGb:
          targetGb === HDD_BENCH_5GB_CAPACITY
            ? HDD_BENCH_5GB_CAPACITY
            : Math.min(HDD_MIN_REGULAR_MEIHDFS_GB, targetGb),
        expectedMaxGb: Math.max(targetGb, DEFAULT_EH55_EXPECTED_MAX_GB),
        allowExtended4tb,
      });
      showToast({ type: 'success', title: translate('errors.pipelines.hddFlashed') });
      return res;
    },
    formatSuccess: res => res.device_path,
  });

  const preflightChecks = useMemo<readonly PreflightCheckItem[]>(
    () => [
      {
        id: 'backup64k',
        title: translate('pipelines.hddFlash.preflight.backup64k'),
        detail: translate('pipelines.hddFlash.preflight.backup64kDetail'),
        severity: 'critical',
        lockedReason: !backupDir.trim() ? translate('pipelines.hddFlash.inputs.backupDirRequired') : undefined,
      },
      {
        id: 'wipeLayout',
        title: translate('pipelines.hddFlash.preflight.wipeLayout'),
        detail: translate('pipelines.hddFlash.preflight.wipeLayoutDetail'),
        severity: 'critical',
      },
      {
        id: 'capacityRange',
        title: translate('pipelines.hddFlash.preflight.capacityRange'),
        detail: translate('pipelines.hddFlash.preflight.capacityRangeDetail'),
        severity: 'warning',
      },
    ],
    [backupDir],
  );

  const preflightReady =
    capacityValid &&
    Boolean(selectedDisk) &&
    Boolean(backupDir.trim()) &&
    preflightChecks.every(c => checkedPreflightIds.includes(c.id));

  const steps = useMemo<readonly PipelineStepDescriptor[]>(
    () => [
      { id: 'capacityPlan', label: translate('pipelines.hddFlash.steps.capacityPlan'), status: capacityValid ? 'verified' : 'failed' },
      { id: 'targetLockout', label: translate('pipelines.hddFlash.steps.targetLockout'), status: selectedDisk ? 'verified' : 'blocked' },
      { id: 'preflightBackup', label: translate('pipelines.hddFlash.steps.preflightBackup'), status: preflightReady ? 'verified' : 'pending' },
      {
        id: 'flashStream',
        label: translate('pipelines.hddFlash.steps.flashStream'),
        status: flashRunner.isBusy ? 'running' : flashRunner.error ? 'failed' : flashRunner.data ? 'verified' : preflightReady ? 'pending' : 'blocked',
      },
    ],
    [capacityValid, flashRunner.data, flashRunner.error, flashRunner.isBusy, preflightReady, selectedDisk],
  );

  return {
    targetGb,
    setTargetGb,
    allowExtended4tb,
    setAllowExtended4tb,
    capacityValid,
    maxAllowedGb,
    previewBlocks,
    disks,
    scanningDisks,
    scanError,
    scanSafeDisks: () => loadDisks(),
    selectedDisk,
    selectSafeDisk,
    imagePath,
    setImagePath,
    backupDir,
    setBackupDir,
    checkedPreflightIds,
    togglePreflight,
    preflightChecks,
    preflightReady,
    confirmTokenInput,
    setConfirmTokenInput,
    expectedConfirmToken,
    flashReport: flashRunner.data,
    flashBusy: flashRunner.isBusy,
    flashError: flashRunner.error,
    flashSuccess: flashRunner.successMessage,
    flashProgress: flashRunner.progress,
    flashLogs: flashRunner.logs,
    runSafeFlash: () => flashRunner.execute(),
    resetFlash: () => {
      flashRunner.reset();
      setConfirmTokenInput('');
    },
    steps,
    copiedHash,
    copyDigest,
  };
}
