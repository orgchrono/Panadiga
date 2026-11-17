import { useEffect } from 'react';

/**
 * Attaches window-level pointer drag listeners only while `active` is true,
 * isolating DOM global subscriptions outside presentational components.
 */
export function useWindowPointerDrag(
  active: boolean,
  onMove: (event: PointerEvent) => void,
  onEnd: () => void,
): void {
  useEffect(() => {
    if (!active || typeof window === 'undefined') return undefined;
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onEnd);
    window.addEventListener('pointercancel', onEnd);
    return () => {
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerup', onEnd);
      window.removeEventListener('pointercancel', onEnd);
    };
  }, [active, onEnd, onMove]);
}

/**
 * Subscribes a simple window `pointerup` / `pointercancel` listener while pointer capture is active.
 */
export function useWindowPointerUp(onPointerUp: () => void): void {
  useEffect(() => {
    if (typeof window === 'undefined') return undefined;
    window.addEventListener('pointerup', onPointerUp);
    window.addEventListener('pointercancel', onPointerUp);
    return () => {
      window.removeEventListener('pointerup', onPointerUp);
      window.removeEventListener('pointercancel', onPointerUp);
    };
  }, [onPointerUp]);
}

/** Reads a persisted finite number from browser storage cleanly for layout splitters. */
export function readPersistedStorageNumber(key: string | undefined): number | null {
  if (!key || typeof window === 'undefined') return null;
  try {
    const raw = window.localStorage.getItem(key);
    const parsed = raw ? Number.parseFloat(raw) : Number.NaN;
    return Number.isFinite(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

/** Writes a persisted string value to browser storage safely. */
export function writePersistedStorageString(key: string | undefined, value: string): void {
  if (!key || typeof window === 'undefined') return;
  try {
    window.localStorage.setItem(key, value);
  } catch {
    // ignore storage restrictions
  }
}
