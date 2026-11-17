/**
 * @file viewmodels/picoDevice/stateHelpers.ts
 * @brief Pure state transitions, transport resolution, and voltage safety helpers for Pico hardware link.
 */

import {
  P7501_BOOTSEL_PID_MARKER,
  P7501_BOOTSEL_USB_TOKEN,
  PICO_BROWNOUT_FLOOR_VOLTS,
  PICO_MAX_VDD_CEILING_VOLTS,
  PICO_MIN_VDD_THRESHOLD_VOLTS,
} from '@/constants';
import type {
  OcdCoreState,
  PicoDeviceStatus,
  PicoTransportMode,
  SerialLogEntry,
} from '@/models';
import { parseDiagnosticOutput } from '@/services/diagnosticParser';
import { getGdbBridgeRunningStatus } from '@/services/picoService';
import { DEFAULT_DISCONNECTED_SESSION_STATUS } from '@/store/picoSessionBridge';
import { isTargetPowered } from '@/utils/formatters';

export interface UsePicoDeviceOptions {
  onLog?: (
    level: SerialLogEntry['level'],
    source: SerialLogEntry['source'],
    message: string,
  ) => void;
}

export const DEFAULT_DISCONNECTED_STATUS = DEFAULT_DISCONNECTED_SESSION_STATUS;

export const PICO_PORT_PATTERN = /VID:2E8A|RP2040|RP2350|Pico|PANADIGA|P7501/i;

export function parseVddFromStatusRaw(rawStatus: string, fallback = 0): number {
  const parsed = parseDiagnosticOutput('STATUS', 'STATUS', rawStatus, 0);
  return parsed.vdd_volts ?? fallback;
}

export function deriveCoreStateFromVoltage(
  prevState: OcdCoreState,
  vddVolts: number,
  commandId?: string,
  commandSuccess?: boolean,
): OcdCoreState {
  if (commandId === 'HANDSHAKE' && commandSuccess) {
    return 'HALTED';
  }
  if (isTargetPowered(vddVolts, PICO_MIN_VDD_THRESHOLD_VOLTS)) {
    return prevState === 'DISCONNECTED' ? 'RUNNING' : prevState;
  }
  return 'RESET';
}

export function buildMobileDeviceStatus(
  prev: PicoDeviceStatus | null,
  portName: string,
  vdd: number,
  chipId?: string,
): PicoDeviceStatus {
  const powered = isTargetPowered(vdd, PICO_MIN_VDD_THRESHOLD_VOLTS);
  return {
    ...(prev || DEFAULT_DISCONNECTED_STATUS),
    connected: true,
    port: portName,
    vdd_volts: vdd,
    target_powered: powered,
    core_state: powered ? 'HALTED' : 'RESET',
    ...(chipId ? { chip_id: chipId } : {}),
    last_keepalive: 'OK',
  };
}

export const isGdbBridgeRunning = getGdbBridgeRunningStatus;

export function resolveTransportMode(portName: string): PicoTransportMode {
  if (portName.startsWith('ANDROID-USB-OTG')) return 'usb-otg';
  if (portName.startsWith('BLE_')) return 'ble';
  return 'usb';
}

export function pickPreferredSerialPort(allPorts: string[]): string | undefined {
  const realPorts = allPorts.filter(
    (p) =>
      !p.startsWith('BLE') &&
      !p.includes(P7501_BOOTSEL_USB_TOKEN) &&
      !p.includes(P7501_BOOTSEL_PID_MARKER),
  );

  return (
    realPorts.find((p) => PICO_PORT_PATTERN.test(p) || p.startsWith('ANDROID-USB-OTG')) ||
    realPorts[0]
  );
}

export type VoltageAlertLevel = 'ok' | 'low' | 'over';

export function classifyVoltageAlert(vddVolts: number): VoltageAlertLevel {
  if (vddVolts > PICO_MAX_VDD_CEILING_VOLTS) {
    return 'over';
  }
  if (vddVolts > PICO_BROWNOUT_FLOOR_VOLTS && vddVolts < PICO_MIN_VDD_THRESHOLD_VOLTS) {
    return 'low';
  }
  return 'ok';
}
