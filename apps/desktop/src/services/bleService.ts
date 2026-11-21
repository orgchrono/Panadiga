/**
 * @file bleService.ts
 * @brief Bluetooth Low Energy (BLE) Nordic UART Service (NUS) Client.
 */

import { translate } from '@/i18n/useTranslation';
import { formatErrorMessage } from '@/utils/formatters';
import {
  connectAndroidBleNus,
  disconnectAndroidBleNus,
  getAndroidCapabilities,
  isAndroidNativeBridgeAvailable,
  sendAndroidBleCommand,
} from './androidBridge';
import {
  BLE_ANDROID_CONNECT_TIMEOUT_MS,
  BLE_CMD_TIMEOUT_MS,
  BLE_GATT_CHUNK_BYTES,
  BLE_MIN_TIMEOUT_MS,
  NUS_RX_CHARACTERISTIC_UUID,
  NUS_SERVICE_UUID,
  NUS_TX_CHARACTERISTIC_UUID,
  extractCharacteristicDataView,
  isBleReplyComplete,
  splitGattChunks,
  writeGattChunksSequential,
  type BleActiveSession,
  type BleDataCallback,
  type BleDeviceInfo,
  type BleUartClient,
} from './bleGattCodec';
import { resolveBluetoothNavigator, subscribeWindowEvent } from './hostBridge';

export {
  NUS_RX_CHARACTERISTIC_UUID,
  NUS_SERVICE_UUID,
  NUS_TX_CHARACTERISTIC_UUID,
  isBleReplyComplete,
  splitGattChunks,
  type BleDataCallback,
  type BleDeviceInfo,
  type BleUartClient,
};

/**
 * Pure closure factory encapsulating Web Bluetooth & Android BLE NUS transport state
 * without OOP classes, `let` bindings, or shared module globals.
 */
export function createBleUartService(): BleUartClient {
  const receiveCallbacks = new Set<BleDataCallback>();
  const sessions = new Map<'active', BleActiveSession>();

  const emitChunk = (chunk: string): void => {
    receiveCallbacks.forEach((cb) => cb(chunk));
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

      const utf8StreamDecoder = new TextDecoder('utf-8');
      await txCharacteristic.startNotifications();
      txCharacteristic.addEventListener('characteristicvaluechanged', (event: Event) => {
        const value = extractCharacteristicDataView(event);
        if (value) {
          emitChunk(utf8StreamDecoder.decode(value, { stream: true }));
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
      throw new Error(translate('errors.ble.connectFailed', { reason: formatErrorMessage(err) }));
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

    const rxChar = active.rxCharacteristic;
    const data = new TextEncoder().encode(message.endsWith('\n') ? message : `${message}\n`);
    await writeGattChunksSequential(rxChar, splitGattChunks(data, BLE_GATT_CHUNK_BYTES));
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
    const effectiveTimeoutMs = Math.max(BLE_MIN_TIMEOUT_MS, timeoutMs);

    return new Promise<string>((resolve, reject) => {
      const settle = (result: string) => {
        cleanupRef.get('off')?.();
        resolve(result);
      };
      const timer = setTimeout(() => {
        cleanupRef.get('off')?.();
        const collected = (chunks.get('buf') ?? '').trim();
        if (collected.length === 0) {
          reject(
            new Error(
              translate('errors.ble.nusTimeout', {
                timeoutMs: effectiveTimeoutMs,
                command,
              }),
            ),
          );
          return;
        }
        resolve(collected);
      }, effectiveTimeoutMs);

      const off = onReceive((chunk) => {
        const next = `${chunks.get('buf') ?? ''}${chunk}`;
        chunks.set('buf', next);
        if (isBleReplyComplete(next)) {
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
