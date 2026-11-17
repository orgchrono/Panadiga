import React from 'react';
import {
  Activity,
  Loader2,
  PanelLeft,
  ShieldCheck,
  Terminal,
} from 'lucide-react';
import { useTranslation } from '@/i18n/useTranslation';
import { cn } from '@/lib/utils';
import type { WorkbenchDocksState } from '@/types/workbench';
import { formatVoltage, isTargetPowered } from '@/utils/formatters';
import { Badge } from './badge';
import { Button } from './button';

export interface WorkbenchStatusBarProps {
  readonly currentStageLabel: string;
  readonly hardwareConnected: boolean;
  readonly vddVolts: number;
  readonly minVddThreshold?: number;
  readonly portName?: string;
  readonly runningCommandId?: string | null;
  readonly logCount: number;
  readonly docks: WorkbenchDocksState;
  readonly onToggleLeftDock: () => void;
  readonly onToggleBottomDock: () => void;
  readonly onToggleRightDock: () => void;
  readonly onOpenCompliance: () => void;
}

export const WorkbenchStatusBar: React.FC<WorkbenchStatusBarProps> = ({
  currentStageLabel,
  hardwareConnected,
  vddVolts,
  minVddThreshold,
  portName,
  runningCommandId = null,
  logCount,
  docks,
  onToggleLeftDock,
  onToggleBottomDock,
  onToggleRightDock,
  onOpenCompliance,
}) => {
  const { t } = useTranslation();
  const powered = isTargetPowered(vddVolts, minVddThreshold);

  const linkIndicator = hardwareConnected
    ? powered
      ? { dot: 'bg-emerald-400', text: 'text-emerald-400', label: t('pico.metrics.badgePowered') }
      : { dot: 'bg-amber-400', text: 'text-amber-400', label: t('pico.metrics.badgeStandby') }
    : { dot: 'bg-rose-400', text: 'text-rose-400', label: t('pico.disconnected') };

  const dockButtons: ReadonlyArray<{
    readonly testId: string;
    readonly active: boolean;
    readonly onClick: () => void;
    readonly label: string;
    readonly Icon: typeof PanelLeft;
    readonly count: number | undefined;
  }> = [
    {
      testId: 'status-toggle-left-dock',
      active: docks.leftOpen,
      onClick: onToggleLeftDock,
      label: t('shell.statusBar.leftDockBtn'),
      Icon: PanelLeft,
      count: undefined,
    },
    {
      testId: 'status-toggle-bottom-dock',
      active: docks.bottomOpen,
      onClick: onToggleBottomDock,
      label: t('shell.statusBar.bottomDockBtn'),
      Icon: Terminal,
      count: logCount,
    },
    {
      testId: 'status-toggle-right-dock',
      active: docks.rightOpen,
      onClick: onToggleRightDock,
      label: t('shell.statusBar.rightDockBtn'),
      Icon: Activity,
      count: undefined,
    },
  ];

  return (
    <footer
      data-testid="workbench-status-bar"
      aria-label={t('shell.statusBar.ariaLabel')}
      className="min-h-9 px-3 py-1 bg-bench-950 border-t border-slate-800 flex flex-wrap items-center justify-between gap-2 font-mono text-[11px] shrink-0 select-none"
    >
      <div className="flex items-center gap-2 min-w-0">
        <span className="inline-flex items-center gap-1.5 font-semibold shrink-0">
          <span className={cn('w-2 h-2 rounded-full', linkIndicator.dot)} />
          <span className={linkIndicator.text}>{linkIndicator.label}</span>
        </span>
        <span className="text-slate-400 truncate">
          {portName ? `${portName} • ` : ''}
          {formatVoltage(vddVolts)}V
        </span>
        <Badge
          variant="outline"
          className="hidden md:inline-flex items-center gap-1 px-2 py-0 text-[10px] border-emerald-500/30 bg-emerald-500/5 text-emerald-300"
        >
          <ShieldCheck className="w-3 h-3" />
          <span>{t('shell.statusBar.shieldLabel')}</span>
        </Badge>
      </div>

      <div className="flex items-center gap-1.5 text-slate-300 min-w-0">
        {runningCommandId ? (
          <>
            <Loader2 className="w-3 h-3 animate-spin text-cyan-400 shrink-0" />
            <span className="text-cyan-300 font-semibold truncate">
              {t('shell.statusBar.runningCmd', { id: runningCommandId })}
            </span>
          </>
        ) : (
          <span className="text-slate-400 truncate">
            {t('shell.statusBar.idleStage', { stage: currentStageLabel })}
          </span>
        )}
      </div>

      <div className="flex items-center gap-1 flex-wrap">
        {dockButtons.length === 0 ? null : dockButtons.map(({ testId, active, onClick, label, Icon, count }) => (
          <Button
            key={testId}
            type="button"
            size="sm"
            variant={active ? 'secondary' : 'outline'}
            data-testid={testId}
            aria-pressed={active}
            onClick={onClick}
            className={cn(
              'h-7 px-2 text-[11px] gap-1.5 border-slate-800',
              active ? 'text-cyan-300 border-cyan-500/40 bg-cyan-500/10' : 'text-slate-300',
            )}
          >
            <Icon className="w-3 h-3" />
            <span>{label}</span>
            {count !== undefined && (
              <span className="px-1 rounded bg-slate-900 text-slate-400 text-[10px] leading-4">
                {count}
              </span>
            )}
          </Button>
        ))}

        <Button
          type="button"
          size="sm"
          variant="outline"
          data-testid="status-open-compliance-btn"
          onClick={onOpenCompliance}
          className="h-7 px-2 text-[11px] gap-1.5 border-emerald-500/30 text-emerald-300 hover:bg-emerald-500/10"
        >
          <ShieldCheck className="w-3 h-3" />
          <span>{t('shell.statusBar.complianceBtn')}</span>
        </Button>
      </div>
    </footer>
  );
};
