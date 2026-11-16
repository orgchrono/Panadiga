/**
 * Build-target gating isolating Android-only DIGA Commander features from Desktop binaries.
 *
 * When Vite compiles production Desktop assets (`npm run build`), `__PANADIGA_ANDROID_APK__`
 * and `import.meta.env.PROD` are constant-folded to `false` and `true`, allowing Rollup to
 * completely dead-code-eliminate the DIGA Commander view, catalog, and IR transport modules.
 */

const ANDROID_RUNTIME_OVERRIDE_KEY = '__PANADIGA_ANDROID_RUNTIME__';

export const IS_ANDROID_APK_BUILD: boolean =
  typeof __PANADIGA_ANDROID_APK__ !== 'undefined' && __PANADIGA_ANDROID_APK__ === true;

/** Returns true strictly in Android APK builds (or Vitest/Playwright dev test fixtures). */
export function isAndroidCommanderEnabled(): boolean {
  if (IS_ANDROID_APK_BUILD) {
    return true;
  }
  if (!import.meta.env.PROD && typeof window !== 'undefined') {
    const g = window as unknown as Record<string, unknown>;
    return g[ANDROID_RUNTIME_OVERRIDE_KEY] === true;
  }
  return false;
}

/** Test-only toggle for Vitest/Playwright suites simulating Android APK runtime inside jsdom/dev. */
export function setAndroidRuntimeTestOverride(enabled: boolean): void {
  if (!import.meta.env.PROD && typeof window !== 'undefined') {
    const g = window as unknown as Record<string, unknown>;
    g[ANDROID_RUNTIME_OVERRIDE_KEY] = enabled;
  }
}
