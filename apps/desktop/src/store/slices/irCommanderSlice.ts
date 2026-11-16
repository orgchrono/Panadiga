import { createSlice, type PayloadAction } from '@reduxjs/toolkit';
import type {
  DynamicKeymapProfile,
  IrTimingOverride,
  IrTransportKind,
  PanasonicCategory,
  RemoteMode,
  RemoteUnitSet,
} from '@/models/ir';

export interface IrCommanderState {
  mode: RemoteMode;
  remoteUnit: RemoteUnitSet;
  transport: IrTransportKind;
  picoGpioPin: number;
  timingUnitOverride: IrTimingOverride;
  repeatCount: number;
  editMode: boolean;
  searchQuery: string;
  selectedCategory: PanasonicCategory;
  activeProfile: DynamicKeymapProfile | null;
  savedProfiles: DynamicKeymapProfile[];
  activeMacroId: string | null;
  macroStepIndex: number;
  macroTotalSteps: number;
  isTransmitting: boolean;
  isBeaconRunning: boolean;
  vfdPrimary: string;
  vfdSecondary: string;
  lastWireHex: string;
  lastProntoHex: string;
  lastBackendUsed: string;
  error: string | null;
}

export const initialIrCommanderState: IrCommanderState = {
  mode: 'DVD',
  remoteUnit: 1,
  transport: 'AUTO',
  picoGpioPin: 6,
  timingUnitOverride: 'AUTO',
  repeatCount: 1,
  editMode: false,
  searchQuery: '',
  selectedCategory: 'ALL',
  activeProfile: null,
  savedProfiles: [],
  activeMacroId: null,
  macroStepIndex: 0,
  macroTotalSteps: 0,
  isTransmitting: false,
  isBeaconRunning: false,
  vfdPrimary: 'DIGA COMMANDER READY',
  vfdSecondary: 'KASEIKYO 37010 Hz · SET 1',
  lastWireHex: '—',
  lastProntoHex: '',
  lastBackendUsed: '—',
  error: null,
};

const irCommanderSlice = createSlice({
  name: 'irCommander',
  initialState: initialIrCommanderState,
  reducers: {
    setMode(state, action: PayloadAction<RemoteMode>) {
      state.mode = action.payload;
    },
    setRemoteUnit(state, action: PayloadAction<RemoteUnitSet>) {
      state.remoteUnit = action.payload;
    },
    setTransport(state, action: PayloadAction<IrTransportKind>) {
      state.transport = action.payload;
    },
    setPicoGpioPin(state, action: PayloadAction<number>) {
      state.picoGpioPin = action.payload;
    },
    setTimingUnitOverride(state, action: PayloadAction<IrTimingOverride>) {
      state.timingUnitOverride = action.payload;
    },
    setRepeatCount(state, action: PayloadAction<number>) {
      state.repeatCount = Math.min(3, Math.max(1, action.payload));
    },
    setEditMode(state, action: PayloadAction<boolean>) {
      state.editMode = action.payload;
    },
    setSearchQuery(state, action: PayloadAction<string>) {
      state.searchQuery = action.payload;
    },
    setSelectedCategory(state, action: PayloadAction<PanasonicCategory>) {
      state.selectedCategory = action.payload;
    },
    setActiveProfile(state, action: PayloadAction<DynamicKeymapProfile | null>) {
      state.activeProfile = action.payload;
    },
    setSavedProfiles(state, action: PayloadAction<DynamicKeymapProfile[]>) {
      state.savedProfiles = action.payload;
    },
    setMacroProgress(
      state,
      action: PayloadAction<{ id: string | null; step: number; total: number }>,
    ) {
      state.activeMacroId = action.payload.id;
      state.macroStepIndex = action.payload.step;
      state.macroTotalSteps = action.payload.total;
    },
    setTransmitting(state, action: PayloadAction<boolean>) {
      state.isTransmitting = action.payload;
    },
    setBeaconRunning(state, action: PayloadAction<boolean>) {
      state.isBeaconRunning = action.payload;
    },
    setTransmissionTelemetry(
      state,
      action: PayloadAction<{
        primary: string;
        secondary: string;
        wireHex: string;
        prontoHex: string;
        backendUsed: string;
      }>,
    ) {
      state.vfdPrimary = action.payload.primary;
      state.vfdSecondary = action.payload.secondary;
      state.lastWireHex = action.payload.wireHex;
      state.lastProntoHex = action.payload.prontoHex;
      state.lastBackendUsed = action.payload.backendUsed;
      state.error = null;
    },
    setError(state, action: PayloadAction<string | null>) {
      state.error = action.payload;
    },
  },
});

export const irCommanderActions = irCommanderSlice.actions;
export default irCommanderSlice.reducer;
