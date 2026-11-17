/**
 * @file store/picoSessionBridge.ts
 * @brief Store-backed Pico session snapshot accessor & updater (replaces module-global `let` store).
 */

import type { PicoDeviceStatus, PicoDiagnosticResult, SerialLogEntry } from '@/models';
import { createIdGenerator } from '@/utils/formatters';
import { store } from './index';
import {
  DEFAULT_DISCONNECTED_SESSION_STATUS,
  setPicoSessionDiagnostic,
  setPicoSessionGdbStatus,
  setPicoSessionModel,
  setPicoSessionPort,
  setPicoSessionRunningCommand,
  setPicoSessionSc0Status,
  setPicoSessionSerialLogs,
  setPicoSessionStatus,
  setQueuedPicoCommandRequest,
  setQueuedPicoSerialLine,
} from './slices/picoSessionSlice';

export { DEFAULT_DISCONNECTED_SESSION_STATUS };

const nextCommandRequestId = createIdGenerator('pico-cmd');
const nextSerialLineId = createIdGenerator('pico-line');

export function getPicoSessionSnapshot() {
  return store.getState().picoSession;
}

export function updatePicoSessionStatus(nextStatus: PicoDeviceStatus | null): void {
  store.dispatch(setPicoSessionStatus(nextStatus));
}

export function updatePicoSessionPort(nextPort: string): void {
  store.dispatch(setPicoSessionPort(nextPort));
}

export function updatePicoSessionModel(nextModel: string): void {
  store.dispatch(setPicoSessionModel(nextModel));
}

export function updatePicoSessionDiagnostic(diagnostic: PicoDiagnosticResult | null): void {
  store.dispatch(setPicoSessionDiagnostic(diagnostic));
}

export function updatePicoSessionRunningCommand(commandId: string | null): void {
  store.dispatch(setPicoSessionRunningCommand(commandId));
}

export function updatePicoSessionSerialLogs(logs: SerialLogEntry[]): void {
  store.dispatch(setPicoSessionSerialLogs(logs));
}

export function updatePicoSessionGdbStatus(running: boolean, listenAddr: string): void {
  store.dispatch(setPicoSessionGdbStatus({ running, listenAddr }));
}

export function updatePicoSessionSc0Status(active: boolean): void {
  store.dispatch(setPicoSessionSc0Status(active));
}

export function enqueuePicoCommandRequest(commandId: string, arg?: string): void {
  store.dispatch(
    setQueuedPicoCommandRequest(
      arg !== undefined
        ? { id: nextCommandRequestId(), commandId, arg }
        : { id: nextCommandRequestId(), commandId },
    ),
  );
}

export function clearPicoCommandRequest(): void {
  store.dispatch(setQueuedPicoCommandRequest(null));
}

export function enqueuePicoSerialLine(line: string): void {
  store.dispatch(setQueuedPicoSerialLine({ id: nextSerialLineId(), line }));
}

export function clearPicoSerialLine(): void {
  store.dispatch(setQueuedPicoSerialLine(null));
}
