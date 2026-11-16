import React from 'react';
import { Button } from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import { Activity, Disc, ShieldCheck, Zap } from 'lucide-react';
import {
  DRIVE_QUICK_COMMANDS,
  RSP_QUICK_PROBES,
  type Sc0BridgeProbeHandlers,
} from './sc0BridgeTypes';

export interface Sc0BridgeQuickProbesProps extends Sc0BridgeProbeHandlers {
  readonly bridgeRunning: boolean;
  readonly isBusy: boolean;
}

export const Sc0BridgeQuickProbes: React.FC<Sc0BridgeQuickProbesProps> = ({
  bridgeRunning,
  isBusy,
  onTriggerWaittx,
  onSendRspPreset,
  onSendDriveCommand,
  onRunDiagnostic,
}) => {
  const { t } = useTranslation();

  return (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-3">
      <div className="rounded-xl border border-slate-800 bg-slate-950/70 p-3 space-y-2">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <span className="text-[11px] font-bold text-cyan-300 flex items-center gap-1.5">
            <Activity className="w-3.5 h-3.5" />
            <span>{t('pico.bridge.rspSection')}</span>
          </span>
          <div className="flex flex-wrap gap-1.5">
            <Button
              type="button"
              size="sm"
              variant="secondary"
              disabled={bridgeRunning || isBusy}
              onClick={() => onTriggerWaittx('8000 JIG:350 "wait 1" 0D 4000')}
              className="text-[10px] h-7 px-2 gap-1"
            >
              <Zap className="w-3 h-3" />
              <span>{t('pico.bridge.waittxWait1')}</span>
            </Button>
            <Button
              type="button"
              size="sm"
              variant="secondary"
              disabled={bridgeRunning || isBusy}
              onClick={() => onTriggerWaittx('20000 JIG:350 "Hit Any Key" 1B0D 6000')}
              className="text-[10px] h-7 px-2 gap-1"
            >
              <Zap className="w-3 h-3" />
              <span>{t('pico.bridge.waittxHitKey')}</span>
            </Button>
            <Button
              type="button"
              size="sm"
              variant="secondary"
              disabled={bridgeRunning || isBusy}
              onClick={() =>
                onRunDiagnostic?.('DIAGWATCH', '25000 JIG:2500 READ:847F0000,20 RELEASE')
              }
              className="text-[10px] h-7 px-2 gap-1"
            >
              <ShieldCheck className="w-3 h-3" />
              <span>{t('pico.bridge.diagwatchReadNvram')}</span>
            </Button>
          </div>
        </div>

        <div className="flex flex-wrap gap-1.5">
          {RSP_QUICK_PROBES.length === 0 ? (
            <span className="text-[11px] text-slate-500">-</span>
          ) : (
            RSP_QUICK_PROBES.map(probe => (
              <Button
                key={probe.label}
                type="button"
                size="sm"
                variant="outline"
                disabled={isBusy}
                onClick={() => onSendRspPreset(probe.mode, probe.bridgePayload, probe.panarspArg)}
                className="text-[11px] h-7 px-2.5 font-mono"
              >
                {probe.label}
              </Button>
            ))
          )}
        </div>
      </div>

      <div className="rounded-xl border border-slate-800 bg-slate-950/70 p-3 space-y-2">
        <div className="flex items-center justify-between gap-2">
          <span className="text-[11px] font-bold text-amber-300 flex items-center gap-1.5">
            <Disc className="w-3.5 h-3.5" />
            <span>{t('pico.bridge.driveSection')}</span>
          </span>
          <span className="text-[10px] font-mono text-amber-400/80">DRVPANA / BRIDGE</span>
        </div>
        <div className="flex flex-wrap gap-1.5">
          {DRIVE_QUICK_COMMANDS.length === 0 ? (
            <span className="text-[11px] text-slate-500">-</span>
          ) : (
            DRIVE_QUICK_COMMANDS.map(item => (
              <Button
                key={item.label}
                type="button"
                size="sm"
                variant="outline"
                title={item.cmd}
                disabled={isBusy}
                onClick={() => onSendDriveCommand(item.cmd)}
                className="text-[11px] h-7 px-2.5 font-mono"
              >
                {item.label}
              </Button>
            ))
          )}
        </div>
      </div>
    </div>
  );
};
