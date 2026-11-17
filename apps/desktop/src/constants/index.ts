/**
 * Panadiga Unified Frontend Engineering Constants (SSOT)
 *
 * Modular hardware geometry, physical memory addresses, USB identity specs, and i18next-backed
 * SSOT specifications live in `./hardwareSpecs/` and `./hddCapacity.ts`.
 */

export * from './hexBookmarks';
export * from './hddCapacity';
export * from './runtimeDefaults';
export * from './uiLayout';
export * from './hardwareSpecs/catalogTokens';
export * from './hardwareSpecs/physicalConstants';
export * from './hardwareSpecs/ssotBuilder';

/** Live polling fallback interval in ms when reactive event streaming is unavailable */
export const TELEMETRY_POLL_INTERVAL_MS = 2000;

/** Default Toast feedback timeout in ms */
export const TOAST_DURATION_MS = 3000;

/** Duration in ms to show copy-to-clipboard confirmation state */
export const CLIPBOARD_FEEDBACK_MS = 2000;
