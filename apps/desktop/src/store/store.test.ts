/**
 * @file store/store.test.ts
 * @brief Unit tests validating Redux slices, immutable transitions, thunk SSOT fetch, and storage persistence.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { createAppStore } from './index';
import { applyTelemetryStatus, fetchHardwareSsot, setCurrentView, setMobileNavOpen } from './slices/shellSlice';
import {
  acceptCompliance,
  markPipelineFlag,
  resetWorkbenchPreferences,
  setTheme,
  toggleBottomDock,
  toggleBottomExpanded,
  toggleHud,
  toggleLeftDock,
  toggleMode,
  toggleRightDock,
} from './slices/workbenchSlice';
import {
  setPicoSessionDiagnostic,
  setPicoSessionGdbStatus,
  setPicoSessionRunningCommand,
  setPicoSessionSc0Status,
} from './slices/picoSessionSlice';
import {
  clearPicoCommandRequest,
  clearPicoSerialLine,
  enqueuePicoCommandRequest,
  enqueuePicoSerialLine,
  getPicoSessionSnapshot,
} from './picoSessionBridge';
import { STORAGE_COMPLIANCE_KEY, STORAGE_DOCKS_KEY } from './storage';
import type { PicoDiagnosticResult } from '@/models';
import { clearToasts, dismissToast, enqueueToast } from './slices/toastSlice';
import { clearLiveCapture, recordLiveCapture } from './slices/nvramCaptureSlice';
import { mockNvramCleanReport } from '@/test/mocks/mockNvram';

describe('Panadiga Redux Store Architecture', () => {
  beforeEach(() => {
    if (typeof window !== 'undefined') {
      window.localStorage.clear();
    }
  });

  it('manages shell navigation, telemetry state, and async SSOT loading cleanly', async () => {
    const testStore = createAppStore();

    expect(testStore.getState().shell.currentView).toBe('dashboard');
    testStore.dispatch(setCurrentView('picobench'));
    testStore.dispatch(setMobileNavOpen(true));
    expect(testStore.getState().shell.currentView).toBe('picobench');
    expect(testStore.getState().shell.mobileNavOpen).toBe(true);

    testStore.dispatch(
      applyTelemetryStatus({
        connected: true,
        port: 'COM10',
        baudRate: 115200,
        vdd_volts: 3.29,
        target_powered: true,
        breakin_mode: 2,
        core_state: 'RUNNING',
        bus_freq_hz: 27000000,
        firmware_version: 'P7501',
        chip_id: 'RP2040',
        total_probes: 0,
        hits_detected: 0,
        open_bus_high: 0,
        open_bus_low: 0,
        ram_loader_injected: false,
        last_keepalive: 'OK',
      }),
    );
    expect(testStore.getState().shell.hardwareStatus).toEqual({
      connected: true,
      vdd_volts: 3.29,
    });

    await testStore.dispatch(fetchHardwareSsot(false));
    expect(testStore.getState().shell.ssotLoaded).toBe(true);
    expect(testStore.getState().shell.ssot?.pico_commands.length).toBeGreaterThanOrEqual(25);
  });

  it('persists workbench accessibility settings and resets storage cleanly', () => {
    const testStore = createAppStore();

    testStore.dispatch(toggleMode());
    expect(testStore.getState().workbench.mode).toBe('basic');
    testStore.dispatch(toggleHud());
    expect(testStore.getState().workbench.hudOpen).toBe(true);
    testStore.dispatch(setTheme('high-contrast'));
    expect(document.documentElement.getAttribute('data-theme')).toBe('high-contrast');

    testStore.dispatch(markPipelineFlag({ key: 'nvramCleaned', value: true }));
    expect(testStore.getState().workbench.pipeline.nvramCleaned).toBe(true);

    testStore.dispatch(resetWorkbenchPreferences());
    expect(testStore.getState().workbench.mode).toBe('advanced');
    expect(testStore.getState().workbench.accessibility.theme).toBe('dark-lab');
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark-lab');
  });

  it('queues and dismisses toasts and tracks live NVRAM captures immutably', () => {
    const testStore = createAppStore();

    testStore.dispatch(
      enqueueToast({ id: 't1', type: 'info', title: 'Teste', message: 'Redux activo' }),
    );
    expect(testStore.getState().toast.items).toHaveLength(1);
    testStore.dispatch(dismissToast('t1'));
    expect(testStore.getState().toast.items).toHaveLength(0);

    testStore.dispatch(enqueueToast({ id: 't2', type: 'success', title: 'OK' }));
    testStore.dispatch(clearToasts());
    expect(testStore.getState().toast.items).toHaveLength(0);

    testStore.dispatch(
      recordLiveCapture({
        source: 'UNIT_TEST',
        port: 'COM10',
        base_bus_address: '0x807F0000',
        captured_bytes: 16,
        captured_at: '12:00:00',
        report: mockNvramCleanReport,
      }),
    );
    expect(testStore.getState().nvramCapture.latestCapture?.source).toBe('UNIT_TEST');

    testStore.dispatch(clearLiveCapture());
    expect(testStore.getState().nvramCapture.latestCapture).toBeNull();
  });

  it('requires initial compliance acknowledgment and persists acceptance across fresh stores', () => {
    const initialStore = createAppStore();
    expect(initialStore.getState().workbench.complianceAccepted).toBe(false);
    expect(initialStore.getState().workbench.complianceModalOpen).toBe(true);
    expect(window.localStorage.getItem(STORAGE_COMPLIANCE_KEY)).toBeNull();

    initialStore.dispatch(acceptCompliance());
    expect(initialStore.getState().workbench.complianceAccepted).toBe(true);
    expect(initialStore.getState().workbench.complianceModalOpen).toBe(false);
    expect(window.localStorage.getItem(STORAGE_COMPLIANCE_KEY)).toBe('accepted');

    const reloadedStore = createAppStore();
    expect(reloadedStore.getState().workbench.complianceAccepted).toBe(true);
    expect(reloadedStore.getState().workbench.complianceModalOpen).toBe(false);
  });

  it('persists dock toggles and resets docks without clearing compliance acknowledgment', () => {
    const testStore = createAppStore();
    testStore.dispatch(acceptCompliance());

    testStore.dispatch(toggleLeftDock());
    testStore.dispatch(toggleRightDock());
    testStore.dispatch(toggleBottomDock());
    testStore.dispatch(toggleBottomExpanded());

    expect(testStore.getState().workbench.docks).toEqual({
      leftOpen: true,
      rightOpen: true,
      bottomOpen: true,
      bottomExpanded: true,
    });
    expect(window.localStorage.getItem(STORAGE_DOCKS_KEY)).toBe(
      JSON.stringify({
        leftOpen: true,
        rightOpen: true,
        bottomOpen: true,
        bottomExpanded: true,
      }),
    );

    const persistedStore = createAppStore();
    expect(persistedStore.getState().workbench.docks).toEqual({
      leftOpen: true,
      rightOpen: true,
      bottomOpen: true,
      bottomExpanded: true,
    });

    persistedStore.dispatch(resetWorkbenchPreferences());
    expect(persistedStore.getState().workbench.docks).toEqual({
      leftOpen: false,
      rightOpen: false,
      bottomOpen: false,
      bottomExpanded: false,
    });
    expect(window.localStorage.getItem(STORAGE_DOCKS_KEY)).toBeNull();
    expect(window.localStorage.getItem(STORAGE_COMPLIANCE_KEY)).toBe('accepted');
  });

  it('updates picoSessionSlice diagnostics, GDB/SC0 status, and queued dock commands', () => {
    const testStore = createAppStore();
    const sampleDiagnostic: PicoDiagnosticResult = {
      command_id: 'PINCHECK',
      raw_command: 'PINCHECK',
      success: true,
      elapsed_ms: 42,
      summary: 'P7501 LVTTL nominal; GP3 Hi-Z confirmed.',
      raw_output: 'OK PINCHECK',
      vdd_volts: 3.29,
      pincheck_rows: [],
      transition_rows: [],
      dump_rows: [],
    };

    testStore.dispatch(setPicoSessionDiagnostic(sampleDiagnostic));
    testStore.dispatch(setPicoSessionRunningCommand('PINCHECK'));
    testStore.dispatch(setPicoSessionGdbStatus({ running: true, listenAddr: '127.0.0.1:3333' }));
    testStore.dispatch(setPicoSessionSc0Status(true));

    const session = testStore.getState().picoSession;
    expect(session.latestDiagnostic?.command_id).toBe('PINCHECK');
    expect(session.runningCommandId).toBe('PINCHECK');
    expect(session.gdbRunning).toBe(true);
    expect(session.gdbListenAddr).toBe('127.0.0.1:3333');
    expect(session.sc0Active).toBe(true);

    enqueuePicoCommandRequest('DUMP', '0x807F0000');
    enqueuePicoSerialLine('STATUS');
    const snapshot = getPicoSessionSnapshot();
    expect(snapshot.queuedCommandRequest?.commandId).toBe('DUMP');
    expect(snapshot.queuedCommandRequest?.arg).toBe('0x807F0000');
    expect(snapshot.queuedSerialLine?.line).toBe('STATUS');

    clearPicoCommandRequest();
    clearPicoSerialLine();
    expect(getPicoSessionSnapshot().queuedCommandRequest).toBeNull();
    expect(getPicoSessionSnapshot().queuedSerialLine).toBeNull();
  });
});
