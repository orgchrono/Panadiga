/**
 * @file services/ssotStudioService.ts
 * @brief Typed Tauri 2 IPC client & RGB565 palette converter for DigaOS Visual SSOT Schema Studio.
 */

import type { DigaOsConfigSpec } from '@/models/generated/digaos';
import { mintOperationNonceId } from './benchStreamService';
import { invokeCommand } from './ipc';

export interface SsotStudioBundle {
  readonly contractHash: number;
  readonly contractHashHex: string;
  readonly config: DigaOsConfigSpec;
  readonly rawJson: string;
  readonly configPath: string;
}

export interface SsotStudioValidationResult {
  readonly valid: boolean;
  readonly errors: readonly string[];
  readonly contractHash: number;
  readonly contractHashHex: string;
}

export interface SsotStudioSaveResult {
  readonly saved: boolean;
  readonly contractHash: number;
  readonly contractHashHex: string;
  readonly updatedPaths: readonly string[];
  readonly message: string;
}

const RGB565_R_SHIFT = 11;
const RGB565_G_SHIFT = 5;
const RGB565_R_MASK = 0x1f;
const RGB565_G_MASK = 0x3f;
const RGB565_B_MASK = 0x1f;
const CHANNEL_MAX = 255;

export function rgb565ToCssHex(rawHex: string): string {
  const clean = rawHex.trim().replace(/^0x/i, '');
  const parsed = Number.parseInt(clean, 16);
  if (!Number.isFinite(parsed) || Number.isNaN(parsed)) {
    return '#1e293b';
  }
  const r5 = (parsed >> RGB565_R_SHIFT) & RGB565_R_MASK;
  const g6 = (parsed >> RGB565_G_SHIFT) & RGB565_G_MASK;
  const b5 = parsed & RGB565_B_MASK;
  const r8 = Math.round((r5 * CHANNEL_MAX) / RGB565_R_MASK);
  const g8 = Math.round((g6 * CHANNEL_MAX) / RGB565_G_MASK);
  const b8 = Math.round((b5 * CHANNEL_MAX) / RGB565_B_MASK);
  const toHex = (v: number): string => v.toString(16).padStart(2, '0');
  return `#${toHex(r8)}${toHex(g8)}${toHex(b8)}`;
}

export const fetchSsotStudioBundle = (): Promise<SsotStudioBundle> =>
  invokeCommand<SsotStudioBundle>('ssot_studio_get_bundle');

export const validateSsotCustomConfig = (
  configJson: string,
): Promise<SsotStudioValidationResult> =>
  invokeCommand<SsotStudioValidationResult>('ssot_studio_validate_config', { configJson });

export async function saveSsotCustomConfig(
  configJson: string,
  contractHashHex: string,
  nonceId?: string,
): Promise<SsotStudioSaveResult> {
  const resolvedNonce = await mintOperationNonceId(
    'ram_probe',
    'digaos_ssot',
    contractHashHex,
    nonceId,
  );
  return invokeCommand<SsotStudioSaveResult>('ssot_studio_save_custom_config', {
    configJson,
    ...(resolvedNonce ? { nonceId: resolvedNonce } : {}),
  });
}
