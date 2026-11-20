import { useCallback, useEffect, useMemo, useState } from 'react';
import { AM34_NVRAM_BASE_ADDR, AM34_NVRAM_WORD_COUNT_16BIT } from '@/constants';
import { useToast } from '@/context/ToastContext';
import { translate } from '@/i18n/useTranslation';
import type { FlashSectorInfo, NvramCleanReport } from '@/models';
import {
  analyzeFlashDump,
  cleanNvramSurgical,
  patchNvramOffset,
} from '@/services/storageCryptoService';
import { emitNvramCleanedEvent } from '@/services/systemService';
import { useAppSelector } from '@/store/hooks';
import { clearLiveNvramCapture, getLatestLiveNvramCapture } from '@/store/nvramCaptureBridge';
import { logger } from '@/utils/logger';
import { buildSectorMapFromReport, captureLivePicoNvramDump } from './nvram/nvramHelpers';

export { buildSectorMapFromReport };

const SECTOR_FILTER_PREDICATES: Readonly<Record<string, (s: FlashSectorInfo) => boolean>> =
  Object.freeze({
    nvram: (sector) => sector.sector_type === 'nvram_microsector',
    bgo: (sector) => sector.sector_type === 'bgo_region',
    corrupted: (sector) => sector.is_corrupted_area,
    bootloader: (sector) => sector.sector_type === 'bootloader',
    os: (sector) => sector.sector_type === 'os_firmware',
  });

export function useNvramViewModel() {
  const { showToast } = useToast();
  const liveCapture = useAppSelector((state) => state.nvramCapture.latestCapture);
  const [report, setReport] = useState<NvramCleanReport | null>(() => liveCapture?.report ?? null);
  const [sectorMap, setSectorMap] = useState<FlashSectorInfo[]>(() =>
    liveCapture?.report ? buildSectorMapFromReport(liveCapture.report) : [],
  );
  const [selectedSector, setSelectedSector] = useState<FlashSectorInfo | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [capturingLive, setCapturingLive] = useState<boolean>(false);
  const [cleaning, setCleaning] = useState<boolean>(false);
  const [cleanSuccess, setCleanSuccess] = useState<boolean>(false);
  const [filterType, setFilterType] = useState<string>('all');
  const [error, setError] = useState<string | null>(null);
  const [patching, setPatching] = useState<boolean>(false);
  const [patchSuccess, setPatchSuccess] = useState<boolean>(false);
  const [patchError, setPatchError] = useState<string | null>(null);

  const applyReportState = useCallback((nextReport: NvramCleanReport) => {
    setReport(nextReport);
    const dynamicSectors = buildSectorMapFromReport(nextReport);
    setSectorMap(dynamicSectors);
    const isCleaned = nextReport.nvram_non_ff_bytes === 0;
    setCleanSuccess(isCleaned);
    emitNvramCleanedEvent(isCleaned);

    const firstCorrupted = dynamicSectors.find((sector) => sector.is_corrupted_area);
    const firstMicro = dynamicSectors.find((sector) => sector.sector_type === 'nvram_microsector');
    setSelectedSector(firstCorrupted ?? firstMicro ?? dynamicSectors[0] ?? null);
  }, []);

  useEffect(() => {
    const captureReport = liveCapture?.report;
    if (!captureReport) return undefined;
    const controller = new AbortController();
    void Promise.resolve().then(() => {
      if (!controller.signal.aborted) {
        applyReportState(captureReport);
        setError(null);
      }
    });
    return () => controller.abort();
  }, [liveCapture, applyReportState]);

  const loadAnalysis = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await analyzeFlashDump();
      if (!getLatestLiveNvramCapture()) {
        applyReportState(data);
      }
    } catch (err) {
      if (!getLatestLiveNvramCapture()) {
        const msg = err instanceof Error ? err.message : String(err);
        setError(msg);
        logger.error('NvramViewModel', 'Failed analyzing flash dump', err);
      }
    } finally {
      setLoading(false);
    }
  }, [applyReportState]);

  useEffect(() => {
    const controller = new AbortController();
    void Promise.resolve().then(() => {
      if (!controller.signal.aborted) {
        void loadAnalysis();
      }
    });
    return () => controller.abort();
  }, [loadAnalysis]);

  const captureFromLivePico = useCallback(
    async (
      explicitPort?: string,
      baseAddr = AM34_NVRAM_BASE_ADDR,
      wordCount = AM34_NVRAM_WORD_COUNT_16BIT,
    ) => {
      setCapturingLive(true);
      setError(null);
      try {
        const captured = await captureLivePicoNvramDump(explicitPort, baseAddr, wordCount);
        showToast({
          type: 'success',
          title: translate('errors.nvram.liveCaptureTitle'),
          message: translate('errors.nvram.liveCaptureBytesMsg', {
            bytes: captured.captured_bytes.toLocaleString(),
            addr: captured.base_bus_address,
          }),
        });
        return captured;
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        setError(msg);
        showToast({
          type: 'error',
          title: translate('errors.nvram.liveCaptureFailedTitle'),
          message: msg,
        });
        logger.error('NvramViewModel', 'Failed live Pico NVRAM capture', err);
        return null;
      } finally {
        setCapturingLive(false);
      }
    },
    [showToast],
  );

  const clearLiveCaptureAndReload = useCallback(async () => {
    clearLiveNvramCapture();
    await loadAnalysis();
  }, [loadAnalysis]);

  const applyFlashReport = useCallback((data: NvramCleanReport) => {
    setReport(data);
    const updatedSectors = buildSectorMapFromReport(data);
    setSectorMap(updatedSectors);
    const firstMicro = updatedSectors.find((sector) => sector.sector_type === 'nvram_microsector');
    if (firstMicro) setSelectedSector(firstMicro);
  }, []);

  const executeSurgicalClean = useCallback(async () => {
    setCleaning(true);
    setError(null);
    try {
      const data = await cleanNvramSurgical();
      applyFlashReport(data);
      setCleanSuccess(true);
      emitNvramCleanedEvent(true);
      showToast({
        type: 'success',
        title: translate('errors.nvram.surgicalCleanBadgeTitle'),
        message: translate('errors.nvram.surgicalCleanMsg'),
      });
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
      showToast({
        type: 'error',
        title: translate('errors.nvram.surgicalCleanFailedTitle'),
        message: msg,
      });
      logger.error('NvramViewModel', 'Failed cleaning NVRAM', err);
    } finally {
      setCleaning(false);
    }
  }, [applyFlashReport, showToast]);

  const executeGuardedPatch = useCallback(
    async (params: {
      inputPath: string;
      outputPath: string;
      nvramOffset: string;
      patchHex: string;
    }): Promise<NvramCleanReport | null> => {
      const inputPath = params.inputPath.trim();
      const outputPath = params.outputPath.trim();
      const nvramOffset = params.nvramOffset.trim() || '0x0';
      const patchHex = params.patchHex.trim();
      if (!inputPath || !outputPath || !patchHex) {
        setPatchError(translate('errors.nvram.cowPatchRequiredFields'));
        return null;
      }
      if (inputPath === outputPath) {
        setPatchError(translate('errors.nvram.cowPatchSameFileBlocked'));
        return null;
      }
      setPatching(true);
      setPatchSuccess(false);
      setPatchError(null);
      try {
        const data = await patchNvramOffset({
          inputPath,
          outputPath,
          nvramOffset,
          patchHex,
        });
        applyFlashReport(data);
        setPatchSuccess(true);
        showToast({
          type: 'success',
          title: translate('errors.nvram.cowPatchTitle'),
          message: translate('errors.nvram.cowPatchSavedMsg', {
            path: data.output_file || outputPath,
          }),
        });
        return data;
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        setPatchError(msg);
        showToast({
          type: 'error',
          title: translate('errors.nvram.cowPatchFailedTitle'),
          message: msg,
        });
        logger.error('NvramViewModel', 'Failed guarded NVRAM CoW patch', err);
        return null;
      } finally {
        setPatching(false);
      }
    },
    [applyFlashReport, showToast],
  );

  const filteredSectors = useMemo(() => {
    const predicate = SECTOR_FILTER_PREDICATES[filterType];
    return predicate ? sectorMap.filter(predicate) : sectorMap;
  }, [filterType, sectorMap]);

  return {
    report,
    sectorMap,
    filteredSectors,
    selectedSector,
    setSelectedSector,
    loading,
    capturingLive,
    liveCapture,
    cleaning,
    cleanSuccess,
    patching,
    patchSuccess,
    patchError,
    filterType,
    setFilterType,
    error,
    executeSurgicalClean,
    executeGuardedPatch,
    captureFromLivePico,
    clearLiveCapture: clearLiveCaptureAndReload,
    refresh: loadAnalysis,
  };
}
