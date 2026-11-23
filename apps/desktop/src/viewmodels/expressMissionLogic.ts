/**
 * @file viewmodels/expressMissionLogic.ts
 * @brief Pure functional orchestration, request builders, and validation helpers for Express Mode missions.
 */

import {
  SD_DEFAULT_VOLUME_MB,
  UPPER_NOR_CONFIRM_TOKEN,
} from '@/constants';
import type { ToastItem } from '@/context/ToastContext';
import { translate } from '@/i18n/useTranslation';
import type {
  DigaModelProfile,
  Eh55BgoAuditReport,
  NorDumpProgressEvent,
  NorRepairExpressOutcome,
  NvramCleanProgressEvent,
  NvramCleanReport,
  PicoEh55NorDumpResponse,
  PicoPanamon1CleanNvramResponse,
  SdCardCreatorReport,
  SdFlashProgressEvent,
  SdTargetDriveInfo,
} from '@/models';
import { requestLiveBgoNonce } from '@/services/benchStreamService';
import {
  getDigaModelCatalog,
  listPicoSerialPorts,
} from '@/services/picoService';
import {
  picoEh55BgoAudit,
  picoEh55BgoRepair,
  picoEh55NorDump,
  picoPanamon1CleanNvram,
  subscribeNorDumpProgress,
  subscribeNvramCleanProgress,
} from '@/services/pipelineService';
import { analyzeFlashDump, cleanNvramSurgical } from '@/services/storageCryptoService';
import { emitNvramCleanedEvent, emitUnbrickCompletedEvent } from '@/services/systemService';
import {
  flashSdRecoveryCard,
  inspectSdCardTargets,
  subscribeSdFlashProgress,
} from '@/services/unbrickService';
import { logger } from '@/utils/logger';

export const EXPRESS_PROGRESS_DONE_DEFAULT = 8;

export type ShowToastFn = (toast: Omit<ToastItem, 'id'>) => void;
export type ExpressSdDriveTarget = Pick<
  SdTargetDriveInfo,
  'device_path' | 'safe_sd_target' | 'expected_confirm_token'
>;

export function formatExpressError(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

export function filterSelectableExpressModels<T extends { readonly model_id: string }>(
  catalog: readonly T[],
): T[] {
  return catalog.filter(item => item.model_id !== 'AUTO');
}

export function pickDefaultSafeSdPath(targets: readonly ExpressSdDriveTarget[]): string | null {
  return targets.find(item => item.safe_sd_target)?.device_path ?? null;
}

export async function fetchExpressSdTargets(): Promise<readonly SdTargetDriveInfo[] | null> {
  try {
    return await inspectSdCardTargets();
  } catch (err) {
    logger.warn('ExpressViewModel', 'Failed enumerating SD card targets', err);
    return null;
  }
}

export async function fetchExpressPicoPorts(): Promise<readonly string[] | null> {
  try {
    return await listPicoSerialPorts();
  } catch (err) {
    logger.warn('ExpressViewModel', 'Failed enumerating Pico serial ports', err);
    return null;
  }
}

export async function fetchInitialExpressDiscovery(): Promise<{
  readonly models: DigaModelProfile[];
  readonly targets: readonly SdTargetDriveInfo[];
  readonly ports: readonly string[];
} | null> {
  try {
    const [catalog, targets, ports] = await Promise.all([
      getDigaModelCatalog(),
      inspectSdCardTargets(),
      listPicoSerialPorts(),
    ]);
    return {
      models: filterSelectableExpressModels(catalog),
      targets,
      ports,
    };
  } catch (err) {
    logger.warn('ExpressViewModel', 'Initial Express discovery failed', err);
    return null;
  }
}

export function validateSdFlashSelection(
  sdTargetMode: 'image' | 'physical',
  selectedSdDrive: ExpressSdDriveTarget | null,
  sdConfirmToken: string,
): string | null {
  if (sdTargetMode !== 'physical') {
    return null;
  }
  if (!selectedSdDrive) {
    return translate('errors.unbrick.sdTargetRequired');
  }
  if (sdConfirmToken.trim() !== selectedSdDrive.expected_confirm_token) {
    return translate('errors.unbrick.sdConfirmRequired', {
      token: selectedSdDrive.expected_confirm_token,
    });
  }
  return null;
}

export function buildSdFlashRequest(params: {
  readonly sdTargetMode: 'image' | 'physical';
  readonly selectedSdDrive: ExpressSdDriveTarget | null;
  readonly sdConfirmToken: string;
  readonly selectedModel: string;
  readonly includePanaSd: boolean;
}) {
  return {
    ...(params.sdTargetMode === 'physical' && params.selectedSdDrive
      ? {
          devicePath: params.selectedSdDrive.device_path,
          confirmToken: params.sdConfirmToken.trim(),
        }
      : {}),
    model: params.selectedModel,
    volumeMb: SD_DEFAULT_VOLUME_MB,
    loaderProfile: 'ver061' as const,
    includePanaSd: params.includePanaSd,
  };
}

export function buildInitialNvramProgress(cleanPort: string): NvramCleanProgressEvent {
  return {
    port: cleanPort,
    stage: 'connect_bridge',
    current: 0,
    total: EXPRESS_PROGRESS_DONE_DEFAULT,
    detail: cleanPort,
  };
}

export function summarizeBgoRepairOutcome(
  bgoRes: Awaited<ReturnType<typeof picoEh55BgoRepair>>,
  targetNor: string | undefined,
  liveUpperNor: boolean,
): NorRepairExpressOutcome {
  return {
    mode: liveUpperNor ? 'bgo_live' : 'bgo_cow',
    summaryPath:
      bgoRes.liveReport?.backup_path ??
      bgoRes.cowOutPath ??
      bgoRes.outPlanPath ??
      targetNor ??
      '',
    sha256: bgoRes.liveReport?.post_bgo_sha256 ?? bgoRes.plan.expected_post_sha256,
  };
}

export async function runSdFlashMission(params: {
  readonly sdTargetMode: 'image' | 'physical';
  readonly selectedSdDrive: SdTargetDriveInfo | null;
  readonly sdConfirmToken: string;
  readonly selectedModel: string;
  readonly includePanaSd: boolean;
  readonly onProgress: (ev: SdFlashProgressEvent) => void;
  readonly onReport: (report: SdCardCreatorReport) => void;
  readonly showToast: ShowToastFn;
}): Promise<void> {
  const unlisten = await subscribeSdFlashProgress(params.onProgress);
  try {
    const report = await flashSdRecoveryCard(buildSdFlashRequest(params));
    params.onReport(report);
    emitUnbrickCompletedEvent(true);
    params.showToast({
      type: 'success',
      title: translate('errors.unbrick.sdFlashTag'),
      message: translate('errors.unbrick.sdFlashDone'),
    });
  } finally {
    unlisten();
  }
}

export async function runU99RescueMission(params: {
  readonly cleanPort: string;
  readonly onProgress: (ev: NvramCleanProgressEvent) => void;
  readonly onReport: (report: PicoPanamon1CleanNvramResponse) => void;
  readonly showToast: ShowToastFn;
}): Promise<void> {
  params.onProgress(buildInitialNvramProgress(params.cleanPort));
  const unlisten = await subscribeNvramCleanProgress(params.onProgress);
  try {
    const report = await picoPanamon1CleanNvram({ portName: params.cleanPort });
    params.onReport(report);
    emitNvramCleanedEvent(true);
    params.showToast({
      type: 'success',
      title: translate('errors.nvram.surgicalCleanBadgeTitle'),
      message: translate('errors.pipelines.panamon1CleanDone'),
    });
  } finally {
    unlisten();
  }
}

export async function runLocalNorInspectMission(params: {
  readonly onAudit: (nvram: NvramCleanReport, bgo: Eh55BgoAuditReport) => void;
}): Promise<void> {
  const nvram = await analyzeFlashDump();
  const bgo = await picoEh55BgoAudit({ sourceMode: 'DUMP_FILE', norPath: nvram.input_file });
  params.onAudit(nvram, bgo);
}

export async function runPicoNorInspectMission(params: {
  readonly cleanPort: string;
  readonly onProgress: (ev: NorDumpProgressEvent) => void;
  readonly onDumpReport: (report: PicoEh55NorDumpResponse) => void;
  readonly onAudit: (nvram: NvramCleanReport, bgo: Eh55BgoAuditReport) => void;
  readonly showToast: ShowToastFn;
}): Promise<void> {
  const unlisten = await subscribeNorDumpProgress(params.onProgress);
  try {
    const dumpRes = await picoEh55NorDump({ portName: params.cleanPort, slice: 'full' });
    params.onDumpReport(dumpRes);
    const [nvram, bgo] = await Promise.all([
      analyzeFlashDump(),
      picoEh55BgoAudit({ sourceMode: 'DUMP_FILE', norPath: dumpRes.outputPath }),
    ]);
    params.onAudit(nvram, bgo);
    params.showToast({ type: 'success', title: translate('errors.pipelines.norDumped') });
  } finally {
    unlisten();
  }
}

export async function runNorRepairMission(params: {
  readonly liveUpperNor: boolean;
  readonly portName: string;
  readonly norAuditNvram: NvramCleanReport | null;
  readonly norDumpReport: PicoEh55NorDumpResponse | null;
  readonly onNvramUpdated: (report: NvramCleanReport) => void;
  readonly onBgoUpdated: (audit: Eh55BgoAuditReport) => void;
  readonly onResult: (result: NorRepairExpressOutcome) => void;
  readonly showToast: ShowToastFn;
}): Promise<void> {
  if (params.norAuditNvram && params.norAuditNvram.nvram_non_ff_bytes > 0) {
    const cleaned = await cleanNvramSurgical();
    params.onNvramUpdated(cleaned);
    emitNvramCleanedEvent(true);
    params.onResult({
      mode: 'nvram_clean',
      summaryPath: cleaned.output_file,
      sha256: cleaned.clean_sha256,
    });
    params.showToast({
      type: 'success',
      title: translate('errors.nvram.surgicalCleanTitle'),
      message: translate('errors.nvram.surgicalCleanMsg'),
    });
    return;
  }
  const targetNor = params.norDumpReport?.outputPath ?? params.norAuditNvram?.input_file;
  const liveNonce = params.liveUpperNor ? await requestLiveBgoNonce(targetNor) : null;
  const bgoRes = await picoEh55BgoRepair({
    norPath: targetNor,
    dryRunOnly: !params.liveUpperNor,
    ...(params.liveUpperNor
      ? {
          portName: params.portName.trim(),
          confirmToken: UPPER_NOR_CONFIRM_TOKEN,
          ...(liveNonce ? { nonceId: liveNonce.nonce_id } : {}),
        }
      : {}),
  });
  params.onBgoUpdated(bgoRes.plan.post_audit);
  params.onResult(summarizeBgoRepairOutcome(bgoRes, targetNor, params.liveUpperNor));
  params.showToast({
    type: 'success',
    title: translate(
      params.liveUpperNor ? 'errors.pipelines.bgoLiveApplied' : 'errors.pipelines.bgoDryRunReady',
    ),
  });
}
