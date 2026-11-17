/**
 * @file bleService.ts
 * @brief Bluetooth Low Energy (BLE) Nordic UART Service (NUS) Client.
 */

import { translate } from '@/i18n/useTranslation';
import {
  connectAndroidBleNus,
  disconnectAndroidBleNus,
  getAndroidCapabilities,
  isAndroidNativeBridgeAvailable,
  sendAndroidBleCommand,
} from './androidBridge';
import {
  resolveBluetoothNavigator,
  subscribeWindowEvent,
  type BluetoothDevice,
  type BluetoothRemoteGATTCharacteristic,
  type BluetoothRemoteGATTServer,
} from './hostBridge';

export const NUS_SERVICE_UUID = '6e400001-b5a3-f393-e0a9-e50e24dcca9e';
export const NUS_RX_CHARACTERISTIC_UUID = '6e400002-b5a3-f393-e0a9-e50e24dcca9e';
export const NUS_TX_CHARACTERISTIC_UUID = '6e400003-b5a3-f393-e0a9-e50e24dcca9e';

const BLE_ANDROID_CONNECT_TIMEOUT_MS = 10000;
const BLE_CMD_TIMEOUT_MS = 4000;
const BLE_REPLY_WAIT_CAP_MS = 1500;

export interface BleDeviceInfo {
  id: string;
  name: string;
  connected: boolean;
}

export type BleDataCallback = (data: string) => void;

export interface BleUartClient {
  readonly isSupported: () => boolean;
  readonly isConnected: () => boolean;
  readonly connect: () => Promise<BleDeviceInfo>;
  readonly send: (message: string) => Promise<void>;
  readonly sendAndAwaitReply: (command: string, timeoutMs?: number) => Promise<string>;
  readonly onReceive: (callback: BleDataCallback) => () => void;
  readonly disconnect: () => void;
}

interface BleActiveSession {
  readonly device: BluetoothDevice | null;
  readonly server: BluetoothRemoteGATTServer | null;
  readonly rxCharacteristic: BluetoothRemoteGATTCharacteristic | null;
  readonly txCharacteristic: BluetoothRemoteGATTCharacteristic | null;
  readonly androidNativeConnected: boolean;
  readonly unsubAndroidRx: (() => void) | null;
}

function extractCharacteristicDataView(event: Event): DataView | null {
  const target: EventTarget | null = event.target;
  if (!target) {
    return null;
  }
  const value: unknown = Reflect.get(target, 'value');
  return value instanceof DataView ? value : null;
}

function isReplyComplete(buffer: string): boolean {
  return (
    buffer.includes('PANADIGA>') ||
    buffer.includes('P7501>') ||
    buffer.includes(': OK') ||
    buffer.includes('FAILED')
  );
}

/**
 * Pure closure factory encapsulating Web Bluetooth & Android BLE NUS transport state
 * without OOP classes, `let` bindings, or shared module globals.
 */
export function createBleUartService(): BleUartClient {
  const receiveCallbacks = new Set<BleDataCallback>();
  const sessions = new Map<'active', BleActiveSession>();

  const emitChunk = (chunk: string): void => {
    receiveCallbacks.forEach(cb => cb(chunk));
  };

  const handleAndroidRxEvent = (event: Event): void => {
    if (!(event instanceof CustomEvent)) {
      return;
    }
    const detail: unknown = event.detail;
    if (typeof detail === 'string') {
      emitChunk(detail);
    }
  };

  const cleanup = (): void => {
    const current = sessions.get('active');
    current?.unsubAndroidRx?.();
    sessions.delete('active');
  };

  const isSupported = (): boolean => {
    if (isAndroidNativeBridgeAvailable()) {
      return Boolean(getAndroidCapabilities()?.bleSupported);
    }
    return resolveBluetoothNavigator() !== null;
  };

  const isConnected = (): boolean => {
    const current = sessions.get('active');
    return Boolean(current?.androidNativeConnected || current?.server?.connected);
  };

  const onReceive = (callback: BleDataCallback): (() => void) => {
    receiveCallbacks.add(callback);
    return () => {
      receiveCallbacks.delete(callback);
    };
  };

  const connect = async (): Promise<BleDeviceInfo> => {
    if (isAndroidNativeBridgeAvailable()) {
      cleanup();
      const unsubAndroidRx = subscribeWindowEvent('panadiga-ble-rx', handleAndroidRxEvent);
      const res = connectAndroidBleNus(BLE_ANDROID_CONNECT_TIMEOUT_MS);
      if (!res.connected) {
        unsubAndroidRx();
        throw new Error(res.message || translate('errors.ble.androidConnectFailed'));
      }
      sessions.set('active', {
        device: null,
        server: null,
        rxCharacteristic: null,
        txCharacteristic: null,
        androidNativeConnected: true,
        unsubAndroidRx,
      });
      return {
        id: res.deviceName || 'ANDROID-BLE-PICO-W',
        name: res.deviceName || 'Pico W BLE (Android GATT)',
        connected: true,
      };
    }

    const bluetooth = resolveBluetoothNavigator();
    if (!bluetooth) {
      throw new Error(translate('errors.ble.webBluetoothUnavailable'));
    }

    try {
      cleanup();
      const device = await bluetooth.requestDevice({
        filters: [
          { namePrefix: 'Pico' },
          { namePrefix: 'Panadiga' },
          { services: [NUS_SERVICE_UUID] },
        ],
        optionalServices: [NUS_SERVICE_UUID],
      });

      device.addEventListener('gattserverdisconnected', cleanup);
      if (!device.gatt) {
        throw new Error(translate('errors.ble.gattUnavailable'));
      }

      const server = await device.gatt.connect();
      const service = await server.getPrimaryService(NUS_SERVICE_UUID);
      const rxCharacteristic = await service.getCharacteristic(NUS_RX_CHARACTERISTIC_UUID);
      const txCharacteristic = await service.getCharacteristic(NUS_TX_CHARACTERISTIC_UUID);

      await txCharacteristic.startNotifications();
      txCharacteristic.addEventListener('characteristicvaluechanged', (event: Event) => {
        const value = extractCharacteristicDataView(event);
        if (value) {
          emitChunk(new TextDecoder('utf-8').decode(value));
        }
      });

      sessions.set('active', {
        device,
        server,
        rxCharacteristic,
        txCharacteristic,
        androidNativeConnected: false,
        unsubAndroidRx: null,
      });

      return {
        id: device.id,
        name: device.name || 'Pico W P7501 UART / Panadiga',
        connected: true,
      };
    } catch (err: unknown) {
      cleanup();
      const reason = err instanceof Error ? err.message : String(err);
      throw new Error(translate('errors.ble.connectFailed', { reason }));
    }
  };

  const send = async (message: string): Promise<void> => {
    const active = sessions.get('active');
    if (active?.androidNativeConnected && isAndroidNativeBridgeAvailable()) {
      const res = sendAndroidBleCommand(message.trim(), BLE_CMD_TIMEOUT_MS);
      if (!res.ok) {
        throw new Error(res.output || translate('errors.ble.gattSendFailed'));
      }
      if (res.output) {
        emitChunk(res.output);
      }
      return;
    }

    if (!resolveBluetoothNavigator()) {
      throw new Error(translate('errors.ble.channelNotConnected'));
    }

    if (!active?.rxCharacteristic) {
      throw new Error(translate('errors.ble.channelUnavailableSend'));
    }

    const data = new TextEncoder().encode(message.endsWith('\n') ? message : `${message}\n`);
    await active.rxCharacteristic.writeValue(data);
  };

  const sendAndAwaitReply = async (
    command: string,
    timeoutMs = BLE_CMD_TIMEOUT_MS,
  ): Promise<string> => {
    const active = sessions.get('active');
    if (active?.androidNativeConnected && isAndroidNativeBridgeAvailable()) {
      const res = sendAndroidBleCommand(command.trim(), timeoutMs);
      if (!res.ok) {
        throw new Error(res.output || translate('errors.ble.gattReplyFailed'));
      }
      return res.output;
    }

    const chunks = new Map<'buf', string>();
    const cleanupRef = new Map<'off', () => void>();

    return new Promise<string>((resolve, reject) => {
      const settle = (result: string) => {
        cleanupRef.get('off')?.();
        resolve(result);
      };
      const timer = setTimeout(() => {
        settle((chunks.get('buf') ?? '').trim() || `${command}: OK`);
      }, Math.min(timeoutMs, BLE_REPLY_WAIT_CAP_MS));

      const off = onReceive(chunk => {
        const next = `${chunks.get('buf') ?? ''}${chunk}`;
        chunks.set('buf', next);
        if (isReplyComplete(next)) {
          clearTimeout(timer);
          settle(next.trim());
        }
      });
      cleanupRef.set('off', off);

      send(command).catch((err: unknown) => {
        clearTimeout(timer);
        off();
        reject(err instanceof Error ? err : new Error(String(err)));
      });
    });
  };

  const disconnect = (): void => {
    const current = sessions.get('active');
    if (current?.androidNativeConnected) {
      disconnectAndroidBleNus();
    }
    if (current?.server?.connected) {
      current.server.disconnect();
    }
    cleanup();
  };

  return Object.freeze({
    isSupported,
    isConnected,
    connect,
    send,
    sendAndAwaitReply,
    onReceive,
    disconnect,
  });
}

export const bleService = createBleUartService();
