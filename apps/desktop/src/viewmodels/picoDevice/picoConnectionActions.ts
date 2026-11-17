/**
 * @file viewmodels/picoDevice/picoConnectionActions.ts
 * @brief Stateless transport command, connect & disconnect routines for USB CDC, Android USB-OTG, and BLE NUS.
 */

import type { Dispatch, SetStateAction } from 'react';
import {
  PICO_BLE_COMMAND_TIMEOUT_MS,
  PICO_BLE_STATUS_TIMEOUT_MS,
  PICO_DEFAULT_BAUD_RATE,
  PICO_NOMINAL_VDD_VOLTS,
  PICO_OTG_SHORT_TIMEOUT_MS,
  PICO_STATUS_POLL_TIMEOUT_MS,
} from '@/constants';
import { translate } from '@/i18n/useTranslation';
import type { PicoDeviceStatus } from '@/models';
import {
  connectAndroidUsbOtg,
  disconnectAndroidUsbOtg,
  sendAndroidUsbOtgCommand,
} from '@/services/androidBridge';
import { bleService } from '@/services/bleService';
import {
  connectAndTestPicoPort,
  sendPicoSerialCommand,
  setPicoTelemetryStream,
} from '@/services/picoService';
import {
  buildMobileDeviceStatus,
  DEFAULT_DISCONNECTED_STATUS,
  parseVddFromStatusRaw,
  type UsePicoDeviceOptions,
} from './stateHelpers';

export async function sendDirectRawCommandForPort(
  port: string,
  command: string,
): Promise<string> {
  if (port.startsWith('ANDROID-USB-OTG')) {
    const res = sendAndroidUsbOtgCommand(command, PICO_OTG_SHORT_TIMEOUT_MS);
    if (!res.ok) throw new Error(res.output || translate('errors.pico.usbOtgNotConnected'));
    return res.output;
  }
  if (port.startsWith('BLE_') && bleService.isConnected()) {
    return bleService.sendAndAwaitReply(command, PICO_BLE_COMMAND_TIMEOUT_MS);
  }
  return sendPicoSerialCommand(port, command);
}

async function measureBleHandshakeVdd(): Promise<number> {
  try {
    const statusOut = await bleService.sendAndAwaitReply('STATUS', PICO_BLE_STATUS_TIMEOUT_MS);
    return parseVddFromStatusRaw(statusOut, PICO_NOMINAL_VDD_VOLTS);
  } catch {
    return PICO_NOMINAL_VDD_VOLTS;
  }
}

export interface TransportActionContext {
  targetPort: string;
  setDeviceStatus: Dispatch<SetStateAction<PicoDeviceStatus | null>>;
  onLog?: UsePicoDeviceOptions['onLog'];
}

export async function disconnectTransportPort({
  targetPort,
  setDeviceStatus,
  onLog,
}: TransportActionContext): Promise<void> {
  try {
    await setPicoTelemetryStream(false).catch(() => {});
    if (targetPort.startsWith('ANDROID-USB-OTG')) {
      await sendDirectRawCommandForPort(targetPort, 'ISOLATE').catch(() => {});
      disconnectAndroidUsbOtg();
    } else if (targetPort.startsWith('BLE_') && bleService.isConnected()) {
      bleService.disconnect();
    } else {
      await sendPicoSerialCommand(targetPort, 'ISOLATE').catch(() => {});
    }
  } finally {
    setDeviceStatus((prev) => ({
      ...(prev || DEFAULT_DISCONNECTED_STATUS),
      connected: false,
      core_state: 'DISCONNECTED',
      vdd_volts: 0.0,
      target_powered: false,
      port: targetPort,
    }));
    onLog?.('warn', 'HOST', translate('errors.pico.disconnectedPort', { port: targetPort }));
  }
}

export async function establishTransportConnection({
  targetPort,
  setDeviceStatus,
  onLog,
}: TransportActionContext): Promise<void> {
  if (targetPort.startsWith('ANDROID-USB-OTG')) {
    const devMatch = targetPort.match(/\[([^\s\]]+)/);
    const otgRes = connectAndroidUsbOtg(devMatch?.[1] || '');
    if (!otgRes.connected) {
      throw new Error(otgRes.message || translate('errors.pico.usbOtgNotConnected'));
    }
    const vdd = parseVddFromStatusRaw(
      sendAndroidUsbOtgCommand('STATUS', PICO_STATUS_POLL_TIMEOUT_MS).output,
    );
    setDeviceStatus((prev) => buildMobileDeviceStatus(prev, targetPort, vdd, 'RP2040/RP2350 OTG'));
    onLog?.('success', 'HOST', `${otgRes.message} | VDD: ${vdd.toFixed(2)}V`);
    return;
  }

  if (targetPort.startsWith('BLE_')) {
    const bleDev = await bleService.connect();
    const measuredVdd = await measureBleHandshakeVdd();
    onLog?.(
      'success',
      'HOST',
      translate('errors.pico.bleConnectedLog', {
        name: bleDev.name,
        vdd: measuredVdd.toFixed(2),
      }),
    );
    setDeviceStatus((prev) => buildMobileDeviceStatus(prev, targetPort, measuredVdd, 'Pico W (BLE)'));
    return;
  }

  const result = await connectAndTestPicoPort(targetPort, PICO_DEFAULT_BAUD_RATE);
  if (!result || !result.connected) {
    throw new Error(result?.message || translate('errors.pico.handshakeFailed'));
  }

  const isRp2350 = result.port_name.includes('2350') || result.firmware_banner.includes('2350');
  setDeviceStatus((prev) => ({
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
    result.message ||
      translate('errors.pico.connectedSuccessLog', {
        port: targetPort,
        vdd: result.vdd_voltage.toFixed(2),
      }),
  );

  await setPicoTelemetryStream(true, targetPort).catch(() => {});
}
