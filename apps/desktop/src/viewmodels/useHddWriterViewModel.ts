import { useCallback, useEffect, useState } from 'react';
import {
  DECIMAL_GB_BYTES,
  HDD_DEFAULT_250GB_SECTORS,
  HDD_MIN_CAPACITY_GB,
  LBA_SECTOR_SIZE_BYTES,
} from '@/constants';
import { useToast } from '@/context/ToastContext';
import { translate } from '@/i18n/useTranslation';
import type { DriveWriteReport, PhysicalDriveInfo } from '@/models';
import { executeDiskWriteCommand, listPhysicalDrives } from '@/services/storageCryptoService';

type HddWriteCommand = 'format_hdd_meihdfs' | 'write_hdd_image' | 'restore_hdd_mirrors';

type HddErrorKey =
  | 'errors.hdd.formatTokenSuffix'
  | 'errors.hdd.formatSuccess'
  | 'errors.hdd.formatError'
  | 'errors.hdd.writeTokenSuffix'
  | 'errors.hdd.writeSuccess'
  | 'errors.hdd.writeError'
  | 'errors.hdd.mirrorTokenSuffix'
  | 'errors.hdd.mirrorSuccess'
  | 'errors.hdd.mirrorError';

interface HddOperationDescriptor {
  readonly cmd: HddWriteCommand;
  readonly invalidTokenSuffixKey: HddErrorKey;
  readonly successTitleKey: HddErrorKey;
  readonly errorTitleKey: HddErrorKey;
  readonly buildArgs?: (
    drive: PhysicalDriveInfo,
    customSizeGb: number | null,
    rawImagePath: string,
  ) => Record<string, unknown>;
}

function resolveTargetSectors(
  selectedDrive: PhysicalDriveInfo | null,
  customSizeGb: number | null,
): number {
  if (customSizeGb !== null && customSizeGb >= HDD_MIN_CAPACITY_GB) {
    return Math.floor((customSizeGb * DECIMAL_GB_BYTES) / LBA_SECTOR_SIZE_BYTES);
  }
  if (selectedDrive && selectedDrive.size_bytes > 0) {
    return Math.floor(selectedDrive.size_bytes / LBA_SECTOR_SIZE_BYTES);
  }
  return HDD_DEFAULT_250GB_SECTORS;
}

const HDD_OPERATION_DESCRIPTORS: Readonly<Record<HddWriteCommand, HddOperationDescriptor>> =
  Object.freeze({
    format_hdd_meihdfs: {
      cmd: 'format_hdd_meihdfs',
      invalidTokenSuffixKey: 'errors.hdd.formatTokenSuffix',
      successTitleKey: 'errors.hdd.formatSuccess',
      errorTitleKey: 'errors.hdd.formatError',
      buildArgs: (drive, customSizeGb) => ({
        totalSectors: resolveTargetSectors(drive, customSizeGb),
      }),
    },
    write_hdd_image: {
      cmd: 'write_hdd_image',
      invalidTokenSuffixKey: 'errors.hdd.writeTokenSuffix',
      successTitleKey: 'errors.hdd.writeSuccess',
      errorTitleKey: 'errors.hdd.writeError',
      buildArgs: (_drive, _customSizeGb, rawImagePath) => ({
        imagePath: rawImagePath.trim() || undefined,
      }),
    },
    restore_hdd_mirrors: {
      cmd: 'restore_hdd_mirrors',
      invalidTokenSuffixKey: 'errors.hdd.mirrorTokenSuffix',
      successTitleKey: 'errors.hdd.mirrorSuccess',
      errorTitleKey: 'errors.hdd.mirrorError',
    },
  });

/**
 * Pure helper computing the required confirmation token for a physical disk target.
 */
export function getExpectedDriveConfirmToken(devicePath: string): string {
  return `CONFIRM-${devicePath.replace(/^\\\\.\\/, '')}`;
}

/**
 * Single Responsibility ViewModel: manages physical ATA/IDE disk enumeration and guarded
 * MEIHDFS write operations (format, raw image write, mirror recovery) via domain services.
 */
export function useHddWriterViewModel() {
  const { showToast } = useToast();
  const [drives, setDrives] = useState<PhysicalDriveInfo[]>([]);
  const [selectedDrive, setSelectedDrive] = useState<PhysicalDriveInfo | null>(null);
  const [loadingDrives, setLoadingDrives] = useState<boolean>(false);
  const [confirmInput, setConfirmInput] = useState<string>('');
  const [customSizeGb, setCustomSizeGb] = useState<number | null>(null);
  const [rawImagePath, setRawImagePath] = useState<string>('');
  const [executing, setExecuting] = useState<boolean>(false);
  const [writeReport, setWriteReport] = useState<DriveWriteReport | null>(null);

  const performFetchDrives = useCallback(
    async (signal?: AbortSignal) => {
      setLoadingDrives(true);
      try {
        const list = await listPhysicalDrives();
        if (signal?.aborted) return;
        setDrives(list);
        const safe = list.find((drive) => drive.safe_to_write);
        if (safe) {
          setSelectedDrive(safe);
        }
      } catch (err) {
        if (signal?.aborted) return;
        showToast({
          type: 'error',
          title: translate('errors.hdd.listDisksError'),
          message: err instanceof Error ? err.message : String(err),
        });
      } finally {
        if (!signal?.aborted) {
          setLoadingDrives(false);
        }
      }
    },
    [showToast],
  );

  const fetchDrives = useCallback(() => performFetchDrives(), [performFetchDrives]);

  useEffect(() => {
    const controller = new AbortController();
    void Promise.resolve().then(() => {
      if (!controller.signal.aborted) {
        void performFetchDrives(controller.signal);
      }
    });
    return () => controller.abort();
  }, [performFetchDrives]);

  const runConfirmedOperation = useCallback(
    async (command: HddWriteCommand) => {
      if (!selectedDrive) return;
      const descriptor = HDD_OPERATION_DESCRIPTORS[command];
      const expected = getExpectedDriveConfirmToken(selectedDrive.device_path);
      const provided = confirmInput.trim();

      if (provided !== expected) {
        showToast({
          type: 'error',
          title: translate('errors.hdd.invalidConfirmToken'),
          message: translate('errors.hdd.tokenPromptMsg', {
            token: expected,
            suffix: translate(descriptor.invalidTokenSuffixKey),
          }),
        });
        return;
      }

      setExecuting(true);
      try {
        const res = await executeDiskWriteCommand(descriptor.cmd, {
          devicePath: selectedDrive.device_path,
          confirmationToken: provided,
          ...(descriptor.buildArgs
            ? descriptor.buildArgs(selectedDrive, customSizeGb, rawImagePath)
            : {}),
        });
        setWriteReport(res);
        showToast({
          type: 'success',
          title: translate(descriptor.successTitleKey),
          message: res.message,
        });
      } catch (err) {
        showToast({
          type: 'error',
          title: translate(descriptor.errorTitleKey),
          message: err instanceof Error ? err.message : String(err),
        });
      } finally {
        setExecuting(false);
      }
    },
    [selectedDrive, confirmInput, customSizeGb, rawImagePath, showToast],
  );

  const handleProceduralFormat = useCallback(
    () => runConfirmedOperation('format_hdd_meihdfs'),
    [runConfirmedOperation],
  );

  const handleWriteRawImage = useCallback(
    () => runConfirmedOperation('write_hdd_image'),
    [runConfirmedOperation],
  );

  const handleRestoreMirrors = useCallback(
    () => runConfirmedOperation('restore_hdd_mirrors'),
    [runConfirmedOperation],
  );

  const expectedToken = selectedDrive ? getExpectedDriveConfirmToken(selectedDrive.device_path) : '';
  const isTokenValid = Boolean(
    selectedDrive && selectedDrive.safe_to_write && confirmInput.trim() === expectedToken,
  );
  const resolvedTargetSectors = resolveTargetSectors(selectedDrive, customSizeGb);

  return {
    drives,
    selectedDrive,
    setSelectedDrive,
    isLoading: loadingDrives || executing,
    loadingDrives,
    confirmInput,
    setConfirmInput,
    customSizeGb,
    setCustomSizeGb,
    rawImagePath,
    setRawImagePath,
    resolvedTargetSectors,
    executing,
    writeReport,
    expectedToken,
    isTokenValid,
    fetchDrives,
    handleProceduralFormat,
    handleWriteRawImage,
    handleRestoreMirrors,
  };
}
