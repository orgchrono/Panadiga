import { useCallback, useEffect, useState } from 'react';
import type {
  FrmModuleExtractionReport,
  MainPassthroughReport,
  MainPassthroughRspPlan,
  NorDumperStubResult,
  SdDiskInspectionReport,
  SdMediaVerificationReport,
  SdUnbrickKitReport,
  SdUnbrickKitRequestDto,
  UnbrickWorkspaceReport,
} from '@/models';
import { emitLocalEvent, invokeCommand } from '@/services/ipc';
import { useToast } from '@/context/ToastContext';
import { logger } from '@/utils/logger';
import {
  emptyChannel,
  useMatshitaEmuActions,
  type AsyncChannelState,
} from './useMatshitaEmuActions';

function formatErr(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

export function useUnbrickViewModel() {
  const { showToast } = useToast();

  const [workbench, setWorkbench] = useState<AsyncChannelState<UnbrickWorkspaceReport>>(() =>
    emptyChannel()
  );
  const [sdKit, setSdKit] = useState<AsyncChannelState<SdUnbrickKitReport>>(() => emptyChannel());
  const [verify, setVerify] = useState<AsyncChannelState<SdMediaVerificationReport>>(() =>
    emptyChannel()
  );
  const [sdDisk, setSdDisk] = useState<AsyncChannelState<SdDiskInspectionReport>>(() =>
    emptyChannel()
  );
  const [extract, setExtract] = useState<AsyncChannelState<FrmModuleExtractionReport>>(() =>
    emptyChannel()
  );
  const [tbc, setTbc] = useState<AsyncChannelState<MainPassthroughReport>>(() => emptyChannel());
  const [rspPlan, setRspPlan] = useState<AsyncChannelState<MainPassthroughRspPlan>>(() =>
    emptyChannel()
  );
  const [norStub, setNorStub] = useState<AsyncChannelState<NorDumperStubResult>>(() =>
    emptyChannel()
  );

  const runChannel = useCallback(
    async <T,>(
      setter: React.Dispatch<React.SetStateAction<AsyncChannelState<T>>>,
      tag: string,
      fn: () => Promise<T>,
      toastTitle?: string
    ): Promise<T | null> => {
      setter(prev => ({ ...prev, loading: true, error: null }));
      try {
        const result = await fn();
        setter({ loading: false, data: result, error: null });
        if (toastTitle) {
          showToast({ type: 'success', title: toastTitle });
        }
        return result;
      } catch (err) {
        const msg = formatErr(err);
        logger.error('UnbrickViewModel', `${tag} failed`, err);
        setter(prev => ({ ...prev, loading: false, error: msg }));
        showToast({ type: 'error', title: tag, message: msg });
        return null;
      }
    },
    [showToast]
  );

  const loadWorkbench = useCallback(
    async (workspacePath?: string): Promise<UnbrickWorkspaceReport | null> => {
      const trimmed = workspacePath?.trim();
      const args = trimmed ? { workspacePath: trimmed, workspace_path: trimmed } : undefined;
      const report = await runChannel(setWorkbench, 'Unbrick Workbench', () =>
        invokeCommand<UnbrickWorkspaceReport>('inspect_unbrick_workbench', args)
      );
      if (report) {
        setTbc(prev => ({ ...prev, data: report.main_passthrough }));
        setRspPlan(prev => ({ ...prev, data: report.rsp_plan }));
        if (report.sd_verification) {
          setVerify(prev => ({ ...prev, data: report.sd_verification }));
        }
      }
      return report;
    },
    [runChannel]
  );

  useEffect(() => {
    void loadWorkbench();
  }, [loadWorkbench]);

  const buildSdKit = useCallback(
    async (request: SdUnbrickKitRequestDto): Promise<SdUnbrickKitReport | null> => {
      const report = await runChannel(
        setSdKit,
        'SD Recovery Kit',
        () => invokeCommand<SdUnbrickKitReport>('prepare_sd_unbrick_kit_command', { request }),
        `Kit SD gravado em ${request.output_dir}`
      );
      if (report) {
        setVerify({ loading: false, data: report.verification, error: null });
        emitLocalEvent('unbrick://completed', true);
      }
      return report;
    },
    [runChannel]
  );

  const verifySdMedia = useCallback(
    async (sdRootDir: string): Promise<SdMediaVerificationReport | null> => {
      const clean = sdRootDir.trim();
      const report = await runChannel(
        setVerify,
        'Verificar Mídia SD',
        () =>
          invokeCommand<SdMediaVerificationReport>('verify_sd_unbrick_media_command', {
            sdRootDir: clean,
            sd_root_dir: clean,
          }),
        `Mídia SD validada: ${clean}`
      );
      if (report) {
        emitLocalEvent('unbrick://completed', true);
      }
      return report;
    },
    [runChannel]
  );

  const inspectSdDisk = useCallback(
    async (target: string): Promise<SdDiskInspectionReport | null> => {
      const clean = target.trim();
      const report = await runChannel(
        setSdDisk,
        'Auditoria FAT16 / BPB SD',
        () =>
          invokeCommand<SdDiskInspectionReport>('inspect_sd_disk_command', {
            target: clean,
          }),
        `Disco/imagem SD auditado: ${clean}`
      );
      return report;
    },
    [runChannel]
  );

  const extractFrmModules = useCallback(
    async (
      frmPath: string,
      outDir: string,
      unpackPayloads = true
    ): Promise<FrmModuleExtractionReport | null> => {
      return runChannel(
        setExtract,
        'Extração de Módulos .FRM',
        () =>
          invokeCommand<FrmModuleExtractionReport>('extract_frm_modules_command', {
            frmPath: frmPath.trim(),
            frm_path: frmPath.trim(),
            outDir: outDir.trim(),
            out_dir: outDir.trim(),
            unpackPayloads,
            unpack_payloads: unpackPayloads,
          }),
        `Módulos extraídos para ${outDir.trim()}`
      );
    },
    [runChannel]
  );

  const runMainPassthrough = useCallback(
    async (params: {
      action: 'inspect' | 'patch';
      inputPath: string;
      outPath?: string;
      mode?: 'av_passthrough' | 'factory_original';
    }): Promise<MainPassthroughReport | null> => {
      const mode = params.mode ?? 'av_passthrough';
      return runChannel(
        setTbc,
        'TBC Passthrough MAIN',
        () =>
          invokeCommand<MainPassthroughReport>('patch_main_passthrough_command', {
            action: params.action,
            inputPath: params.inputPath.trim(),
            input_path: params.inputPath.trim(),
            outPath: params.outPath?.trim() || undefined,
            out_path: params.outPath?.trim() || undefined,
            mode,
          }),
        params.action === 'patch'
          ? `MAIN TBC (${mode}) gravado com sucesso`
          : 'Sites TBC inspecionados em MAIN'
      );
    },
    [runChannel]
  );

  const generateRspPlan = useCallback(
    async (callbackTargetVa?: string): Promise<MainPassthroughRspPlan | null> => {
      const cb = callbackTargetVa?.trim() || undefined;
      return runChannel(
        setRspPlan,
        'Plano RSP _dev_SERIAL',
        () =>
          invokeCommand<MainPassthroughRspPlan>('patch_main_passthrough_command', {
            action: 'rsp_plan',
            callbackTargetVa: cb,
            callback_target_va: cb,
          }),
        'Frames RSP _dev_SERIAL atualizados'
      );
    },
    [runChannel]
  );

  const buildNorStub = useCallback(
    async (params: {
      startAddr?: string;
      totalLen?: string;
      blockSize?: number;
      loadAddr?: string;
      outPath?: string;
      wrapAsDigadiag?: boolean;
    }): Promise<NorDumperStubResult | null> => {
      const wrap = Boolean(params.wrapAsDigadiag);
      return runChannel(
        setNorStub,
        'NOR Dumper Stub MN103S',
        () =>
          invokeCommand<NorDumperStubResult>('build_nor_dumper_stub_command', {
            startAddr: params.startAddr?.trim() || '0x84000000',
            totalLen: params.totalLen?.trim() || '0x800000',
            blockSize: params.blockSize ?? 4096,
            loadAddr: params.loadAddr?.trim() || (wrap ? '0x50400000' : '0x51800000'),
            outPath: params.outPath?.trim() || undefined,
            wrapAsDigadiag: wrap,
            wrap_as_digadiag: wrap,
          }),
        'Stub MN103S NOR Dumper gerado'
      );
    },
    [runChannel]
  );

  const matshita = useMatshitaEmuActions(runChannel);

  return {
    workbench,
    sdKit,
    verify,
    sdDisk,
    extract,
    tbc,
    rspPlan,
    norStub,
    ...matshita,
    loadWorkbench,
    buildSdKit,
    verifySdMedia,
    inspectSdDisk,
    extractFrmModules,
    runMainPassthrough,
    generateRspPlan,
    buildNorStub,
  };
}
