import { useCallback, useEffect, useRef } from 'react';

/**
 * Manages a cancellable `AbortController` slot without mutating `.current` directly,
 * cancelling any previously active controller on re-acquire or component unmount.
 */
export function useAbortSlot() {
  const activeControllersRef = useRef<Set<AbortController>>(new Set());

  const abortAll = useCallback(() => {
    activeControllersRef.current.forEach((ctrl) => ctrl.abort());
    activeControllersRef.current.clear();
  }, []);

  const acquire = useCallback((): AbortController => {
    abortAll();
    const controller = new AbortController();
    activeControllersRef.current.add(controller);
    return controller;
  }, [abortAll]);

  const release = useCallback((controller: AbortController) => {
    activeControllersRef.current.delete(controller);
  }, []);

  useEffect(() => abortAll, [abortAll]);

  return { acquire, abortAll, release };
}
