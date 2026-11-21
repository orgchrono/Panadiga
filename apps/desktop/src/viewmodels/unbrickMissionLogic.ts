/**
 * @file viewmodels/unbrickMissionLogic.ts
 * @brief Pure functional request builders and storage target selectors for Unbrick ViewModel.
 */

import {
  AM34_FLASH_SIZE_HEX,
  AM34_FLASH_UNCACHED_ADDR_HEX,
  AM34_SDRAM_STUB_RAW_ADDR,
  AM34_SDRAM_STUB_WRAP_ADDR,
  NOR_DUMPER_DEFAULT_BLOCK_SIZE,
  SD_DEFAULT_VOLUME_MB,
} from '@/constants';
import type { PhysicalDriveInfo } from '@/models';
import { resolvePrimaryDriveTarget } from '@/utils/storageDrives';

export function buildWorkbenchInspectArgs(workspacePath?: string) {
  const trimmed = workspacePath?.trim();
  return trimmed ? { workspacePath: trimmed, workspace_path: trimmed } : undefined;
}

export function shouldNotifyUnbrickVerified(overallStatus?: string | null): boolean {
  return Boolean(
    overallStatus && overallStatus !== 'FAIL' && overallStatus !== 'INCOMPLETE_OR_INVALID',
  );
}

export function buildFat16SdFlashPayload(params: {
  readonly panaDvdPath?: string;
  readonly includePanaSd?: boolean;
  readonly loaderProfile?: 'ver061' | 'ver063';
}) {
  return {
    model: 'DMR-EH55',
    volumeMb: SD_DEFAULT_VOLUME_MB,
    loaderProfile: params.loaderProfile ?? 'ver061',
    panaDvdPath: params.panaDvdPath?.trim() || undefined,
    includePanaSd: Boolean(params.includePanaSd),
  };
}

export function buildFrmExtractPayload(
  frmPath: string,
  outDir: string,
  unpackPayloads: boolean,
) {
  return {
    frmPath: frmPath.trim(),
    outDir: outDir.trim(),
    unpackPayloads,
  };
}

export function buildMainPassthroughPayload(params: {
  readonly action: 'inspect' | 'patch';
  readonly inputPath: string;
  readonly outPath?: string;
  readonly mode?: 'av_passthrough' | 'factory_original';
}) {
  const mode = params.mode ?? 'av_passthrough';
  return {
    action: params.action,
    inputPath: params.inputPath.trim(),
    outPath: params.outPath?.trim() || undefined,
    mode,
  };
}

export function buildNorStubPayload(params: {
  readonly startAddr?: string;
  readonly totalLen?: string;
  readonly blockSize?: number;
  readonly loadAddr?: string;
  readonly outPath?: string;
  readonly wrapAsDigadiag?: boolean;
}) {
  const wrap = Boolean(params.wrapAsDigadiag);
  return {
    startAddr: params.startAddr?.trim() || AM34_FLASH_UNCACHED_ADDR_HEX,
    totalLen: params.totalLen?.trim() || AM34_FLASH_SIZE_HEX,
    blockSize: params.blockSize ?? NOR_DUMPER_DEFAULT_BLOCK_SIZE,
    loadAddr:
      params.loadAddr?.trim() ||
      (wrap ? AM34_SDRAM_STUB_WRAP_ADDR : AM34_SDRAM_STUB_RAW_ADDR),
    outPath: params.outPath?.trim() || undefined,
    wrapAsDigadiag: wrap,
  };
}

export function resolveAutoSdDriveTarget(
  filteredDrives: readonly PhysicalDriveInfo[],
  allDrives: readonly PhysicalDriveInfo[],
) {
  const candidate =
    filteredDrives.find((d) => d.safe_to_write) ??
    allDrives.find((d) => d.drive_role === 'sd_card' && d.safe_to_write);
  return candidate ? resolvePrimaryDriveTarget(candidate, true) : null;
}
