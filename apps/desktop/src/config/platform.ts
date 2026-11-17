/**
 * @file config/platform.ts
 * @brief Compile-time & guarded dev/test platform detection for Android-only UI modules.
 *
 * When Vite compiles production Desktop assets (`npm run build`), `import.meta.env.MODE === 'android'`
 * and `import.meta.env.PROD` are constant-folded to `false` and `true`, allowing Rollup to
 * completely eliminate Android-only views, IR codebases, and native Android bridge wrappers.
 */

export const ANDROID_RUNTIME_OVERRIDE_KEY = '__PANADIGA_ANDROID_RUNTIME__';

export const IS_ANDROID_APK_BUILD: boolean = import.meta.env.MODE === 'android';

/** Returns true strictly in Android APK builds (or Vitest/Playwright dev test fixtures). */
export function isAndroidCommanderEnabled(): boolean {
  if (IS_ANDROID_APK_BUILD) {
    return true;
  }
  if (!import.meta.env.PROD && typeof window !== 'undefined') {
    const flag: unknown = Reflect.get(window, ANDROID_RUNTIME_OVERRIDE_KEY);
    return flag === true;
  }
  return false;
}
