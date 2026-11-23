/**
 * Stateful mock Pico probe status & SC0 UART bridge store shared by test IPC handlers.
 */

import type { Sc0BridgeStatus } from '@/models';
import { mockPicoDeviceStatus } from './mockData';

function createMockPicoStateStore() {
  let connected = false;
  let port = '';
  let bootselDetected = false;
  let bridgeStatus: Sc0BridgeStatus = {
    running: false,
    port_name: '',
    baud_rate: 115200,
    boot_mode: 'PASSIVE',
    bytes_rx: 0,
    bytes_tx: 0,
    last_event: 'Ponte SC0 em standby',
  };

  return Object.freeze({
    isConnected: () => connected,
    getPort: () => port,
    isBootselDetected: () => bootselDetected,
    getBridgeStatus: () => ({ ...bridgeStatus }),
    setConnection: (nextConnected: boolean, nextPort?: string) => {
      connected = nextConnected;
      if (nextPort !== undefined) {
        port = nextPort;
      }
    },
    setBootselDetected: (detected: boolean) => {
      bootselDetected = detected;
    },
    startBridge: (portName: string, baud: number, bootMode: string): Sc0BridgeStatus => {
      bridgeStatus = {
        running: true,
        port_name: portName,
        baud_rate: baud,
        boot_mode: bootMode,
        bytes_rx: 32,
        bytes_tx: 0,
        last_event: `BRIDGE ${baud} 8N1 ativa em ${portName}`,
      };
      return { ...bridgeStatus };
    },
    recordBridgeExchange: (txLen: number, rxLen: number, lastEvent: string): void => {
      bridgeStatus = {
        ...bridgeStatus,
        bytes_tx: bridgeStatus.bytes_tx + txLen,
        bytes_rx: bridgeStatus.bytes_rx + rxLen,
        last_event: lastEvent,
      };
    },
    stopBridge: (): Sc0BridgeStatus => {
      bridgeStatus = {
        ...bridgeStatus,
        running: false,
        last_event: 'Ponte SC0 encerrada cleanly (~~~)',
      };
      return { ...bridgeStatus };
    },
  });
}

export const mockPicoState = createMockPicoStateStore();

export function buildMockPicoStatus(fallbackPort = ''): typeof mockPicoDeviceStatus {
  const connected = mockPicoState.isConnected();
  const port = mockPicoState.getPort() || fallbackPort;
  return {
    ...mockPicoDeviceStatus,
    connected,
    port,
    vdd_volts: connected ? 3.28 : 0.0,
    target_powered: connected,
    core_state: connected ? 'HALTED' : 'DISCONNECTED',
    wdt_active: connected,
    wdt_timeout_ms: connected ? 4000 : undefined,
    wdt_recoveries: connected ? 0 : undefined,
    wdt_reboot_src: connected ? 'CLEAN' : undefined,
    wdt_fault_stage: connected ? 'IDLE' : undefined,
    wdt_last_cmd: connected ? 'STATUS' : undefined,
  };
}

