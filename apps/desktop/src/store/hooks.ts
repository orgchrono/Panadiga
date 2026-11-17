/**
 * @file store/hooks.ts
 * @brief Strongly-typed Redux hooks supporting both `<Provider>` trees and isolated hook unit tests.
 */

import { useContext, useSyncExternalStore } from 'react';
import { ReactReduxContext } from 'react-redux';
import { isRecord } from '@/utils/typeGuards';
import { store as defaultAppStore, type AppDispatch, type AppStore, type RootState } from './index';

function isAppStore(candidate: unknown): candidate is AppStore {
  if (!isRecord(candidate)) return false;
  if (
    typeof candidate.getState !== 'function' ||
    typeof candidate.dispatch !== 'function' ||
    typeof candidate.subscribe !== 'function'
  ) {
    return false;
  }
  const state: unknown = candidate.getState();
  return (
    isRecord(state) &&
    'shell' in state &&
    'workbench' in state &&
    'toast' in state &&
    'nvramCapture' in state
  );
}

export function useActiveAppStore(): AppStore {
  const ctx = useContext(ReactReduxContext);
  return isAppStore(ctx?.store) ? ctx.store : defaultAppStore;
}

export function useAppDispatch(): AppDispatch {
  return useActiveAppStore().dispatch;
}

export function useAppSelector<T>(selector: (state: RootState) => T): T {
  const activeStore = useActiveAppStore();
  return useSyncExternalStore(
    activeStore.subscribe,
    () => selector(activeStore.getState()),
    () => selector(activeStore.getState()),
  );
}
