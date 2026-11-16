import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { MainCapacityInspectionReport, MainCapacityPatchReport } from '@/models';
import { emitLocalEvent, invokeCommand } from '@/services/ipc';
import { useToast } from '@/context/ToastContext';
import {
  computePreviewBlocks,
  resolveEffectiveTargetGb,
  type CapacityPassthroughMode,
  type PatchMode,
} from '@/components/hdd/capacity/capacityHelpers';

export function useHddCapacityViewModel() {
  const { showToast } = useToast();
  const mountedRef = useRef(true);
  const [inputPath, setInputPath] = useState<string>('');
  const [outputPath, setOutputPath] = useState<string>('');
  const [mode, setMode] = useState<PatchMode>('dynamic');
  const [targetGb, setTargetGb] = useState<number>(250);
  const [limitInDynamicMode, setLimitInDynamicMode] = useState<boolean>(false);
  const [passthroughMode, setPassthroughMode] = useState<CapacityPassthroughMode>('unchanged');
  const [inspection, setInspection] = useState<MainCapacityInspectionReport | null>(null);
  const [patchReport, setPatchReport] = useState<MainCapacityPatchReport | null>(null);
  const [inspecting, setInspecting] = useState<boolean>(false);
  const [patching, setPatching] = useState<boolean>(false);
  const [inspectError, setInspectError] = useState<string | null>(null);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const runInspect = useCallback(
    async (customInput?: string) => {
      setInspecting(true);
      setInspectError(null);
      const pathVal = (customInput !== undefined ? customInput : inputPath).trim();
      try {
        const res = await invokeCommand<MainCapacityInspectionReport>(
          'inspect_hdd_capacity_patch',
          pathVal ? { inputPath: pathVal } : {}
        );
        if (!mountedRef.current) return;
        setInspection(res);
        emitLocalEvent('hdd://inspected', true);
        if (!pathVal && res.input_path) {
          setInputPath(res.input_path);
        }
      } catch (err) {
        if (!mountedRef.current) return;
        setInspectError(err instanceof Error ? err.message : String(err));
      } finally {
        if (mountedRef.current) {
          setInspecting(false);
        }
      }
    },
    [inputPath]
  );

  useEffect(() => {
    void runInspect('');
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const previewBlocks = useMemo(
    () => computePreviewBlocks(mode, targetGb, limitInDynamicMode),
    [mode, targetGb, limitInDynamicMode]
  );

  const handleApplyPatch = useCallback(async () => {
    setPatching(true);
    try {
      const effectiveTargetGb = resolveEffectiveTargetGb(mode, targetGb, limitInDynamicMode);
      const effectivePt = passthroughMode === 'unchanged' ? undefined : passthroughMode;
      const res = await invokeCommand<MainCapacityPatchReport>('apply_hdd_capacity_patch', {
        inputPath: inputPath.trim() || undefined,
        outputPath: outputPath.trim() || undefined,
        mode,
        targetGb: effectiveTargetGb ?? undefined,
        passthroughMode: effectivePt,
      });
      if (!mountedRef.current) return;
      setPatchReport(res);
      setInspection(res.inspection_after);
      emitLocalEvent('hdd://inspected', true);
      showToast({
        type: 'success',
        title: 'Patch de Capacidade Aplicado!',
        message: `Salvo em ${res.output_path} (${res.applied_blocks_2kb_hex})`,
      });
    } catch (err) {
      if (!mountedRef.current) return;
      showToast({
        type: 'error',
        title: 'Falha ao Aplicar Patch de Capacidade',
        message: err instanceof Error ? err.message : String(err),
      });
    } finally {
      if (mountedRef.current) {
        setPatching(false);
      }
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
    inspecting,
    patching,
    inspectError,
    previewBlocks,
    runInspect,
    handleApplyPatch,
  };
}
