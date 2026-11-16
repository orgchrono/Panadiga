import { useCallback, useEffect, useRef } from 'react';

/**
 * Manages scheduled `setTimeout` handles within a component or ViewModel and cancels
 * any pending timers automatically upon unmount so detached callbacks never leak state updates.
 */
export function useManagedTimeouts() {
  const handlesRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());

  const clearAllTimeouts = useCallback(() => {
    handlesRef.current.forEach(handle => clearTimeout(handle));
    handlesRef.current.clear();
  }, []);

  const scheduleTimeout = useCallback(
    (callback: () => void, delayMs: number): ReturnType<typeof setTimeout> => {
      const handle = setTimeout(() => {
        handlesRef.current.delete(handle);
        callback();
      }, delayMs);
      handlesRef.current.add(handle);
      return handle;
    },
    []
  );

  const cancelTimeout = useCallback((handle: ReturnType<typeof setTimeout> | null) => {
    if (handle === null) return;
    clearTimeout(handle);
    handlesRef.current.delete(handle);
  }, []);

  useEffect(() => clearAllTimeouts, [clearAllTimeouts]);

  return { scheduleTimeout, cancelTimeout, clearAllTimeouts };
}
