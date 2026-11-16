import React from 'react';
import { AlertTriangle, Cpu, Radio, Square, Wifi } from 'lucide-react';
import { useTranslation } from '@/i18n/useTranslation';
import { Badge, Button, Card, CardContent } from '@/components/ui';
import type { IrTransportKind, RemoteMode, RemoteUnitSet } from '@/models/ir';

interface IrVfdBannerProps {
  vfdPrimary: string;
  vfdSecondary: string;
  lastWireHex: string;
  lastBackendUsed: string;
  mode: RemoteMode;
  remoteUnit: RemoteUnitSet;
  transport: IrTransportKind;
  isTransmitting: boolean;
  isBeaconRunning: boolean;
  activeMacroId: string | null;
  macroStepIndex: number;
  macroTotalSteps: number;
  error: string | null;
  onAbortMacro: () => void;
}

export const IrVfdBanner: React.FC<IrVfdBannerProps> = ({
  vfdPrimary,
  vfdSecondary,
  lastWireHex,
  lastBackendUsed,
  mode,
  remoteUnit,
  transport,
  isTransmitting,
  isBeaconRunning,
  activeMacroId,
  macroStepIndex,
  macroTotalSteps,
  error,
  onAbortMacro,
}) => {
  const { t } = useTranslation();

  return (
    <Card className="bg-bench-950 border-emerald-500/30 shadow-lg">
      <CardContent className="p-4 space-y-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <Radio className="w-4 h-4 text-emerald-400" />
            <span className="text-[11px] font-mono uppercase tracking-wider text-emerald-400">
              {t('irCommander.vfd.header')}
            </span>
          </div>
          <div className="flex flex-wrap items-center gap-1.5">
            <Badge variant="outline" className="font-mono text-[10px] border-cyan-500/40 text-cyan-300">
              {mode} · SET {remoteUnit}
            </Badge>
            <Badge variant="outline" className="font-mono text-[10px] border-slate-700 text-slate-300 gap-1">
              {transport.includes('BLE') ? <Wifi className="w-3 h-3" /> : <Cpu className="w-3 h-3" />}
              {lastBackendUsed !== '—' ? lastBackendUsed : transport}
            </Badge>
            {(isTransmitting || isBeaconRunning) && (
              <Badge className="font-mono text-[10px] bg-emerald-500/20 text-emerald-300 border border-emerald-500/40 animate-pulse">
                {t('irCommander.vfd.txActive')}
              </Badge>
            )}
          </div>
        </div>

        <div className="rounded-lg bg-black/80 border border-emerald-500/25 px-3.5 py-2.5 font-mono">
          <div className="text-sm sm:text-base font-bold tracking-wide text-emerald-300 truncate">
            {vfdPrimary}
          </div>
          <div className="flex flex-wrap items-center justify-between gap-2 text-[11px] text-emerald-400/75 mt-1">
            <span className="truncate">{vfdSecondary}</span>
            <span className="text-cyan-300/90">WIRE: {lastWireHex}</span>
          </div>
        </div>

        {activeMacroId && (
          <div className="flex items-center justify-between gap-3 rounded-lg bg-amber-500/10 border border-amber-500/40 px-3 py-2 text-xs font-mono text-amber-200">
            <span>
              {t('irCommander.macro.runningProgress', {
                step: macroStepIndex,
                total: macroTotalSteps,
              })}
            </span>
            <Button
              type="button"
              size="sm"
              variant="destructive"
              onClick={onAbortMacro}
              data-testid="ir-abort-macro-btn"
              className="h-7 px-2.5 gap-1"
            >
              <Square className="w-3 h-3" />
              {t('irCommander.macro.abortBtn')}
            </Button>
          </div>
        )}

        {error && (
          <div
            role="alert"
            className="flex items-start gap-2 rounded-lg bg-rose-500/10 border border-rose-500/40 px-3 py-2 text-xs text-rose-200"
          >
            <AlertTriangle className="w-4 h-4 shrink-0 text-rose-400 mt-0.5" />
            <span className="break-words">{error}</span>
          </div>
        )}
      </CardContent>
    </Card>
  );
};
