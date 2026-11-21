import { useCallback, useEffect, useMemo, useState } from 'react';
import { useToast } from '@/context/ToastContext';
import { translate } from '@/i18n/useTranslation';
import type {
  FrmModuleExtractionReport, MainPassthroughReport, MainPassthroughRspPlan,
  NorDumperStubResult, SdCardCreatorReport, SdDiskInspectionReport,
  SdMediaVerificationReport, SdUnbrickKitReport, SdUnbrickKitRequestDto,
  UnbrickWorkspaceReport,
} from '@/models';
import { emitUnbrickCompletedEvent } from '@/services/systemService';
import {
  buildNorDumperStub, extractFrmModules as extractFrmModulesRpc, flashSdRecoveryCard,
  generateMainRspPlan, inspectSdDisk as inspectSdDiskRpc, inspectUnbrickWorkbench,
  patchMainPassthrough, prepareSdUnbrickKit, verifySdUnbrickMedia,
} from '@/services/unbrickService';
import { formatErrorMessage } from '@/utils/formatters';
import { logger } from '@/utils/logger';
import { emptyChannel, useMatshitaEmuActions, type AsyncChannelState } from './useMatshitaEmuActions';
import { useStorageDrives } from './useStorageDrives';
import {
  buildFat16SdFlashPayload, buildFrmExtractPayload, buildMainPassthroughPayload,
  buildNorStubPayload, buildWorkbenchInspectArgs, resolveAutoSdDriveTarget,
  shouldNotifyUnbrickVerified,
} from './unbrickMissionLogic';

export function useUnbrickViewModel() {
  const { showToast } = useToast();

  const [workbench, setWorkbench] = useState<AsyncChannelState<UnbrickWorkspaceReport>>(() => ({
    loading: true,
    data: null,
    error: null,
  }));
  const [sdKit, setSdKit] = useState<AsyncChannelState<SdUnbrickKitReport>>(() => emptyChannel());
  const [verify, setVerify] = useState<AsyncChannelState<SdMediaVerificationReport>>(() => emptyChannel());
  const [sdDisk, setSdDisk] = useState<AsyncChannelState<SdDiskInspectionReport>>(() => emptyChannel());
  const [extract, setExtract] = useState<AsyncChannelState<FrmModuleExtractionReport>>(() => emptyChannel());
  const [tbc, setTbc] = useState<AsyncChannelState<MainPassthroughReport>>(() => emptyChannel());
  const [rspPlan, setRspPlan] = useState<AsyncChannelState<MainPassthroughRspPlan>>(() => emptyChannel());
  const [norStub, setNorStub] = useState<AsyncChannelState<NorDumperStubResult>>(() => emptyChannel());
  const [sdFat16Image, setSdFat16Image] = useState<AsyncChannelState<SdCardCreatorReport>>(() => emptyChannel());

  const runChannel = useCallback(
    async <T>(
      setter: React.Dispatch<React.SetStateAction<AsyncChannelState<T>>>,
      tag: string,
      fn: () => Promise<T>,
      toastTitle?: string,
    ): Promise<T | null> => {
      setter((prev) => ({ ...prev, loading: true, error: null }));
      try {
        const result = await fn();
        setter({ loading: false, data: result, error: null });
        if (toastTitle) showToast({ type: 'success', title: toastTitle });
        return result;
      } catch (err) {
        const msg = formatErrorMessage(err);
        logger.error('UnbrickViewModel', `${tag} failed`, err);
        setter((prev) => ({ ...prev, loading: false, error: msg }));
        showToast({ type: 'error', title: tag, message: msg });
        return null;
      }
    },
    [showToast],
  );

  const loadWorkbench = useCallback(
    async (workspacePath?: string): Promise<UnbrickWorkspaceReport | null> => {
      const args = buildWorkbenchInspectArgs(workspacePath);
      const report = await runChannel(setWorkbench, translate('errors.unbrick.workbenchTag'), () =>
        inspectUnbrickWorkbench(args),
      );
      if (report) {
        setTbc((prev) => ({ ...prev, data: report.main_passthrough }));
        setRspPlan((prev) => ({ ...prev, data: report.rsp_plan }));
        if (report.sd_verification) {
          setVerify((prev) => ({ ...prev, data: report.sd_verification }));
          if (shouldNotifyUnbrickVerified(report.sd_verification.overall_status)) {
            emitUnbrickCompletedEvent(true);
          }
        }
      }
      return report;
    },
    [runChannel],
  );

  useEffect(() => {
    const controller = new AbortController();
    void Promise.resolve().then(() => {
      if (!controller.signal.aborted) void loadWorkbench();
    });
    return () => controller.abort();
  }, [loadWorkbench]);

  const buildSdKit = useCallback(
    async (request: SdUnbrickKitRequestDto): Promise<SdUnbrickKitReport | null> => {
      const report = await runChannel(
        setSdKit, translate('errors.unbrick.sdKitTag'),
        () => prepareSdUnbrickKit(request),
        translate('errors.unbrick.sdKitSaved', { dir: request.output_dir }),
      );
      if (report) {
        setVerify({ loading: false, data: report.verification, error: null });
        emitUnbrickCompletedEvent(true);
      }
      return report;
    },
    [runChannel],
  );

  const verifySdMedia = useCallback(
    async (sdRootDir: string): Promise<SdMediaVerificationReport | null> => {
      const clean = sdRootDir.trim();
      const report = await runChannel(
        setVerify, translate('errors.unbrick.verifySdTag'),
        () => verifySdUnbrickMedia(clean),
        translate('errors.unbrick.verifySdDone', { dir: clean }),
      );
      if (report) emitUnbrickCompletedEvent(true);
      return report;
    },
    [runChannel],
  );

  const createFat16Image = useCallback(
    async (params: { panaDvdPath?: string; includePanaSd?: boolean; loaderProfile?: 'ver061' | 'ver063' }): Promise<SdCardCreatorReport | null> => {
      const rep = await runChannel(
        setSdFat16Image, translate('errors.unbrick.sdFlashTag'),
        () => flashSdRecoveryCard(buildFat16SdFlashPayload(params)),
        translate('errors.unbrick.sdFlashDone'),
      );
      if (rep) emitUnbrickCompletedEvent(true);
      return rep;
    },
    [runChannel],
  );

  const inspectSdDisk = useCallback(
    async (target: string): Promise<SdDiskInspectionReport | null> => {
      const clean = target.trim();
      return runChannel(
        setSdDisk, translate('errors.unbrick.inspectDiskTag'),
        () => inspectSdDiskRpc(clean),
        translate('errors.unbrick.inspectDiskDone', { target: clean }),
      );
    },
    [runChannel],
  );

  const extractFrmModules = useCallback(
    async (frmPath: string, outDir: string, unpackPayloads = true): Promise<FrmModuleExtractionReport | null> =>
      runChannel(
        setExtract, translate('errors.unbrick.extractFrmTag'),
        () => extractFrmModulesRpc(buildFrmExtractPayload(frmPath, outDir, unpackPayloads)),
        translate('errors.unbrick.extractFrmDone', { dir: outDir.trim() }),
      ),
    [runChannel],
  );

  const runMainPassthrough = useCallback(
    async (params: {
      action: 'inspect' | 'patch';
      inputPath: string;
      outPath?: string;
      mode?: 'av_passthrough' | 'factory_original';
    }): Promise<MainPassthroughReport | null> => {
      const payload = buildMainPassthroughPayload(params);
      return runChannel(
        setTbc, translate('errors.unbrick.tbcTag'),
        () => patchMainPassthrough(payload),
        params.action === 'patch'
          ? translate('errors.unbrick.tbcPatched', { mode: payload.mode })
          : translate('errors.unbrick.tbcInspected'),
      );
    },
    [runChannel],
  );

  const generateRspPlan = useCallback(
    async (callbackTargetVa?: string): Promise<MainPassthroughRspPlan | null> => {
      const cb = callbackTargetVa?.trim() || undefined;
      return runChannel(
        setRspPlan, translate('errors.unbrick.rspPlanTag'),
        () => generateMainRspPlan(cb),
        translate('errors.unbrick.rspPlanUpdated'),
      );
    },
    [runChannel],
  );

  const buildNorStub = useCallback(
    async (params: {
      startAddr?: string; totalLen?: string; blockSize?: number;
      loadAddr?: string; outPath?: string; wrapAsDigadiag?: boolean;
    }): Promise<NorDumperStubResult | null> =>
      runChannel(
        setNorStub, translate('errors.unbrick.norStubTag'),
        () => buildNorDumperStub(buildNorStubPayload(params)),
        translate('errors.unbrick.norStubBuilt'),
      ),
    [runChannel],
  );

  const matshita = useMatshitaEmuActions(runChannel);
  const sdDrivesVm = useStorageDrives('sd_card', true);

  const autoSdResolved = useMemo(
    () => resolveAutoSdDriveTarget(sdDrivesVm.filteredDrives, sdDrivesVm.drives),
    [sdDrivesVm.drives, sdDrivesVm.filteredDrives],
  );

  return {
    workbench, sdKit, verify, sdDisk, extract, tbc, rspPlan, norStub, ...matshita,
    sdDrives: sdDrivesVm.filteredDrives, sdDriveFilter: sdDrivesVm.roleFilter,
    setSdDriveFilter: sdDrivesVm.setRoleFilter, sdDrivesLoading: sdDrivesVm.loading,
    refreshSdDrives: sdDrivesVm.refresh,
    autoSdDiskTarget: autoSdResolved?.diskTarget ?? null,
    autoSdMountDir: autoSdResolved?.mountPoint ?? null,
    sdFat16Image, createFat16Image, loadWorkbench, buildSdKit, verifySdMedia,
    inspectSdDisk, extractFrmModules, runMainPassthrough, generateRspPlan, buildNorStub,
  };
}
