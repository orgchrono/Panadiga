import { useState, useEffect, useCallback, useSyncExternalStore } from 'react';
import { NvramCleanReport, FlashSectorInfo } from '@/models';
import { emitLocalEvent, invokeCommand } from '@/services/ipc';
import { useToast } from '@/context/ToastContext';
import {
  clearLiveNvramCapture,
  getLatestLiveNvramCapture,
  subscribeLiveNvramCapture,
} from '@/services/liveNvramCaptureStore';
import { logger } from '@/utils/logger';
import { buildSectorMapFromReport, captureLivePicoNvramDump } from './nvram/nvramHelpers';

export { buildSectorMapFromReport };

export function useNvramViewModel() {
  const { showToast } = useToast();
  const liveCapture = useSyncExternalStore(
    subscribeLiveNvramCapture,
    getLatestLiveNvramCapture
  );
  const [report, setReport] = useState<NvramCleanReport | null>(() => liveCapture?.report ?? null);
  const [sectorMap, setSectorMap] = useState<FlashSectorInfo[]>(() =>
    liveCapture?.report ? buildSectorMapFromReport(liveCapture.report) : []
  );
  const [selectedSector, setSelectedSector] = useState<FlashSectorInfo | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [capturingLive, setCapturingLive] = useState<boolean>(false);
  const [cleaning, setCleaning] = useState<boolean>(false);
  const [cleanSuccess, setCleanSuccess] = useState<boolean>(false);
  const [filterType, setFilterType] = useState<string>('all');
  const [error, setError] = useState<string | null>(null);

  const applyReportState = useCallback((nextReport: NvramCleanReport) => {
    setReport(nextReport);
    const dynamicSectors = buildSectorMapFromReport(nextReport);
    setSectorMap(dynamicSectors);
    const isCleaned = nextReport.nvram_non_ff_bytes === 0;
    setCleanSuccess(isCleaned);
    emitLocalEvent('nvram://cleaned', isCleaned);

    const firstCorrupted = dynamicSectors.find(s => s.is_corrupted_area);
    const firstMicro = dynamicSectors.find(s => s.sector_type === 'nvram_microsector');
    setSelectedSector(firstCorrupted || firstMicro || dynamicSectors[0] || null);
  }, []);

  useEffect(() => {
    if (liveCapture?.report) {
      applyReportState(liveCapture.report);
      setError(null);
    }
  }, [liveCapture, applyReportState]);

  const loadAnalysis = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await invokeCommand<NvramCleanReport>('analyze_flash_dump');
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
    loadAnalysis();
  }, [loadAnalysis]);

  const captureFromLivePico = useCallback(
    async (explicitPort?: string, baseAddr = 0x807f0000, wordCount = 16384) => {
      setCapturingLive(true);
      setError(null);
      try {
        const captured = await captureLivePicoNvramDump(explicitPort, baseAddr, wordCount);
        showToast({
          type: 'success',
          title: 'Captura NVRAM ao Vivo Concluída',
          message: `${captured.captured_bytes.toLocaleString()} bytes capturados em ${captured.base_bus_address}`,
        });
        return captured;
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        setError(msg);
        showToast({
          type: 'error',
          title: 'Falha na Captura NVRAM via Pico',
          message: msg,
        });
        logger.error('NvramViewModel', 'Failed live Pico NVRAM capture', err);
        return null;
      } finally {
        setCapturingLive(false);
      }
    },
    [showToast]
  );

  const clearLiveCaptureAndReload = useCallback(async () => {
    clearLiveNvramCapture();
    await loadAnalysis();
  }, [loadAnalysis]);

  const [patching, setPatching] = useState(false);
  const [patchSuccess, setPatchSuccess] = useState(false);
  const [patchError, setPatchError] = useState<string | null>(null);

  const applyFlashReport = useCallback((data: NvramCleanReport) => {
    setReport(data);
    const updatedSectors = buildSectorMapFromReport(data);
    setSectorMap(updatedSectors);
    const firstMicro = updatedSectors.find(s => s.sector_type === 'nvram_microsector');
    if (firstMicro) setSelectedSector(firstMicro);
  }, []);

  const executeSurgicalClean = useCallback(async () => {
    setCleaning(true);
    setError(null);
    try {
      const data = await invokeCommand<NvramCleanReport>('clean_nvram');
      applyFlashReport(data);
      setCleanSuccess(true);
      emitLocalEvent('nvram://cleaned', true);
      showToast({
        type: 'success',
        title: 'NVRAM U99 Sanitizada (0xFF)',
        message: 'Microsetores SA134..SA141 limpos preservando IPL/Bootloader.',
      });
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
      showToast({
        type: 'error',
        title: 'Erro na Limpeza Cirúrgica NVRAM',
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
        setPatchError('Informe dump de entrada, saída Copy-on-Write e payload hexadecimal.');
        return null;
      }
      if (inputPath === outputPath) {
        setPatchError('Copy-on-Write obrigatório: o arquivo de saída não pode sobrescrever a origem.');
        return null;
      }
      setPatching(true);
      setPatchSuccess(false);
      setPatchError(null);
      try {
        const data = await invokeCommand<NvramCleanReport>('patch_nvram_command', {
          inputPath,
          outputPath,
          nvramOffset,
          patchHex,
        });
        applyFlashReport(data);
        setPatchSuccess(true);
        showToast({
          type: 'success',
          title: 'Patch Copy-on-Write Aplicado',
          message: `Salvo em ${data.output_file || outputPath}`,
        });
        return data;
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        setPatchError(msg);
        showToast({
          type: 'error',
          title: 'Patch NVRAM Bloqueado / Falhou',
          message: msg,
        });
        logger.error('NvramViewModel', 'Failed guarded NVRAM CoW patch', err);
        return null;
      } finally {
        setPatching(false);
      }
    },
    [applyFlashReport, showToast]
  );

  const filteredSectors = sectorMap.filter(s => {
    if (filterType === 'nvram') return s.sector_type === 'nvram_microsector';
    if (filterType === 'corrupted') return s.is_corrupted_area;
    if (filterType === 'bootloader') return s.sector_type === 'bootloader';
    if (filterType === 'os') return s.sector_type === 'os_firmware';
    return true;
  });

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
