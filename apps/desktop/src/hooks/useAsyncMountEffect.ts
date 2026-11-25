/**
 * @file hooks/useAsyncMountEffect.ts
 * @brief Shared DRY React hook wrapping mount-time async loading with AbortController and loading state.
 */

import { useEffect, useState } from 'react';

export function useAsyncMountEffect<T>(
  loader: () => Promise<T>,
  onSuccess: (data: T) => void,
): boolean {
  const [isLoading, setIsLoading] = useState<boolean>(true);

  useEffect(() => {
    const controller = new AbortController();
    void loader()
      .then((data) => {
        if (!controller.signal.aborted) {
          onSuccess(data);
        }
      })
      .catch(() => {
        // Callers preserve safe compile-time defaults if backend is unavailable
      })
      .finally(() => {
        if (!controller.signal.aborted) {
          setIsLoading(false);
        }
      });
    return () => {
      controller.abort();
    };
  }, [loader, onSuccess]);

  return isLoading;
}
