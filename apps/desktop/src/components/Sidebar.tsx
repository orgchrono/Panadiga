import React from 'react';
import {
  Activity,
  Binary,
  BookOpen,
  Cpu,
  HardDrive,
  Layers,
  LayoutDashboard,
  Lock,
  Radio,
  Settings,
  ShieldAlert,
  Smartphone,
} from 'lucide-react';
import { Badge, Button, Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui';
import { isAndroidCommanderEnabled } from '@/config/platform';
import { PICO_BROWNOUT_FLOOR_VOLTS } from '@/constants';
import type { TranslationKey } from '@/i18n/translations';
import { useTranslation } from '@/i18n/useTranslation';
import { cn } from '@/lib/utils';
import type { ViewKey } from '@/models';
import { isTargetPowered } from '@/utils/formatters';

export type { ViewKey };

export interface SidebarProps {
  currentView: ViewKey;
  onSelectView: (view: ViewKey) => void;
  vddVolts?: number;
  minVddThreshold?: number;
  hardwareConnected?: boolean;
  nvramCleaned?: boolean;
  indexedManualsCount?: number | null;
  flashSizeMb?: number;
}

type BadgeVariant = 'default' | 'secondary' | 'success' | 'warning' | 'destructive' | 'outline';

interface NavItem {
  readonly key: ViewKey;
  readonly label: string;
  readonly sublabel: string;
  readonly icon: React.ReactNode;
  readonly badge?: string;
  readonly badgeVariant?: BadgeVariant;
}

interface NavDomainGroup {
  readonly id: string;
  readonly label: string;
  readonly items: readonly NavItem[];
}

function buildNavGroups(
  t: (key: TranslationKey, options?: Record<string, unknown>) => string,
  vddVolts: number,
  powered: boolean,
  nvramCleaned: boolean,
  indexedManualsCount: number | null,
): readonly NavDomainGroup[] {
  const showIr = (import.meta.env.MODE === 'android' || !import.meta.env.PROD) && isAndroidCommanderEnabled();
  return [
    {
      id: 'storage',
      label: t('nav.groups.storage'),
      items: [
        { key: 'dashboard', label: t('nav.dashboard'), sublabel: t('nav.dashboard.sub'), icon: <LayoutDashboard className="w-3.5 h-3.5" /> },
        { key: 'hdd', label: t('nav.hdd'), sublabel: t('nav.hdd.sub'), icon: <HardDrive className="w-3.5 h-3.5" />, badge: '100 MB', badgeVariant: 'default' },
      ],
    },
    {
      id: 'nvram',
      label: t('nav.groups.nvram'),
      items: [
        { key: 'nvram', label: t('nav.nvram'), sublabel: t('nav.nvram.sub'), icon: <Cpu className="w-3.5 h-3.5" />, badge: nvramCleaned ? t('nav.nvram.badgeClean') : 'U99', badgeVariant: nvramCleaned ? 'success' : 'destructive' },
      ],
    },
    {
      id: 'sdCrypto',
      label: t('nav.groups.sdCrypto'),
      items: [
        { key: 'unbrick', label: t('nav.unbrick'), sublabel: t('nav.unbrick.sub'), icon: <ShieldAlert className="w-3.5 h-3.5" />, badge: 'UNBRICK', badgeVariant: 'success' },
        { key: 'frmcrypto', label: t('nav.frmcrypto'), sublabel: t('nav.frmcrypto.sub'), icon: <Lock className="w-3.5 h-3.5" />, badge: '7.999b', badgeVariant: 'secondary' },
      ],
    },
    {
      id: 'picoProbe',
      label: t('nav.groups.picoProbe'),
      items: [
        { key: 'picobench', label: t('nav.picobench'), sublabel: t('nav.picobench.sub'), icon: <Radio className="w-3.5 h-3.5" />, badge: `${vddVolts.toFixed(1)}V`, badgeVariant: powered ? 'success' : 'warning' },
        ...(showIr ? [{ key: 'ircommander' as const, label: t('nav.ircommander'), sublabel: t('nav.ircommander.sub'), icon: <Smartphone className="w-3.5 h-3.5" />, badge: 'IR APK', badgeVariant: 'default' as const }] : []),
      ],
    },
    {
      id: 'forensicsDocs',
      label: t('nav.groups.forensicsDocs'),
      items: [
        { key: 'hexinspector', label: t('nav.hexinspector'), sublabel: t('nav.hexinspector.sub'), icon: <Binary className="w-3.5 h-3.5" /> },
        { key: 'manuals', label: t('nav.manuals'), sublabel: t('nav.manuals.subDynamic', { count: String(indexedManualsCount ?? '--') }), icon: <BookOpen className="w-3.5 h-3.5" />, badge: `${indexedManualsCount ?? '--'}`, badgeVariant: 'secondary' },
        { key: 'settings', label: t('nav.settings'), sublabel: t('nav.settings.sub'), icon: <Settings className="w-3.5 h-3.5" /> },
      ],
    },
  ];
}

function getGroupStepOffset(groups: readonly NavDomainGroup[], groupIdx: number): number {
  return groups.slice(0, groupIdx).reduce((sum, group) => sum + group.items.length, 0);
}

const NavTabButton: React.FC<{
  readonly item: NavItem;
  readonly stepIndex: number;
  readonly isActive: boolean;
  readonly layout: 'horizontal' | 'vertical';
  readonly onSelect: () => void;
}> = ({ item, stepIndex, isActive, layout, onSelect }) => (
  <Button
    type="button"
    variant="ghost"
    data-testid={`nav-${item.key}`}
    onClick={onSelect}
    aria-current={isActive ? 'page' : undefined}
    className={cn(
      'border transition-colors font-mono',
      layout === 'horizontal'
        ? 'h-8 px-2.5 py-1 rounded-lg gap-1.5 shrink-0 whitespace-nowrap'
        : 'w-full h-auto justify-between px-3 py-2.5 rounded-xl text-left',
      isActive
        ? 'bg-cyan-500/10 text-cyan-300 border-cyan-500/40 hover:bg-cyan-500/15 hover:text-cyan-200 shadow-sm shadow-cyan-950/30'
        : 'text-slate-400 hover:text-slate-100 hover:bg-slate-900 border-transparent',
    )}
  >
    <div className="flex items-center gap-2 min-w-0">
      <span className="text-[10px] text-slate-500 font-bold shrink-0">
        {String(stepIndex + 1).padStart(2, '0')}
      </span>
      <span className={cn(isActive ? 'text-cyan-400' : 'text-slate-500', 'shrink-0')}>
        {item.icon}
      </span>
      <div className="truncate">
        <div className="text-xs font-semibold leading-tight">{item.label}</div>
        {layout === 'vertical' && (
          <div className="text-[10px] text-slate-500 truncate leading-tight font-sans">
            {item.sublabel}
          </div>
        )}
      </div>
    </div>
    {item.badge && (
      <Badge
        variant={item.badgeVariant}
        className={cn('text-[10px] px-1.5 py-0 leading-4 shrink-0', layout === 'vertical' ? 'ml-2' : 'ml-0.5')}
      >
        {item.badge}
      </Badge>
    )}
  </Button>
);

export const Sidebar: React.FC<SidebarProps> = ({
  currentView,
  onSelectView,
  vddVolts = 0.0,
  minVddThreshold,
  nvramCleaned = false,
  indexedManualsCount = null,
}) => {
  const { t } = useTranslation();
  const powered = isTargetPowered(vddVolts, minVddThreshold);
  const navGroups = buildNavGroups(t, vddVolts, powered, nvramCleaned, indexedManualsCount);

  return (
    <div className="hidden lg:block w-full bg-bench-950/95 border-b border-slate-800 px-3 py-1.5 select-none shrink-0">
      <div className="flex items-center gap-3">
        <div className="flex items-center gap-2 pr-3 border-r border-slate-800 shrink-0">
          <div className="w-7 h-7 rounded-lg bg-gradient-to-br from-cyan-500 to-blue-600 flex items-center justify-center shadow shadow-cyan-950 text-white">
            <Layers className="w-4 h-4" />
          </div>
          <h1 className="text-xs font-bold tracking-tight text-slate-100 font-mono whitespace-nowrap">
            DIGA SUITE
          </h1>
        </div>
        <nav
          aria-label={t('nav.menuAria')}
          className="flex items-center gap-1.5 overflow-x-auto scrollbar-thin scrollbar-thumb-slate-800 flex-1 min-w-0 py-0.5"
        >
          {navGroups.length === 0 ? (
            <span className="text-xs text-slate-500 px-2">{t('nav.empty')}</span>
          ) : (
            navGroups.map((group, groupIdx) => {
              const offset = getGroupStepOffset(navGroups, groupIdx);
              return (
                <fieldset
                  key={group.id}
                  data-testid={`nav-group-${group.id}`}
                  aria-label={group.label}
                  className="flex items-center gap-1 pr-2 mr-1 m-0 p-0 border-0 border-r border-slate-800/80 last:border-r-0 last:pr-0 last:mr-0 shrink-0"
                >
                  <span className="text-[9px] font-mono uppercase tracking-wider text-slate-500 px-1 shrink-0">
                    {group.label}
                  </span>
                  {group.items.map((item, idx) => (
                    <NavTabButton
                      key={item.key}
                      item={item}
                      stepIndex={offset + idx}
                      isActive={currentView === item.key}
                      layout="horizontal"
                      onSelect={() => onSelectView(item.key)}
                    />
                  ))}
                </fieldset>
              );
            })
          )}
        </nav>
      </div>
    </div>
  );
};

export interface MobileSidebarDrawerProps extends SidebarProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export const MobileSidebarDrawer: React.FC<MobileSidebarDrawerProps> = ({
  open,
  onOpenChange,
  currentView,
  onSelectView,
  vddVolts = 0.0,
  minVddThreshold,
  hardwareConnected = false,
  nvramCleaned = false,
  indexedManualsCount = null,
  flashSizeMb = 8,
}) => {
  const { t } = useTranslation();
  const powered = isTargetPowered(vddVolts, minVddThreshold);
  const navGroups = buildNavGroups(t, vddVolts, powered, nvramCleaned, indexedManualsCount);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="left-0 top-0 translate-x-0 translate-y-0 h-full w-72 max-w-[85vw] p-0 bg-bench-950 border-r border-slate-800 rounded-none flex flex-col">
        <DialogHeader className="sr-only">
          <DialogTitle>{t('nav.drawerTitle')}</DialogTitle>
        </DialogHeader>
        <div className="flex-1 overflow-y-auto pt-2 flex flex-col justify-between">
          <div>
            <div className="p-4 border-b border-slate-800/80 bg-bench-900/40 flex items-center gap-2.5">
              <div className="w-8 h-8 rounded-xl bg-gradient-to-br from-cyan-500 to-blue-600 flex items-center justify-center text-white">
                <Layers className="w-4 h-4" />
              </div>
              <span className="text-sm font-bold tracking-tight text-slate-100 font-mono">
                DIGA SUITE
              </span>
            </div>
            <nav className="p-3 space-y-3" aria-label={t('nav.menuAria')}>
              {navGroups.map((group, groupIdx) => {
                const offset = getGroupStepOffset(navGroups, groupIdx);
                return (
                  <div key={group.id} className="space-y-1">
                    <div className="px-2 text-[10px] font-mono uppercase tracking-wider text-slate-500">
                      {group.label}
                    </div>
                    {group.items.map((item, idx) => (
                      <NavTabButton
                        key={item.key}
                        item={item}
                        stepIndex={offset + idx}
                        isActive={currentView === item.key}
                        layout="vertical"
                        onSelect={() => {
                          onSelectView(item.key);
                          onOpenChange(false);
                        }}
                      />
                    ))}
                  </div>
                );
              })}
            </nav>
          </div>

          <div className="p-3 border-t border-slate-800/80 bg-bench-900/50">
            <div className="p-2.5 rounded-xl bg-slate-950 border border-slate-800/80 space-y-1.5 font-mono text-[11px]">
              <div className="flex items-center justify-between text-slate-400">
                <span className="flex items-center gap-1.5">
                  <Activity className="w-3.5 h-3.5 text-cyan-400" />
                  <span>{t('shell.targetRailLabel')}</span>
                </span>
                <span className="text-slate-200 font-bold">
                  {vddVolts > PICO_BROWNOUT_FLOOR_VOLTS ? `${vddVolts.toFixed(2)} V` : t('shell.targetRailZero')}
                </span>
              </div>
              <div className="flex items-center justify-between text-slate-400">
                <span>AM34</span>
                <span className={hardwareConnected && powered ? 'text-emerald-400' : 'text-amber-400'}>
                  {hardwareConnected && powered ? t('shell.coreHalted') : t('shell.coreStandby')}
                </span>
              </div>
              <div className="flex items-center justify-between text-slate-400">
                <span>{t('shell.spansionNorLabel')}</span>
                <span className="text-slate-200">{flashSizeMb} MB (TSOP48)</span>
              </div>
            </div>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
};
