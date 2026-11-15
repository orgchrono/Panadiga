/**
 * @file viewmodels/picoDevice/stateHelpers.ts
 * @brief Pure state transitions, transport resolution, session store, and voltage safety helpers for Pico hardware link.
 */

import type {
  OcdCoreState,
  PicoDeviceStatus,
  PicoTransportMode,
  SerialLogEntry,
} from '@/models';
import {
  PICO_MAX_VDD_CEILING_VOLTS,
  PICO_MIN_VDD_THRESHOLD_VOLTS,
} from '@/constants';
import { isTargetPowered } from '@/utils/formatters';
import { invokeCommand } from '@/services/ipc';
import { parseDiagnosticOutput } from '@/services/diagnosticParser';

export interface UsePicoDeviceOptions {
  onLog?: (
    level: SerialLogEntry['level'],
    source: SerialLogEntry['source'],
    message: string
  ) => void;
}

export const DEFAULT_DISCONNECTED_STATUS: PicoDeviceStatus = {
  connected: false,
  port: '',
  baudRate: 115200,
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
};

export const PICO_PORT_PATTERN = /VID:2E8A|RP2040|RP2350|Pico|PANADIGA|P7501/i;

export function parseVddFromStatusRaw(rawStatus: string, fallback = 0): number {
  const parsed = parseDiagnosticOutput('STATUS', 'STATUS', rawStatus, 25);
  return parsed.vdd_volts ?? fallback;
}

export function deriveCoreStateFromVoltage(
  prevState: OcdCoreState,
  vddVolts: number,
  commandId?: string,
  commandSuccess?: boolean
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
  chipId?: string
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

export async function isGdbBridgeRunning(): Promise<boolean> {
  try {
    const status = await invokeCommand<{ running?: boolean }>('get_gdb_bridge_status_command');
    return Boolean(status?.running);
  } catch {
    return false;
  }
}

export function resolveTransportMode(portName: string): PicoTransportMode {
  if (portName.startsWith('ANDROID-USB-OTG')) return 'usb-otg';
  if (portName.startsWith('BLE_')) return 'ble';
  return 'usb';
}

function createPicoSessionStore() {
  let status: PicoDeviceStatus | null = DEFAULT_DISCONNECTED_STATUS;
  let port = '';
  let model = 'AUTO';

  return Object.freeze({
    getSnapshot: () => ({ status, port, model }),
    setStatus: (nextStatus: PicoDeviceStatus | null) => {
      status = nextStatus;
    },
    setPort: (nextPort: string) => {
      port = nextPort;
    },
    setModel: (nextModel: string) => {
      model = nextModel;
    },
  });
}

export const picoSessionStore = createPicoSessionStore();

export function pickPreferredSerialPort(allPorts: string[]): string | undefined {
  const realPorts = allPorts.filter(
    p =>
      !p.startsWith('BLE') &&
      !p.includes('BOOTSEL Mode') &&
      !p.includes('BOOTSEL') &&
      !p.includes('PID:0003')
  );

  return (
    realPorts.find(
      p =>
        p.includes('VID:2E8A') ||
        p.startsWith('ANDROID-USB-OTG') ||
        p.includes('RP2040') ||
        p.includes('RP2350') ||
        p.includes('Pico') ||
        p.includes('PANADIGA') ||
        p.includes('P7501')
    ) || realPorts[0]
  );
}

export type VoltageAlertLevel = 'ok' | 'low' | 'over';

export function classifyVoltageAlert(vddVolts: number): VoltageAlertLevel {
  if (vddVolts > PICO_MAX_VDD_CEILING_VOLTS) {
    return 'over';
  }
  if (vddVolts > 0.2 && vddVolts < PICO_MIN_VDD_THRESHOLD_VOLTS) {
    return 'low';
  }
  return 'ok';
}
