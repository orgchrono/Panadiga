import type {
  DynamicKeymapProfile,
  IrCatalogStats,
  IrDeviceInfo,
  IrFrequencyRange,
  IrPreparedSignal,
  IrTimingOverride,
  KeyProtocol,
  PanasonicCatalogCodeset,
  PanasonicCatalogSummary,
  PanasonicCategory,
  VerifiedIrMacro,
} from '@/models/ir';
import { invokeCommand } from './ipc';

export async function irCatalogStats(): Promise<IrCatalogStats> {
  return invokeCommand<IrCatalogStats>('ir_catalog_stats');
}

export async function irSearchCatalog(
  query: string,
  category: PanasonicCategory,
  limit = 60,
): Promise<PanasonicCatalogSummary[]> {
  return invokeCommand<PanasonicCatalogSummary[]>('ir_search_catalog', {
    query,
    category,
    limit,
  });
}

export async function irAssembleCodeset(
  codesetId: string,
): Promise<{ codeset: PanasonicCatalogCodeset; profile: DynamicKeymapProfile }> {
  return invokeCommand<{ codeset: PanasonicCatalogCodeset; profile: DynamicKeymapProfile }>(
    'ir_assemble_codeset',
    { codesetId },
  );
}

export async function irImportProfile(
  format: 'irplus' | 'flipper',
  content: string,
  fallbackName?: string,
): Promise<DynamicKeymapProfile> {
  return invokeCommand<DynamicKeymapProfile>('ir_import_profile', {
    format,
    content,
    fallbackName: fallbackName ?? null,
  });
}

export async function irExportIrplus(profile: DynamicKeymapProfile): Promise<string> {
  return invokeCommand<string>('ir_export_irplus', { profile });
}

export async function irPrepareTransmission(params: {
  protocol: KeyProtocol;
  device: number;
  subdevice: number;
  obc: number;
  respectRemoteMode: boolean;
  remoteUnit: 1 | 2 | 3;
  prontoHex?: string | null;
  ranges: IrFrequencyRange[];
  deviceInfo: IrDeviceInfo;
  hasSecIrNode: boolean;
  hasFpgaNode: boolean;
  unitOverride: IrTimingOverride;
}): Promise<IrPreparedSignal> {
  return invokeCommand<IrPreparedSignal>('ir_prepare_transmission', { request: params });
}

export async function irGetVerifiedMacros(): Promise<VerifiedIrMacro[]> {
  return invokeCommand<VerifiedIrMacro[]>('ir_get_verified_macros');
}
