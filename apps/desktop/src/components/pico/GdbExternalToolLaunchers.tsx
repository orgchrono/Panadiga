import React, { useRef, useState } from 'react';
import { Bug, FileCode, Binary, Terminal, RefreshCw, Cpu, type LucideIcon } from 'lucide-react';
import { Button } from '@/components/ui';
import { GDB_DEFAULT_TCP_PORT } from '@/constants/hardwareSpecs/physicalConstants';
import { useManagedTimeouts } from '@/hooks/useManagedTimeouts';
import { useTranslation } from '@/i18n/useTranslation';
import type { TranslationKey } from '@/i18n/translations';

export type ExternalToolName = 'gdb' | 'ghidra' | 'ghidra_sync' | 'ghidra_mcp' | 'hxd' | 'terminal';

export interface GdbExternalToolLaunchersProps {
  onLaunchTool?: (tool: ExternalToolName) => Promise<void>;
  isAndroidHost?: boolean;
}

const TOOL_LAUNCHERS: Array<{
  id: ExternalToolName;
  labelKey: TranslationKey;
  titleKey: TranslationKey;
  className: string;
  Icon: LucideIcon;
}> = [
  {
    id: 'gdb',
    labelKey: 'pico.launchGdb',
    titleKey: 'pico.toolsRail.gdbTitle',
    className: 'bg-cyan-500/10 hover:bg-cyan-500/20 text-cyan-300 border-cyan-500/30',
    Icon: Bug,
  },
  {
    id: 'ghidra',
    labelKey: 'pico.launchGhidra',
    titleKey: 'pico.toolsRail.ghidraTitle',
    className: 'bg-emerald-500/10 hover:bg-emerald-500/20 text-emerald-300 border-emerald-500/30',
    Icon: FileCode,
  },
  {
    id: 'ghidra_sync',
    labelKey: 'pico.toolsRail.syncLabel',
    titleKey: 'pico.toolsRail.syncTitle',
    className: 'bg-teal-500/10 hover:bg-teal-500/20 text-teal-300 border-teal-500/30',
    Icon: RefreshCw,
  },
  {
    id: 'ghidra_mcp',
    labelKey: 'pico.toolsRail.mcpLabel',
    titleKey: 'pico.toolsRail.mcpTitle',
    className: 'bg-indigo-500/10 hover:bg-indigo-500/20 text-indigo-300 border-indigo-500/30',
    Icon: Cpu,
  },
  {
    id: 'hxd',
    labelKey: 'pico.launchHxd',
    titleKey: 'pico.toolsRail.hxdTitle',
    className: 'bg-slate-800 hover:bg-slate-700 text-slate-200 border-slate-700',
    Icon: Binary,
  },
  {
    id: 'terminal',
    labelKey: 'pico.launchTerminal',
    titleKey: 'pico.toolsRail.terminalTitle',
    className: 'bg-slate-800 hover:bg-slate-700 text-slate-300 border-slate-700',
    Icon: Terminal,
  },
];

export const GdbExternalToolLaunchers: React.FC<GdbExternalToolLaunchersProps> = ({
  onLaunchTool,
  isAndroidHost = false,
}) => {
  const { t } = useTranslation();
  const [activeTool, setActiveTool] = useState<string | null>(null);
  const resetTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const { scheduleTimeout, cancelTimeout } = useManagedTimeouts();
  const androidHost = Boolean(isAndroidHost);

  if (androidHost) {
    return (
      <div className="bg-slate-950/60 p-3 rounded-xl border border-slate-800/80 text-xs font-mono text-slate-400">
        {t('pico.toolsRail.androidDesktopOnly')}
      </div>
    );
  }

  const handleLaunch = async (tool: ExternalToolName) => {
    if (!onLaunchTool) return;
    setActiveTool(tool);
    try {
      await onLaunchTool(tool);
    } finally {
      cancelTimeout(resetTimerRef.current);
      resetTimerRef.current = scheduleTimeout(() => {
        resetTimerRef.current = null;
        setActiveTool(null);
      }, 1200);
    }
  };

  return (
    <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 bg-slate-950/60 p-3 rounded-xl border border-slate-800/80">
      <span className="text-xs font-semibold text-slate-300">
        {t('pico.toolsRail.sectionTitle')}
      </span>
      <div className="flex items-center gap-2 flex-wrap">
        {TOOL_LAUNCHERS.length === 0 ? (
          <span className="text-xs text-slate-500">{t('pico.toolsRail.emptyTools')}</span>
        ) : (
          TOOL_LAUNCHERS.map(({ id, labelKey, titleKey, className, Icon }) => (
            <Button
              key={id}
              type="button"
              variant="outline"
              size="sm"
              onClick={() => handleLaunch(id)}
              disabled={activeTool === id}
              title={t(titleKey, { port: GDB_DEFAULT_TCP_PORT })}
              className={`font-mono font-semibold text-xs gap-1.5 h-8 ${className}`}
            >
              <Icon className="w-3.5 h-3.5" />
              <span>{t(labelKey)}</span>
            </Button>
          ))
        )}
      </div>
    </div>
  );
};
