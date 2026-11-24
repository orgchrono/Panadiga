/**
 * @file services/unbrickService.ts
 * @brief Domain RPC wrapper for Pre-MAIN Unbrick workbench, SD kit builder, TBC passthrough, & Matshita emulator.
 */

import type {
  BootSequenceReport,
  FlashSdRecoveryCardRequest,
  FrmModuleExtractionReport,
  FrmUnlockVerificationSummary,
  MainPassthroughReport,
  MainPassthroughRspPlan,
  MatshitaBootScenario,
  MatshitaInjectWriteResult,
  NorDumperStubResult,
  PackUnlockFrmResult,
  SdCardCreatorReport,
  SdDiskInspectionReport,
  SdFlashProgressEvent,
  SdMediaVerificationReport,
  SdTargetDriveInfo,
  SdUnbrickKitReport,
  SdUnbrickKitRequestDto,
  SerialCliResponse,
  UnbrickWorkspaceReport,
} from '@/models';
import { mintOperationNonceId } from './benchStreamService';
import { invokeCommand, listenEvent } from './ipc';

export const inspectUnbrickWorkbench = (
  args?: Record<string, unknown>,
): Promise<UnbrickWorkspaceReport> => invokeCommand<UnbrickWorkspaceReport>('inspect_unbrick_workbench', args);

export const prepareSdUnbrickKit = (request: SdUnbrickKitRequestDto): Promise<SdUnbrickKitReport> =>
  invokeCommand<SdUnbrickKitReport>('prepare_sd_unbrick_kit_command', { request });

export const verifySdUnbrickMedia = (sdRootDir: string): Promise<SdMediaVerificationReport> =>
  invokeCommand<SdMediaVerificationReport>('verify_sd_unbrick_media_command', {
    sdRootDir,
    sd_root_dir: sdRootDir,
  });

export const inspectSdDisk = (target: string): Promise<SdDiskInspectionReport> =>
  invokeCommand<SdDiskInspectionReport>('inspect_sd_disk_command', { target });

export const extractFrmModules = (params: {
  frmPath: string;
  outDir: string;
  unpackPayloads: boolean;
}): Promise<FrmModuleExtractionReport> =>
  invokeCommand<FrmModuleExtractionReport>('extract_frm_modules_command', {
    frmPath: params.frmPath,
    frm_path: params.frmPath,
    outDir: params.outDir,
    out_dir: params.outDir,
    unpackPayloads: params.unpackPayloads,
    unpack_payloads: params.unpackPayloads,
  });

export const patchMainPassthrough = (params: {
  action: 'inspect' | 'patch';
  inputPath: string;
  outPath?: string;
  mode: string;
}): Promise<MainPassthroughReport> =>
  invokeCommand<MainPassthroughReport>('patch_main_passthrough_command', {
    action: params.action,
    inputPath: params.inputPath,
    input_path: params.inputPath,
    outPath: params.outPath,
    out_path: params.outPath,
    mode: params.mode,
  });

export const generateMainRspPlan = (callbackTargetVa?: string): Promise<MainPassthroughRspPlan> =>
  invokeCommand<MainPassthroughRspPlan>('patch_main_passthrough_command', {
    action: 'rsp_plan',
    callbackTargetVa,
    callback_target_va: callbackTargetVa,
  });

export const buildNorDumperStub = (params: {
  startAddr: string;
  totalLen: string;
  blockSize: number;
  loadAddr: string;
  outPath?: string;
  wrapAsDigadiag: boolean;
}): Promise<NorDumperStubResult> =>
  invokeCommand<NorDumperStubResult>('build_nor_dumper_stub_command', {
    startAddr: params.startAddr,
    totalLen: params.totalLen,
    blockSize: params.blockSize,
    loadAddr: params.loadAddr,
    outPath: params.outPath,
    wrapAsDigadiag: params.wrapAsDigadiag,
    wrap_as_digadiag: params.wrapAsDigadiag,
  });

export const runMatshitaBootEmu = (scenario: MatshitaBootScenario): Promise<BootSequenceReport> =>
  invokeCommand<BootSequenceReport>('run_matshita_boot_emu', { scenario });

export const injectMatshitaAtapiWrite = (params: {
  targetVa?: string;
  offset: number;
  mode: number;
  useVendorEd: boolean;
  payloadHex: string;
}): Promise<MatshitaInjectWriteResult> =>
  invokeCommand<MatshitaInjectWriteResult>('inject_matshita_atapi_write', params);

export const runMatshitaSerialCmd = (command: string, unlockPass: boolean): Promise<SerialCliResponse> =>
  invokeCommand<SerialCliResponse>('run_matshita_serial_cmd', { command, unlockPass });

export const verifyFrmUnlockContainer = (
  containerKind: string,
  frmPath: string,
): Promise<FrmUnlockVerificationSummary> =>
  invokeCommand<FrmUnlockVerificationSummary>('verify_frm_unlock_command', {
    containerKind,
    frmPath,
  });

export const packFrmUnlockContainer = (params: {
  containerKind: string;
  outPath?: string;
  payloadHex?: string;
}): Promise<PackUnlockFrmResult> =>
  invokeCommand<PackUnlockFrmResult>('pack_unlock_frm_command', params);

export const inspectSdCardTargets = (): Promise<SdTargetDriveInfo[]> =>
  invokeCommand<SdTargetDriveInfo[]>('inspect_sd_card_targets_command');

export const flashSdRecoveryCard = async (
  request: FlashSdRecoveryCardRequest,
): Promise<SdCardCreatorReport> => {
  const devPath = request.devicePath?.trim();
  const nonceId = devPath
    ? await mintOperationNonceId(
        'sd_kit_build',
        devPath,
        request.confirmToken?.trim() || 'sd_flash',
        request.nonceId,
      )
    : request.nonceId;
  return invokeCommand<SdCardCreatorReport>('flash_sd_recovery_card_command', {
    request: { ...request, ...(nonceId ? { nonceId } : {}) },
  });
};

export const subscribeSdFlashProgress = (
  handler: (payload: SdFlashProgressEvent) => void,
): Promise<() => void> => listenEvent('sd://flash-progress', handler);
