import React from 'react';
import { Activity, Cpu, Loader2, Play, Radio, Terminal, Wrench, X } from 'lucide-react';
import { HARDWARE_THRESHOLDS_SPEC } from '@/constants';
import { useTranslation } from '@/i18n/useTranslation';
import { cn } from '@/lib/utils';
import type { ColdBreakinMode } from '@/models';
import { Badge } from './badge';
import { Button } from './button';

export interface DockDrawerShellProps {
  readonly side: 'left' | 'right';
  readonly testId: string;
  readonly widthClassName: string;
  readonly title: string;
  readonly subtitle: string;
  readonly icon: React.ReactNode;
  readonly closeLabel: string;
  readonly onClose: () => void;
  readonly children: React.ReactNode;
}

export const DockDrawerShell: React.FC<DockDrawerShellProps> = ({
  side, testId, widthClassName, title, subtitle, icon, closeLabel, onClose, children,
}) => (
  <>
    <button type="button" aria-label={closeLabel} onClick={onClose} className="fixed inset-0 z-30 bg-black/60 lg:hidden" />
    <aside
      data-testid={testId}
      aria-label={title}
      className={cn(
        'fixed inset-y-0 z-40 lg:static lg:z-auto flex shrink-0 flex-col bg-bench-950/95 shadow-2xl lg:shadow-none font-mono',
        widthClassName,
        side === 'left' ? 'left-0 border-r border-slate-800' : 'right-0 border-l border-slate-800',
      )}
    >
      <div className="flex items-start justify-between gap-2 px-3 py-2.5 border-b border-slate-800 bg-bench-900/60">
        <div className="min-w-0">
          <div className="flex items-center gap-1.5 text-xs font-bold text-slate-100">
            {icon}
            <span className="truncate">{title}</span>
          </div>
          <p className="text-[10px] text-slate-500 font-sans mt-0.5 leading-snug">{subtitle}</p>
        </div>
        <Button type="button" variant="ghost" size="icon" onClick={onClose} title={closeLabel} aria-label={closeLabel} className="h-7 w-7 shrink-0">
          <X className="w-3.5 h-3.5" />
        </Button>
      </div>
      <div className="flex-1 overflow-y-auto p-3 space-y-4 scrollbar-thin scrollbar-thumb-slate-800">{children}</div>
    </aside>
  </>
);

export interface LeftCommandDockProps {
  readonly open: boolean;
  readonly onClose: () => void;
  readonly onRunCommand: (commandId: string, arg?: string) => void;
  readonly onOpenPicoBench: () => void;
  readonly runningCommandId?: string | null;
  readonly hardwareConnected?: boolean;
  readonly breakinMode?: ColdBreakinMode;
  readonly hasSsotCommands?: boolean;
}

interface QuickCommandItem {
  readonly id: string;
  readonly arg?: string;
}

const PROBE_COMMANDS: readonly QuickCommandItem[] = [
  { id: 'PINCHECK' },
  { id: 'STATUS' },
  { id: 'BUSDIAG' },
  { id: 'BOOTTRACE', arg: 'PREJIG:3000 WAKE:45000' },
  { id: 'BAUDSCAN', arg: 'PASSIVE APPLY WAKE:45000' },
];

const SURGERY_COMMANDS: readonly QuickCommandItem[] = [
  { id: 'DUMP', arg: HARDWARE_THRESHOLDS_SPEC.default_nvram_base_hex },
];

const CommandGroupSection: React.FC<{
  readonly title: string;
  readonly icon: React.ReactNode;
  readonly commands: readonly QuickCommandItem[];
  readonly runningCommandId: string | null;
  readonly onRunCommand: (commandId: string, arg?: string) => void;
  readonly bordered?: boolean;
  readonly footer?: React.ReactNode;
}> = ({ title, icon, commands, runningCommandId, onRunCommand, bordered = false, footer }) => {
  const { t } = useTranslation();
  const isBusy = runningCommandId !== null;
  return (
    <div className={cn('space-y-1.5', bordered && 'pt-2 border-t border-slate-800/80')}>
      <div className="flex items-center gap-1.5 text-[10px] font-bold uppercase tracking-wider text-slate-400">
        {icon}
        <span>{title}</span>
      </div>
      <div className="space-y-1.5">
        {commands.length === 0 ? (
          <p className="text-[11px] text-slate-500">{t('shell.docks.commandsEmpty')}</p>
        ) : (
          commands.map((cmd) => (
            <Button
              key={cmd.id}
              type="button"
              variant="outline"
              size="sm"
              disabled={isBusy}
              data-testid={`left-dock-cmd-${cmd.id}`}
              onClick={() => onRunCommand(cmd.id, cmd.arg)}
              className="w-full h-auto justify-between px-2.5 py-2 rounded-lg bg-slate-950 border-slate-800 hover:border-cyan-500/40 text-left"
            >
              <div className="min-w-0">
                <div className="text-[11px] font-bold text-slate-100">{cmd.id}</div>
                {cmd.arg && <div className="text-[10px] text-slate-500 truncate">{cmd.arg}</div>}
              </div>
              {runningCommandId === cmd.id ? (
                <Loader2 className="w-3.5 h-3.5 animate-spin text-cyan-400 shrink-0" />
              ) : (
                <Play className="w-3 h-3 text-cyan-400 shrink-0" />
              )}
            </Button>
          ))
        )}
        {footer}
      </div>
    </div>
  );
};

export const LeftCommandDock: React.FC<LeftCommandDockProps> = ({
  open, onClose, onRunCommand, onOpenPicoBench,
  runningCommandId = null, hardwareConnected = false, breakinMode = 0, hasSsotCommands = true,
}) => {
  const { t } = useTranslation();
  if (!open) return null;

  return (
    <DockDrawerShell
      side="left"
      testId="left-command-dock"
      widthClassName="w-72 max-w-[85vw]"
      title={t('shell.docks.commandsTitle')}
      subtitle={t('shell.docks.commandsSubtitle')}
      icon={<Terminal className="w-3.5 h-3.5 text-cyan-400 shrink-0" />}
      closeLabel={t('shell.docks.closeDock')}
      onClose={onClose}
    >
      {!hasSsotCommands ? (
        <p className="text-xs text-slate-500 font-sans">{t('shell.docks.commandsEmpty')}</p>
      ) : (
        <>
          <CommandGroupSection
            title={t('shell.docks.groupProbe')}
            icon={<Activity className="w-3 h-3 text-cyan-400" />}
            commands={PROBE_COMMANDS}
            runningCommandId={runningCommandId}
            onRunCommand={onRunCommand}
          />
          <CommandGroupSection
            title={t('shell.docks.groupSurgery')}
            icon={<Cpu className="w-3 h-3 text-amber-400" />}
            commands={SURGERY_COMMANDS}
            runningCommandId={runningCommandId}
            onRunCommand={onRunCommand}
            bordered
            footer={
              <Button type="button" variant="secondary" size="sm" onClick={onOpenPicoBench} className="w-full h-8 justify-start gap-1.5 text-[11px]">
                <Radio className="w-3.5 h-3.5 text-cyan-400" />
                <span>{t('shell.docks.openPicoBenchBtn')}</span>
              </Button>
            }
          />
          <div className="space-y-2 pt-2 border-t border-slate-800/80">
            <div className="flex items-center justify-between gap-2">
              <div className="flex items-center gap-1.5 text-[10px] font-bold uppercase tracking-wider text-slate-400">
                <Wrench className="w-3 h-3 text-emerald-400" />
                <span>{t('shell.docks.groupBreakin')}</span>
              </div>
              {hardwareConnected && (
                <Badge variant="success" className="text-[9px] px-1.5 py-0">
                  {t('shell.docks.breakinModeLabel', { mode: breakinMode })}
                </Badge>
              )}
            </div>
            <div className="grid grid-cols-2 gap-1.5">
              <Button type="button" variant="outline" size="sm" onClick={onOpenPicoBench} className="h-8 px-2 text-[10px] border-slate-800">
                {t('shell.docks.openSc0BridgeBtn')}
              </Button>
              <Button type="button" variant="outline" size="sm" onClick={onOpenPicoBench} className="h-8 px-2 text-[10px] border-slate-800">
                {t('shell.docks.openGdbServerBtn')}
              </Button>
            </div>
          </div>
        </>
      )}
    </DockDrawerShell>
  );
};
