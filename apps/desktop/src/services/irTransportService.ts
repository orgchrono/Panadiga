import { SAFE_PICO_IR_GPIOS } from '@/constants';
import { translate } from '@/i18n/useTranslation';
import { invokeCommand } from './ipc';
import {
  connectAndroidBleNus,
  connectAndroidUsbOtg,
  isAndroidNativeBridgeAvailable,
  sendAndroidBleCommand,
  sendAndroidUsbOtgCommand,
} from './androidBridge';
import {
  getAndroidIrCapabilities,
  isAndroidIrBridgeAvailable,
  transmitViaAndroidIrHal,
} from './androidIrBridge';
import { irPrepareTransmission } from './irIpc';
import type {
  AndroidIrCapabilities,
  IrDeviceInfo,
  IrPreparedSignal,
  IrTimingOverride,
  IrTransportKind,
  KeyProtocol,
  RemoteUnitSet,
} from '@/models/ir';

export { SAFE_PICO_IR_GPIOS };

const DEFAULT_GPIO_PIN = 6;
const MIN_IR_REPEAT = 1;
const MAX_IR_REPEAT = 8;
const BYTE_MASK = 0xff;
const NIBBLE_MASK = 0x0f;
const BYTE_SHIFT_BITS = 8;
const HEX_RADIX = 16;
const DEV_SUB_HEX_WIDTH = 4;
const CMD_HEX_WIDTH = 2;
const DIGA_KASEIKYO_DEVICE_ID = 0xb0;
const ANDROID_FALLBACK_SDK_INT = 34;
const USB_IRTX_TIMEOUT_MS = 2500;
const BLE_IRTX_CONNECT_TIMEOUT_MS = 6000;
const BLE_IRTX_CMD_TIMEOUT_MS = 3000;

const DEFAULT_FALLBACK_DEVICE: IrDeviceInfo = {
  manufacturer: 'Generic',
  model: 'Android-CIR',
  hardware: 'generic',
  board: 'generic',
  product: 'generic',
  device: 'generic',
  android_release: '14',
  sdk_int: ANDROID_FALLBACK_SDK_INT,
};

export interface IrDispatchOutcome {
  ok: boolean;
  backendUsed: string;
  prepared: IrPreparedSignal;
  error?: string;
}

export function formatPicoIrtxCommand(
  device: number,
  subdevice: number,
  obc: number,
  gpioPin: number,
  repeatCount: number,
): string {
  const safeGpio = SAFE_PICO_IR_GPIOS.includes(gpioPin) ? gpioPin : DEFAULT_GPIO_PIN;
  const clampedRepeat = Math.min(MAX_IR_REPEAT, Math.max(MIN_IR_REPEAT, Math.round(repeatCount)));
  const devSub = ((device & BYTE_MASK) << BYTE_SHIFT_BITS) | (subdevice & NIBBLE_MASK);
  const lhsHex = devSub.toString(HEX_RADIX).toUpperCase().padStart(DEV_SUB_HEX_WIDTH, '0');
  const cmdHex = (obc & BYTE_MASK).toString(HEX_RADIX).toUpperCase().padStart(CMD_HEX_WIDTH, '0');
  return `IRTX ${lhsHex}:${cmdHex} GP${safeGpio} ${clampedRepeat}`;
}

export function readAndroidIrStatus(): AndroidIrCapabilities | null {
  return getAndroidIrCapabilities();
}

export async function dispatchIrSignal(params: {
  protocol: KeyProtocol;
  device: number;
  subdevice: number;
  obc: number;
  respectRemoteMode: boolean;
  remoteUnit: RemoteUnitSet;
  prontoHex?: string | null;
  transport: IrTransportKind;
  picoGpioPin: number;
  unitOverride: IrTimingOverride;
  repeatCount: number;
}): Promise<IrDispatchOutcome> {
  const caps = getAndroidIrCapabilities();
  const prepared = await irPrepareTransmission({
    protocol: params.protocol,
    device: params.device,
    subdevice: params.subdevice,
    obc: params.obc,
    respectRemoteMode: params.respectRemoteMode,
    remoteUnit: params.remoteUnit,
    prontoHex: params.prontoHex,
    ranges: caps?.ranges ?? [],
    deviceInfo: caps?.device ?? DEFAULT_FALLBACK_DEVICE,
    hasSecIrNode: caps?.hasSecIrNode ?? false,
    hasFpgaNode: caps?.hasFpgaNode ?? false,
    unitOverride: params.unitOverride,
  });

  const resolvedTransport = resolveTransport(params.transport, caps);
  if (resolvedTransport === 'ANDROID_HAL') {
    const halRes = transmitViaAndroidIrHal(
      prepared.carrier_hz,
      prepared.encoded_pattern,
      prepared.sysfs_payload,
      params.repeatCount,
    );
    return {
      ok: halRes.ok,
      backendUsed: halRes.backendUsed ?? 'Android ConsumerIR / Exynos HAL',
      prepared,
      error: halRes.error,
    };
  }

  return dispatchViaPicoTransport({
    transport: resolvedTransport,
    device: params.device,
    subdevice: params.subdevice,
    obc: params.obc,
    respectRemoteMode: params.respectRemoteMode,
    remoteUnit: params.remoteUnit,
    gpioPin: params.picoGpioPin,
    repeatCount: params.repeatCount,
    prepared,
  });
}

function resolveTransport(
  requested: IrTransportKind,
  caps: AndroidIrCapabilities | null,
): Exclude<IrTransportKind, 'AUTO'> {
  if (requested !== 'AUTO') {
    return requested;
  }
  if (
    isAndroidIrBridgeAvailable() &&
    (caps?.hasEmitter || caps?.hasSecIrNode || caps?.hasFpgaNode || caps?.suAvailable)
  ) {
    return 'ANDROID_HAL';
  }
  if (isAndroidNativeBridgeAvailable()) {
    return 'PICO_USB_OTG';
  }
  return 'PICO_TAURI';
}

async function dispatchViaPicoTransport(params: {
  transport: 'PICO_USB_OTG' | 'PICO_BLE' | 'PICO_TAURI';
  device: number;
  subdevice: number;
  obc: number;
  respectRemoteMode: boolean;
  remoteUnit: RemoteUnitSet;
  gpioPin: number;
  repeatCount: number;
  prepared: IrPreparedSignal;
}): Promise<IrDispatchOutcome> {
  const effectiveSub =
    params.respectRemoteMode && params.device === DIGA_KASEIKYO_DEVICE_ID && params.remoteUnit > 1
      ? ((params.remoteUnit - 1) & NIBBLE_MASK) | (params.subdevice & NIBBLE_MASK)
      : params.subdevice;

  if ((params.device & BYTE_MASK) === 0) {
    return {
      ok: false,
      backendUsed: `Pico IRTX (${params.transport})`,
      prepared: params.prepared,
      error: translate('errors.androidIr.deviceIdZeroUnsupported'),
    };
  }

  const cmd = formatPicoIrtxCommand(
    params.device,
    effectiveSub,
    params.obc,
    params.gpioPin,
    params.repeatCount,
  );

  if (params.transport === 'PICO_USB_OTG') {
    connectAndroidUsbOtg('');
    const res = sendAndroidUsbOtgCommand(cmd, USB_IRTX_TIMEOUT_MS);
    return {
      ok: res.ok && !res.output.includes('IRTX: ERROR'),
      backendUsed: `Pico USB-OTG (GP${params.gpioPin})`,
      prepared: params.prepared,
      error: res.ok ? undefined : res.output,
    };
  }

  if (params.transport === 'PICO_BLE') {
    connectAndroidBleNus(BLE_IRTX_CONNECT_TIMEOUT_MS);
    const res = sendAndroidBleCommand(cmd, BLE_IRTX_CMD_TIMEOUT_MS);
    return {
      ok: res.ok && !res.output.includes('IRTX: ERROR'),
      backendUsed: `Pico BLE NUS (GP${params.gpioPin})`,
      prepared: params.prepared,
      error: res.ok ? undefined : res.output,
    };
  }

  try {
    const out = await invokeCommand<string>('pico_send_serial_command', { command: cmd });
    return {
      ok: !out.includes('IRTX: ERROR'),
      backendUsed: `Pico IPC (GP${params.gpioPin})`,
      prepared: params.prepared,
      error: out.includes('IRTX: ERROR') ? out : undefined,
    };
  } catch (err) {
    return {
      ok: false,
      backendUsed: `Pico IPC (GP${params.gpioPin})`,
      prepared: params.prepared,
      error: err instanceof Error ? err.message : String(err),
    };
  }
}
