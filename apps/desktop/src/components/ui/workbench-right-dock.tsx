import React from 'react';
import {
  Activity,
  CheckCircle2,
  Link2,
  ShieldCheck,
  XCircle,
} from 'lucide-react';
import { GDB_DEFAULT_LISTEN_ADDR } from '@/constants';
import { useTranslation } from '@/i18n/useTranslation';
import type { PicoDiagnosticResult, PicoTransportMode } from '@/models';
import { formatElapsedMs, formatVoltage, isTargetPowered } from '@/utils/formatters';
import { Badge } from './badge';
import { DockDrawerShell } from './workbench-left-dock';

export interface RightTelemetryDockProps {
  readonly open: boolean;
  readonly onClose: () => void;
  readonly hardwareConnected: boolean;
  readonly vddVolts: number;
  readonly minVddThreshold?: number;
  readonly portName?: string;
  readonly modelId?: string;
  readonly transportMode?: PicoTransportMode;
  readonly gdbRunning?: boolean;
  readonly gdbListenAddr?: string;
  readonly sc0Running?: boolean;
  readonly latestDiagnostic?: PicoDiagnosticResult | null;
}

const TRANSPORT_LABELS: Readonly<Record<PicoTransportMode, string>> = {
  usb: 'USB',
  'usb-otg': 'USB-OTG',
  ble: 'BLE',
};

const LOCKED_INVARIANT_BADGES: readonly string[] = [
  '3.3V LVTTL',
  'NSW5V GP3 Hi-Z',
  'XRESET Open-Drain',
  'SA0..SA10 RO',
];

const LatestDiagnosticCard: React.FC<{
  readonly latestDiagnostic: PicoDiagnosticResult | null;
}> = ({ latestDiagnostic }) => {
  const { t } = useTranslation();
  if (!latestDiagnostic) {
    return (
      <div className="rounded-xl border border-slate-800/80 bg-slate-950/60 p-2.5 text-slate-500 font-sans">
        {t('shell.docks.noReturnYet')}
      </div>
    );
  }
  const findingsCount = latestDiagnostic.findings?.length ?? 0;

  return (
    <div
      data-testid="right-dock-latest-diagnostic"
      className="rounded-xl border border-slate-800 bg-slate-950/90 p-2.5 space-y-2"
    >
      <div className="flex items-center justify-between gap-1.5 flex-wrap">
        <Badge variant="default" className="text-[10px] px-1.5 py-0">
          {latestDiagnostic.command_id}
        </Badge>
        <Badge
          variant={latestDiagnostic.success ? 'success' : 'destructive'}
          className="text-[10px] px-1.5 py-0 gap-1"
        >
          {latestDiagnostic.success ? (
            <CheckCircle2 className="w-3 h-3" />
          ) : (
            <XCircle className="w-3 h-3" />
          )}
          <span>
            {latestDiagnostic.success ? t('pico.studio.statusOk') : t('pico.studio.statusFail')}
          </span>
        </Badge>
      </div>

      <div className="flex items-center justify-between text-[10px] text-slate-400">
        <span>{formatElapsedMs(latestDiagnostic.elapsed_ms)}</span>
        {typeof latestDiagnostic.vdd_volts === 'number' && (
          <span className="text-cyan-300">
            {t('pico.studio.vddBadge', { volts: formatVoltage(latestDiagnostic.vdd_volts) })}
          </span>
        )}
        {findingsCount > 0 && (
          <Badge variant="warning" className="text-[9px] px-1.5 py-0">
            {findingsCount}
          </Badge>
        )}
      </div>

      <p className="text-[11px] text-slate-200 font-sans leading-relaxed break-words">
        {latestDiagnostic.summary}
      </p>
    </div>
  );
};

export const RightTelemetryDock: React.FC<RightTelemetryDockProps> = ({
  open,
  onClose,
  hardwareConnected,
  vddVolts,
  minVddThreshold,
  portName,
  modelId,
  transportMode = 'usb',
  gdbRunning = false,
  gdbListenAddr = GDB_DEFAULT_LISTEN_ADDR,
  sc0Running = false,
  latestDiagnostic = null,
}) => {
  const { t } = useTranslation();
  if (!open) return null;

  const powered = isTargetPowered(vddVolts, minVddThreshold);
  const vddBadgeVariant = !hardwareConnected ? 'secondary' : powered ? 'success' : 'warning';

  return (
    <DockDrawerShell
      side="right"
      testId="right-telemetry-dock"
      widthClassName="w-80 max-w-[90vw]"
      title={t('shell.docks.telemetryTitle')}
      subtitle={t('shell.docks.telemetrySubtitle')}
      icon={<Activity className="w-3.5 h-3.5 text-cyan-400 shrink-0" />}
      closeLabel={t('shell.docks.closeDock')}
      onClose={onClose}
    >
      <div className="space-y-4 text-[11px]">
        <section className="space-y-2">
          <div className="flex items-center gap-1.5 text-[10px] font-bold uppercase tracking-wider text-slate-400">
            <Link2 className="w-3 h-3 text-cyan-400" />
            <span>{t('shell.docks.activeLinksTitle')}</span>
          </div>

          <div className="rounded-xl border border-slate-800 bg-slate-950/90 p-2.5 space-y-1.5">
            <div className="flex items-center justify-between gap-2">
              <span className="text-slate-400">{t('shell.docks.portLabel')}</span>
              <span className="text-slate-100 font-semibold truncate">{portName || '--'}</span>
            </div>
            <div className="flex items-center justify-between gap-2">
              <span className="text-slate-400">{t('shell.docks.modelLabel')}</span>
              <span className="text-slate-200 truncate">{modelId || '--'}</span>
            </div>
            <div className="flex items-center justify-between gap-2">
              <span className="text-slate-400">{t('shell.docks.transportLabel')}</span>
              <Badge variant="outline" className="text-[9px] px-1.5 py-0">
                {TRANSPORT_LABELS[transportMode]}
              </Badge>
            </div>
            <div className="flex items-center justify-between gap-2">
              <span className="text-slate-400">P7501 VDD</span>
              <Badge variant={vddBadgeVariant} className="text-[9px] px-1.5 py-0">
                {formatVoltage(vddVolts)}V
              </Badge>
            </div>
            <div className="flex items-center justify-between gap-2">
              <span className="text-slate-400">{t('shell.docks.gdbLinkLabel')}</span>
              <Badge variant={gdbRunning ? 'success' : 'secondary'} className="text-[9px] px-1.5 py-0">
                {gdbRunning
                  ? t('shell.docks.gdbListening', { addr: gdbListenAddr })
                  : t('shell.docks.gdbIdle')}
              </Badge>
            </div>
            <div className="flex items-center justify-between gap-2">
              <span className="text-slate-400">{t('shell.docks.sc0LinkLabel')}</span>
              <Badge variant={sc0Running ? 'default' : 'secondary'} className="text-[9px] px-1.5 py-0">
                {sc0Running ? t('shell.docks.sc0Active') : t('shell.docks.sc0Standby')}
              </Badge>
            </div>
          </div>
        </section>

        <section className="space-y-2 pt-2 border-t border-slate-800/80">
          <div className="text-[10px] font-bold uppercase tracking-wider text-slate-400">
            {t('shell.docks.lastReturnTitle')}
          </div>
          <LatestDiagnosticCard latestDiagnostic={latestDiagnostic} />
        </section>

        <section className="space-y-2 pt-2 border-t border-slate-800/80">
          <div className="flex items-center gap-1.5 text-[10px] font-bold uppercase tracking-wider text-slate-400">
            <ShieldCheck className="w-3 h-3 text-emerald-400" />
            <span>{t('shell.docks.invariantsTitle')}</span>
          </div>
          <div className="flex flex-wrap gap-1.5">
            {LOCKED_INVARIANT_BADGES.length === 0 ? null : LOCKED_INVARIANT_BADGES.map((token) => (
              <Badge
                key={token}
                variant="outline"
                className="text-[10px] px-2 py-0.5 border-emerald-500/30 bg-emerald-500/5 text-emerald-300"
              >
                {token}
              </Badge>
            ))}
          </div>
        </section>
      </div>
    </DockDrawerShell>
  );
};
