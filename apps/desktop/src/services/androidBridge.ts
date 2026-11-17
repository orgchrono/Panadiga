/**
 * Android Native Hardware Bridge Client (USB-C OTG CDC-ACM + Bluetooth LE NUS)
 * Wraps `window.__PANADIGA_ANDROID_BRIDGE__` via `resolveAndroidNativeBridge()` + strict guards.
 */

import { translate } from '@/i18n/useTranslation';
import { isRecord, safeParseJson } from '@/utils/typeGuards';
import { resolveAndroidNativeBridge } from './hostBridge';

const DEFAULT_USB_TIMEOUT_MS = 3000;
const DEFAULT_BLE_CONNECT_TIMEOUT_MS = 10000;
const DEFAULT_BLE_CMD_TIMEOUT_MS = 4000;

export interface AndroidBridgeCapabilities {
  platform: 'android';
  usbHostSupported: boolean;
  bleSupported: boolean;
  bluetoothEnabled: boolean;
}

export interface AndroidUsbDeviceInfo {
  deviceName: string;
  vendorId: number;
  productId: number;
  productName: string;
  isRaspberryPi: boolean;
  isBootsel: boolean;
}

export interface AndroidBridgeConnectResult {
  connected: boolean;
  deviceName?: string;
  message: string;
}

export interface AndroidBridgeCommandResult {
  ok: boolean;
  output: string;
}

function isAndroidBridgeCapabilities(value: unknown): value is AndroidBridgeCapabilities {
  return (
    isRecord(value) &&
    value.platform === 'android' &&
    typeof value.usbHostSupported === 'boolean' &&
    typeof value.bleSupported === 'boolean' &&
    typeof value.bluetoothEnabled === 'boolean'
  );
}

function isAndroidUsbDeviceInfo(value: unknown): value is AndroidUsbDeviceInfo {
  return (
    isRecord(value) &&
    typeof value.deviceName === 'string' &&
    typeof value.vendorId === 'number' &&
    typeof value.productId === 'number' &&
    typeof value.productName === 'string' &&
    typeof value.isRaspberryPi === 'boolean' &&
    typeof value.isBootsel === 'boolean'
  );
}

function isAndroidUsbDeviceList(value: unknown): value is AndroidUsbDeviceInfo[] {
  return Array.isArray(value) && value.every(isAndroidUsbDeviceInfo);
}

function isAndroidBridgeConnectResult(value: unknown): value is AndroidBridgeConnectResult {
  return (
    isRecord(value) &&
    typeof value.connected === 'boolean' &&
    typeof value.message === 'string' &&
    (value.deviceName === undefined || typeof value.deviceName === 'string')
  );
}

function isAndroidBridgeCommandResult(value: unknown): value is AndroidBridgeCommandResult {
  return isRecord(value) && typeof value.ok === 'boolean' && typeof value.output === 'string';
}

export function isAndroidNativeBridgeAvailable(): boolean {
  return resolveAndroidNativeBridge()?.isAvailable() ?? false;
}

export function getAndroidCapabilities(): AndroidBridgeCapabilities | null {
  const bridge = resolveAndroidNativeBridge();
  if (!bridge) return null;
  return safeParseJson<AndroidBridgeCapabilities | null>(
    bridge.getCapabilitiesJson(),
    (value): value is AndroidBridgeCapabilities | null => isAndroidBridgeCapabilities(value),
    null,
  );
}

export function listAndroidUsbOtgDevices(): AndroidUsbDeviceInfo[] {
  const bridge = resolveAndroidNativeBridge();
  if (!bridge) return [];
  return safeParseJson(bridge.listUsbOtgDevicesJson(), isAndroidUsbDeviceList, []);
}

export function connectAndroidUsbOtg(deviceName = ''): AndroidBridgeConnectResult {
  const bridge = resolveAndroidNativeBridge();
  if (!bridge) {
    return { connected: false, message: translate('errors.androidBridge.usbOtgDesktopOnly') };
  }
  return safeParseJson(bridge.connectUsbOtg(deviceName), isAndroidBridgeConnectResult, {
    connected: false,
    message: translate('errors.androidBridge.usbOtgInvalidConnectJson'),
  });
}

export function sendAndroidUsbOtgCommand(
  command: string,
  timeoutMs = DEFAULT_USB_TIMEOUT_MS,
): AndroidBridgeCommandResult {
  const bridge = resolveAndroidNativeBridge();
  if (!bridge) {
    return { ok: false, output: translate('errors.androidBridge.usbOtgUnavailable') };
  }
  return safeParseJson(bridge.sendUsbOtgCommand(command, timeoutMs), isAndroidBridgeCommandResult, {
    ok: false,
    output: translate('errors.androidBridge.usbOtgInvalidCmdJson'),
  });
}

export function disconnectAndroidUsbOtg(): void {
  resolveAndroidNativeBridge()?.disconnectUsbOtg();
}

export function connectAndroidBleNus(
  timeoutMs = DEFAULT_BLE_CONNECT_TIMEOUT_MS,
): AndroidBridgeConnectResult {
  const bridge = resolveAndroidNativeBridge();
  if (!bridge) {
    return { connected: false, message: translate('errors.androidBridge.bleDesktopOnly') };
  }
  return safeParseJson(bridge.connectBleNus(timeoutMs), isAndroidBridgeConnectResult, {
    connected: false,
    message: translate('errors.androidBridge.bleInvalidConnectJson'),
  });
}

export function sendAndroidBleCommand(
  command: string,
  timeoutMs = DEFAULT_BLE_CMD_TIMEOUT_MS,
): AndroidBridgeCommandResult {
  const bridge = resolveAndroidNativeBridge();
  if (!bridge) {
    return { ok: false, output: translate('errors.androidBridge.bleUnavailable') };
  }
  return safeParseJson(bridge.sendBleCommand(command, timeoutMs), isAndroidBridgeCommandResult, {
    ok: false,
    output: translate('errors.androidBridge.bleInvalidCmdJson'),
  });
}

export function disconnectAndroidBleNus(): void {
  resolveAndroidNativeBridge()?.disconnectBleNus();
}
