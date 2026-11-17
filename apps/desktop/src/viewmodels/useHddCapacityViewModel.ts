import { useCallback, useEffect, useMemo, useState } from 'react';
import { useToast } from '@/context/ToastContext';
import { translate } from '@/i18n/useTranslation';
import type { MainCapacityInspectionReport, MainCapacityPatchReport } from '@/models';
import { inspectHddCapacity, patchHddCapacity } from '@/services/storageCryptoService';
import { emitHddInspectedEvent } from '@/services/systemService';
import { getRuntimeSettingsSnapshot } from '@/store/runtimeSettingsBridge';
import {
  computePreviewBlocks,
  resolveEffectiveTargetGb,
  type CapacityPassthroughMode,
  type PatchMode,
} from '@/utils/hddCapacity';

export function useHddCapacityViewModel() {
  const { showToast } = useToast();
  const [inputPath, setInputPath] = useState<string>('');
  const [outputPath, setOutputPath] = useState<string>('');
  const [mode, setMode] = useState<PatchMode>('dynamic');
  const [targetGb, setTargetGb] = useState<number>(
    () => getRuntimeSettingsSnapshot().storage.hdd_default_capacity_gb,
  );
  const [limitInDynamicMode, setLimitInDynamicMode] = useState<boolean>(false);
  const [passthroughMode, setPassthroughMode] = useState<CapacityPassthroughMode>('unchanged');
  const [inspection, setInspection] = useState<MainCapacityInspectionReport | null>(null);
  const [patchReport, setPatchReport] = useState<MainCapacityPatchReport | null>(null);
  const [inspecting, setInspecting] = useState<boolean>(false);
  const [patching, setPatching] = useState<boolean>(false);
  const [inspectError, setInspectError] = useState<string | null>(null);

  const performInspect = useCallback(async (pathVal: string, signal?: AbortSignal) => {
    setInspecting(true);
    setInspectError(null);
    try {
      const res = await inspectHddCapacity(pathVal);
      if (signal?.aborted) return;
      setInspection(res);
      emitHddInspectedEvent(true);
      if (!pathVal && res.input_path) {
        setInputPath(res.input_path);
      }
    } catch (err) {
      if (signal?.aborted) return;
      setInspectError(err instanceof Error ? err.message : String(err));
    } finally {
      if (!signal?.aborted) {
        setInspecting(false);
      }
    }
  }, []);

  const runInspect = useCallback(
    async (customInput?: string) => {
      const pathVal = (customInput !== undefined ? customInput : inputPath).trim();
      await performInspect(pathVal);
    },
    [inputPath, performInspect],
  );

  useEffect(() => {
    const controller = new AbortController();
    void Promise.resolve().then(() => {
      if (!controller.signal.aborted) {
        void performInspect('', controller.signal);
      }
    });
    return () => controller.abort();
  }, [performInspect]);

  const previewBlocks = useMemo(
    () => computePreviewBlocks(mode, targetGb, limitInDynamicMode),
    [mode, targetGb, limitInDynamicMode],
  );

  const handleApplyPatch = useCallback(async () => {
    setPatching(true);
    try {
      const effectiveTargetGb = resolveEffectiveTargetGb(mode, targetGb, limitInDynamicMode);
      const effectivePt = passthroughMode === 'unchanged' ? undefined : passthroughMode;
      const res = await patchHddCapacity({
        inputPath: inputPath.trim() || undefined,
        outputPath: outputPath.trim() || undefined,
        mode,
        targetGb: effectiveTargetGb ?? undefined,
        passthroughMode: effectivePt,
      });
      setPatchReport(res);
      setInspection(res.inspection_after);
      emitHddInspectedEvent(true);
      showToast({
        type: 'success',
        title: translate('errors.hdd.capacitySuccess'),
        message: translate('errors.hdd.capacitySavedMsg', {
          path: res.output_path,
          hex: res.applied_blocks_2kb_hex,
        }),
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: translate('errors.hdd.capacityError'),
        message: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setPatching(false);
    }
  }, [inputPath, limitInDynamicMode, mode, outputPath, passthroughMode, showToast, targetGb]);

  return {
    inputPath,
    setInputPath,
    outputPath,
    setOutputPath,
    mode,
    setMode,
    targetGb,
    setTargetGb,
    limitInDynamicMode,
    setLimitInDynamicMode,
    passthroughMode,
    setPassthroughMode,
    inspection,
    patchReport,
    isLoading: inspecting || patching,
    inspecting,
    patching,
    inspectError,
    previewBlocks,
    runInspect,
    handleApplyPatch,
  };
}
