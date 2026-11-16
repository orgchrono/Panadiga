/**
 * @file store/slices/workbenchSlice.ts
 * @brief Pure immutable Redux slice managing workbench experience mode, HUD, accessibility, and pipeline progress.
 */

import { createSlice, type PayloadAction } from '@reduxjs/toolkit';
import type {
  AccessibilitySettings,
  FontScaleMode,
  PipelineProgressState,
  WorkbenchExperienceMode,
  WorkbenchTheme,
} from '@/types/workbench';
import {
  DEFAULT_ACCESSIBILITY,
  readInitialAccessibility,
  readInitialWorkbenchMode,
} from '../storage';

export const DEFAULT_PIPELINE: PipelineProgressState = Object.freeze({
  picoReady: false,
  nvramCleaned: false,
  hddInspected: false,
  unbrickReady: false,
});

export interface WorkbenchSliceState {
  readonly mode: WorkbenchExperienceMode;
  readonly hudOpen: boolean;
  readonly accessibility: AccessibilitySettings;
  readonly pipeline: PipelineProgressState;
}

export function createInitialWorkbenchState(): WorkbenchSliceState {
  return {
    mode: readInitialWorkbenchMode(),
    hudOpen: false,
    accessibility: readInitialAccessibility(),
    pipeline: DEFAULT_PIPELINE,
  };
}

const workbenchSlice = createSlice({
  name: 'workbench',
  initialState: createInitialWorkbenchState,
  reducers: {
    setMode(state, action: PayloadAction<WorkbenchExperienceMode>) {
      return { ...state, mode: action.payload };
    },
    toggleMode(state) {
      return {
        ...state,
        mode: state.mode === 'advanced' ? 'basic' : 'advanced',
      };
    },
    setHudOpen(state, action: PayloadAction<boolean>) {
      return { ...state, hudOpen: action.payload };
    },
    toggleHud(state) {
      return { ...state, hudOpen: !state.hudOpen };
    },
    setTheme(state, action: PayloadAction<WorkbenchTheme>) {
      return {
        ...state,
        accessibility: { ...state.accessibility, theme: action.payload },
      };
    },
    setColorblindSafe(state, action: PayloadAction<boolean>) {
      return {
        ...state,
        accessibility: { ...state.accessibility, colorblindSafe: action.payload },
      };
    },
    setReducedMotion(state, action: PayloadAction<boolean>) {
      return {
        ...state,
        accessibility: { ...state.accessibility, reducedMotion: action.payload },
      };
    },
    setFontScale(state, action: PayloadAction<FontScaleMode>) {
      return {
        ...state,
        accessibility: { ...state.accessibility, fontScale: action.payload },
      };
    },
    markPipelineFlag(
      state,
      action: PayloadAction<{ key: keyof PipelineProgressState; value: boolean }>,
    ) {
      const { key, value } = action.payload;
      if (state.pipeline[key] === value) {
        return state;
      }
      return {
        ...state,
        pipeline: { ...state.pipeline, [key]: value },
      };
    },
    resetPipeline(state) {
      return { ...state, pipeline: DEFAULT_PIPELINE };
    },
    resetWorkbenchPreferences(state) {
      return {
        ...state,
        mode: 'advanced',
        hudOpen: false,
        accessibility: DEFAULT_ACCESSIBILITY,
      };
    },
  },
});

export const {
  setMode,
  toggleMode,
  setHudOpen,
  toggleHud,
  setTheme,
  setColorblindSafe,
  setReducedMotion,
  setFontScale,
  markPipelineFlag,
  resetPipeline,
  resetWorkbenchPreferences,
} = workbenchSlice.actions;

export const workbenchReducer = workbenchSlice.reducer;
