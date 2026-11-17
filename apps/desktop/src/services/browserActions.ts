/**
 * @file services/browserActions.ts
 * @brief Platform-safe clipboard, external URL, and WebUSB / WebSerial hotplug event adapters.
 */

import { isRecord } from '@/utils/typeGuards';

function resolveEventTargetProp(obj: unknown, key: string): EventTarget | null {
  if (!isRecord(obj)) return null;
  const candidate = obj[key];
  if (!isRecord(candidate)) return null;
  return typeof candidate.addEventListener === 'function' &&
    typeof candidate.removeEventListener === 'function'
    ? (candidate instanceof EventTarget ? candidate : null)
    : null;
}

/** Copies text using Web Clipboard API when available without leaking `navigator` to UI layers. */
export async function copyToClipboard(text: string): Promise<boolean> {
  if (typeof navigator === 'undefined' || !navigator.clipboard?.writeText) {
    return false;
  }
  await navigator.clipboard.writeText(text);
  return true;
}

/** Opens an external documentation URL safely in a detached tab/window. */
export function openExternalUrl(url: string): boolean {
  if (typeof window === 'undefined') return false;
  window.open(url, '_blank', 'noopener,noreferrer');
  return true;
}

/** Subscribes to WebUSB / WebSerial connect & disconnect events where supported by the runtime. */
export function subscribeBrowserUsbHotplug(
  onConnect: () => void,
  onDisconnect: () => void,
): () => void {
  if (typeof navigator === 'undefined') {
    return () => {};
  }
  const targets = [
    resolveEventTargetProp(navigator, 'serial'),
    resolveEventTargetProp(navigator, 'usb'),
  ].filter((t): t is EventTarget => t !== null);

  targets.forEach(target => {
    target.addEventListener('connect', onConnect);
    target.addEventListener('disconnect', onDisconnect);
  });

  return () => {
    targets.forEach(target => {
      target.removeEventListener('connect', onConnect);
      target.removeEventListener('disconnect', onDisconnect);
    });
  };
}
