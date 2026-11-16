/**
 * @file store/slices/shellSlice.ts
 * @brief Redux slice for top-level navigation view, mobile drawer, live telemetry summary, and Rust Hardware SSOT.
 */

import { createAsyncThunk, createSlice, type PayloadAction } from '@reduxjs/toolkit';
import type { HardwareSsotSpec, PicoDeviceStatus, ViewKey } from '@/models';
import { invokeCommand } from '@/services/ipc';
import type { ShellTelemetryStatus } from '@/types/workbench';

export interface ShellSliceState {
  readonly currentView: ViewKey;
  readonly mobileNavOpen: boolean;
  readonly hardwareStatus: ShellTelemetryStatus;
  readonly ssot: HardwareSsotSpec | null;
  readonly ssotLoaded: boolean;
  readonly ssotLoading: boolean;
  readonly ssotError: string | null;
}

const initialState: ShellSliceState = {
  currentView: 'dashboard',
  mobileNavOpen: false,
  hardwareStatus: { connected: false, vdd_volts: 0.0 },
  ssot: null,
  ssotLoaded: false,
  ssotLoading: false,
  ssotError: null,
};

function mergeTelemetryStatus(
  prev: ShellTelemetryStatus,
  incoming: PicoDeviceStatus,
): ShellTelemetryStatus {
  const connected = Boolean(incoming.connected);
  const rawVdd = incoming.vdd_volts;
  const vddVolts =
    typeof rawVdd === 'number' && rawVdd > 0
      ? rawVdd
      : prev.connected && !connected
        ? 0.0
        : prev.vdd_volts > 0 && connected
          ? prev.vdd_volts
          : Number(rawVdd ?? 0);
  return { connected, vdd_volts: vddVolts };
}

export const fetchHardwareSsot = createAsyncThunk<
  HardwareSsotSpec,
  boolean | undefined,
  { state: { shell: ShellSliceState } }
>(
  'shell/fetchHardwareSsot',
  async (_forceReload, { rejectWithValue }) => {
    try {
      const spec = await invokeCommand<HardwareSsotSpec>('get_hardware_ssot_spec');
      if (!spec || !Array.isArray(spec.pico_commands) || spec.pico_commands.length === 0) {
        return rejectWithValue('Especificação SSOT de hardware vazia ou inválida recebida do backend.');
      }
      return spec;
    } catch (err) {
      return rejectWithValue(err instanceof Error ? err.message : String(err));
    }
  },
  {
    condition: (forceReload, { getState }) => {
      if (forceReload) return true;
      const { ssotLoaded, ssotLoading } = getState().shell;
      return !ssotLoaded && !ssotLoading;
    },
  },
);

const shellSlice = createSlice({
  name: 'shell',
  initialState,
  reducers: {
    setCurrentView(state, action: PayloadAction<ViewKey>) {
      return { ...state, currentView: action.payload };
    },
    setMobileNavOpen(state, action: PayloadAction<boolean>) {
      return { ...state, mobileNavOpen: action.payload };
    },
    applyTelemetryStatus(state, action: PayloadAction<PicoDeviceStatus>) {
      return {
        ...state,
        hardwareStatus: mergeTelemetryStatus(state.hardwareStatus, action.payload),
      };
    },
    resetSsotState(state) {
      return {
        ...state,
        ssot: null,
        ssotLoaded: false,
        ssotLoading: false,
        ssotError: null,
      };
    },
  },
  extraReducers: builder => {
    builder
      .addCase(fetchHardwareSsot.pending, state => ({
        ...state,
        ssotLoading: true,
        ssotError: null,
      }))
      .addCase(fetchHardwareSsot.fulfilled, (state, action) => ({
        ...state,
        ssot: action.payload,
        ssotLoaded: true,
        ssotLoading: false,
        ssotError: null,
      }))
      .addCase(fetchHardwareSsot.rejected, (state, action) => ({
        ...state,
        ssot: null,
        ssotLoaded: false,
        ssotLoading: false,
        ssotError:
          typeof action.payload === 'string'
            ? action.payload
            : action.error.message ?? 'Falha ao carregar especificação SSOT.',
      }));
  },
});

export const { setCurrentView, setMobileNavOpen, applyTelemetryStatus, resetSsotState } =
  shellSlice.actions;

export const shellReducer = shellSlice.reducer;
