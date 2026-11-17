import { createSlice, type PayloadAction } from '@reduxjs/toolkit';
import { IR_MAX_REPEAT_COUNT } from '@/constants';
import type {
  DynamicKeymapProfile,
  IrTimingOverride,
  IrTransportKind,
  PanasonicCategory,
  RemoteMode,
  RemoteUnitSet,
} from '@/models/ir';

const DEFAULT_IR_GPIO_PIN = 6;

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
  picoGpioPin: DEFAULT_IR_GPIO_PIN,
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
      return { ...state, mode: action.payload };
    },
    setRemoteUnit(state, action: PayloadAction<RemoteUnitSet>) {
      return { ...state, remoteUnit: action.payload };
    },
    setTransport(state, action: PayloadAction<IrTransportKind>) {
      return { ...state, transport: action.payload };
    },
    setPicoGpioPin(state, action: PayloadAction<number>) {
      return { ...state, picoGpioPin: action.payload };
    },
    setTimingUnitOverride(state, action: PayloadAction<IrTimingOverride>) {
      return { ...state, timingUnitOverride: action.payload };
    },
    setRepeatCount(state, action: PayloadAction<number>) {
      return {
        ...state,
        repeatCount: Math.min(IR_MAX_REPEAT_COUNT, Math.max(1, action.payload)),
      };
    },
    setEditMode(state, action: PayloadAction<boolean>) {
      return { ...state, editMode: action.payload };
    },
    setSearchQuery(state, action: PayloadAction<string>) {
      return { ...state, searchQuery: action.payload };
    },
    setSelectedCategory(state, action: PayloadAction<PanasonicCategory>) {
      return { ...state, selectedCategory: action.payload };
    },
    setActiveProfile(state, action: PayloadAction<DynamicKeymapProfile | null>) {
      return { ...state, activeProfile: action.payload };
    },
    setSavedProfiles(state, action: PayloadAction<DynamicKeymapProfile[]>) {
      return { ...state, savedProfiles: action.payload };
    },
    setMacroProgress(
      state,
      action: PayloadAction<{ id: string | null; step: number; total: number }>,
    ) {
      return {
        ...state,
        activeMacroId: action.payload.id,
        macroStepIndex: action.payload.step,
        macroTotalSteps: action.payload.total,
      };
    },
    setTransmitting(state, action: PayloadAction<boolean>) {
      return { ...state, isTransmitting: action.payload };
    },
    setBeaconRunning(state, action: PayloadAction<boolean>) {
      return { ...state, isBeaconRunning: action.payload };
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
      return {
        ...state,
        vfdPrimary: action.payload.primary,
        vfdSecondary: action.payload.secondary,
        lastWireHex: action.payload.wireHex,
        lastProntoHex: action.payload.prontoHex,
        lastBackendUsed: action.payload.backendUsed,
        error: null,
      };
    },
    setError(state, action: PayloadAction<string | null>) {
      return { ...state, error: action.payload };
    },
  },
});

export const irCommanderActions = irCommanderSlice.actions;
export default irCommanderSlice.reducer;
