/**
 * Android Native Infrared HAL Client.
 * Wraps `window.__PANADIGA_IR_BRIDGE__` registered in MainActivity.kt on Android APK builds.
 */

import type {
  AndroidIrCapabilities,
  AndroidIrStoredSettings,
  AndroidIrTransmitResult,
  IrTimingOverride,
} from '@/models/ir';

interface RawAndroidIrBridge {
  isAvailable(): boolean;
  getCapabilitiesJson(): string;
  getSettingsJson(): string;
  saveSettings(timingUnitOverride: string, repeatCount: number): string;
  saveCustomProfilesJson(profilesJson: string): boolean;
  transmitPreparedJson(
    carrierHz: number,
    encodedPatternJson: string,
    sysfsPayload: string,
    repeatCount: number,
  ): string;
}

declare global {
  interface Window {
    __PANADIGA_IR_BRIDGE__?: RawAndroidIrBridge;
  }
}

function getRawIrBridge(): RawAndroidIrBridge | null {
  if (typeof window === 'undefined') return null;
  const candidate = window.__PANADIGA_IR_BRIDGE__;
  return candidate && typeof candidate.isAvailable === 'function' ? candidate : null;
}

function parseIrJson<T>(raw: string, fallback: T): T {
  try {
    return JSON.parse(raw) as T;
  } catch {
    return fallback;
  }
}

export function isAndroidIrBridgeAvailable(): boolean {
  return getRawIrBridge()?.isAvailable() ?? false;
}

export function getAndroidIrCapabilities(): AndroidIrCapabilities | null {
  const bridge = getRawIrBridge();
  if (!bridge) return null;
  return parseIrJson<AndroidIrCapabilities | null>(bridge.getCapabilitiesJson(), null);
}

export function getAndroidIrSettings(): AndroidIrStoredSettings | null {
  const bridge = getRawIrBridge();
  if (!bridge) return null;
  return parseIrJson<AndroidIrStoredSettings | null>(bridge.getSettingsJson(), null);
}

export function saveAndroidIrSettings(
  timingUnitOverride: IrTimingOverride,
  repeatCount: number,
): boolean {
  const bridge = getRawIrBridge();
  if (!bridge) return false;
  const res = parseIrJson<{ ok?: boolean }>(
    bridge.saveSettings(timingUnitOverride, repeatCount),
    {},
  );
  return res.ok === true;
}

export function saveAndroidCustomProfilesJson(profilesJson: string): boolean {
  return getRawIrBridge()?.saveCustomProfilesJson(profilesJson) ?? false;
}

export function transmitViaAndroidIrHal(
  carrierHz: number,
  encodedPattern: readonly number[],
  sysfsPayload: string,
  repeatCount: number,
): AndroidIrTransmitResult {
  const bridge = getRawIrBridge();
  if (!bridge) {
    return {
      ok: false,
      error: 'Bridge IR Android indisponível neste ambiente.',
    };
  }
  return parseIrJson<AndroidIrTransmitResult>(
    bridge.transmitPreparedJson(
      carrierHz,
      JSON.stringify(encodedPattern),
      sysfsPayload,
      repeatCount,
    ),
    { ok: false, error: 'Resposta JSON inválida da HAL Android IR.' },
  );
}
