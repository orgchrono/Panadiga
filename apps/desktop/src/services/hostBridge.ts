/**
 * @file services/hostBridge.ts
 * @brief Zero-global-declaration runtime boundary for native WebView/WebBluetooth/test host bridges.
 *
 * Reads external host-injected objects via `Reflect.get` and narrows them exclusively through
 * structural runtime type guards so no `declare global`, `any`, or `as` casts leak into modules.
 */

import { isRecord } from '@/utils/typeGuards';

const TEST_IPC_BRIDGE_KEY = '__PANADIGA_TEST_IPC__';
const ANDROID_BRIDGE_KEY = '__PANADIGA_ANDROID_BRIDGE__';
const ANDROID_IR_BRIDGE_KEY = '__PANADIGA_IR_BRIDGE__';

export interface PanadigaTestIpcBridge {
  invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T>;
  listen<T>(eventName: string, handler: (payload: T) => void): (() => void) | Promise<() => void>;
}

export interface RawAndroidBridge {
  isAvailable(): boolean;
  getCapabilitiesJson(): string;
  requestPermissions(): boolean;
  listUsbOtgDevicesJson(): string;
  connectUsbOtg(targetDeviceName: string): string;
  disconnectUsbOtg(): void;
  sendUsbOtgCommand(command: string, timeoutMs: number): string;
  connectBleNus(timeoutMs: number): string;
  sendBleCommand(command: string, timeoutMs: number): string;
  disconnectBleNus(): void;
}

export interface RawAndroidIrBridge {
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

export interface BluetoothRemoteGATTCharacteristic extends EventTarget {
  startNotifications(): Promise<BluetoothRemoteGATTCharacteristic>;
  writeValue(value: BufferSource): Promise<void>;
  addEventListener(type: string, listener: (event: Event) => void): void;
}

export interface BluetoothRemoteGATTService {
  getCharacteristic(characteristic: string): Promise<BluetoothRemoteGATTCharacteristic>;
}

export interface BluetoothRemoteGATTServer {
  connected: boolean;
  connect(): Promise<BluetoothRemoteGATTServer>;
  disconnect(): void;
  getPrimaryService(service: string): Promise<BluetoothRemoteGATTService>;
}

export interface BluetoothDevice extends EventTarget {
  id: string;
  name?: string;
  gatt?: BluetoothRemoteGATTServer;
  addEventListener(type: string, listener: () => void): void;
}

export interface BluetoothRequestOptions {
  filters?: Array<{ namePrefix?: string; services?: string[] }>;
  optionalServices?: string[];
}

export interface BluetoothNavigator {
  requestDevice(options: BluetoothRequestOptions): Promise<BluetoothDevice>;
}

function hasMethods(candidate: Record<string, unknown>, methodNames: readonly string[]): boolean {
  return methodNames.every(name => typeof candidate[name] === 'function');
}

function readWindowHostValue(key: string): unknown {
  if (typeof window === 'undefined') {
    return undefined;
  }
  return Reflect.get(window, key);
}

function isTestIpcBridge(value: unknown): value is PanadigaTestIpcBridge {
  return isRecord(value) && hasMethods(value, ['invoke', 'listen']);
}

const ANDROID_BRIDGE_METHODS = [
  'isAvailable',
  'getCapabilitiesJson',
  'requestPermissions',
  'listUsbOtgDevicesJson',
  'connectUsbOtg',
  'disconnectUsbOtg',
  'sendUsbOtgCommand',
  'connectBleNus',
  'sendBleCommand',
  'disconnectBleNus',
] as const;

function isRawAndroidBridge(value: unknown): value is RawAndroidBridge {
  return isRecord(value) && hasMethods(value, ANDROID_BRIDGE_METHODS);
}

const ANDROID_IR_BRIDGE_METHODS = [
  'isAvailable',
  'getCapabilitiesJson',
  'getSettingsJson',
  'saveSettings',
  'saveCustomProfilesJson',
  'transmitPreparedJson',
] as const;

function isRawAndroidIrBridge(value: unknown): value is RawAndroidIrBridge {
  return isRecord(value) && hasMethods(value, ANDROID_IR_BRIDGE_METHODS);
}

function isBluetoothNavigator(value: unknown): value is BluetoothNavigator {
  return isRecord(value) && typeof value.requestDevice === 'function';
}

/** Resolves the injected unit/E2E IPC bridge when running in dev or headless test environments. */
export function resolveTestIpcBridge(): PanadigaTestIpcBridge | null {
  const isTestOrDevHarness = import.meta.env.DEV || import.meta.env.MODE === 'test';
  if (!isTestOrDevHarness) {
    return null;
  }
  const candidate = readWindowHostValue(TEST_IPC_BRIDGE_KEY);
  return isTestIpcBridge(candidate) ? candidate : null;
}

/** Resolves the native Android USB-OTG / BLE JavascriptInterface bridge if present and valid. */
export function resolveAndroidNativeBridge(): RawAndroidBridge | null {
  const candidate = readWindowHostValue(ANDROID_BRIDGE_KEY);
  return isRawAndroidBridge(candidate) ? candidate : null;
}

/** Resolves the native Android Infrared HAL JavascriptInterface bridge if present and valid. */
export function resolveAndroidIrBridge(): RawAndroidIrBridge | null {
  const candidate = readWindowHostValue(ANDROID_IR_BRIDGE_KEY);
  return isRawAndroidIrBridge(candidate) ? candidate : null;
}

/** Resolves Web Bluetooth navigator capability without mutating or extending global `Navigator`. */
export function resolveBluetoothNavigator(): BluetoothNavigator | null {
  if (typeof navigator === 'undefined') {
    return null;
  }
  const candidate: unknown = Reflect.get(navigator, 'bluetooth');
  return isBluetoothNavigator(candidate) ? candidate : null;
}

/** Subscribes to custom window events injected by native Android WebView dispatchers. */
export function subscribeWindowEvent(
  eventName: string,
  handler: (event: Event) => void,
): () => void {
  if (typeof window === 'undefined') {
    return () => {};
  }
  window.addEventListener(eventName, handler);
  return () => {
    window.removeEventListener(eventName, handler);
  };
}
