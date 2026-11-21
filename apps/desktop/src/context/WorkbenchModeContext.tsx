/**
 * @file context/WorkbenchModeContext.tsx
 * @brief Redux-backed ViewModel facade and Tauri pipeline event synchronizer for Workbench Mode & Accessibility.
 */

import React, { useCallback, useEffect, type ReactNode } from 'react';
import type { RuntimeSettings } from '@/models';
import {
  fetchRuntimeSettings,
  installRustTelemetryBridge,
  isRustSettingsBacked,
} from '@/services/settingsService';
import { subscribeWorkbenchPipelineEvents } from '@/services/systemService';
import { useAppDispatch, useAppSelector } from '@/store/hooks';
import { syncAccessibilityDom } from '@/store/storage';
import {
  acceptCompliance as acceptComplianceAction,
  acknowledgePicoSafetySheet as acknowledgePicoSafetySheetAction,
  markPipelineFlag as markPipelineFlagAction,
  resetPipeline as resetPipelineAction,
  resetWorkbenchPreferences as resetWorkbenchPreferencesAction,
  setColorblindSafe as setColorblindSafeAction,
  setComplianceModalOpen as setComplianceModalOpenAction,
  setDocksState as setDocksStateAction,
  setFontScale as setFontScaleAction,
  setHudOpen as setHudOpenAction,
  setMode as setModeAction,
  setReducedMotion as setReducedMotionAction,
  setRuntimeSettings as setRuntimeSettingsAction,
  setRuntimeSettingsLoading as setRuntimeSettingsLoadingAction,
  setTheme as setThemeAction,
  toggleBottomDock as toggleBottomDockAction,
  toggleBottomExpanded as toggleBottomExpandedAction,
  toggleHud as toggleHudAction,
  toggleLeftDock as toggleLeftDockAction,
  toggleMode as toggleModeAction,
  toggleRightDock as toggleRightDockAction,
} from '@/store/slices/workbenchSlice';
import type {
  AccessibilitySettings,
  FontScaleMode,
  PipelineProgressState,
  WorkbenchDocksState,
  WorkbenchExperienceMode,
  WorkbenchTheme,
} from '@/types/workbench';
import { logger, mapRuntimeLogLevelToPino } from '@/utils/logger';
import { registerAsyncSubscriptions } from '@/utils/subscription';

export type {
  AccessibilitySettings,
  FontScaleMode,
  PipelineProgressState,
  WorkbenchDocksState,
  WorkbenchExperienceMode,
  WorkbenchTheme,
};

interface WorkbenchModeContextValue {
  readonly mode: WorkbenchExperienceMode;
  readonly isExpress: boolean;
  readonly isAdvanced: boolean;
  readonly setMode: (mode: WorkbenchExperienceMode) => void;
  readonly toggleMode: () => void;
  readonly hudOpen: boolean;
  readonly setHudOpen: (open: boolean) => void;
  readonly toggleHud: () => void;
  readonly accessibility: AccessibilitySettings;
  readonly setTheme: (theme: WorkbenchTheme) => void;
  readonly setColorblindSafe: (enabled: boolean) => void;
  readonly setReducedMotion: (enabled: boolean) => void;
  readonly setFontScale: (scale: FontScaleMode) => void;
  readonly resetWorkbenchPreferences: () => void;
  readonly resetPipeline: () => void;
  readonly pipeline: PipelineProgressState;
  readonly markPipelineFlag: (key: keyof PipelineProgressState, value?: boolean) => void;
  readonly runtimeSettings: RuntimeSettings;
  readonly setRuntimeSettings: (next: RuntimeSettings) => void;
  readonly isLoading: boolean;
  readonly complianceAccepted: boolean;
  readonly complianceModalOpen: boolean;
  readonly acceptCompliance: () => void;
  readonly setComplianceModalOpen: (open: boolean) => void;
  readonly picoSafetySheetAcknowledged: boolean;
  readonly acknowledgePicoSafetySheet: () => void;
  readonly docks: WorkbenchDocksState;
  readonly toggleLeftDock: () => void;
  readonly toggleRightDock: () => void;
  readonly toggleBottomDock: () => void;
  readonly toggleBottomExpanded: () => void;
  readonly setDocksState: (next: Partial<WorkbenchDocksState>) => void;
}

export const WorkbenchModeProvider: React.FC<{ children: ReactNode }> = ({ children }) => {
  const dispatch = useAppDispatch();
  const initialAccessibility = useAppSelector(state => state.workbench.accessibility);
  const currentLogLevel = useAppSelector(state => state.workbench.runtimeSettings.workbench.log_level);

  useEffect(() => {
    syncAccessibilityDom(initialAccessibility);
  }, [initialAccessibility]);

  useEffect(() => {
    logger.setMinLevel(mapRuntimeLogLevelToPino(currentLogLevel));
  }, [currentLogLevel]);

  useEffect(() => {
    const controller = new AbortController();
    const unsubTelemetry = installRustTelemetryBridge();
    if (isRustSettingsBacked()) {
      void Promise.resolve().then(async () => {
        if (controller.signal.aborted) return;
        dispatch(setRuntimeSettingsLoadingAction(true));
        try {
          const loaded = await fetchRuntimeSettings();
          if (!controller.signal.aborted) {
            dispatch(setRuntimeSettingsAction(loaded));
          }
        } catch {
          // Retain safe defaults when backend config is temporarily inaccessible
        } finally {
          if (!controller.signal.aborted) {
            dispatch(setRuntimeSettingsLoadingAction(false));
          }
        }
      });
    }
    return () => {
      controller.abort();
      unsubTelemetry();
    };
  }, [dispatch]);

  useEffect(() => {
    const setFlag = (key: keyof PipelineProgressState, value: boolean) => {
      dispatch(markPipelineFlagAction({ key, value }));
    };

    return registerAsyncSubscriptions([
      () =>
        subscribeWorkbenchPipelineEvents({
          onTelemetry: status => {
            if (status?.connected) {
              setFlag('picoReady', true);
            }
          },
          onNvramLiveCapture: evt => {
            if (evt?.report) {
              setFlag('picoReady', true);
              if (evt.report.nvram_non_ff_bytes === 0) {
                setFlag('nvramCleaned', true);
              }
            }
          },
          onNvramCleaned: cleaned => setFlag('nvramCleaned', Boolean(cleaned)),
          onHddInspected: inspected => setFlag('hddInspected', Boolean(inspected)),
          onUnbrickCompleted: completed => setFlag('unbrickReady', Boolean(completed)),
        }),
    ]);
  }, [dispatch]);

  return children;
};

export function useWorkbenchMode(): WorkbenchModeContextValue {
  const dispatch = useAppDispatch();
  const workbench = useAppSelector(state => state.workbench);

  const setMode = useCallback(
    (next: WorkbenchExperienceMode) => dispatch(setModeAction(next)),
    [dispatch],
  );
  const toggleMode = useCallback(() => dispatch(toggleModeAction()), [dispatch]);
  const setHudOpen = useCallback((open: boolean) => dispatch(setHudOpenAction(open)), [dispatch]);
  const toggleHud = useCallback(() => dispatch(toggleHudAction()), [dispatch]);
  const setTheme = useCallback((theme: WorkbenchTheme) => dispatch(setThemeAction(theme)), [dispatch]);
  const setColorblindSafe = useCallback(
    (enabled: boolean) => dispatch(setColorblindSafeAction(enabled)),
    [dispatch],
  );
  const setReducedMotion = useCallback(
    (enabled: boolean) => dispatch(setReducedMotionAction(enabled)),
    [dispatch],
  );
  const setFontScale = useCallback(
    (scale: FontScaleMode) => dispatch(setFontScaleAction(scale)),
    [dispatch],
  );
  const resetWorkbenchPreferences = useCallback(
    () => dispatch(resetWorkbenchPreferencesAction()),
    [dispatch],
  );
  const resetPipeline = useCallback(() => dispatch(resetPipelineAction()), [dispatch]);
  const markPipelineFlag = useCallback(
    (key: keyof PipelineProgressState, value = true) =>
      dispatch(markPipelineFlagAction({ key, value })),
    [dispatch],
  );
  const setRuntimeSettings = useCallback(
    (next: RuntimeSettings) => dispatch(setRuntimeSettingsAction(next)),
    [dispatch],
  );
  const acceptCompliance = useCallback(
    () => dispatch(acceptComplianceAction()),
    [dispatch],
  );
  const setComplianceModalOpen = useCallback(
    (open: boolean) => dispatch(setComplianceModalOpenAction(open)),
    [dispatch],
  );
  const acknowledgePicoSafetySheet = useCallback(
    () => dispatch(acknowledgePicoSafetySheetAction()),
    [dispatch],
  );
  const toggleLeftDock = useCallback(() => dispatch(toggleLeftDockAction()), [dispatch]);
  const toggleRightDock = useCallback(() => dispatch(toggleRightDockAction()), [dispatch]);
  const toggleBottomDock = useCallback(() => dispatch(toggleBottomDockAction()), [dispatch]);
  const toggleBottomExpanded = useCallback(
    () => dispatch(toggleBottomExpandedAction()),
    [dispatch],
  );
  const setDocksState = useCallback(
    (next: Partial<WorkbenchDocksState>) => dispatch(setDocksStateAction(next)),
    [dispatch],
  );

  return {
    mode: workbench.mode,
    isExpress: workbench.mode === 'express',
    isAdvanced: workbench.mode === 'engineering',
    isLoading: workbench.isLoading,
    setMode,
    toggleMode,
    hudOpen: workbench.hudOpen,
    setHudOpen,
    toggleHud,
    accessibility: workbench.accessibility,
    setTheme,
    setColorblindSafe,
    setReducedMotion,
    setFontScale,
    resetWorkbenchPreferences,
    resetPipeline,
    pipeline: workbench.pipeline,
    markPipelineFlag,
    runtimeSettings: workbench.runtimeSettings,
    setRuntimeSettings,
    complianceAccepted: workbench.complianceAccepted,
    complianceModalOpen: workbench.complianceModalOpen,
    acceptCompliance,
    setComplianceModalOpen,
    picoSafetySheetAcknowledged: workbench.picoSafetySheetAcknowledged,
    acknowledgePicoSafetySheet,
    docks: workbench.docks,
    toggleLeftDock,
    toggleRightDock,
    toggleBottomDock,
    toggleBottomExpanded,
    setDocksState,
  };
}
