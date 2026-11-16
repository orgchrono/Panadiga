/**
 * @file viewmodels/picoDevice/picoConnectionActions.ts
 * @brief Transport connection & disconnection routines for USB CDC, Android USB-OTG, and BLE NUS.
 */

import type { Dispatch, SetStateAction } from 'react';
import type { PicoDeviceStatus } from '@/models';
import { invokeCommand } from '@/services/ipc';
import { bleService } from '@/services/bleService';
import {
  connectAndroidUsbOtg,
  disconnectAndroidUsbOtg,
  sendAndroidUsbOtgCommand,
} from '@/services/androidBridge';
import {
  buildMobileDeviceStatus,
  DEFAULT_DISCONNECTED_STATUS,
  parseVddFromStatusRaw,
  type UsePicoDeviceOptions,
} from './stateHelpers';

export interface TransportActionContext {
  targetPort: string;
  setDeviceStatus: Dispatch<SetStateAction<PicoDeviceStatus | null>>;
  sendDirectRawCommand: (command: string) => Promise<string>;
  onLog?: UsePicoDeviceOptions['onLog'];
}

export async function disconnectTransportPort({
  targetPort,
  setDeviceStatus,
  sendDirectRawCommand,
  onLog,
}: TransportActionContext): Promise<void> {
  try {
    await invokeCommand('set_pico_telemetry_stream', { active: false }).catch(() => {});
    if (targetPort.startsWith('ANDROID-USB-OTG')) {
      await sendDirectRawCommand('ISOLATE').catch(() => {});
      disconnectAndroidUsbOtg();
    } else if (targetPort.startsWith('BLE_') && bleService.isConnected()) {
      bleService.disconnect();
    } else {
      await invokeCommand<string>('pico_send_serial_command', {
        portName: targetPort,
        port_name: targetPort,
        command: 'ISOLATE',
      }).catch(() => {});
    }
  } finally {
    setDeviceStatus(prev => ({
      ...(prev || DEFAULT_DISCONNECTED_STATUS),
      connected: false,
      core_state: 'DISCONNECTED',
      vdd_volts: 0.0,
      target_powered: false,
      port: targetPort,
    }));
    onLog?.(
      'warn',
      'HOST',
      `Desconectado da interface P7501 UART / OCD (${targetPort}). Pinos colocados em Hi-Z.`
    );
  }
}

export async function establishTransportConnection({
  targetPort,
  setDeviceStatus,
  onLog,
}: Omit<TransportActionContext, 'sendDirectRawCommand'>): Promise<void> {
  if (targetPort.startsWith('ANDROID-USB-OTG')) {
    const devMatch = targetPort.match(/\[([^\s\]]+)/);
    const otgRes = connectAndroidUsbOtg(devMatch?.[1] || '');
    if (!otgRes.connected) {
      throw new Error(otgRes.message || 'Dispositivo USB-OTG não conectado.');
    }
    const vdd = parseVddFromStatusRaw(sendAndroidUsbOtgCommand('STATUS', 2000).output);
    setDeviceStatus(prev => buildMobileDeviceStatus(prev, targetPort, vdd, 'RP2040/RP2350 OTG'));
    onLog?.('success', 'HOST', `${otgRes.message} | VDD: ${vdd.toFixed(2)}V`);
    return;
  }

  if (targetPort.startsWith('BLE_')) {
    const bleDev = await bleService.connect();
    let measuredVdd = 3.3;
    try {
      const statusOut = await bleService.sendAndAwaitReply('STATUS', 2500);
      measuredVdd = parseVddFromStatusRaw(statusOut, 3.3);
    } catch {
      // Keep default BLE handshake if status reply times out on first packet
    }
    onLog?.('success', 'HOST', `Conectado via Bluetooth LE NUS: ${bleDev.name} (VDD: ${measuredVdd.toFixed(2)}V)`);
    setDeviceStatus(prev => buildMobileDeviceStatus(prev, targetPort, measuredVdd, 'Pico W (BLE)'));
    return;
  }

  const result = await invokeCommand<{
    connected: boolean;
    port_name: string;
    baud_rate: number;
    vdd_voltage: number;
    target_powered: boolean;
    firmware_banner: string;
    message: string;
  }>('pico_connect_and_test', {
    portName: targetPort,
    port_name: targetPort,
    baudRate: 115200,
    baud_rate: 115200,
  });

  if (!result || !result.connected) {
    throw new Error(result?.message || 'Dispositivo não respondeu ao handshake inicial.');
  }

  const isRp2350 = result.port_name.includes('2350') || result.firmware_banner.includes('2350');
  setDeviceStatus(prev => ({
    ...(prev || DEFAULT_DISCONNECTED_STATUS),
    connected: true,
    port: targetPort,
    baudRate: result.baud_rate,
    vdd_volts: result.vdd_voltage,
    target_powered: result.target_powered,
    core_state: result.target_powered ? 'HALTED' : 'RUNNING',
    firmware_version: result.firmware_banner || 'P7501 UART / Panadiga Master',
    chip_id: isRp2350 ? 'RP2350-A2' : 'RP2040-B2',
    last_keepalive: 'OK',
  }));

  onLog?.('success', 'PICO', result.firmware_banner);
  onLog?.(
    'success',
    'HOST',
    result.message || `Conexão estabelecida com sucesso em ${targetPort}! VDD: ${result.vdd_voltage.toFixed(2)}V`
  );

  await invokeCommand('set_pico_telemetry_stream', {
    active: true,
    portName: targetPort,
    port_name: targetPort,
  }).catch(() => {});
}
