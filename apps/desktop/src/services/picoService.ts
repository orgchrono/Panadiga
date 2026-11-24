/**
 * @file services/picoService.ts
 * @brief Domain RPC & event-stream facade for Raspberry Pi Pico (P7501 UART, OCD, BOOTSEL, SC0 & GDB).
 */

import type {
  Am34RegisterState,
  ColdBreakinMode,
  DigaModelProfile,
  FirstContactBackupResult,
  FlashReport,
  GdbBridgeStatus,
  GdbPacketTelemetryEvent,
  InCircuitNvramEraseResult,
  LiveNvramCaptureEvent,
  PicoDeviceStatus,
  PicoDiagnosticResult,
  PicoUf2Variant,
  ReactiveSerialEvent,
  Sc0BridgeDataEvent,
  Sc0BridgeInputMode,
  Sc0BridgeStatus,
} from '@/models';
import { mintOperationNonceId } from './benchStreamService';
import { emitLocalEvent, hasTestIpcBridge, invokeCommand, isTauriEnvironment, listenEvent } from './ipc';

export const isPicoIpcAvailable = (): boolean => isTauriEnvironment() || hasTestIpcBridge();

export const listPicoSerialPorts = (): Promise<string[]> =>
  invokeCommand<string[]>('list_pico_serial_ports');

export const getDigaModelCatalog = (): Promise<DigaModelProfile[]> =>
  invokeCommand<DigaModelProfile[]>('get_diga_model_catalog');

export const getPicoStatus = (portName: string): Promise<PicoDeviceStatus> =>
  invokeCommand<PicoDeviceStatus>('pico_get_status', { portName });

export const setPicoBreakinMode = (portName: string, mode: ColdBreakinMode): Promise<PicoDeviceStatus> =>
  invokeCommand<PicoDeviceStatus>('pico_set_breakin_mode', { portName, mode });

export const setPicoTelemetryStream = (active: boolean, portName?: string): Promise<unknown> =>
  invokeCommand('set_pico_telemetry_stream', {
    active,
    ...(portName ? { portName } : {}),
  });

export const sendPicoSerialCommand = (portName: string, command: string): Promise<string> =>
  invokeCommand<string>('pico_send_serial_command', {
    portName,
    command,
  });

export const triggerPicoBootselReboot = (portName: string): Promise<string> =>
  invokeCommand<string>('pico_trigger_bootsel_reboot', { portName });

export const detectPicoBootselDrives = (): Promise<string[]> =>
  invokeCommand<string[]>('pico_detect_bootsel_drives');

export const flashPicoFirmwareUf2 = async (targetFamily: PicoUf2Variant): Promise<FlashReport> => {
  const nonceId = await mintOperationNonceId(
    'nor_write',
    targetFamily.trim() || 'auto',
    'pico_uf2',
  );
  return invokeCommand<FlashReport>('pico_flash_firmware', {
    targetFamily,
    ...(nonceId ? { nonceId } : {}),
  });
};

export const capturePicoNvramLive = (
  portName: string,
  baseAddr: number,
  wordCount: number,
): Promise<LiveNvramCaptureEvent> =>
  invokeCommand<LiveNvramCaptureEvent>('pico_capture_nvram_live', {
    portName,
    baseAddr,
    wordCount,
  });

export type ExternalToolId = 'gdb' | 'ghidra' | 'ghidra_sync' | 'ghidra_mcp' | 'hxd' | 'terminal';

export interface PicoConnectTestResult {
  connected: boolean;
  port_name: string;
  baud_rate: number;
  vdd_voltage: number;
  target_powered: boolean;
  firmware_banner: string;
  message: string;
}

export const connectAndTestPicoPort = (
  portName: string,
  baudRate: number,
): Promise<PicoConnectTestResult> =>
  invokeCommand<PicoConnectTestResult>('pico_connect_and_test', {
    portName,
    baudRate,
  });

export const executePicoDiagnostic = (
  portName: string,
  commandId: string,
  arg?: string | null,
): Promise<PicoDiagnosticResult> =>
  invokeCommand<PicoDiagnosticResult>('pico_execute_diagnostic', {
    portName,
    commandId,
    arg: arg ?? null,
  });

export const parsePicoDiagnosticOutput = (
  commandId: string,
  rawCommand: string,
  rawOutput: string,
  elapsedMs: number,
): Promise<PicoDiagnosticResult> =>
  invokeCommand<PicoDiagnosticResult>('pico_parse_diagnostic_output', {
    commandId,
    rawCommand,
    rawOutput,
    elapsedMs,
  });

export const runPicoFirstContactBackup = (
  portName: string,
  timeoutMs?: number,
): Promise<FirstContactBackupResult> =>
  invokeCommand<FirstContactBackupResult>('pico_first_contact_backup', {
    portName,
    ...(timeoutMs !== undefined ? { timeoutMs } : {}),
  });

export const runPicoInCircuitErase = async (
  portName: string,
  confirmToken?: string,
): Promise<InCircuitNvramEraseResult> => {
  const nonceId = await mintOperationNonceId(
    'ram_probe',
    portName.trim() || 'auto',
    'ram_loader',
  );
  return invokeCommand<InCircuitNvramEraseResult>('pico_inject_ram_loader', {
    portName,
    ...(confirmToken !== undefined ? { confirmToken } : {}),
    ...(nonceId ? { nonceId } : {}),
  });
};

export const getGdbBridgeStatus = (): Promise<GdbBridgeStatus> =>
  invokeCommand<GdbBridgeStatus>('get_gdb_bridge_status_command');

export async function getGdbBridgeRunningStatus(): Promise<boolean> {
  try {
    const status = await getGdbBridgeStatus();
    return Boolean(status?.running);
  } catch {
    return false;
  }
}

export const getAm34Registers = (): Promise<Am34RegisterState> =>
  invokeCommand<Am34RegisterState>('get_am34_registers_command');

export const getGdbTelemetry = (): Promise<GdbPacketTelemetryEvent[]> =>
  invokeCommand<GdbPacketTelemetryEvent[]>('get_gdb_telemetry_command');

export const startGdbBridge = (listenAddr: string, portName?: string): Promise<GdbBridgeStatus> =>
  invokeCommand<GdbBridgeStatus>('start_gdb_bridge_command', {
    portName: portName || undefined,
    listenAddr,
  });

export const stopGdbBridge = (): Promise<GdbBridgeStatus> =>
  invokeCommand<GdbBridgeStatus>('stop_gdb_bridge_command');

export const launchExternalTool = (tool: ExternalToolId): Promise<string> =>
  invokeCommand<string>('launch_external_tool_command', { tool });

export const getSc0BridgeStatus = (): Promise<Sc0BridgeStatus> =>
  invokeCommand<Sc0BridgeStatus>('pico_get_bridge_status');

export const startSc0Bridge = (
  portName: string,
  baudRate: number,
  bootMode: string,
): Promise<Sc0BridgeStatus> =>
  invokeCommand<Sc0BridgeStatus>('pico_start_sc0_bridge', {
    portName,
    baudRate,
    bootMode,
  });

export const stopSc0Bridge = (): Promise<Sc0BridgeStatus> =>
  invokeCommand<Sc0BridgeStatus>('pico_stop_sc0_bridge');

export const sendSc0BridgeInput = (inputMode: Sc0BridgeInputMode, payload: string): Promise<number> =>
  invokeCommand<number>('pico_send_bridge_input', { inputMode, payload });

export const emitPicoTelemetry = (status: PicoDeviceStatus): void =>
  emitLocalEvent('pico://telemetry', status);

export const subscribePicoTelemetry = (handler: (status: PicoDeviceStatus) => void) =>
  listenEvent('pico://telemetry', handler);

export const subscribePicoDiagnosticResult = (handler: (diag: PicoDiagnosticResult) => void) =>
  listenEvent('pico://diagnostic-result', handler);

export const subscribePicoPortsChanged = (handler: (ports: string[]) => void) =>
  listenEvent('pico://ports-changed', handler);

export const subscribePicoNvramLiveCapture = (handler: (evt: LiveNvramCaptureEvent) => void) =>
  listenEvent('pico://nvram-live-capture', handler);

export const subscribePicoSerialLog = (handler: (evt: ReactiveSerialEvent) => void) =>
  listenEvent('pico://serial-log', handler);

export const subscribePicoBridgeData = (handler: (chunk: Sc0BridgeDataEvent) => void) =>
  listenEvent('pico://bridge-data', handler);
