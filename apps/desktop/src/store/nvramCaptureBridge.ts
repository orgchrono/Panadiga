/**
 * @file store/nvramCaptureBridge.ts
 * @brief Store-backed live P7501 NVRAM capture coordinator (decouples Redux dispatch from services/).
 */

import type { LiveNvramCaptureEvent } from '@/models';
import { analyzeLiveNvramDump } from '@/services/storageCryptoService';
import { store } from './index';
import { clearLiveCapture, recordLiveCapture } from './slices/nvramCaptureSlice';

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
 * Decodes real `DUMP` hex words streamed from Desktop CDC, Android USB-OTG, or BLE NUS,
 * validates via Rust NVRAM analyzer service, and dispatches into the Redux store.
 */
export async function ingestP7501DumpOutput(
  rawOutput: string,
  source: string,
  port: string,
  baseAddressOverride?: number,
): Promise<LiveNvramCaptureEvent | null> {
  const event = await analyzeLiveNvramDump(rawOutput, source, port, baseAddressOverride);
  if (!event) return null;
  recordLiveNvramCapture(event);
  return event;
}
