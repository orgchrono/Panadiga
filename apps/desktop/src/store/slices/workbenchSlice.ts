/**
 * @file store/slices/workbenchSlice.ts
 * @brief Pure immutable Redux slice managing workbench experience mode, HUD, accessibility, and pipeline progress.
 */

import { createSlice, type PayloadAction } from '@reduxjs/toolkit';
import type { RuntimeSettings } from '@/models';
import type {
  AccessibilitySettings,
  FontScaleMode,
  PipelineProgressState,
  WorkbenchDocksState,
  WorkbenchExperienceMode,
  WorkbenchTheme,
} from '@/types/workbench';
import {
  DEFAULT_ACCESSIBILITY,
  DEFAULT_DOCKS_STATE,
  readFallbackRuntimeSettings,
  readInitialAccessibility,
  readInitialComplianceAccepted,
  readInitialDocksState,
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
  readonly runtimeSettings: RuntimeSettings;
  readonly isLoading: boolean;
  readonly complianceAccepted: boolean;
  readonly complianceModalOpen: boolean;
  readonly picoSafetySheetAcknowledged: boolean;
  readonly docks: WorkbenchDocksState;
}

export function createInitialWorkbenchState(): WorkbenchSliceState {
  const initialMode = readInitialWorkbenchMode();
  const complianceAccepted = readInitialComplianceAccepted();
  return {
    mode: initialMode,
    hudOpen: false,
    accessibility: readInitialAccessibility(),
    pipeline: DEFAULT_PIPELINE,
    runtimeSettings: readFallbackRuntimeSettings(),
    isLoading: false,
    complianceAccepted,
    complianceModalOpen: initialMode === 'engineering' && !complianceAccepted,
    picoSafetySheetAcknowledged: false,
    docks: readInitialDocksState(),
  };
}

const workbenchSlice = createSlice({
  name: 'workbench',
  initialState: createInitialWorkbenchState,
  reducers: {
    setMode(state, action: PayloadAction<WorkbenchExperienceMode>) {
      const nextMode = action.payload;
      return {
        ...state,
        mode: nextMode,
        complianceModalOpen:
          nextMode === 'engineering' ? !state.complianceAccepted : false,
      };
    },
    toggleMode(state) {
      const nextMode: WorkbenchExperienceMode =
        state.mode === 'engineering' ? 'express' : 'engineering';
      return {
        ...state,
        mode: nextMode,
        complianceModalOpen:
          nextMode === 'engineering' ? !state.complianceAccepted : false,
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
    setRuntimeSettings(state, action: PayloadAction<RuntimeSettings>) {
      return { ...state, runtimeSettings: action.payload };
    },
    setRuntimeSettingsLoading(state, action: PayloadAction<boolean>) {
      return { ...state, isLoading: action.payload };
    },
    acceptCompliance(state) {
      return { ...state, complianceAccepted: true, complianceModalOpen: false };
    },
    setComplianceModalOpen(state, action: PayloadAction<boolean>) {
      return { ...state, complianceModalOpen: action.payload };
    },
    acknowledgePicoSafetySheet(state) {
      return { ...state, picoSafetySheetAcknowledged: true };
    },
    toggleLeftDock(state) {
      return { ...state, docks: { ...state.docks, leftOpen: !state.docks.leftOpen } };
    },
    toggleRightDock(state) {
      return { ...state, docks: { ...state.docks, rightOpen: !state.docks.rightOpen } };
    },
    toggleBottomDock(state) {
      return { ...state, docks: { ...state.docks, bottomOpen: !state.docks.bottomOpen } };
    },
    toggleBottomExpanded(state) {
      return {
        ...state,
        docks: {
          ...state.docks,
          bottomOpen: true,
          bottomExpanded: !state.docks.bottomExpanded,
        },
      };
    },
    setDocksState(state, action: PayloadAction<Partial<WorkbenchDocksState>>) {
      return { ...state, docks: { ...state.docks, ...action.payload } };
    },
    resetWorkbenchPreferences(state) {
      return {
        ...state,
        mode: 'express',
        hudOpen: false,
        complianceModalOpen: false,
        accessibility: DEFAULT_ACCESSIBILITY,
        docks: DEFAULT_DOCKS_STATE,
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
  setRuntimeSettings,
  setRuntimeSettingsLoading,
  acceptCompliance,
  setComplianceModalOpen,
  acknowledgePicoSafetySheet,
  toggleLeftDock,
  toggleRightDock,
  toggleBottomDock,
  toggleBottomExpanded,
  setDocksState,
  resetWorkbenchPreferences,
} = workbenchSlice.actions;

export const workbenchReducer = workbenchSlice.reducer;
