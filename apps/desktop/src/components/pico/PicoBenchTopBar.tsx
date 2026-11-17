import React, { useState } from 'react';
import { useTranslation } from '@/i18n/useTranslation';
import type {
  DigaModelProfile,
  PicoDeviceStatus,
  PicoDiagnosticResult,
  PicoTransportMode,
  PicoUf2Variant,
} from '@/models';
import {
  detectPicoTransportFlags,
  isTargetPowered,
  resolveEffectiveVddVolts,
} from '@/utils/formatters';
import { Button } from '@/components/ui';
import { Radio, RefreshCw, ShieldCheck, Sparkles } from 'lucide-react';
import { PicoBenchTopBarHeader, PicoLowVddAlert } from './PicoBenchTopBarHeader';

export interface PicoBenchTopBarProps {
  deviceStatus: PicoDeviceStatus | null;
  latestDiagnostic?: PicoDiagnosticResult | null;
  availablePorts: string[];
  selectedPort: string;
  transportMode?: PicoTransportMode;
  modelCatalog?: DigaModelProfile[];
  selectedModel?: string;
  isErasing: boolean;
  isBackingUp?: boolean;
  isFlashing?: boolean;
  onSelectPort: (port: string) => void;
  onSelectModel?: (modelId: string) => void;
  onToggleConnection: () => void;
  onInjectAndErase: () => void;
  onFirstContactBackup?: () => void;
  onRefreshPorts?: () => void;
  onRefreshStatus?: () => void;
  onQuickPincheck?: () => void;
  onBootselReboot?: () => Promise<void> | void;
  onFlashFirmware?: (variant: PicoUf2Variant) => Promise<void> | void;
}

const SELECT_CLASS =
  'w-full px-3 py-2 rounded-xl bg-slate-950 border border-slate-800 text-xs text-slate-200 focus:outline-none focus:border-cyan-500 font-mono truncate';

function resolveConnectLabel(
  connected: boolean,
  isBle: boolean,
  isOtg: boolean,
  t: ReturnType<typeof useTranslation>['t'],
): string {
  if (connected) return t('pico.topBar.disconnectBtn');
  if (isBle) return t('pico.topBar.connectBleBtn');
  if (isOtg) return t('pico.topBar.connectOtgBtn');
  return t('pico.topBar.connectSerialBtn');
}

export const PicoBenchTopBar: React.FC<PicoBenchTopBarProps> = ({
  deviceStatus,
  latestDiagnostic,
  availablePorts,
  selectedPort,
  transportMode = 'usb',
  modelCatalog = [],
  selectedModel = 'AUTO',
  isErasing,
  isBackingUp = false,
  isFlashing = false,
  onSelectPort,
  onSelectModel,
  onToggleConnection,
  onInjectAndErase,
  onFirstContactBackup,
  onRefreshPorts,
  onRefreshStatus,
  onQuickPincheck,
  onBootselReboot,
  onFlashFirmware,
}) => {
  const { t } = useTranslation();
  const [uf2Variant, setUf2Variant] = useState<PicoUf2Variant>('unified');

  const { isBle, isOtg, isPicoPort } = detectPicoTransportFlags(transportMode, selectedPort);
  const effectiveVddVolts = resolveEffectiveVddVolts(
    deviceStatus?.vdd_volts,
    latestDiagnostic?.vdd_volts,
  );
  const powered = isTargetPowered(effectiveVddVolts);
  const isConnected = Boolean(deviceStatus?.connected);
  const connectLabel = resolveConnectLabel(isConnected, isBle, isOtg, t);

  return (
    <div className="p-4 sm:p-5 rounded-2xl bg-bench-900 border border-slate-800 shadow-xl space-y-4 font-mono">
      <PicoBenchTopBarHeader
        deviceStatus={deviceStatus}
        isBle={isBle}
        isOtg={isOtg}
        isPicoPort={isPicoPort}
        isFlashing={isFlashing}
        uf2Variant={uf2Variant}
        onChangeUf2Variant={setUf2Variant}
        onRefreshStatus={onRefreshStatus}
        onFlashFirmware={onFlashFirmware}
        onBootselReboot={onBootselReboot}
      />

      {isConnected && !powered && (
        <PicoLowVddAlert effectiveVddVolts={effectiveVddVolts} onQuickPincheck={onQuickPincheck} />
      )}

      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-5 gap-3 items-end">
        <div className="flex flex-col gap-1">
          <label htmlFor="pico-model-select" className="text-[10px] text-slate-500 uppercase font-semibold">
            {t('pico.topBar.modelLabel')}
          </label>
          <select
            id="pico-model-select"
            aria-label={t('pico.topBar.modelAria')}
            value={selectedModel}
            onChange={(e) => onSelectModel?.(e.target.value)}
            className={`${SELECT_CLASS} text-cyan-300`}
          >
            {modelCatalog.map((m) => (
              <option key={m.model_id} value={m.model_id}>
                {m.model_id === 'AUTO'
                  ? t('pico.topBar.modelAuto')
                  : t('pico.topBar.modelOption', { id: m.model_id, flashMb: m.flash_size_mb })}
              </option>
            ))}
          </select>
        </div>

        <div className="flex flex-col gap-1">
          <div className="flex items-center justify-between">
            <label htmlFor="pico-port-select" className="text-[10px] text-slate-500 uppercase font-semibold">
              {t('pico.topBar.portLabel')}
            </label>
            {onRefreshPorts && (
              <button
                type="button"
                onClick={onRefreshPorts}
                title={t('pico.topBar.refreshPortsTitle')}
                className="text-[10px] text-cyan-400 hover:text-cyan-300 flex items-center gap-1 transition-colors focus:outline-none"
              >
                <RefreshCw className="w-2.5 h-2.5" />
                <span>{t('pico.topBar.refreshPortsBtn')}</span>
              </button>
            )}
          </div>
          <select
            id="pico-port-select"
            aria-label={t('pico.topBar.selectPortAria')}
            value={selectedPort}
            onChange={(e) => onSelectPort(e.target.value)}
            className={SELECT_CLASS}
          >
            {availablePorts.length === 0 ? (
              <option value="">{t('pico.topBar.noPorts')}</option>
            ) : (
              availablePorts.map((p) => (
                <option key={p} value={p}>
                  {p}
                </option>
              ))
            )}
          </select>
        </div>

        <div className="flex flex-col gap-1 justify-end">
          <span className="text-[10px] text-transparent select-none">{t('pico.topBar.connectionSpacer')}</span>
          <Button
            type="button"
            variant={isConnected ? 'secondary' : 'default'}
            onClick={onToggleConnection}
            className={`w-full font-bold text-xs gap-1.5 ${
              !isConnected && isBle ? 'bg-blue-600 hover:bg-blue-500 text-white' : ''
            }`}
          >
            <Radio className="w-3.5 h-3.5" />
            <span>{connectLabel}</span>
          </Button>
        </div>

        <div className="flex flex-col gap-1 justify-end">
          <span className="text-[10px] text-cyan-400 font-semibold uppercase">
            {t('pico.topBar.firstContactLabel')}
          </span>
          <Button
            type="button"
            variant="outline"
            onClick={onFirstContactBackup}
            disabled={isBackingUp || !isConnected}
            title={t('pico.topBar.firstContactTooltip')}
            className="w-full bg-cyan-500/10 hover:bg-cyan-500/20 text-cyan-300 border-cyan-500/30 font-bold text-xs gap-1.5"
          >
            {isBackingUp ? (
              <>
                <RefreshCw className="w-3.5 h-3.5 animate-spin" />
                <span>{t('pico.topBar.readingNvram')}</span>
              </>
            ) : (
              <>
                <ShieldCheck className="w-3.5 h-3.5" />
                <span>{t('pico.topBar.firstContactBtn')}</span>
              </>
            )}
          </Button>
        </div>

        <div className="flex flex-col gap-1 justify-end">
          <span className="text-[10px] text-emerald-400 font-semibold uppercase">
            {t('pico.topBar.sanitizeLabel')}
          </span>
          <Button
            type="button"
            variant="emerald"
            onClick={onInjectAndErase}
            disabled={isErasing || !isConnected}
            className="w-full font-bold text-xs gap-1.5"
          >
            {isErasing ? (
              <>
                <RefreshCw className="w-3.5 h-3.5 animate-spin" />
                <span>{t('pico.topBar.injectingBtn')}</span>
              </>
            ) : (
              <>
                <Sparkles className="w-3.5 h-3.5" />
                <span>{t('pico.topBar.injectCleanBtn')}</span>
              </>
            )}
          </Button>
        </div>
      </div>
    </div>
  );
};
