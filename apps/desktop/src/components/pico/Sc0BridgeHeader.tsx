import React from 'react';
import { CardHeader, CardTitle, Button, Badge } from '@/components/ui';
import { StatusBadge } from '@/components/StatusBadge';
import { useTranslation } from '@/i18n/useTranslation';
import type { Sc0BridgeStatus } from '@/models';
import { Play, Radio, Square } from 'lucide-react';

export interface Sc0BridgeHeaderProps {
  readonly status: Sc0BridgeStatus;
  readonly baudRate: string;
  readonly onChangeBaudRate: (baud: string) => void;
  readonly bootMode: string;
  readonly onChangeBootMode: (mode: string) => void;
  readonly isBusy: boolean;
  readonly onStart: () => void;
  readonly onStop: () => void;
}

export const Sc0BridgeHeader: React.FC<Sc0BridgeHeaderProps> = ({
  status,
  baudRate,
  onChangeBaudRate,
  bootMode,
  onChangeBootMode,
  isBusy,
  onStart,
  onStop,
}) => {
  const { t } = useTranslation();

  return (
    <CardHeader className="pb-3 border-b border-slate-800/80 flex flex-col lg:flex-row lg:items-center lg:justify-between gap-3">
      <div className="space-y-1">
        <div className="flex items-center gap-2 flex-wrap">
          <Radio className="w-4 h-4 text-cyan-400" />
          <CardTitle className="text-sm font-bold text-slate-100">
            {t('pico.bridge.title')}
          </CardTitle>
          <StatusBadge
            variant={status.running ? 'success' : 'neutral'}
            label={status.running ? t('pico.bridge.active') : t('pico.bridge.standby')}
          />
          <Badge variant="outline" className="text-[10px]">
            {t('pico.bridge.bytesRx', { count: status.bytes_rx })} |{' '}
            {t('pico.bridge.bytesTx', { count: status.bytes_tx })}
          </Badge>
        </div>
        <p className="text-xs text-slate-400 font-sans">{t('pico.bridge.subtitle')}</p>
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <label className="flex items-center gap-1.5 text-[11px] text-slate-400">
          <span>{t('pico.bridge.baudLabel')}</span>
          <select
            aria-label={t('pico.bridge.baudLabel')}
            value={baudRate}
            disabled={status.running || isBusy}
            onChange={e => onChangeBaudRate(e.target.value)}
            className="rounded-lg bg-slate-950 border border-slate-700 px-2 py-1 text-xs text-slate-100"
          >
            <option value="115200">115200</option>
            <option value="57600">57600</option>
            <option value="38400">38400</option>
            <option value="9600">9600</option>
          </select>
        </label>

        <label className="flex items-center gap-1.5 text-[11px] text-slate-400">
          <span>{t('pico.bridge.bootLabel')}</span>
          <select
            aria-label={t('pico.bridge.bootLabel')}
            value={bootMode}
            disabled={status.running || isBusy}
            onChange={e => onChangeBootMode(e.target.value)}
            className="rounded-lg bg-slate-950 border border-slate-700 px-2 py-1 text-xs text-slate-100"
          >
            <option value="JIG:350">JIG:350</option>
            <option value="NORMAL">NORMAL</option>
            <option value="NOPULL">NOPULL</option>
            <option value="PASSIVE">{t('pico.bridge.passiveOption')}</option>
          </select>
        </label>

        {status.running ? (
          <Button
            type="button"
            variant="destructive"
            size="sm"
            disabled={isBusy}
            onClick={onStop}
            className="gap-1.5 text-xs font-bold"
          >
            <Square className="w-3.5 h-3.5" />
            <span>{t('pico.bridge.stop')}</span>
          </Button>
        ) : (
          <Button
            type="button"
            variant="default"
            size="sm"
            disabled={isBusy}
            onClick={onStart}
            className="gap-1.5 text-xs font-bold"
          >
            <Play className="w-3.5 h-3.5" />
            <span>{t('pico.bridge.start')}</span>
          </Button>
        )}
      </div>
    </CardHeader>
  );
};
