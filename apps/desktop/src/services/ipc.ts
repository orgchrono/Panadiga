/**
 * IPC Communication & Reactive Event Stream Transport for Panasonic DIGA Suite
 * Single Responsibility: Wraps @tauri-apps/api/core and @tauri-apps/api/event
 * with an injected test IPC bridge restricted strictly to dev/test harnesses.
 */

import { isTauri } from '@tauri-apps/api/core';
import type {
  BgoRepairProgressEvent,
  HddFlashProgressEvent,
  LiveNvramCaptureEvent,
  NorDumpProgressEvent,
  PicoDeviceStatus,
  PicoDiagnosticResult,
  PicoProbeDavisIdeResponse,
  ReactiveSerialEvent,
  Sc0BridgeDataEvent,
} from '@/models';
import { translate } from '@/i18n/useTranslation';
import { logger } from '@/utils/logger';
import { resolveTestIpcBridge, type PanadigaTestIpcBridge } from './hostBridge';

export type { PanadigaTestIpcBridge };

export interface PanadigaEventMap {
  readonly 'unbrick://completed': boolean;
  readonly 'nvram://cleaned': boolean;
  readonly 'hdd://inspected': boolean;
  readonly 'hdd://flash-progress': HddFlashProgressEvent;
  readonly 'nor://dump-progress': NorDumpProgressEvent;
  readonly 'bgo://repair-progress': BgoRepairProgressEvent;
  readonly 'davis://probe-result': PicoProbeDavisIdeResponse;
  readonly pico_telemetry: PicoDeviceStatus;
  readonly 'pico://telemetry': PicoDeviceStatus;
  readonly 'pico://nvram-live-capture': LiveNvramCaptureEvent;
  readonly 'pico://diagnostic-result': PicoDiagnosticResult;
  readonly 'pico://bridge-data': Sc0BridgeDataEvent;
  readonly 'pico://serial-log': ReactiveSerialEvent;
  readonly 'pico://ports-changed': string[];
}

export const isTauriEnvironment = (): boolean => isTauri();

export const hasTestIpcBridge = (): boolean => resolveTestIpcBridge() !== null;

function toCamelCaseKey(key: string): string {
  return key.replace(/_([a-z0-9])/g, (_, char: string) => char.toUpperCase());
}

/**
 * Normalizes argument dictionaries so that both camelCase (required by Tauri v2 IPC commands)
 * and snake_case (legacy/internal) keys are populated without mutating input objects.
 */
export function normalizeArgsForTauri(
  args?: Record<string, unknown>,
): Record<string, unknown> | undefined {
  if (!args) return undefined;
  return Object.fromEntries(
    Object.entries(args).flatMap(([key, value]) => {
      const camel = toCamelCaseKey(key);
      const originalPair: readonly [string, unknown] = [key, value];
      const camelPair: readonly [string, unknown] = [camel, value];
      return camel !== key ? [camelPair, originalPair] : [originalPair];
    }),
  );
}

/**
 * Universal invoke wrapper with full TypeScript safety.
 * Strictly prioritizes real native Tauri IPC pipeline.
 */
export async function invokeCommand<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  const normalizedArgs = normalizeArgsForTauri(args);

  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return await invoke<T>(cmd, normalizedArgs);
  }

  const testBridge = resolveTestIpcBridge();
  if (testBridge) {
    return await testBridge.invoke<T>(cmd, normalizedArgs);
  }

  throw new Error(translate('errors.ipc.requiresTauri', { cmd }));
}

type EventListenerRegistry = {
  readonly [K in keyof PanadigaEventMap]: Set<(payload: PanadigaEventMap[K]) => void>;
};

function createLocalEventBus() {
  const registry: EventListenerRegistry = {
    'unbrick://completed': new Set(),
    'nvram://cleaned': new Set(),
    'hdd://inspected': new Set(),
    'hdd://flash-progress': new Set(),
    'nor://dump-progress': new Set(),
    'bgo://repair-progress': new Set(),
    'davis://probe-result': new Set(),
    pico_telemetry: new Set(),
    'pico://telemetry': new Set(),
    'pico://nvram-live-capture': new Set(),
    'pico://diagnostic-result': new Set(),
    'pico://bridge-data': new Set(),
    'pico://serial-log': new Set(),
    'pico://ports-changed': new Set(),
  };

  const emit = <K extends keyof PanadigaEventMap>(
    eventName: K,
    payload: PanadigaEventMap[K],
  ): void => {
    registry[eventName].forEach(fn => {
      try {
        fn(payload);
      } catch (e) {
        logger.warn('IPC', `Error in local event handler for ${eventName}`, e);
      }
    });
  };

  const subscribe = <K extends keyof PanadigaEventMap>(
    eventName: K,
    handler: (payload: PanadigaEventMap[K]) => void,
  ): (() => void) => {
    const channel = registry[eventName];
    channel.add(handler);
    return () => {
      channel.delete(handler);
    };
  };

  return Object.freeze({ emit, subscribe });
}

const localEventBus = createLocalEventBus();

/**
 * Dispatches an event across the frontend subscriber network.
 */
export function emitLocalEvent<K extends keyof PanadigaEventMap>(
  eventName: K,
  payload: PanadigaEventMap[K],
): void {
  localEventBus.emit(eventName, payload);
}

async function subscribeExternalEvent<K extends keyof PanadigaEventMap>(
  eventName: K,
  handler: (payload: PanadigaEventMap[K]) => void,
): Promise<() => void> {
  if (!isTauri()) {
    const testBridge = resolveTestIpcBridge();
    if (testBridge) {
      return await testBridge.listen<PanadigaEventMap[K]>(eventName, handler);
    }
    return () => {};
  }

  try {
    const { listen } = await import('@tauri-apps/api/event');
    return await listen<PanadigaEventMap[K]>(eventName, event => {
      handler(event.payload);
    });
  } catch (err) {
    logger.warn('IPC', `Failed registering Tauri event listener for ${eventName}`, err);
    return () => {};
  }
}

/**
 * Reactive Event Stream Listener for Tauri & Web Environments.
 */
export async function listenEvent<K extends keyof PanadigaEventMap>(
  eventName: K,
  handler: (payload: PanadigaEventMap[K]) => void,
): Promise<() => void> {
  const unlistenLocal = localEventBus.subscribe(eventName, handler);
  const unlistenExternal = await subscribeExternalEvent(eventName, handler);
  return () => {
    unlistenLocal();
    unlistenExternal();
  };
}
