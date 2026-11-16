/**
 * @file store/storage.ts
 * @brief Safe browser storage and DOM attribute helpers backing Redux workbench state.
 */

import type { AccessibilitySettings, WorkbenchExperienceMode, WorkbenchTheme, FontScaleMode } from '@/types/workbench';

export const STORAGE_MODE_KEY = 'panadiga.workbench.mode';
export const STORAGE_A11Y_KEY = 'panadiga.workbench.accessibility';
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

function readStorageString(key: string): string | null {
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

export function readInitialWorkbenchMode(): WorkbenchExperienceMode {
  const stored = readStorageString(STORAGE_MODE_KEY);
  return stored === 'basic' || stored === 'advanced' ? stored : 'advanced';
}

export function readInitialAccessibility(): AccessibilitySettings {
  const raw = readStorageString(STORAGE_A11Y_KEY);
  if (raw) {
    try {
      const parsed = JSON.parse(raw) as Partial<AccessibilitySettings>;
      const validTheme: WorkbenchTheme =
        parsed.theme === 'dark-lab' ||
        parsed.theme === 'high-contrast' ||
        parsed.theme === 'daylight-bench' ||
        parsed.theme === 'amber-crt'
          ? parsed.theme
          : 'dark-lab';
      const validScale: FontScaleMode = parsed.fontScale === 'large' ? 'large' : 'normal';
      return {
        theme: validTheme,
        colorblindSafe: Boolean(parsed.colorblindSafe),
        reducedMotion: Boolean(parsed.reducedMotion),
        fontScale: validScale,
      };
    } catch {
      // fall through to system defaults
    }
  }
  const prefersReduced =
    typeof window !== 'undefined' &&
    typeof window.matchMedia === 'function' &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  return { ...DEFAULT_ACCESSIBILITY, reducedMotion: prefersReduced };
}

export function clearWorkbenchStorage(): void {
  if (typeof window === 'undefined') return;
  try {
    window.localStorage.removeItem(STORAGE_A11Y_KEY);
    window.localStorage.removeItem(STORAGE_MODE_KEY);
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
  root.style.colorScheme = accessibility.theme === 'daylight-bench' ? 'light' : 'dark';
}
