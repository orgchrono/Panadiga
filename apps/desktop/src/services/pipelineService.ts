/**
 * @file services/pipelineService.ts
 * @brief Typed Tauri IPC wrappers & reactive streaming listeners for Wave 3-A workbench pipelines.
 */

import type {
  ApplyMainTbcBypassesRequest,
  ApplyMainTbcBypassesResponse,
  ApplyRegionCalibrationRequest,
  ApplyRegionCalibrationResponse,
  BgoRepairProgressEvent,
  Eh55BgoAuditReport,
  Eh55HddFlashReport,
  FlashEh55HddSafeRequest,
  HardenSdUnbrickKitRequest,
  HardenSdUnbrickKitResponse,
  HddFlashProgressEvent,
  InspectRegionCalibrationRequest,
  NorDumpProgressEvent,
  PicoEh55BgoAuditRequest,
  PicoEh55BgoRepairRequest,
  PicoEh55BgoRepairResponse,
  PicoEh55NorDumpRequest,
  PicoEh55NorDumpResponse,
  PicoProbeDavisIdeRequest,
  PicoProbeDavisIdeResponse,
  SafeTargetDiskInfo,
  TvCalibrationBundleReport,
} from '@/models';
import { invokeCommand, listenEvent } from './ipc';

export const inspectRegionCalibration = (
  request: InspectRegionCalibrationRequest,
): Promise<TvCalibrationBundleReport> =>
  invokeCommand<TvCalibrationBundleReport>('inspect_region_calibration_command', { request });

export const applyRegionCalibration = (
  request: ApplyRegionCalibrationRequest,
): Promise<ApplyRegionCalibrationResponse> =>
  invokeCommand<ApplyRegionCalibrationResponse>('apply_region_calibration_command', { request });

export const hardenSdUnbrickKit = (
  request: HardenSdUnbrickKitRequest,
): Promise<HardenSdUnbrickKitResponse> =>
  invokeCommand<HardenSdUnbrickKitResponse>('harden_sd_unbrick_kit_command', { request });

export const applyMainTbcBypasses = (
  request: ApplyMainTbcBypassesRequest,
): Promise<ApplyMainTbcBypassesResponse> =>
  invokeCommand<ApplyMainTbcBypassesResponse>('apply_main_tbc_bypasses_command', { request });

export const inspectSafeTargetDisks = (): Promise<SafeTargetDiskInfo[]> =>
  invokeCommand<SafeTargetDiskInfo[]>('inspect_safe_target_disks_command');

export const flashEh55HddSafe = (
  request: FlashEh55HddSafeRequest,
): Promise<Eh55HddFlashReport> =>
  invokeCommand<Eh55HddFlashReport>('flash_eh55_hdd_safe_command', { request });

export const picoEh55NorDump = (
  request: PicoEh55NorDumpRequest,
): Promise<PicoEh55NorDumpResponse> =>
  invokeCommand<PicoEh55NorDumpResponse>('pico_eh55_nor_dump_command', { request });

export const picoEh55BgoAudit = (
  request: PicoEh55BgoAuditRequest,
): Promise<Eh55BgoAuditReport> =>
  invokeCommand<Eh55BgoAuditReport>('pico_eh55_bgo_audit_command', { request });

export const picoEh55BgoRepair = (
  request: PicoEh55BgoRepairRequest,
): Promise<PicoEh55BgoRepairResponse> =>
  invokeCommand<PicoEh55BgoRepairResponse>('pico_eh55_bgo_repair_command', { request });

export const picoProbeDavisIde = (
  request: PicoProbeDavisIdeRequest,
): Promise<PicoProbeDavisIdeResponse> =>
  invokeCommand<PicoProbeDavisIdeResponse>('pico_probe_davis_ide_command', { request });

export const subscribeHddFlashProgress = (
  handler: (payload: HddFlashProgressEvent) => void,
): Promise<() => void> => listenEvent('hdd://flash-progress', handler);

export const subscribeNorDumpProgress = (
  handler: (payload: NorDumpProgressEvent) => void,
): Promise<() => void> => listenEvent('nor://dump-progress', handler);

export const subscribeBgoRepairProgress = (
  handler: (payload: BgoRepairProgressEvent) => void,
): Promise<() => void> => listenEvent('bgo://repair-progress', handler);

export const subscribeDavisProbeResult = (
  handler: (payload: PicoProbeDavisIdeResponse) => void,
): Promise<() => void> => listenEvent('davis://probe-result', handler);
