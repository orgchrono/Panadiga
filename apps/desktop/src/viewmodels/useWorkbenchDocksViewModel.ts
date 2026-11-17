/**
 * @file viewmodels/useWorkbenchDocksViewModel.ts
 * @brief MVVM ViewModel for AppShell Left/Right/Bottom docks and StatusBar backed by Redux picoSession & shell slices.
 */

import { useCallback, useMemo, useState } from 'react';
import type { ColdBreakinMode, ViewKey } from '@/models';
import { useAppDispatch, useAppSelector } from '@/store/hooks';
import {
  enqueuePicoCommandRequest,
  enqueuePicoSerialLine,
  updatePicoSessionSerialLogs,
} from '@/store/picoSessionBridge';
import { setCurrentView } from '@/store/slices/shellSlice';
import { resolveTransportMode } from './picoDevice/stateHelpers';

export function useWorkbenchDocksViewModel() {
  const dispatch = useAppDispatch();
  const picoSession = useAppSelector((s) => s.picoSession);
  const ssot = useAppSelector((s) => s.shell.ssot);
  const currentView = useAppSelector((s) => s.shell.currentView);
  const [consoleInput, setConsoleInput] = useState<string>('');

  const transportMode = useMemo(
    () => resolveTransportMode(picoSession.port),
    [picoSession.port],
  );

  const ensurePicoBenchView = useCallback(() => {
    if (currentView !== 'picobench') {
      dispatch(setCurrentView('picobench'));
    }
  }, [currentView, dispatch]);

  const navigateToView = useCallback(
    (view: ViewKey) => {
      dispatch(setCurrentView(view));
    },
    [dispatch],
  );

  const navigateAndFocusPicoSection = useCallback(
    (_section?: string) => {
      ensurePicoBenchView();
    },
    [ensurePicoBenchView],
  );

  const runQuickCommand = useCallback(
    (commandId: string, arg?: string) => {
      ensurePicoBenchView();
      enqueuePicoCommandRequest(commandId, arg);
    },
    [ensurePicoBenchView],
  );

  const sendQuickSerialLine = useCallback(
    (line: string) => {
      if (!line.trim()) return;
      ensurePicoBenchView();
      enqueuePicoSerialLine(line);
    },
    [ensurePicoBenchView],
  );

  const submitConsoleInput = useCallback(() => {
    sendQuickSerialLine(consoleInput);
    setConsoleInput('');
  }, [consoleInput, sendQuickSerialLine]);

  const clearQuickLogs = useCallback(() => {
    updatePicoSessionSerialLogs([]);
  }, []);

  const hardwareConnected = Boolean(picoSession.status?.connected);
  const vddVolts = picoSession.status?.vdd_volts ?? 0;
  const breakinMode: ColdBreakinMode = picoSession.status?.breakin_mode ?? 2;

  return {
    picoStatus: picoSession.status,
    hardwareConnected,
    vddVolts,
    breakinMode,
    selectedPort: picoSession.port,
    selectedModel: picoSession.model,
    transportMode,
    latestDiagnostic: picoSession.latestDiagnostic,
    runningCommandId: picoSession.runningCommandId,
    serialLogs: picoSession.serialLogs,
    gdbRunning: picoSession.gdbRunning,
    gdbListenAddr: picoSession.gdbListenAddr,
    sc0Active: picoSession.sc0Active,
    ssot,
    currentView,
    consoleInput,
    setConsoleInput,
    submitConsoleInput,
    clearQuickLogs,
    runQuickCommand,
    sendQuickSerialLine,
    navigateAndFocusPicoSection,
    navigateToView,
  };
}
