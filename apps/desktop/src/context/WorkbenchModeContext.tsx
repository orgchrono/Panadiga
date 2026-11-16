/**
 * @file context/WorkbenchModeContext.tsx
 * @brief Redux-backed ViewModel facade and Tauri pipeline event synchronizer for Workbench Mode & Accessibility.
 */

import React, { useCallback, useEffect, type ReactNode } from 'react';
import type { LiveNvramCaptureEvent, PicoDeviceStatus } from '@/models';
import { listenEvent } from '@/services/ipc';
import { useAppDispatch, useAppSelector } from '@/store/hooks';
import {
  markPipelineFlag as markPipelineFlagAction,
  resetPipeline as resetPipelineAction,
  resetWorkbenchPreferences as resetWorkbenchPreferencesAction,
  setColorblindSafe as setColorblindSafeAction,
  setFontScale as setFontScaleAction,
  setHudOpen as setHudOpenAction,
  setMode as setModeAction,
  setReducedMotion as setReducedMotionAction,
  setTheme as setThemeAction,
  toggleHud as toggleHudAction,
  toggleMode as toggleModeAction,
} from '@/store/slices/workbenchSlice';
import type {
  AccessibilitySettings,
  FontScaleMode,
  PipelineProgressState,
  WorkbenchExperienceMode,
  WorkbenchTheme,
} from '@/types/workbench';

export type {
  AccessibilitySettings,
  FontScaleMode,
  PipelineProgressState,
  WorkbenchExperienceMode,
  WorkbenchTheme,
};

interface WorkbenchModeContextValue {
  readonly mode: WorkbenchExperienceMode;
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
}

export const WorkbenchModeProvider: React.FC<{ children: ReactNode }> = ({ children }) => {
  const dispatch = useAppDispatch();

  useEffect(() => {
    let active = true;
    const unlisteners: Array<() => void> = [];

    const bind = async <T,>(eventName: string, handler: (payload: T) => void) => {
      const fn = await listenEvent<T>(eventName, payload => {
        if (active) handler(payload);
      });
      if (active) {
        unlisteners.push(fn);
      } else {
        fn();
      }
    };

    const setFlag = (key: keyof PipelineProgressState, value: boolean) => {
      dispatch(markPipelineFlagAction({ key, value }));
    };

    void bind<PicoDeviceStatus>('pico://telemetry', status => {
      if (status?.connected) {
        setFlag('picoReady', true);
      }
    });
    void bind<LiveNvramCaptureEvent>('pico://nvram-live-capture', evt => {
      if (evt?.report) {
        setFlag('picoReady', true);
        if (evt.report.nvram_non_ff_bytes === 0) {
          setFlag('nvramCleaned', true);
        }
      }
    });
    void bind<boolean>('nvram://cleaned', cleaned => {
      setFlag('nvramCleaned', Boolean(cleaned));
    });
    void bind<boolean>('hdd://inspected', inspected => {
      setFlag('hddInspected', Boolean(inspected));
    });
    void bind<boolean>('unbrick://completed', completed => {
      setFlag('unbrickReady', Boolean(completed));
    });

    return () => {
      active = false;
      unlisteners.forEach(fn => fn());
    };
  }, [dispatch]);

  return <>{children}</>;
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
  const setTheme = useCallback(
    (theme: WorkbenchTheme) => dispatch(setThemeAction(theme)),
    [dispatch],
  );
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

  return {
    mode: workbench.mode,
    isAdvanced: workbench.mode === 'advanced',
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
  };
}
