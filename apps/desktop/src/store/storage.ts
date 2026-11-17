/**
 * @file store/storage.ts
 * @brief Safe browser storage and DOM attribute helpers backing Redux workbench state.
 */

import { DEFAULT_DOCKS_STATE, DEFAULT_RUNTIME_SETTINGS } from '@/constants';
import type { RuntimeSettings } from '@/models';
import type {
  AccessibilitySettings,
  FontScaleMode,
  WorkbenchDocksState,
  WorkbenchExperienceMode,
  WorkbenchTheme,
} from '@/types/workbench';
import { runtimeSettingsSchema } from '@/utils/runtimeSettingsSchema';
import {
  isRecord,
  oneOfGuard,
  readBooleanProp,
  safeParseJson,
  safeParseWithSchema,
} from '@/utils/typeGuards';

export { DEFAULT_DOCKS_STATE };

export const STORAGE_MODE_KEY = 'panadiga.workbench.mode';
export const STORAGE_A11Y_KEY = 'panadiga.workbench.accessibility';
export const STORAGE_RUNTIME_SETTINGS_KEY = 'panadiga.workbench.runtime_settings';
export const STORAGE_COMPLIANCE_KEY = 'panadiga.workbench.compliance_ack_v1';
export const STORAGE_DOCKS_KEY = 'panadiga.workbench.docks_v1';
export const SPLIT_STORAGE_KEYS: readonly string[] = [
  'panadiga.split.hex',
  'panadiga.split.manuals',
  'panadiga.split.picobench',
];

export const DEFAULT_ACCESSIBILITY: AccessibilitySettings = Object.freeze({
  theme: 'dark-lab',
  colorblindSafe: false,
  reducedMotion: false,
  fontScale: 'normal',
});

const isWorkbenchTheme: (value: unknown) => value is WorkbenchTheme = oneOfGuard([
  'dark-lab',
  'high-contrast',
  'daylight-bench',
  'amber-crt',
] as const);

const isFontScaleMode: (value: unknown) => value is FontScaleMode = oneOfGuard([
  'normal',
  'large',
] as const);

export function readStorageString(key: string): string | null {
  if (typeof window === 'undefined') return null;
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}

export function writeStorageString(key: string, value: string): void {
  if (typeof window === 'undefined') return;
  try {
    window.localStorage.setItem(key, value);
  } catch {
    // ignore storage restrictions
  }
}

export function readStoragePercent(
  key: string | undefined,
  minPercent: number,
  maxPercent: number,
): number | null {
  if (!key) return null;
  const raw = readStorageString(key);
  if (!raw) return null;
  const parsed = Number.parseFloat(raw);
  if (!Number.isFinite(parsed) || parsed < minPercent || parsed > maxPercent) {
    return null;
  }
  return parsed;
}

export function readInitialWorkbenchMode(): WorkbenchExperienceMode {
  const stored = readStorageString(STORAGE_MODE_KEY);
  return stored === 'basic' || stored === 'advanced' ? stored : 'advanced';
}

export function readInitialAccessibility(): AccessibilitySettings {
  const raw = readStorageString(STORAGE_A11Y_KEY);
  if (raw) {
    const parsed = safeParseJson<Record<string, unknown> | null>(
      raw,
      (v): v is Record<string, unknown> | null => isRecord(v),
      null,
    );
    if (parsed) {
      return {
        theme: isWorkbenchTheme(parsed.theme) ? parsed.theme : 'dark-lab',
        colorblindSafe: Boolean(parsed.colorblindSafe),
        reducedMotion: Boolean(parsed.reducedMotion),
        fontScale: isFontScaleMode(parsed.fontScale) ? parsed.fontScale : 'normal',
      };
    }
  }
  const prefersReduced =
    typeof window !== 'undefined' &&
    typeof window.matchMedia === 'function' &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  return { ...DEFAULT_ACCESSIBILITY, reducedMotion: prefersReduced };
}

export function readFallbackRuntimeSettings(): RuntimeSettings {
  const raw = readStorageString(STORAGE_RUNTIME_SETTINGS_KEY);
  if (!raw) return DEFAULT_RUNTIME_SETTINGS;
  return safeParseWithSchema(raw, runtimeSettingsSchema, DEFAULT_RUNTIME_SETTINGS);
}

export function readInitialComplianceAccepted(): boolean {
  return readStorageString(STORAGE_COMPLIANCE_KEY) === 'accepted';
}

export function writeComplianceAccepted(accepted: boolean): void {
  if (accepted) {
    writeStorageString(STORAGE_COMPLIANCE_KEY, 'accepted');
  }
}

export function readInitialDocksState(): WorkbenchDocksState {
  const raw = readStorageString(STORAGE_DOCKS_KEY);
  if (!raw) return DEFAULT_DOCKS_STATE;
  const parsed = safeParseJson<Record<string, unknown> | null>(
    raw,
    (v): v is Record<string, unknown> | null => isRecord(v),
    null,
  );
  if (!parsed) return DEFAULT_DOCKS_STATE;
  return {
    leftOpen: readBooleanProp(parsed, 'leftOpen') ?? DEFAULT_DOCKS_STATE.leftOpen,
    rightOpen: readBooleanProp(parsed, 'rightOpen') ?? DEFAULT_DOCKS_STATE.rightOpen,
    bottomOpen: readBooleanProp(parsed, 'bottomOpen') ?? DEFAULT_DOCKS_STATE.bottomOpen,
    bottomExpanded:
      readBooleanProp(parsed, 'bottomExpanded') ?? DEFAULT_DOCKS_STATE.bottomExpanded,
  };
}

export function writeDocksState(docks: WorkbenchDocksState): void {
  writeStorageString(STORAGE_DOCKS_KEY, JSON.stringify(docks));
}

export function clearWorkbenchStorage(): void {
  if (typeof window === 'undefined') return;
  try {
    window.localStorage.removeItem(STORAGE_A11Y_KEY);
    window.localStorage.removeItem(STORAGE_MODE_KEY);
    window.localStorage.removeItem(STORAGE_RUNTIME_SETTINGS_KEY);
    window.localStorage.removeItem(STORAGE_DOCKS_KEY);
    SPLIT_STORAGE_KEYS.forEach(key => window.localStorage.removeItem(key));
  } catch {
    // ignore storage errors
  }
}

export function syncAccessibilityDom(accessibility: AccessibilitySettings): void {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  root.setAttribute('data-theme', accessibility.theme);
  root.setAttribute('data-colorblind', accessibility.colorblindSafe ? 'true' : 'false');
  root.setAttribute('data-reduced-motion', accessibility.reducedMotion ? 'true' : 'false');
  root.setAttribute('data-font-scale', accessibility.fontScale);
  root.style.setProperty('color-scheme', accessibility.theme === 'daylight-bench' ? 'light' : 'dark');
}
