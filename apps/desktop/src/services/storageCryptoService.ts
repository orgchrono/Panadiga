/**
 * @file services/storageCryptoService.ts
 * @brief Domain RPC client for NVRAM surgery, MEIHFS HDD forensics, physical writer, and FRM crypto.
 */

import { AM34_NVRAM_BASE_ADDR } from '@/constants';
import type {
  CodebookReport,
  CryptanalysisReport,
  DriveWriteReport,
  ExtrheadUnpackReport,
  FrmPackReport,
  FrmTocDumpReport,
  HddAnalysisReport,
  LiveNvramCaptureEvent,
  MainCapacityInspectionReport,
  MainCapacityPatchReport,
  MeihdfsAllocationReport,
  NvramCleanReport,
  PackFrmRequest,
  PartitionExtractionManifest,
  PhysicalDriveInfo,
} from '@/models';
import { formatHex32 } from '@/utils/formatters';
import { parseP7501DumpBytes } from './diagnosticParser';
import { invokeCommand } from './ipc';

export const analyzeFlashDump = (): Promise<NvramCleanReport> =>
  invokeCommand<NvramCleanReport>('analyze_flash_dump');

export const cleanNvramSurgical = (): Promise<NvramCleanReport> =>
  invokeCommand<NvramCleanReport>('clean_nvram');

export const patchNvramOffset = (params: {
  inputPath: string;
  outputPath: string;
  nvramOffset: number;
  patchHex: string;
}): Promise<NvramCleanReport> =>
  invokeCommand<NvramCleanReport>('patch_nvram_command', params);

/** Decodes P7501 DUMP output into a typed live capture event without touching Redux store directly. */
export async function analyzeLiveNvramDump(
  rawOutput: string,
  source: string,
  port: string,
  baseAddressOverride?: number,
): Promise<LiveNvramCaptureEvent | null> {
  const { baseAddress, bytes } = parseP7501DumpBytes(rawOutput);
  if (bytes.length === 0) return null;

  const effectiveBase = baseAddressOverride ?? baseAddress ?? AM34_NVRAM_BASE_ADDR;
  const baseHex = formatHex32(effectiveBase);
  const report = await invokeCommand<NvramCleanReport>('analyze_live_nvram_bytes', {
    bytes,
    baseBusAddress: effectiveBase,
    sourceLabel: `Live Pico ${source} (${port}) @ ${baseHex}`,
  });

  return {
    source,
    port,
    base_bus_address: baseHex,
    captured_bytes: bytes.length,
    captured_at: new Date().toLocaleTimeString('pt-BR', { hour12: false }),
    report,
  };
}

export async function inspectHddImage(imagePath?: string): Promise<{
  report: HddAnalysisReport;
  allocReport: MeihdfsAllocationReport;
}> {
  const trimmed = imagePath?.trim();
  const args = trimmed ? { image_path: trimmed } : {};
  const [report, allocReport] = await Promise.all([
    invokeCommand<HddAnalysisReport>('inspect_hdd', args),
    invokeCommand<MeihdfsAllocationReport>('decode_meihdfs_allocations', args),
  ]);
  return { report, allocReport };
}

export const extractHddPartitions = (imagePath?: string): Promise<PartitionExtractionManifest> =>
  invokeCommand<PartitionExtractionManifest>(
    'extract_partitions',
    imagePath?.trim() ? { image_path: imagePath.trim() } : {},
  );

export const inspectHddCapacity = (inputPath?: string): Promise<MainCapacityInspectionReport> => {
  const clean = inputPath?.trim();
  return invokeCommand<MainCapacityInspectionReport>(
    'inspect_hdd_capacity_patch',
    clean ? { inputPath: clean } : {},
  );
};

export const patchHddCapacity = (params: Record<string, unknown>): Promise<MainCapacityPatchReport> =>
  invokeCommand<MainCapacityPatchReport>('apply_hdd_capacity_patch', params);

export const listPhysicalDrives = (role?: string): Promise<PhysicalDriveInfo[]> =>
  invokeCommand<PhysicalDriveInfo[]>(
    'list_physical_drives',
    role ? { role } : undefined,
  );

export const listHddDrives = (): Promise<PhysicalDriveInfo[]> =>
  invokeCommand<PhysicalDriveInfo[]>('list_hdd_drives');

export const listSdCardDrives = (): Promise<PhysicalDriveInfo[]> =>
  invokeCommand<PhysicalDriveInfo[]>('list_sd_card_drives');

export const unmountPhysicalDrive = (devicePath: string): Promise<string[]> =>
  invokeCommand<string[]>('unmount_physical_drive', { devicePath });

export const executeDiskWriteCommand = (
  cmd: 'format_hdd_meihdfs' | 'write_hdd_image' | 'restore_hdd_mirrors',
  args: Record<string, unknown>,
): Promise<DriveWriteReport> => invokeCommand<DriveWriteReport>(cmd, args);

export const analyzeFrmBundle = (frmPath: string) => {
  const trimmedPath = frmPath.trim();
  const frmArgs = trimmedPath ? { frmPath: trimmedPath, frm_path: trimmedPath } : undefined;
  return Promise.allSettled([
    invokeCommand<CryptanalysisReport>('analyze_frm', frmArgs),
    invokeCommand<FrmTocDumpReport>('dump_frm_toc', frmArgs),
    invokeCommand<CodebookReport>('get_codebook', frmArgs),
  ]);
};

export const listFrmContainers = (): Promise<string[]> =>
  invokeCommand<string[]>('list_frm_containers_command');

export const packFrmContainer = (request: PackFrmRequest): Promise<FrmPackReport> =>
  invokeCommand<FrmPackReport>('pack_frm_command', { request });

export const unpackExtrheadPayload = (params: {
  inputPath: string;
  mode: 'auto' | 'main' | 'drv' | 'plain';
  outPath?: string;
}): Promise<ExtrheadUnpackReport> =>
  invokeCommand<ExtrheadUnpackReport>('unpack_extrhead_command', {
    inputPath: params.inputPath,
    mode: params.mode,
    outPath: params.outPath || undefined,
  });
