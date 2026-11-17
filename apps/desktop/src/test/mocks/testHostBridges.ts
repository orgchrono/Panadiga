/**
 * @file test/mocks/testHostBridges.ts
 * @brief Test-only global host bridge installers (eliminates `declare global` from production code).
 */

import { ANDROID_RUNTIME_OVERRIDE_KEY } from '@/config/platform';
import type { PanadigaTestIpcBridge, RawAndroidBridge, RawAndroidIrBridge } from '@/services/hostBridge';
import { STORAGE_COMPLIANCE_KEY } from '@/store/storage';

function writeWindowProp(key: string, value: unknown): void {
  if (typeof window === 'undefined') return;
  if (value === undefined) {
    Reflect.deleteProperty(window, key);
  } else {
    Reflect.set(window, key, value);
  }
}

export function getInstalledTestIpcBridge(): unknown {
  if (typeof window === 'undefined') return undefined;
  return Reflect.get(window, '__PANADIGA_TEST_IPC__');
}

export function installTestIpcBridge(bridge: PanadigaTestIpcBridge | unknown): void {
  writeWindowProp('__PANADIGA_TEST_IPC__', bridge);
  if (typeof window !== 'undefined' && typeof window.localStorage !== 'undefined') {
    try {
      if (window.localStorage.getItem(STORAGE_COMPLIANCE_KEY) === null) {
        window.localStorage.setItem(STORAGE_COMPLIANCE_KEY, 'accepted');
      }
    } catch {
      // ignore storage errors
    }
  }
}

export function setAndroidRuntimeTestOverride(enabled: boolean): void {
  writeWindowProp(ANDROID_RUNTIME_OVERRIDE_KEY, enabled);
}

export function installAndroidIrTestBridge(bridge?: RawAndroidIrBridge): void {
  writeWindowProp('__PANADIGA_IR_BRIDGE__', bridge);
}

export function installAndroidNativeTestBridge(bridge?: RawAndroidBridge): void {
  writeWindowProp('__PANADIGA_ANDROID_BRIDGE__', bridge);
}
