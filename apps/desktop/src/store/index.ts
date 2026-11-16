/**
 * @file store/index.ts
 * @brief Root Redux Toolkit store configuration and side-effect persistence listeners.
 */

import { configureStore, createListenerMiddleware, isAnyOf } from '@reduxjs/toolkit';
import { shellReducer } from './slices/shellSlice';
import {
  workbenchReducer,
  resetWorkbenchPreferences,
  setColorblindSafe,
  setFontScale,
  setMode,
  setReducedMotion,
  setTheme,
  toggleMode,
} from './slices/workbenchSlice';
import { toastReducer } from './slices/toastSlice';
import { nvramCaptureReducer } from './slices/nvramCaptureSlice';
import {
  clearWorkbenchStorage,
  STORAGE_A11Y_KEY,
  STORAGE_MODE_KEY,
  syncAccessibilityDom,
  writeStorageString,
} from './storage';

function buildWorkbenchPersistenceMiddleware() {
  const listener = createListenerMiddleware();

  listener.startListening({
    actionCreator: resetWorkbenchPreferences,
    effect: (_action, api) => {
      clearWorkbenchStorage();
      const state = api.getState() as { workbench: ReturnType<typeof workbenchReducer> };
      syncAccessibilityDom(state.workbench.accessibility);
    },
  });

  listener.startListening({
    matcher: isAnyOf(setMode, toggleMode),
    effect: (_action, api) => {
      const state = api.getState() as { workbench: ReturnType<typeof workbenchReducer> };
      writeStorageString(STORAGE_MODE_KEY, state.workbench.mode);
    },
  });

  listener.startListening({
    matcher: isAnyOf(setTheme, setColorblindSafe, setReducedMotion, setFontScale),
    effect: (_action, api) => {
      const state = api.getState() as { workbench: ReturnType<typeof workbenchReducer> };
      writeStorageString(STORAGE_A11Y_KEY, JSON.stringify(state.workbench.accessibility));
      syncAccessibilityDom(state.workbench.accessibility);
    },
  });

  return listener.middleware;
}

export function createAppStore() {
  const persistenceMiddleware = buildWorkbenchPersistenceMiddleware();
  const appStore = configureStore({
    reducer: {
      shell: shellReducer,
      workbench: workbenchReducer,
      toast: toastReducer,
      nvramCapture: nvramCaptureReducer,
    },
    middleware: getDefaultMiddleware =>
      getDefaultMiddleware({
        serializableCheck: true,
      }).prepend(persistenceMiddleware),
  });

  syncAccessibilityDom(appStore.getState().workbench.accessibility);
  return appStore;
}

export const store = createAppStore();

export type AppStore = ReturnType<typeof createAppStore>;
export type RootState = ReturnType<AppStore['getState']>;
export type AppDispatch = AppStore['dispatch'];
