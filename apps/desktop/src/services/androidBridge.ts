/**
 * Android Native Hardware Bridge Client (USB-C OTG CDC-ACM + Bluetooth LE NUS)
 * Wraps `window.__PANADIGA_ANDROID_BRIDGE__` injected by MainActivity.kt on Tauri Android APKs.
 */

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

interface RawAndroidBridge {
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

declare global {
  interface Window {
    __PANADIGA_ANDROID_BRIDGE__?: RawAndroidBridge;
  }
}

function getRawBridge(): RawAndroidBridge | null {
  if (typeof window === 'undefined') return null;
  const candidate = window.__PANADIGA_ANDROID_BRIDGE__;
  return candidate && typeof candidate.isAvailable === 'function' ? candidate : null;
}

export function isAndroidNativeBridgeAvailable(): boolean {
  return getRawBridge()?.isAvailable() ?? false;
}

function safeParseBridgeJson<T>(raw: string, fallback: T): T {
  try {
    return JSON.parse(raw) as T;
  } catch {
    return fallback;
  }
}

export function getAndroidCapabilities(): AndroidBridgeCapabilities | null {
  const bridge = getRawBridge();
  if (!bridge) return null;
  return safeParseBridgeJson<AndroidBridgeCapabilities | null>(bridge.getCapabilitiesJson(), null);
}

export function listAndroidUsbOtgDevices(): AndroidUsbDeviceInfo[] {
  const bridge = getRawBridge();
  if (!bridge) return [];
  return safeParseBridgeJson<AndroidUsbDeviceInfo[]>(bridge.listUsbOtgDevicesJson(), []);
}

export function connectAndroidUsbOtg(deviceName = ''): AndroidBridgeConnectResult {
  const bridge = getRawBridge();
  if (!bridge) {
    return { connected: false, message: 'Bridge USB OTG disponível apenas no APK Android.' };
  }
  return safeParseBridgeJson<AndroidBridgeConnectResult>(bridge.connectUsbOtg(deviceName), {
    connected: false,
    message: 'Resposta JSON inválida da bridge USB OTG Android.',
  });
}

export function sendAndroidUsbOtgCommand(
  command: string,
  timeoutMs = 3000
): AndroidBridgeCommandResult {
  const bridge = getRawBridge();
  if (!bridge) {
    return { ok: false, output: 'Bridge USB OTG indisponível.' };
  }
  return safeParseBridgeJson<AndroidBridgeCommandResult>(
    bridge.sendUsbOtgCommand(command, timeoutMs),
    { ok: false, output: 'Resposta JSON inválida do comando USB OTG.' }
  );
}

export function disconnectAndroidUsbOtg(): void {
  getRawBridge()?.disconnectUsbOtg();
}

export function connectAndroidBleNus(timeoutMs = 10000): AndroidBridgeConnectResult {
  const bridge = getRawBridge();
  if (!bridge) {
    return { connected: false, message: 'Bridge BLE nativo disponível apenas no APK Android.' };
  }
  return safeParseBridgeJson<AndroidBridgeConnectResult>(bridge.connectBleNus(timeoutMs), {
    connected: false,
    message: 'Resposta JSON inválida da bridge BLE Android.',
  });
}

export function sendAndroidBleCommand(
  command: string,
  timeoutMs = 4000
): AndroidBridgeCommandResult {
  const bridge = getRawBridge();
  if (!bridge) {
    return { ok: false, output: 'Bridge BLE nativo indisponível.' };
  }
  return safeParseBridgeJson<AndroidBridgeCommandResult>(
    bridge.sendBleCommand(command, timeoutMs),
    { ok: false, output: 'Resposta JSON inválida do comando BLE Android.' }
  );
}

export function disconnectAndroidBleNus(): void {
  getRawBridge()?.disconnectBleNus();
}
