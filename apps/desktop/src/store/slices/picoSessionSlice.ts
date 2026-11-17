/**
 * @file store/slices/picoSessionSlice.ts
 * @brief Immutable Redux slice preserving Pico hardware link session across tab navigation.
 */

import { createSlice, type PayloadAction } from '@reduxjs/toolkit';
import { GDB_DEFAULT_LISTEN_ADDR, PICO_DEFAULT_BAUD_RATE } from '@/constants';
import type { PicoDeviceStatus, PicoDiagnosticResult, SerialLogEntry } from '@/models';

export const DEFAULT_DISCONNECTED_SESSION_STATUS: PicoDeviceStatus = Object.freeze({
  connected: false,
  port: '',
  baudRate: PICO_DEFAULT_BAUD_RATE,
  vdd_volts: 0.0,
  target_powered: false,
  breakin_mode: 2,
  core_state: 'DISCONNECTED',
  bus_freq_hz: 0,
  firmware_version: 'P7501 UART / Panadiga Master',
  chip_id: '',
  total_probes: 0,
  hits_detected: 0,
  open_bus_high: 0,
  open_bus_low: 0,
  ram_loader_injected: false,
  last_keepalive: '',
});

export interface QueuedPicoCommandRequest {
  readonly id: string;
  readonly commandId: string;
  readonly arg?: string;
}

export interface QueuedPicoSerialLine {
  readonly id: string;
  readonly line: string;
}

export interface PicoSessionState {
  readonly status: PicoDeviceStatus | null;
  readonly port: string;
  readonly model: string;
  readonly latestDiagnostic: PicoDiagnosticResult | null;
  readonly runningCommandId: string | null;
  readonly serialLogs: SerialLogEntry[];
  readonly gdbRunning: boolean;
  readonly gdbListenAddr: string;
  readonly sc0Active: boolean;
  readonly queuedCommandRequest: QueuedPicoCommandRequest | null;
  readonly queuedSerialLine: QueuedPicoSerialLine | null;
}

const initialState: PicoSessionState = {
  status: DEFAULT_DISCONNECTED_SESSION_STATUS,
  port: '',
  model: 'AUTO',
  latestDiagnostic: null,
  runningCommandId: null,
  serialLogs: [],
  gdbRunning: false,
  gdbListenAddr: GDB_DEFAULT_LISTEN_ADDR,
  sc0Active: false,
  queuedCommandRequest: null,
  queuedSerialLine: null,
};

const picoSessionSlice = createSlice({
  name: 'picoSession',
  initialState,
  reducers: {
    setPicoSessionStatus(state, action: PayloadAction<PicoDeviceStatus | null>) {
      return { ...state, status: action.payload };
    },
    setPicoSessionPort(state, action: PayloadAction<string>) {
      return { ...state, port: action.payload };
    },
    setPicoSessionModel(state, action: PayloadAction<string>) {
      return { ...state, model: action.payload };
    },
    setPicoSessionDiagnostic(state, action: PayloadAction<PicoDiagnosticResult | null>) {
      return { ...state, latestDiagnostic: action.payload };
    },
    setPicoSessionRunningCommand(state, action: PayloadAction<string | null>) {
      return { ...state, runningCommandId: action.payload };
    },
    setPicoSessionSerialLogs(state, action: PayloadAction<SerialLogEntry[]>) {
      return { ...state, serialLogs: action.payload };
    },
    setPicoSessionGdbStatus(
      state,
      action: PayloadAction<{ readonly running: boolean; readonly listenAddr: string }>,
    ) {
      return {
        ...state,
        gdbRunning: action.payload.running,
        gdbListenAddr: action.payload.listenAddr,
      };
    },
    setPicoSessionSc0Status(state, action: PayloadAction<boolean>) {
      return { ...state, sc0Active: action.payload };
    },
    setQueuedPicoCommandRequest(state, action: PayloadAction<QueuedPicoCommandRequest | null>) {
      return { ...state, queuedCommandRequest: action.payload };
    },
    setQueuedPicoSerialLine(state, action: PayloadAction<QueuedPicoSerialLine | null>) {
      return { ...state, queuedSerialLine: action.payload };
    },
  },
});

export const {
  setPicoSessionStatus,
  setPicoSessionPort,
  setPicoSessionModel,
  setPicoSessionDiagnostic,
  setPicoSessionRunningCommand,
  setPicoSessionSerialLogs,
  setPicoSessionGdbStatus,
  setPicoSessionSc0Status,
  setQueuedPicoCommandRequest,
  setQueuedPicoSerialLine,
} = picoSessionSlice.actions;
export const picoSessionReducer = picoSessionSlice.reducer;
