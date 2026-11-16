/**
 * @file components/pico/PicoBenchTopBarHeader.tsx
 * @brief Top bar header with transport indicator, BOOTSEL UF2 flasher controls, and Low-VDD bench alert.
 */

import React from 'react';
import type { PicoDeviceStatus, PicoUf2Variant } from '@/models';
import { StatusBadge } from '@/components/StatusBadge';
import { Alert, AlertDescription, AlertTitle, Button } from '@/components/ui';
import { isPicoUf2Variant } from '@/utils/typeGuards';
import { useTranslation } from '@/i18n/useTranslation';
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
}) => {
  const { t } = useTranslation();

  return (
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
              {t('pico.title')}
            </h2>
            {deviceStatus?.connected ? (
              isBle ? (
                <StatusBadge variant="info" label={t('pico.topHeader.bleConnected')} />
              ) : isOtg ? (
                <StatusBadge variant="info" label={t('pico.topHeader.otgConnected')} />
              ) : (
                <StatusBadge variant="success" label={t('pico.connected')} />
              )
            ) : (
              <StatusBadge variant="error" label={t('pico.disconnected')} />
            )}
            {deviceStatus?.connected && onRefreshStatus && (
              <Button
                type="button"
                variant="outline"
                size="icon"
                onClick={onRefreshStatus}
                title={t('pico.topHeader.refreshTooltip')}
                className="h-7 w-7"
              >
                <RefreshCw className="w-3 h-3" />
              </Button>
            )}
          </div>
          <p data-testid="pico-primary-interface-subtitle" className="text-xs text-slate-400 mt-0.5">
            {t('pico.topHeader.subtitle')}
          </p>
        </div>
      </div>

      <div className="flex items-center gap-2 shrink-0 flex-wrap">
        <select
          aria-label={t('pico.topHeader.uf2VariantAria')}
          data-testid="pico-uf2-variant-select"
          value={uf2Variant}
          onChange={e => {
            if (isPicoUf2Variant(e.target.value)) {
              onChangeUf2Variant(e.target.value);
            }
          }}
          disabled={isFlashing || isOtg || isBle}
          className="h-8 px-2.5 rounded-lg bg-slate-950 border border-slate-800 text-[11px] text-purple-300 font-mono focus:outline-none focus:border-purple-500"
        >
          <option value="unified">{t('pico.topHeader.uf2Unified')}</option>
          <option value="rp2040">{t('pico.topHeader.uf2Rp2040')}</option>
          <option value="rp2350">{t('pico.topHeader.uf2Rp2350')}</option>
        </select>

        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={() => void onFlashFirmware?.(uf2Variant)}
          disabled={isFlashing || isOtg}
          title={t('pico.topHeader.flashTooltip')}
          className="bg-purple-500/10 hover:bg-purple-500/20 text-purple-300 border-purple-500/30 font-bold gap-1.5"
        >
          {isFlashing ? (
            <>
              <RefreshCw className="w-3.5 h-3.5 animate-spin" />
              <span>{t('pico.topHeader.flashingUf2')}</span>
            </>
          ) : (
            <>
              <DownloadCloud className="w-3.5 h-3.5" />
              <span>{t('pico.topHeader.flashBtn')}</span>
            </>
          )}
        </Button>

        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={() => void onBootselReboot?.()}
          disabled={isBle || isOtg || !isPicoPort}
          title={t('pico.topHeader.bootselTooltip')}
          className="bg-indigo-500/10 hover:bg-indigo-500/20 text-indigo-300 border-indigo-500/30 font-bold gap-1.5"
        >
          <RefreshCw className="w-3.5 h-3.5" />
          <span>{t('pico.topHeader.bootselBtn')}</span>
        </Button>
      </div>
    </div>
  );
};

interface PicoLowVddAlertProps {
  effectiveVddVolts: number;
  onQuickPincheck?: () => void;
}

export const PicoLowVddAlert: React.FC<PicoLowVddAlertProps> = ({
  effectiveVddVolts,
  onQuickPincheck,
}) => {
  const { t } = useTranslation();
  return (
    <Alert variant="warning" className="items-center justify-between flex-wrap">
      <div className="flex items-start gap-2.5">
        <AlertTriangle className="w-4 h-4 text-amber-400 shrink-0 mt-0.5" />
        <div>
          <AlertTitle>
            {t('pico.topHeader.lowVddTitle', { voltage: effectiveVddVolts.toFixed(2) })}
          </AlertTitle>
          <AlertDescription>{t('pico.topHeader.lowVddDesc')}</AlertDescription>
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
          {t('pico.studio.runCmd', { id: 'PINCHECK' })}
        </Button>
      )}
    </Alert>
  );
};
