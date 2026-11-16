/**
 * @file components/pico/PicoBenchTopBarHeader.tsx
 * @brief Top bar header with transport indicator, BOOTSEL UF2 flasher controls, and Low-VDD bench alert.
 */

import React from 'react';
import type { PicoDeviceStatus, PicoUf2Variant } from '@/models';
import { StatusBadge } from '@/components/StatusBadge';
import { Alert, AlertDescription, AlertTitle, Button } from '@/components/ui';
import {
  AlertTriangle,
  Bluetooth,
  DownloadCloud,
  Radio,
  RefreshCw,
  Usb,
} from 'lucide-react';

interface PicoBenchTopBarHeaderProps {
  deviceStatus: PicoDeviceStatus | null;
  isBle: boolean;
  isOtg: boolean;
  isPicoPort: boolean;
  isFlashing: boolean;
  uf2Variant: PicoUf2Variant;
  onChangeUf2Variant: (variant: PicoUf2Variant) => void;
  onRefreshStatus?: () => void;
  onFlashFirmware?: (variant: PicoUf2Variant) => Promise<void> | void;
  onBootselReboot?: () => Promise<void> | void;
}

export const PicoBenchTopBarHeader: React.FC<PicoBenchTopBarHeaderProps> = ({
  deviceStatus,
  isBle,
  isOtg,
  isPicoPort,
  isFlashing,
  uf2Variant,
  onChangeUf2Variant,
  onRefreshStatus,
  onFlashFirmware,
  onBootselReboot,
}) => (
  <div className="flex flex-col xl:flex-row items-start xl:items-center justify-between gap-3 border-b border-slate-800 pb-4">
    <div className="flex items-center gap-3.5 min-w-0">
      <div
        className={`p-3 rounded-xl border shrink-0 ${
          isBle
            ? 'bg-blue-500/10 text-blue-400 border-blue-500/30'
            : isOtg
              ? 'bg-cyan-500/10 text-cyan-400 border-cyan-500/30'
              : 'bg-emerald-500/10 text-emerald-400 border-emerald-500/30'
        }`}
      >
        {isBle ? (
          <Bluetooth className="w-6 h-6" />
        ) : isOtg ? (
          <Usb className="w-6 h-6" />
        ) : (
          <Radio className="w-6 h-6" />
        )}
      </div>
      <div className="min-w-0">
        <div className="flex items-center gap-2 flex-wrap">
          <h2 className="text-sm sm:text-base font-bold text-slate-100">
            Raspberry Pi Pico (RP2040 / RP2350) — P7501 UART (115200 8N1)
          </h2>
          {deviceStatus?.connected ? (
            isBle ? (
              <StatusBadge variant="info" label="BLE CONECTADO" />
            ) : isOtg ? (
              <StatusBadge variant="info" label="USB-OTG CONECTADO" />
            ) : (
              <StatusBadge variant="success" label="CONECTADO" />
            )
          ) : (
            <StatusBadge variant="error" label="DESCONECTADO" />
          )}
          {deviceStatus?.connected && onRefreshStatus && (
            <Button
              type="button"
              variant="outline"
              size="icon"
              onClick={onRefreshStatus}
              title="Atualizar telemetria e verificar status do dispositivo"
              className="h-7 w-7"
            >
              <RefreshCw className="w-3 h-3" />
            </Button>
          )}
        </div>
        <p className="text-xs text-slate-400 mt-0.5">
          Interface Primária P7501 UART 115200 8N1 &amp; Controle Discreto SW_UART / XRESET / NSW5V
        </p>
      </div>
    </div>

    <div className="flex items-center gap-2 shrink-0 flex-wrap">
      <select
        aria-label="Variante UF2"
        value={uf2Variant}
        onChange={e => onChangeUf2Variant(e.target.value as PicoUf2Variant)}
        disabled={isFlashing || isOtg || isBle}
        className="h-8 px-2.5 rounded-lg bg-slate-950 border border-slate-800 text-[11px] text-purple-300 font-mono focus:outline-none focus:border-purple-500"
      >
        <option value="unified">UF2 Unificado (RP2040+RP2350)</option>
        <option value="rp2040">UF2 RP2040 (Pico 1)</option>
        <option value="rp2350">UF2 RP2350 (Pico 2)</option>
      </select>

      <Button
        type="button"
        variant="outline"
        size="sm"
        onClick={() => void onFlashFirmware?.(uf2Variant)}
        disabled={isFlashing || isOtg}
        title="Grava o firmware P7501 Async UART (RP2040 / RP2350) no drive BOOTSEL (RPI-RP2 / RP2350)"
        className="bg-purple-500/10 hover:bg-purple-500/20 text-purple-300 border-purple-500/30 font-bold gap-1.5"
      >
        {isFlashing ? (
          <>
            <RefreshCw className="w-3.5 h-3.5 animate-spin" />
            <span>Gravando UF2...</span>
          </>
        ) : (
          <>
            <DownloadCloud className="w-3.5 h-3.5" />
            <span>Flash Pico (BOOTSEL)</span>
          </>
        )}
      </Button>

      <Button
        type="button"
        variant="outline"
        size="sm"
        onClick={() => void onBootselReboot?.()}
        disabled={isBle || isOtg || !isPicoPort}
        title="Reinicia o Pico em modo BOOTSEL via 1200-baud touch / comando CDC"
        className="bg-indigo-500/10 hover:bg-indigo-500/20 text-indigo-300 border-indigo-500/30 font-bold gap-1.5"
      >
        <RefreshCw className="w-3.5 h-3.5" />
        <span>BOOTSEL Reset</span>
      </Button>
    </div>
  </div>
);

interface PicoLowVddAlertProps {
  effectiveVddVolts: number;
  onQuickPincheck?: () => void;
}

export const PicoLowVddAlert: React.FC<PicoLowVddAlertProps> = ({
  effectiveVddVolts,
  onQuickPincheck,
}) => (
  <Alert variant="warning" className="items-center justify-between flex-wrap">
    <div className="flex items-start gap-2.5">
      <AlertTriangle className="w-4 h-4 text-amber-400 shrink-0 mt-0.5" />
      <div>
        <AlertTitle>
          Sonda Pico Operacional, mas Trilho Alvo em {effectiveVddVolts.toFixed(2)}V (&lt; 2.50V)
        </AlertTitle>
        <AlertDescription>
          Verifique se o Panasonic DMR-EH55 está energizado na bancada e se os pinos P7501 (Pino 4 GND, Pino 1 SW_UART / ADC0) estão conectados corretamente.
        </AlertDescription>
      </div>
    </div>
    {onQuickPincheck && (
      <Button
        type="button"
        variant="outline"
        size="sm"
        onClick={onQuickPincheck}
        className="border-amber-500/40 bg-amber-500/15 text-amber-200 hover:bg-amber-500/25 shrink-0"
      >
        Executar PINCHECK
      </Button>
    )}
  </Alert>
);
