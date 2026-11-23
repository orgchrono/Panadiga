import React from 'react';
import {
  Activity,
  Cpu,
  Layers,
  Menu,
  PanelBottom,
  PanelLeft,
  PanelRight,
  RefreshCw,
  Settings,
  ShieldCheck,
} from 'lucide-react';
import { Button } from '@/components/ui';
import { PICO_BROWNOUT_FLOOR_VOLTS } from '@/constants';
import { useTranslation } from '@/i18n/useTranslation';
import type { ViewKey, WorkbenchExperienceMode } from '@/models';
import type { WorkbenchDocksState } from '@/types/workbench';
import { isTargetPowered } from '@/utils/formatters';
import { StatusBadge } from './StatusBadge';

export interface HeaderProps {
  currentView?: ViewKey;
  currentViewTitle: string;
  mode?: WorkbenchExperienceMode;
  onToggleMode?: () => void;
  onRefresh?: () => void;
  onOpenMobileNav?: () => void;
  onOpenSettings?: () => void;
  vddVolts?: number;
  minVddThreshold?: number;
  hardwareConnected?: boolean;
  docks?: WorkbenchDocksState;
  onToggleLeftDock?: () => void;
  onToggleBottomDock?: () => void;
  onToggleRightDock?: () => void;
  onOpenCompliance?: () => void;
}

export const Header: React.FC<HeaderProps> = ({
  currentView,
  currentViewTitle,
  mode = 'engineering',
  onToggleMode,
  onRefresh,
  onOpenMobileNav,
  onOpenSettings,
  vddVolts = 0.0,
  minVddThreshold,
  hardwareConnected = false,
  docks,
  onToggleLeftDock,
  onToggleBottomDock,
  onToggleRightDock,
  onOpenCompliance,
}) => {
  const { t } = useTranslation();
  const powered = isTargetPowered(vddVolts, minVddThreshold);

  const dockToggles: ReadonlyArray<{
    readonly key: string;
    readonly testId: string;
    readonly handler: (() => void) | undefined;
    readonly active: boolean;
    readonly label: string;
    readonly Icon: typeof PanelLeft;
  }> = [
    {
      key: 'left',
      testId: 'header-left-dock-btn',
      handler: onToggleLeftDock,
      active: Boolean(docks?.leftOpen),
      label: t('shell.statusBar.leftDockBtn'),
      Icon: PanelLeft,
    },
    {
      key: 'bottom',
      testId: 'header-bottom-dock-btn',
      handler: onToggleBottomDock,
      active: Boolean(docks?.bottomOpen),
      label: t('shell.statusBar.bottomDockBtn'),
      Icon: PanelBottom,
    },
    {
      key: 'right',
      testId: 'header-right-dock-btn',
      handler: onToggleRightDock,
      active: Boolean(docks?.rightOpen),
      label: t('shell.statusBar.rightDockBtn'),
      Icon: PanelRight,
    },
  ];

  return (
    <header
      data-testid="app-header"
      data-view={currentView}
      className="min-h-14 px-4 sm:px-6 py-2 bg-bench-900/90 backdrop-blur border-b border-slate-800 flex items-center justify-between gap-2 select-none z-10 shrink-0"
    >
      <div className="flex items-center gap-2.5 min-w-0">
        {onOpenMobileNav && (
          <Button
            type="button"
            variant="outline"
            size="icon"
            data-testid="header-mobile-nav-btn"
            onClick={onOpenMobileNav}
            className="lg:hidden h-9 w-9 shrink-0 bg-slate-900 border-slate-700"
            aria-label={t('shell.openMainNav')}
          >
            <Menu className="w-4 h-4 text-cyan-400" />
          </Button>
        )}

        <div className="flex items-center gap-2 min-w-0">
          <div className="hidden sm:flex w-8 h-8 rounded-lg bg-cyan-500/10 border border-cyan-500/30 items-center justify-center text-cyan-400 shrink-0">
            <Cpu className="w-4 h-4" />
          </div>
          <div className="min-w-0">
            <div className="flex items-center gap-2 min-w-0">
              <span className="text-xs font-mono font-medium text-slate-400 uppercase tracking-wider shrink-0">
                PANADIGA // DMR-EH55
              </span>
              <span className="text-slate-600 hidden sm:inline">/</span>
              <span
                data-testid="header-view-title"
                className="text-xs sm:text-sm font-semibold text-slate-100 font-mono truncate"
              >
                {currentViewTitle}
              </span>
            </div>
            <p className="hidden md:block text-[11px] text-slate-500 font-mono truncate">
              {t('shell.hardwareSubtitle')}
            </p>
          </div>
        </div>
      </div>

      <div className="flex items-center gap-1.5 sm:gap-2 shrink-0">
        <div className="hidden md:flex items-center gap-2 px-3 py-1.5 rounded-lg bg-bench-950 border border-slate-800 font-mono text-xs">
          <Activity className="w-3.5 h-3.5 text-cyan-400 animate-pulse" />
          <span className="text-slate-400">P7501 UART:</span>
          {hardwareConnected ? (
            powered ? (
              <span className="text-emerald-400 font-semibold flex items-center gap-1">
                ONLINE <span className="text-[10px] text-slate-500">({vddVolts.toFixed(2)}V)</span>
              </span>
            ) : (
              <span className="text-amber-400 font-semibold flex items-center gap-1">
                {t('shell.picoConnected')}{' '}
                <span className="text-[10px] text-slate-500">
                  (
                  {vddVolts > PICO_BROWNOUT_FLOOR_VOLTS
                    ? `${vddVolts.toFixed(2)}V`
                    : t('shell.targetUnpowered')}
                  )
                </span>
              </span>
            )
          ) : (
            <span className="text-rose-400 font-semibold">OFFLINE</span>
          )}
        </div>

        <div className="hidden md:flex items-center gap-1">
          {dockToggles.length === 0 ? null : dockToggles.map(({ key, testId, handler, active, label, Icon }) =>
            handler ? (
              <Button
                key={key}
                type="button"
                variant={active ? 'secondary' : 'outline'}
                size="icon"
                data-testid={testId}
                onClick={handler}
                aria-pressed={active}
                title={label}
                aria-label={label}
                className="h-8 w-8 bg-bench-950 border-slate-700"
              >
                <Icon className={active ? 'w-3.5 h-3.5 text-cyan-400' : 'w-3.5 h-3.5 text-slate-400'} />
              </Button>
            ) : null,
          )}
        </div>

        {onOpenCompliance && (
          <Button
            type="button"
            variant="outline"
            size="icon"
            data-testid="header-compliance-btn"
            onClick={onOpenCompliance}
            title={t('shell.statusBar.complianceBtn')}
            aria-label={t('shell.statusBar.complianceBtn')}
            className="hidden sm:inline-flex h-8 w-8 bg-bench-950 border-slate-700"
          >
            <ShieldCheck className="w-3.5 h-3.5 text-emerald-400" />
          </Button>
        )}

        {onToggleMode && (
          <Button
            type="button"
            variant="outline"
            size="sm"
            data-testid="header-mode-toggle"
            onClick={onToggleMode}
            title={t('shell.modeTooltip')}
            aria-label={mode === 'express' ? t('shell.modeBasic') : t('shell.modeAdvanced')}
            className="inline-flex h-8 px-2 sm:px-2.5 text-[11px] font-mono gap-1.5 bg-bench-950 border-slate-700"
          >
            <Layers className="w-3.5 h-3.5 text-cyan-400" />
            <span className="hidden sm:inline">{mode === 'express' ? t('shell.modeBasic') : t('shell.modeAdvanced')}</span>
          </Button>
        )}

        {onOpenSettings && (
          <Button
            type="button"
            variant="outline"
            size="icon"
            data-testid="header-settings-btn"
            onClick={onOpenSettings}
            className="h-8 w-8 bg-bench-950 border-slate-700"
            title={t('shell.openSettings')}
            aria-label={t('shell.openSettings')}
          >
            <Settings className="w-3.5 h-3.5 text-slate-300" />
          </Button>
        )}

        <StatusBadge
          variant="warning"
          label={t('shell.suiteBadge')}
          className="hidden xl:inline-flex"
        />

        {onRefresh && (
          <Button
            type="button"
            variant="outline"
            size="icon"
            onClick={onRefresh}
            className="h-8 w-8 bg-slate-800/80 border-slate-700"
            title={t('shell.refreshTelemetry')}
            aria-label={t('shell.refreshTelemetry')}
          >
            <RefreshCw className="w-3.5 h-3.5" />
          </Button>
        )}
      </div>
    </header>
  );
};
