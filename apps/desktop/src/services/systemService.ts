/**
 * @file services/systemService.ts
 * @brief Domain RPC & workbench pipeline event adapter for System, Manuals, RAG, Docs, and Hex Inspector.
 */

import type {
  Am34BootLogReport,
  EngineeringDocEntry,
  HardwareSsotSpec,
  HexDumpRow,
  LauncherInstallReport,
  LiveNvramCaptureEvent,
  ManualSearchResult,
  PicoDeviceStatus,
  RagQueryResult,
  ServiceManualCatalogItem,
  SystemOverview,
} from '@/models';
import { emitLocalEvent, invokeCommand, listenEvent } from './ipc';

export const fetchSystemOverview = (): Promise<SystemOverview> =>
  invokeCommand<SystemOverview>('get_system_overview');

export const fetchHardwareSsotSpec = (): Promise<HardwareSsotSpec> =>
  invokeCommand<HardwareSsotSpec>('get_hardware_ssot_spec');

export const listServiceManuals = (): Promise<ServiceManualCatalogItem[]> =>
  invokeCommand<ServiceManualCatalogItem[]>('list_service_manuals');

export const searchServiceManuals = (query: string): Promise<ManualSearchResult[]> =>
  invokeCommand<ManualSearchResult[]>('search_manuals', { query });

export const queryRagEngine = (query: string): Promise<RagQueryResult> =>
  invokeCommand<RagQueryResult>('query_rag_engine', { query });

export const listEngineeringDocs = (): Promise<EngineeringDocEntry[]> =>
  invokeCommand<EngineeringDocEntry[]>('list_engineering_docs');

export const readEngineeringDoc = (relativePath: string): Promise<string> =>
  invokeCommand<string>('read_engineering_doc', { relativePath });

export const decodeEngineeringLog = (relativePath: string): Promise<Am34BootLogReport> =>
  invokeCommand<Am34BootLogReport>('decode_engineering_log', { relativePath });

export const readHexDump = (offset: number, count: number): Promise<HexDumpRow[]> =>
  invokeCommand<HexDumpRow[]>('read_hex_dump', { offset, count });

export const installDesktopShortcut = (
  createDesktopShortcut = true,
): Promise<LauncherInstallReport> =>
  invokeCommand<LauncherInstallReport>('install_desktop_shortcut', {
    createDesktopShortcut,
  });

export interface WorkbenchPipelineListeners {
  readonly onTelemetry: (status: PicoDeviceStatus) => void;
  readonly onNvramLiveCapture: (evt: LiveNvramCaptureEvent) => void;
  readonly onNvramCleaned: (cleaned: boolean) => void;
  readonly onHddInspected: (inspected: boolean) => void;
  readonly onUnbrickCompleted: (completed: boolean) => void;
}

/** Binds all reactive pipeline state events cleanly for WorkbenchModeProvider. */
export async function subscribeWorkbenchPipelineEvents(
  listeners: WorkbenchPipelineListeners,
): Promise<() => void> {
  const unsubs = await Promise.all([
    listenEvent('pico://telemetry', listeners.onTelemetry),
    listenEvent('pico://nvram-live-capture', listeners.onNvramLiveCapture),
    listenEvent('nvram://cleaned', listeners.onNvramCleaned),
    listenEvent('hdd://inspected', listeners.onHddInspected),
    listenEvent('unbrick://completed', listeners.onUnbrickCompleted),
  ]);
  return () => unsubs.forEach(fn => fn());
}

export const emitNvramCleanedEvent = (cleaned: boolean): void =>
  emitLocalEvent('nvram://cleaned', cleaned);

export const emitHddInspectedEvent = (inspected: boolean): void =>
  emitLocalEvent('hdd://inspected', inspected);

export const emitUnbrickCompletedEvent = (completed: boolean): void =>
  emitLocalEvent('unbrick://completed', completed);
