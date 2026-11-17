import {
  getAndroidIrSettings,
  saveAndroidCustomProfilesJson,
  saveAndroidIrSettings,
} from '@/services/androidIrBridge';
import type { IrDispatchOutcome } from '@/services/irTransportService';
import type {
  DynamicKeymapProfile,
  DynamicRemoteKey,
  IrMacroStep,
  IrTimingOverride,
  RemoteMode,
} from '@/models/ir';

export const DECK_CODESET_MAP: Record<Exclude<RemoteMode, 'CATALOG'>, string> = {
  DVD: 'curated_diga_eh55_eur7659',
  VCR: 'curated_panasonic_vcr_144',
  TV: 'curated_viera_tv_128',
};

export const COMBO_DECK_CODESET_ID = 'curated_diga_combo_vhs_dvd';

export interface StoredIrSettingsSnapshot {
  timingUnitOverride: IrTimingOverride;
  repeatCount: number;
  savedProfiles: DynamicKeymapProfile[];
}

export function getErrorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

export function loadStoredIrSettings(): StoredIrSettingsSnapshot | null {
  const settings = getAndroidIrSettings();
  if (!settings) return null;

  let savedProfiles: DynamicKeymapProfile[] = [];
  try {
    const parsed = JSON.parse(settings.customProfilesJson) as DynamicKeymapProfile[];
    if (Array.isArray(parsed)) {
      savedProfiles = parsed;
    }
  } catch {
    // Ignore empty or corrupt stored custom profiles
  }

  return {
    timingUnitOverride: settings.timingUnitOverride,
    repeatCount: settings.repeatCount,
    savedProfiles,
  };
}

export function persistIrSettings(unit: IrTimingOverride, repeatCount: number): void {
  saveAndroidIrSettings(unit, repeatCount);
}

export function upsertSavedProfiles(
  activeProfile: DynamicKeymapProfile,
  savedProfiles: readonly DynamicKeymapProfile[],
): DynamicKeymapProfile[] {
  const others = savedProfiles.filter(p => p.id !== activeProfile.id);
  const updated = [activeProfile, ...others];
  saveAndroidCustomProfilesJson(JSON.stringify(updated));
  return updated;
}

export function createBlankCustomKey(): DynamicRemoteKey {
  return {
    id: `custom_${Date.now()}`,
    label: 'Custom Key',
    section: 'CUSTOM',
    protocol: 'KASEIKYO',
    device: 0xb0,
    subdevice: 0x00,
    obc: 0x92,
    respect_remote_mode: true,
    pronto_hex: null,
    tone: 'NEUTRAL',
    confirm_required: false,
    wire_preview: '02 20 B0 00 92 22',
  };
}

export function replaceProfileKey(
  profile: DynamicKeymapProfile,
  updated: DynamicRemoteKey,
): DynamicKeymapProfile {
  return {
    ...profile,
    is_user_modified: true,
    keys: profile.keys.map(k => (k.id === updated.id ? updated : k)),
  };
}

export function removeProfileKey(
  profile: DynamicKeymapProfile,
  keyId: string,
): DynamicKeymapProfile {
  return {
    ...profile,
    is_user_modified: true,
    keys: profile.keys.filter(k => k.id !== keyId),
  };
}

export function buildKeyTelemetryPayload(
  label: string,
  repeatCount: number,
  outcome: IrDispatchOutcome,
) {
  return {
    primary: `TX: ${label.toUpperCase()}`,
    secondary: `${outcome.prepared.carrier_hz} Hz · ${outcome.prepared.effective_unit} · x${repeatCount}`,
    wireHex: outcome.prepared.wire_hex,
    prontoHex: outcome.prepared.pronto_hex,
    backendUsed: outcome.backendUsed,
  };
}

export function buildMacroStepTelemetryPayload(
  title: string,
  stepIdx: number,
  total: number,
  step: IrMacroStep,
  outcome: IrDispatchOutcome,
) {
  return {
    primary: `${title} [${stepIdx + 1}/${total}]`,
    secondary: `${step.label} · delay ${step.post_delay_ms}ms`,
    wireHex: outcome.prepared.wire_hex,
    prontoHex: outcome.prepared.pronto_hex,
    backendUsed: outcome.backendUsed,
  };
}
