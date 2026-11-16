import React from 'react';
import { motion } from 'framer-motion';
import {
  CheckCircle2,
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
import type { ViewKey } from '@/models';
import { useWorkbenchMode } from '@/context/WorkbenchModeContext';
import { cn } from '@/lib/utils';
import { Button } from './button';

export interface GuidedPipelineBarProps {
  readonly currentView: ViewKey;
  readonly onNavigate: (view: ViewKey) => void;
}

const PIPELINE_STEPS: ReadonlyArray<{
  readonly step: number;
  readonly view: ViewKey;
  readonly title: string;
  readonly hint: string;
  readonly Icon: React.ComponentType<{ className?: string }>;
  readonly flag: 'picoReady' | 'nvramCleaned' | 'hddInspected' | 'unbrickReady';
}> = [
  {
    step: 1,
    view: 'picobench',
    title: '1. Sonda P7501 UART',
    hint: 'Conectar & VDD 3.3V',
    Icon: Radio,
    flag: 'picoReady',
  },
  {
    step: 2,
    view: 'nvram',
    title: '2. Cirurgia NVRAM U99',
    hint: 'Limpar SA134..SA141',
    Icon: Cpu,
    flag: 'nvramCleaned',
  },
  {
    step: 3,
    view: 'hdd',
    title: '3. Forense & Capacidade HDD',
    hint: 'MEIHDFS & LBA48',
    Icon: HardDrive,
    flag: 'hddInspected',
  },
  {
    step: 4,
    view: 'unbrick',
    title: '4. Unbrick SD & FRM',
    hint: 'DIGADIAG / TBC',
    Icon: Lock,
    flag: 'unbrickReady',
  },
];

export const GuidedPipelineBar: React.FC<GuidedPipelineBarProps> = ({
  currentView,
  onNavigate,
}) => {
  const { mode, setMode, hudOpen, toggleHud, pipeline } = useWorkbenchMode();
  const completedCount = PIPELINE_STEPS.filter(item => pipeline[item.flag]).length;

  return (
    <section
      aria-label="Pipeline Guiado de Recuperação Panadiga"
      className="mb-4 rounded-2xl border border-slate-800/90 bg-bench-900/75 px-3.5 py-2.5 shadow-lg font-mono"
    >
      <div className="flex flex-wrap items-center justify-between gap-2 pb-2 mb-2 border-b border-slate-800/80">
        <div className="flex items-center gap-2 text-[11px] text-slate-300">
          <Sparkles className="w-3.5 h-3.5 text-cyan-400 shrink-0" />
          <span className="font-bold">Pipeline Operacional</span>
          <span className="text-slate-500">
            ({completedCount}/{PIPELINE_STEPS.length} etapas concluídas)
          </span>
        </div>

        <div className="flex flex-wrap items-center gap-1.5">
          <div
            role="group"
            aria-label="Alternar complexidade da interface"
            className="inline-flex rounded-lg bg-slate-950 border border-slate-800 p-0.5 text-[11px]"
          >
            <button
              type="button"
              onClick={() => setMode('basic')}
              aria-pressed={mode === 'basic'}
              className={cn(
                'px-2.5 py-1 rounded-md transition-colors',
                mode === 'basic'
                  ? 'bg-emerald-500/20 text-emerald-300 font-bold'
                  : 'text-slate-400 hover:text-slate-200'
              )}
            >
              Básico (Guiado)
            </button>
            <button
              type="button"
              onClick={() => setMode('advanced')}
              aria-pressed={mode === 'advanced'}
              className={cn(
                'px-2.5 py-1 rounded-md transition-colors',
                mode === 'advanced'
                  ? 'bg-cyan-500/20 text-cyan-300 font-bold'
                  : 'text-slate-400 hover:text-slate-200'
              )}
            >
              Avançado (Engenharia)
            </button>
          </div>

          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={toggleHud}
            aria-pressed={hudOpen}
            className="h-7 px-2.5 text-[11px] gap-1.5 border-slate-700"
          >
            <SlidersHorizontal className="w-3 h-3 text-cyan-400" />
            <span>{hudOpen ? 'Ocultar HUD' : 'HUD Pinout'}</span>
          </Button>
        </div>
      </div>

      {PIPELINE_STEPS.length === 0 ? (
        <span className="text-xs text-slate-500">Nenhuma etapa de pipeline definida.</span>
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-2">
          {PIPELINE_STEPS.map(item => {
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
                  active
                    ? 'bg-cyan-500/10 border-cyan-500/50 text-cyan-200'
                    : done
                      ? 'bg-emerald-500/5 border-emerald-500/30 text-emerald-200 hover:bg-emerald-500/10'
                      : 'bg-slate-950/70 border-slate-800 text-slate-300 hover:border-slate-700'
                )}
              >
                <div
                  className={cn(
                    'w-7 h-7 rounded-lg flex items-center justify-center shrink-0 border',
                    done
                      ? 'bg-emerald-500/20 border-emerald-500/40 text-emerald-300'
                      : active
                        ? 'bg-cyan-500/20 border-cyan-500/40 text-cyan-300'
                        : 'bg-slate-900 border-slate-800 text-slate-400'
                  )}
                >
                  {done ? <CheckCircle2 className="w-4 h-4" /> : <Icon className="w-4 h-4" />}
                </div>
                <div className="min-w-0 flex-1">
                  <div className="text-[11px] font-bold truncate">{item.title}</div>
                  <div className="text-[10px] text-slate-400 truncate">{item.hint}</div>
                </div>
              </motion.button>
            );
          })}
        </div>
      )}
    </section>
  );
};

export interface DraggableBenchHudProps {
  readonly vddVolts: number;
  readonly hardwareConnected: boolean;
  readonly onNavigate: (view: ViewKey) => void;
}

const HUD_PINS: ReadonlyArray<{ readonly pin: string; readonly note: string; readonly warn?: boolean }> = [
  { pin: 'Pin 1 • SW_UART (GP26)', note: 'ADC0 / Open-Drain LOW boot' },
  { pin: 'Pin 2 • XRESET (GP5)', note: 'Estrito Open-Drain (15ms)' },
  { pin: 'Pin 3 • UARTM2P (GP2)', note: 'AM34 TX → Pico RX (115200)' },
  { pin: 'Pin 4 • GND', note: 'Referência comum obrigatória' },
  { pin: 'Pin 5 • NSW5V (GP3)', note: 'Hi-Z Input SEMPRE (5.15V!)', warn: true },
  { pin: 'Pin 6 • UARTP2M (GP4)', note: 'Pico TX → AM34 RX (3.3V)' },
];

export const DraggableBenchHud: React.FC<DraggableBenchHudProps> = ({
  vddVolts,
  hardwareConnected,
  onNavigate,
}) => {
  const { hudOpen, setHudOpen } = useWorkbenchMode();
  if (!hudOpen) return null;

  return (
    <motion.aside
      drag
      dragMomentum={false}
      initial={{ opacity: 0, scale: 0.96, y: 12 }}
      animate={{ opacity: 1, scale: 1, y: 0 }}
      aria-label="HUD Flutuante Arrastável de Pinout P7501"
      className="fixed bottom-5 left-5 z-40 w-80 max-w-[calc(100vw-2.5rem)] rounded-2xl border border-cyan-500/40 bg-slate-950/95 shadow-2xl backdrop-blur-md font-mono overflow-hidden"
    >
      <div className="flex items-center justify-between px-3 py-2 bg-bench-900 border-b border-slate-800 cursor-grab active:cursor-grabbing select-none">
        <div className="flex items-center gap-2 text-[11px] font-bold text-cyan-300">
          <GripHorizontal className="w-4 h-4 text-slate-400" />
          <span>HUD P7501 (Arraste livre)</span>
        </div>
        <button
          type="button"
          aria-label="Fechar HUD Flutuante"
          onClick={() => setHudOpen(false)}
          className="p-1 rounded-md text-slate-400 hover:text-slate-100 hover:bg-slate-800"
        >
          <X className="w-3.5 h-3.5" />
        </button>
      </div>

      <div className="p-3 space-y-2.5 text-[11px]">
        <div className="flex items-center justify-between px-2.5 py-1.5 rounded-lg bg-bench-900 border border-slate-800">
          <span className="text-slate-400">
            Link: <strong className={hardwareConnected ? 'text-emerald-400' : 'text-rose-400'}>{hardwareConnected ? 'ON' : 'OFF'}</strong>
          </span>
          <span className="text-cyan-300 font-bold">VDD {vddVolts.toFixed(2)}V</span>
        </div>

        <div className="space-y-1">
          {HUD_PINS.map(row => (
            <div
              key={row.pin}
              className={cn(
                'flex items-center justify-between gap-2 px-2 py-1 rounded border text-[10px]',
                row.warn
                  ? 'bg-rose-950/40 border-rose-500/40 text-rose-200'
                  : 'bg-bench-900/60 border-slate-800/80 text-slate-300'
              )}
            >
              <span className="font-bold truncate flex items-center gap-1">
                {row.warn && <ShieldAlert className="w-3 h-3 text-rose-400 shrink-0" />}
                {row.pin}
              </span>
              <span className="text-slate-400 shrink-0">{row.note}</span>
            </div>
          ))}
        </div>

        <div className="flex items-center gap-1.5 pt-1">
          <Button type="button" size="sm" variant="outline" className="h-7 flex-1 text-[10px]" onClick={() => onNavigate('picobench')}>
            PicoBench
          </Button>
          <Button type="button" size="sm" variant="outline" className="h-7 flex-1 text-[10px]" onClick={() => onNavigate('nvram')}>
            NVRAM U99
          </Button>
        </div>
      </div>
    </motion.aside>
  );
};
