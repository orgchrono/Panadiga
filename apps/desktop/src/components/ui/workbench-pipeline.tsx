import React from 'react';
import { motion } from 'framer-motion';
import {
  CheckCircle2,
  Compass,
  Cpu,
  GripHorizontal,
  HardDrive,
  Lock,
  Radio,
  ShieldAlert,
  SlidersHorizontal,
  Sparkles,
  X,
} from 'lucide-react';
import type { TranslationKey } from '@/i18n/translations';
import { useTranslation } from '@/i18n/useTranslation';
import { cn } from '@/lib/utils';
import type { ViewKey, WorkbenchExperienceMode, WorkbenchPipelineState } from '@/models';
import { isTargetPowered } from '@/utils/formatters';
import { Badge } from './badge';
import { Button } from './button';

export interface GuidedPipelineBarProps {
  readonly currentView: ViewKey;
  readonly onNavigate: (view: ViewKey) => void;
  readonly mode: WorkbenchExperienceMode;
  readonly onModeChange: (mode: WorkbenchExperienceMode) => void;
  readonly hudOpen: boolean;
  readonly onToggleHud: () => void;
  readonly pipeline: WorkbenchPipelineState;
  readonly hardwareConnected?: boolean;
  readonly vddVolts?: number;
  readonly runningCommandId?: string | null;
}

const VIEW_GUIDANCE_HINT_MAP: Partial<Record<ViewKey, TranslationKey>> = {
  dashboard: 'pipeline.guidance.hintDashboard',
  picobench: 'pipeline.guidance.hintPico',
  ssotstudio: 'pipeline.guidance.hintSsotStudio',
  siliconexplorer: 'pipeline.guidance.hintSiliconExplorer',
  nvram: 'pipeline.guidance.hintNvram',
  hdd: 'pipeline.guidance.hintHdd',
  frmcrypto: 'pipeline.guidance.hintFrm',
  unbrick: 'pipeline.guidance.hintUnbrick',
  hexinspector: 'pipeline.guidance.hintHex',
  manuals: 'pipeline.guidance.hintManuals',
  settings: 'pipeline.guidance.hintSettings',
  ...(import.meta.env.MODE === 'android' || !import.meta.env.PROD
    ? { ircommander: 'pipeline.guidance.hintIr' as const }
    : {}),
};

const PIPELINE_STEPS: ReadonlyArray<{
  readonly view: ViewKey;
  readonly titleKey: TranslationKey;
  readonly hintKey: TranslationKey;
  readonly Icon: React.ComponentType<{ className?: string }>;
  readonly flag: 'picoReady' | 'nvramCleaned' | 'hddInspected' | 'unbrickReady';
}> = [
  { view: 'picobench', titleKey: 'pipeline.steps.picoTitle', hintKey: 'pipeline.steps.picoHint', Icon: Radio, flag: 'picoReady' },
  { view: 'nvram', titleKey: 'pipeline.steps.nvramTitle', hintKey: 'pipeline.steps.nvramHint', Icon: Cpu, flag: 'nvramCleaned' },
  { view: 'hdd', titleKey: 'pipeline.steps.hddTitle', hintKey: 'pipeline.steps.hddHint', Icon: HardDrive, flag: 'hddInspected' },
  { view: 'unbrick', titleKey: 'pipeline.steps.unbrickTitle', hintKey: 'pipeline.steps.unbrickHint', Icon: Lock, flag: 'unbrickReady' },
];

export const GuidedPipelineBar: React.FC<GuidedPipelineBarProps> = ({
  currentView, onNavigate, mode, onModeChange, hudOpen, onToggleHud, pipeline,
  hardwareConnected = false, vddVolts = 0, runningCommandId = null,
}) => {
  const { t } = useTranslation();
  const completedCount = PIPELINE_STEPS.filter((item) => pipeline[item.flag]).length;
  const powered = isTargetPowered(vddVolts);
  const statusBadge = runningCommandId
    ? { label: t('pipeline.guidance.running', { id: runningCommandId }), variant: 'default' as const }
    : powered
      ? { label: t('pipeline.guidance.active'), variant: 'success' as const }
      : hardwareConnected
        ? { label: t('pipeline.guidance.standby'), variant: 'warning' as const }
        : { label: t('pipeline.guidance.offline'), variant: 'secondary' as const };

  return (
    <section aria-label={t('pipeline.sectionAria')} className="mb-4 rounded-2xl border border-slate-800/90 bg-bench-900/75 px-3.5 py-2.5 shadow-lg font-mono">
      <div className="flex flex-wrap items-center justify-between gap-2 pb-2 mb-2 border-b border-slate-800/80">
        <div className="flex items-center gap-2 text-[11px] text-slate-300">
          <Sparkles className="w-3.5 h-3.5 text-cyan-400 shrink-0" />
          <span className="font-bold">{t('pipeline.title')}</span>
          <span className="text-slate-500">{t('pipeline.progress', { done: completedCount, total: PIPELINE_STEPS.length })}</span>
        </div>
        <div className="flex flex-wrap items-center gap-1.5">
          <fieldset aria-label={t('pipeline.modeGroupAria')} className="inline-flex rounded-lg bg-slate-950 border border-slate-800 p-0.5 m-0 text-[11px]">
            <button type="button" onClick={() => onModeChange('express')} aria-pressed={mode === 'express'} className={cn('px-2.5 py-1 rounded-md transition-colors', mode === 'express' ? 'bg-emerald-500/20 text-emerald-300 font-bold' : 'text-slate-400 hover:text-slate-200')}>
              {t('pipeline.basicBtn')}
            </button>
            <button type="button" onClick={() => onModeChange('engineering')} aria-pressed={mode === 'engineering'} className={cn('px-2.5 py-1 rounded-md transition-colors', mode === 'engineering' ? 'bg-cyan-500/20 text-cyan-300 font-bold' : 'text-slate-400 hover:text-slate-200')}>
              {t('pipeline.advancedBtn')}
            </button>
          </fieldset>
          <Button type="button" variant="outline" size="sm" onClick={onToggleHud} aria-pressed={hudOpen} className="h-7 px-2.5 text-[11px] gap-1.5 border-slate-700">
            <SlidersHorizontal className="w-3 h-3 text-cyan-400" />
            <span>{hudOpen ? t('pipeline.hudHide') : t('pipeline.hudShow')}</span>
          </Button>
        </div>
      </div>

      {PIPELINE_STEPS.length === 0 ? (
        <span className="text-xs text-slate-500">{t('pipeline.emptySteps')}</span>
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-2">
          {PIPELINE_STEPS.map((item) => {
            const done = pipeline[item.flag];
            const active = currentView === item.view;
            const Icon = item.Icon;
            return (
              <motion.button
                key={item.view}
                type="button"
                whileHover={{ y: -1 }}
                whileTap={{ scale: 0.99 }}
                onClick={() => onNavigate(item.view)}
                className={cn(
                  'flex items-center gap-2.5 rounded-xl border px-3 py-2 text-left transition-colors min-w-0',
                  active ? 'bg-cyan-500/10 border-cyan-500/50 text-cyan-200' : done ? 'bg-emerald-500/5 border-emerald-500/30 text-emerald-200 hover:bg-emerald-500/10' : 'bg-slate-950/70 border-slate-800 text-slate-300 hover:border-slate-700',
                )}
              >
                <div className={cn('w-7 h-7 rounded-lg flex items-center justify-center shrink-0 border', done ? 'bg-emerald-500/20 border-emerald-500/40 text-emerald-300' : active ? 'bg-cyan-500/20 border-cyan-500/40 text-cyan-300' : 'bg-slate-900 border-slate-800 text-slate-400')}>
                  {done ? <CheckCircle2 className="w-4 h-4" /> : <Icon className="w-4 h-4" />}
                </div>
                <div className="min-w-0 flex-1">
                  <div className="text-[11px] font-bold truncate">{t(item.titleKey)}</div>
                  <div className="text-[10px] text-slate-400 truncate">{t(item.hintKey)}</div>
                </div>
              </motion.button>
            );
          })}
        </div>
      )}

      <div data-testid="pipeline-step-guide" className="mt-2 pt-2 border-t border-slate-800/70 flex flex-wrap items-center gap-2 text-[11px]">
        <span className="inline-flex items-center gap-1 text-cyan-300 font-semibold shrink-0">
          <Compass className="w-3.5 h-3.5" />
          <span>{t('pipeline.guidance.title')}:</span>
        </span>
        <Badge variant={statusBadge.variant} className="text-[10px] px-2 py-0">{statusBadge.label}</Badge>
        <span className="text-slate-300 font-sans truncate">{t(VIEW_GUIDANCE_HINT_MAP[currentView] ?? 'pipeline.guidance.hintDashboard')}</span>
      </div>
    </section>
  );
};

export interface DraggableBenchHudProps {
  readonly vddVolts: number;
  readonly hardwareConnected: boolean;
  readonly hudOpen: boolean;
  readonly onCloseHud: () => void;
  readonly onNavigate: (view: ViewKey) => void;
}

const HUD_PINS: ReadonlyArray<{ readonly pin: string; readonly noteKey: TranslationKey; readonly warn?: boolean }> = [
  { pin: 'Pin 1 • SW_UART (GP26)', noteKey: 'hud.pins.pin1' },
  { pin: 'Pin 2 • XRESET (GP5)', noteKey: 'hud.pins.pin2' },
  { pin: 'Pin 3 • UARTM2P (GP4)', noteKey: 'hud.pins.pin3' },
  { pin: 'Pin 4 • GND', noteKey: 'hud.pins.pin4' },
  { pin: 'Pin 5 • NSW5V (GP3)', noteKey: 'hud.pins.pin5', warn: true },
  { pin: 'Pin 6 • UARTP2M (GP2)', noteKey: 'hud.pins.pin6' },
];

export const DraggableBenchHud: React.FC<DraggableBenchHudProps> = ({
  vddVolts, hardwareConnected, hudOpen, onCloseHud, onNavigate,
}) => {
  const { t } = useTranslation();
  if (!hudOpen) return null;

  return (
    <motion.aside
      drag
      dragMomentum={false}
      initial={{ opacity: 0, scale: 0.96, y: 12 }}
      animate={{ opacity: 1, scale: 1, y: 0 }}
      aria-label={t('hud.ariaLabel')}
      className="fixed bottom-5 left-5 z-40 w-80 max-w-[calc(100vw-2.5rem)] rounded-2xl border border-cyan-500/40 bg-slate-950/95 shadow-2xl backdrop-blur-md font-mono overflow-hidden"
    >
      <div className="flex items-center justify-between px-3 py-2 bg-bench-900 border-b border-slate-800 cursor-grab active:cursor-grabbing select-none">
        <div className="flex items-center gap-2 text-[11px] font-bold text-cyan-300">
          <GripHorizontal className="w-4 h-4 text-slate-400" />
          <span>{t('hud.dragTitle')}</span>
        </div>
        <button type="button" aria-label={t('hud.closeAria')} onClick={onCloseHud} className="p-1 rounded-md text-slate-400 hover:text-slate-100 hover:bg-slate-800">
          <X className="w-3.5 h-3.5" />
        </button>
      </div>
      <div className="p-3 space-y-2.5 text-[11px]">
        <div className="flex items-center justify-between px-2.5 py-1.5 rounded-lg bg-bench-900 border border-slate-800">
          <span className="text-slate-400">
            {t('hud.linkLabel')}{' '}
            <strong className={hardwareConnected ? 'text-emerald-400' : 'text-rose-400'}>
              {hardwareConnected ? t('hud.on') : t('hud.off')}
            </strong>
          </span>
          <span className="text-cyan-300 font-bold">{t('hud.vddLabel', { volts: vddVolts.toFixed(2) })}</span>
        </div>
        <div className="space-y-1">
          {HUD_PINS.map((row) => (
            <div key={row.pin} className={cn('flex items-center justify-between gap-2 px-2 py-1 rounded border text-[10px]', row.warn ? 'bg-rose-950/40 border-rose-500/40 text-rose-200' : 'bg-bench-900/60 border-slate-800/80 text-slate-300')}>
              <span className="font-bold truncate flex items-center gap-1">
                {row.warn && <ShieldAlert className="w-3 h-3 text-rose-400 shrink-0" />}
                {row.pin}
              </span>
              <span className="text-slate-400 shrink-0">{t(row.noteKey)}</span>
            </div>
          ))}
        </div>
        <div className="flex items-center gap-1.5 pt-1">
          <Button type="button" size="sm" variant="outline" className="h-7 flex-1 text-[10px]" onClick={() => onNavigate('picobench')}>{t('hud.navPico')}</Button>
          <Button type="button" size="sm" variant="outline" className="h-7 flex-1 text-[10px]" onClick={() => onNavigate('nvram')}>{t('hud.navNvram')}</Button>
        </div>
      </div>
    </motion.aside>
  );
};
