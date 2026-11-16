/**
 * @file store/hooks.ts
 * @brief Strongly-typed Redux hooks supporting both `<Provider>` trees and isolated hook unit tests.
 */

import { useContext, useSyncExternalStore } from 'react';
import { ReactReduxContext } from 'react-redux';
import type { Store } from '@reduxjs/toolkit';
import { store as defaultAppStore, type AppDispatch, type RootState } from './index';

export function useActiveAppStore(): Store<RootState> {
  const ctx = useContext(ReactReduxContext);
  return (ctx?.store as Store<RootState> | undefined) ?? defaultAppStore;
}

export function useAppDispatch(): AppDispatch {
  return useActiveAppStore().dispatch as AppDispatch;
}

export function useAppSelector<T>(selector: (state: RootState) => T): T {
  const activeStore = useActiveAppStore();
  return useSyncExternalStore(
    activeStore.subscribe,
    () => selector(activeStore.getState()),
    () => selector(activeStore.getState()),
  );
}
