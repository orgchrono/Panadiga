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

export const SAFE_PICO_IR_GPIOS: readonly number[] = [6, 7, 8, 9, 16, 17, 18, 19, 20, 21, 22];

const DEFAULT_FALLBACK_DEVICE: IrDeviceInfo = {
  manufacturer: 'Generic',
  model: 'Android-CIR',
  hardware: 'generic',
  board: 'generic',
  product: 'generic',
  device: 'generic',
  android_release: '14',
  sdk_int: 34,
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
  const safeGpio = SAFE_PICO_IR_GPIOS.includes(gpioPin) ? gpioPin : 6;
  const clampedRepeat = Math.min(8, Math.max(1, Math.round(repeatCount)));
  const devSub = ((device & 0xff) << 8) | (subdevice & 0x0f);
  const lhsHex = devSub.toString(16).toUpperCase().padStart(4, '0');
  const cmdHex = (obc & 0xff).toString(16).toUpperCase().padStart(2, '0');
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
    params.respectRemoteMode && params.device === 0xb0 && params.remoteUnit > 1
      ? ((params.remoteUnit - 1) & 0x0f) | (params.subdevice & 0x0f)
      : params.subdevice;

  if ((params.device & 0xff) === 0) {
    return {
      ok: false,
      backendUsed: `Pico IRTX (${params.transport})`,
      prepared: params.prepared,
      error:
        'Pico IRTX hardware parser supports Panasonic device >= 0x01 (e.g. DIGA 0xB0 / VCR 0x90 / TV 0x80). Use Android CIR HAL for device 0x00 codes.',
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
    const res = sendAndroidUsbOtgCommand(cmd, 2500);
    return {
      ok: res.ok && !res.output.includes('IRTX: ERROR'),
      backendUsed: `Pico USB-OTG (GP${params.gpioPin})`,
      prepared: params.prepared,
      error: res.ok ? undefined : res.output,
    };
  }

  if (params.transport === 'PICO_BLE') {
    connectAndroidBleNus(6000);
    const res = sendAndroidBleCommand(cmd, 3000);
    return {
      ok: res.ok && !res.output.includes('IRTX: ERROR'),
      backendUsed: `Pico BLE NUS (GP${params.gpioPin})`,
      prepared: params.prepared,
      error: res.ok ? undefined : res.output,
    };
  }

  try {
    const out = await invokeCommand<string>('send_pico_command', { command: cmd });
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
