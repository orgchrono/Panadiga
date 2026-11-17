/**
 * @file utils/subscription.ts
 * @brief Race-free functional async effect subscription cleanup coordinator (zero `let`, zero `.push()`).
 */

/**
 * Registers multiple async cleanup-returning subscriptions inside a React `useEffect`,
 * guaranteeing deterministic disposal even if unmounted before promises settle.
 */
export function registerAsyncSubscriptions(
  subscribeFns: ReadonlyArray<() => Promise<() => void>>,
): () => void {
  const controller = new AbortController();
  const settledPromise = Promise.allSettled(subscribeFns.map(fn => fn()));

  void settledPromise.then(results => {
    if (!controller.signal.aborted) return;
    results.forEach(result => {
      if (result.status === 'fulfilled') {
        result.value();
      }
    });
  });

  return () => {
    controller.abort();
    void settledPromise.then(results => {
      results.forEach(result => {
        if (result.status === 'fulfilled') {
          result.value();
        }
      });
    });
  };
}
