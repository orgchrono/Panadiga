/**
 * @file store/index.ts
 * @brief Root Redux Toolkit store configuration and side-effect persistence listeners.
 */

import { configureStore, createListenerMiddleware, isAnyOf } from '@reduxjs/toolkit';
import { shellReducer } from './slices/shellSlice';
import {
  acceptCompliance,
  resetWorkbenchPreferences,
  setColorblindSafe,
  setDocksState,
  setFontScale,
  setMode,
  setReducedMotion,
  setRuntimeSettings,
  setTheme,
  toggleBottomDock,
  toggleBottomExpanded,
  toggleLeftDock,
  toggleMode,
  toggleRightDock,
  workbenchReducer,
} from './slices/workbenchSlice';
import { toastReducer } from './slices/toastSlice';
import { nvramCaptureReducer } from './slices/nvramCaptureSlice';
import { picoSessionReducer } from './slices/picoSessionSlice';
import {
  clearWorkbenchStorage,
  STORAGE_A11Y_KEY,
  STORAGE_MODE_KEY,
  STORAGE_RUNTIME_SETTINGS_KEY,
  syncAccessibilityDom,
  writeComplianceAccepted,
  writeDocksState,
  writeStorageString,
} from './storage';

interface PersistenceRootState {
  readonly workbench: ReturnType<typeof workbenchReducer>;
}

function buildWorkbenchPersistenceMiddleware() {
  const listener = createListenerMiddleware<PersistenceRootState>();

  listener.startListening({
    actionCreator: resetWorkbenchPreferences,
    effect: (_action, api) => {
      clearWorkbenchStorage();
      syncAccessibilityDom(api.getState().workbench.accessibility);
    },
  });

  listener.startListening({
    matcher: isAnyOf(setMode, toggleMode),
    effect: (_action, api) => {
      writeStorageString(STORAGE_MODE_KEY, api.getState().workbench.mode);
    },
  });

  listener.startListening({
    matcher: isAnyOf(setTheme, setColorblindSafe, setReducedMotion, setFontScale),
    effect: (_action, api) => {
      const a11y = api.getState().workbench.accessibility;
      writeStorageString(STORAGE_A11Y_KEY, JSON.stringify(a11y));
      syncAccessibilityDom(a11y);
    },
  });

  listener.startListening({
    actionCreator: setRuntimeSettings,
    effect: (_action, api) => {
      writeStorageString(
        STORAGE_RUNTIME_SETTINGS_KEY,
        JSON.stringify(api.getState().workbench.runtimeSettings),
      );
    },
  });

  listener.startListening({
    actionCreator: acceptCompliance,
    effect: () => {
      writeComplianceAccepted(true);
    },
  });

  listener.startListening({
    matcher: isAnyOf(
      toggleLeftDock,
      toggleRightDock,
      toggleBottomDock,
      toggleBottomExpanded,
      setDocksState,
    ),
    effect: (_action, api) => {
      writeDocksState(api.getState().workbench.docks);
    },
  });

  return listener.middleware;
}

export function createAppStore() {
  const persistenceMiddleware = buildWorkbenchPersistenceMiddleware();
  return configureStore({
    reducer: {
      shell: shellReducer,
      workbench: workbenchReducer,
      toast: toastReducer,
      nvramCapture: nvramCaptureReducer,
      picoSession: picoSessionReducer,
    },
    middleware: getDefaultMiddleware =>
      getDefaultMiddleware({
        serializableCheck: true,
      }).prepend(persistenceMiddleware),
  });
}

export const store = createAppStore();

export type AppStore = ReturnType<typeof createAppStore>;
export type RootState = ReturnType<AppStore['getState']>;
export type AppDispatch = AppStore['dispatch'];
