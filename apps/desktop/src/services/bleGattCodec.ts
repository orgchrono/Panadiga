/**
 * @file bleGattCodec.ts
 * @brief Pure Nordic UART Service (NUS) BLE constants, reply completion codec, and 20-byte GATT chunker.
 */

import { SSOT_WIRE_TERMINAL_MARKERS } from '@/models/generated/ssot';
import type {
  BluetoothDevice,
  BluetoothRemoteGATTCharacteristic,
  BluetoothRemoteGATTServer,
} from './hostBridge';

export const NUS_SERVICE_UUID = '6e400001-b5a3-f393-e0a9-e50e24dcca9e';
export const NUS_RX_CHARACTERISTIC_UUID = '6e400002-b5a3-f393-e0a9-e50e24dcca9e';
export const NUS_TX_CHARACTERISTIC_UUID = '6e400003-b5a3-f393-e0a9-e50e24dcca9e';

export const BLE_ANDROID_CONNECT_TIMEOUT_MS = 10000;
export const BLE_CMD_TIMEOUT_MS = 4000;
export const BLE_MIN_TIMEOUT_MS = 400;
export const BLE_GATT_CHUNK_BYTES = 20;

const BLE_RES_PREFIXES: readonly string[] = SSOT_WIRE_TERMINAL_MARKERS.filter((m) =>
  m.startsWith('@'),
);
const BLE_PROMPTS: readonly string[] = SSOT_WIRE_TERMINAL_MARKERS.filter((m) => !m.startsWith('@'));
const BLE_LEGACY_CLI_STATUS_RE =
  /^[A-Z0-9_]+:\s+(?:OK|ERROR|ERR|FAILED|OFFLINE|TIMEOUT|DONE)\b/;

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

export interface BleActiveSession {
  readonly device: BluetoothDevice | null;
  readonly server: BluetoothRemoteGATTServer | null;
  readonly rxCharacteristic: BluetoothRemoteGATTCharacteristic | null;
  readonly txCharacteristic: BluetoothRemoteGATTCharacteristic | null;
  readonly androidNativeConnected: boolean;
  readonly unsubAndroidRx: (() => void) | null;
}

export function extractCharacteristicDataView(event: Event): DataView | null {
  const target: EventTarget | null = event.target;
  if (!target) {
    return null;
  }
  const value: unknown = Reflect.get(target, 'value');
  return value instanceof DataView ? value : null;
}

export function isBleReplyComplete(buffer: string): boolean {
  const trimmedEnd = buffer.trimEnd();
  if (trimmedEnd.length === 0) {
    return false;
  }
  if (BLE_PROMPTS.some((prompt) => trimmedEnd.endsWith(prompt))) {
    return true;
  }
  if (!buffer.endsWith('\n') && !buffer.endsWith('\r')) {
    return false;
  }
  const lines = buffer.split(/\r?\n/);
  if (lines.some((line) => BLE_RES_PREFIXES.some((prefix) => line.trimStart().startsWith(prefix)))) {
    return true;
  }
  const hasStructuredFrames = lines.some((line) => {
    const t = line.trimStart();
    return (
      t.startsWith('@PICO_') ||
      t.startsWith('@P7501_') ||
      t.startsWith('@PROG ') ||
      t.startsWith('@EVT ')
    );
  });
  if (hasStructuredFrames) {
    return false;
  }
  return lines.some((line) => {
    const trimmed = line.trim();
    return trimmed === 'PONG' || BLE_LEGACY_CLI_STATUS_RE.test(trimmed);
  });
}

export function splitGattChunks(
  payload: Uint8Array,
  chunkSize = BLE_GATT_CHUNK_BYTES,
): Uint8Array<ArrayBuffer>[] {
  const safeChunkSize = Math.max(1, chunkSize);
  const count = Math.ceil(payload.byteLength / safeChunkSize);
  return Array.from({ length: count }, (_, idx) => {
    const start = idx * safeChunkSize;
    return payload.slice(start, start + safeChunkSize);
  });
}

export async function writeGattChunksSequential(
  rxChar: BluetoothRemoteGATTCharacteristic,
  chunks: readonly Uint8Array<ArrayBuffer>[],
  index = 0,
): Promise<void> {
  if (index >= chunks.length) {
    return;
  }
  await rxChar.writeValue(chunks[index]);
  await writeGattChunksSequential(rxChar, chunks, index + 1);
}
