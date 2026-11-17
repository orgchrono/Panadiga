import React from 'react';
import { Activity, Camera, Cpu, ShieldCheck } from 'lucide-react';
import {
  Alert,
  AlertDescription,
  AlertTitle,
  Badge,
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Select,
} from '@/components/ui';
import { DEFAULT_IR_PICO_GPIO } from '@/constants';
import { useTranslation } from '@/i18n/useTranslation';
import type {
  AndroidIrCapabilities,
  IrTimingOverride,
  IrTransportKind,
} from '@/models/ir';
import { oneOfGuard } from '@/utils/typeGuards';

interface IrDiagnosticsTabProps {
  capabilities: AndroidIrCapabilities | null;
  transport: IrTransportKind;
  picoGpioPin: number;
  safePicoGpios: readonly number[];
  timingUnitOverride: IrTimingOverride;
  repeatCount: number;
  lastProntoHex: string;
  isBeaconRunning: boolean;
  onChangeTransport: (t: IrTransportKind) => void;
  onChangePicoGpio: (gpio: number) => void;
  onChangeTimingUnit: (unit: IrTimingOverride) => void;
  onChangeRepeatCount: (repeat: number) => void;
  onToggleCameraBeacon: () => void;
  onRefreshCapabilities: () => void;
}

const TIMING_OPTIONS: readonly IrTimingOverride[] = [
  'AUTO',
  'MICROSECONDS',
  'CARRIER_CYCLES',
  'PERIOD_SCALED',
];

const TRANSPORT_OPTIONS: readonly IrTransportKind[] = [
  'AUTO',
  'ANDROID_HAL',
  'PICO_USB_OTG',
  'PICO_BLE',
  'PICO_TAURI',
];

const isIrTransport = oneOfGuard(TRANSPORT_OPTIONS);
const isIrTimingOverride = oneOfGuard(TIMING_OPTIONS);

export const IrDiagnosticsTab: React.FC<IrDiagnosticsTabProps> = ({
  capabilities,
  transport,
  picoGpioPin,
  safePicoGpios,
  timingUnitOverride,
  repeatCount,
  lastProntoHex,
  isBeaconRunning,
  onChangeTransport,
  onChangePicoGpio,
  onChangeTimingUnit,
  onChangeRepeatCount,
  onToggleCameraBeacon,
  onRefreshCapabilities,
}) => {
  const { t } = useTranslation();

  const carrierSpansLabel =
    !capabilities || capabilities.ranges.length === 0
      ? t('irCommander.diag.defaultCarrierSpans')
      : capabilities.ranges.map((r) => `${r.min_hz}-${r.max_hz}Hz`).join(', ');

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader className="pb-2">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <CardTitle className="flex items-center gap-2 text-sm">
              <Cpu className="w-4 h-4 text-cyan-400" />
              <span>{t('irCommander.diag.halTitle')}</span>
            </CardTitle>
            <Button type="button" size="sm" variant="outline" onClick={onRefreshCapabilities}>
              {t('irCommander.diag.refreshBtn')}
            </Button>
          </div>
        </CardHeader>
        <CardContent className="space-y-3 text-xs font-mono">
          <div className="flex flex-wrap gap-1.5">
            <Badge variant="outline">
              {t('irCommander.diag.cirStatus', {
                status: capabilities?.hasEmitter
                  ? t('irCommander.diag.statusReady')
                  : t('irCommander.diag.statusNone'),
              })}
            </Badge>
            <Badge variant="outline">
              {t('irCommander.diag.secIrStatus', {
                status: capabilities?.hasSecIrNode
                  ? t('irCommander.diag.statusPresent')
                  : t('irCommander.diag.statusAbsent'),
              })}
            </Badge>
            <Badge variant="outline">
              {t('irCommander.diag.fpgaStatus', {
                status: capabilities?.hasFpgaNode
                  ? t('irCommander.diag.statusPresent')
                  : t('irCommander.diag.statusAbsent'),
              })}
            </Badge>
            <Badge variant="outline">
              {t('irCommander.diag.magiskSuStatus', {
                status: capabilities?.suAvailable
                  ? t('irCommander.diag.statusAvailable')
                  : t('irCommander.diag.statusUnavailable'),
              })}
            </Badge>
          </div>

          {capabilities && (
            <div className="rounded-lg bg-bench-950 border border-slate-800 p-3 text-[11px] text-slate-300 space-y-1">
              <div>
                {t('irCommander.diag.deviceModelLine', {
                  manufacturer: capabilities.device.manufacturer,
                  model: capabilities.device.model,
                  sdk: capabilities.device.sdk_int,
                  release: capabilities.device.android_release,
                })}
              </div>
              <div>
                {t('irCommander.diag.socBoardLine', {
                  hardware: capabilities.device.hardware,
                  board: capabilities.device.board,
                  product: capabilities.device.product,
                  device: capabilities.device.device,
                })}
              </div>
              <div>
                {t('irCommander.diag.carrierSpansLine', { spans: carrierSpansLabel })}
              </div>
            </div>
          )}

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <div>
              <label htmlFor="ir-transport-select" className="text-slate-400 block mb-1">
                {t('irCommander.diag.transportLabel')}
              </label>
              <Select
                id="ir-transport-select"
                value={transport}
                onChange={(e) => {
                  if (isIrTransport(e.target.value)) onChangeTransport(e.target.value);
                }}
                data-testid="ir-transport-select"
              >
                {TRANSPORT_OPTIONS.map((opt) => (
                  <option key={opt} value={opt}>
                    {opt}
                  </option>
                ))}
              </Select>
            </div>

            <div>
              <label htmlFor="ir-gpio-select" className="text-slate-400 block mb-1">
                {t('irCommander.diag.picoGpioLabel')}
              </label>
              <Select
                id="ir-gpio-select"
                value={String(picoGpioPin)}
                onChange={(e) => onChangePicoGpio(Number.parseInt(e.target.value, 10))}
                data-testid="ir-gpio-select"
              >
                {safePicoGpios.map((pin) => (
                  <option key={pin} value={String(pin)}>
                    {`GP${pin} (PWM 38 kHz — ${pin === DEFAULT_IR_PICO_GPIO ? t('irCommander.diag.gpioDefault') : t('irCommander.diag.gpioSpareSafe')})`}
                  </option>
                ))}
              </Select>
            </div>

            <div>
              <label htmlFor="ir-timing-unit-select" className="text-slate-400 block mb-1">
                {t('irCommander.diag.timingUnitLabel')}
              </label>
              <Select
                id="ir-timing-unit-select"
                value={timingUnitOverride}
                onChange={(e) => {
                  if (isIrTimingOverride(e.target.value)) onChangeTimingUnit(e.target.value);
                }}
                data-testid="ir-timing-unit-select"
              >
                {TIMING_OPTIONS.map((u) => (
                  <option key={u} value={u}>
                    {u}
                  </option>
                ))}
              </Select>
            </div>

            <div>
              <label htmlFor="ir-repeat-select" className="text-slate-400 block mb-1">
                {t('irCommander.diag.repeatLabel')}
              </label>
              <Select
                id="ir-repeat-select"
                value={String(repeatCount)}
                onChange={(e) => onChangeRepeatCount(Number.parseInt(e.target.value, 10))}
                data-testid="ir-repeat-select"
              >
                <option value="1">{t('irCommander.diag.repeat1x')}</option>
                <option value="2">{t('irCommander.diag.repeat2x')}</option>
                <option value="3">{t('irCommander.diag.repeat3x')}</option>
              </Select>
            </div>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="flex items-center gap-2 text-sm">
            <Activity className="w-4 h-4 text-emerald-400" />
            <span>{t('irCommander.diag.beaconTitle')}</span>
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-3 text-xs font-mono">
          <Button
            type="button"
            variant={isBeaconRunning ? 'destructive' : 'emerald'}
            onClick={onToggleCameraBeacon}
            data-testid="ir-beacon-btn"
            className="gap-2"
          >
            <Camera className="w-4 h-4" />
            {isBeaconRunning
              ? t('irCommander.diag.beaconStopBtn')
              : t('irCommander.diag.beaconStartBtn')}
          </Button>

          {lastProntoHex && (
            <div className="rounded-lg bg-bench-950 border border-slate-800 p-2.5 text-[10px] text-slate-300 break-all">
              <div className="text-slate-400 mb-1">{t('irCommander.diag.prontoHexHeader')}</div>
              {lastProntoHex}
            </div>
          )}

          <Alert variant="info">
            <AlertTitle className="flex items-center gap-1.5">
              <ShieldCheck className="w-4 h-4 text-cyan-400" />
              <span>{t('irCommander.diag.safetyTitle')}</span>
            </AlertTitle>
            <AlertDescription className="text-xs">
              {t('irCommander.diag.safetyBody')}
            </AlertDescription>
          </Alert>
        </CardContent>
      </Card>
    </div>
  );
};
