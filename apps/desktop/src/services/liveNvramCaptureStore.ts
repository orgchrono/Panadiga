/**
 * @file services/liveNvramCaptureStore.ts
 * @brief Redux-backed live in-circuit NVRAM capture dispatcher and IPC dump ingestion helper.
 */

import { AM34_NVRAM_BASE_ADDR } from '@/constants';
import type { LiveNvramCaptureEvent, NvramCleanReport } from '@/models';
import { store } from '@/store';
import { clearLiveCapture, recordLiveCapture } from '@/store/slices/nvramCaptureSlice';
import { parseP7501DumpBytes } from './diagnosticParser';
import { invokeCommand } from './ipc';

export function getLatestLiveNvramCapture(): LiveNvramCaptureEvent | null {
  return store.getState().nvramCapture.latestCapture;
}

export function recordLiveNvramCapture(event: LiveNvramCaptureEvent): void {
  store.dispatch(recordLiveCapture(event));
}

export function clearLiveNvramCapture(): void {
  store.dispatch(clearLiveCapture());
}

/**
 * Decodes real `DUMP` hex words streamed from Desktop CDC, Android USB-OTG, or BLE NUS
 * and runs Rust core NVRAM/microsector structural analysis into the Redux store.
 */
export async function ingestP7501DumpOutput(
  rawOutput: string,
  source: string,
  port: string,
  baseAddressOverride?: number,
): Promise<LiveNvramCaptureEvent | null> {
  const { baseAddress, bytes } = parseP7501DumpBytes(rawOutput);
  if (bytes.length === 0) {
    return null;
  }

  const effectiveBase = baseAddressOverride ?? baseAddress ?? AM34_NVRAM_BASE_ADDR;
  const baseHex = `0x${effectiveBase.toString(16).toUpperCase().padStart(8, '0')}`;
  const sourceLabel = `Live Pico ${source} (${port}) @ ${baseHex}`;

  const report = await invokeCommand<NvramCleanReport>('analyze_live_nvram_bytes', {
    bytes,
    baseBusAddress: effectiveBase,
    sourceLabel,
  });

  const event: LiveNvramCaptureEvent = {
    source,
    port,
    base_bus_address: baseHex,
    captured_bytes: bytes.length,
    captured_at: new Date().toLocaleTimeString('pt-BR', { hour12: false }),
    report,
  };

  recordLiveNvramCapture(event);
  return event;
}
