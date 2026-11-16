import React from 'react';
import { Cpu, HardDrive, ShieldCheck, Sparkles, Unlock } from 'lucide-react';
import {
  QUICK_CAPACITIES_GB,
  type CapacityPassthroughMode,
  type PatchMode,
} from './capacityHelpers';

const QUICK_CAPACITIES: readonly number[] = QUICK_CAPACITIES_GB;

export interface CapacityStrategyPanelProps {
  mode: PatchMode;
  onModeChange: (nextMode: PatchMode) => void;
  targetGb: number;
  onTargetGbChange: (gb: number) => void;
  limitInDynamicMode: boolean;
  onLimitInDynamicChange: (checked: boolean) => void;
  passthroughMode?: CapacityPassthroughMode;
  onPassthroughModeChange?: (mode: CapacityPassthroughMode) => void;
  previewBlocks: number;
  patching: boolean;
  onApply: () => void;
}

export const CapacityStrategyPanel: React.FC<CapacityStrategyPanelProps> = ({
  mode,
  onModeChange,
  targetGb,
  onTargetGbChange,
  limitInDynamicMode,
  onLimitInDynamicChange,
  passthroughMode = 'unchanged',
  onPassthroughModeChange,
  previewBlocks,
  patching,
  onApply,
}) => (
  <div className="p-5 rounded-xl bg-slate-950 border border-slate-800 space-y-4">
    <div className="flex items-center gap-2 text-xs font-bold text-slate-200">
      <Cpu className="w-4 h-4 text-cyan-400" />
      <span>Estratégia de Tamanho Dinâmico / Desbloqueio</span>
    </div>

    <div className="grid grid-cols-1 md:grid-cols-3 gap-3 text-xs">
      <button
        type="button"
        onClick={() => onModeChange('dynamic')}
        className={`p-3.5 rounded-xl border text-left transition-all ${
          mode === 'dynamic'
            ? 'bg-emerald-500/10 border-emerald-500/60 text-emerald-200 shadow-lg'
            : 'bg-bench-900 border-slate-800 text-slate-400 hover:border-slate-700'
        }`}
      >
        <div className="font-bold flex items-center gap-1.5">
          <Sparkles className="w-3.5 h-3.5 text-emerald-400" />
          <span>Dinâmico Auto-Detect (LBA48)</span>
        </div>
        <p className="text-[11px] text-slate-400 mt-1">
          Troca <code>bls +12</code> por <code>bra +12</code> (<code>0xCA</code>) e usa 100% da capacidade real do
          HDD instalado (250 GB, 500 GB, 1 TB, 2 TB...).
        </p>
      </button>

      <button
        type="button"
        onClick={() => onModeChange('cap')}
        className={`p-3.5 rounded-xl border text-left transition-all ${
          mode === 'cap'
            ? 'bg-cyan-500/10 border-cyan-500/60 text-cyan-200 shadow-lg'
            : 'bg-bench-900 border-slate-800 text-slate-400 hover:border-slate-700'
        }`}
      >
        <div className="font-bold flex items-center gap-1.5">
          <HardDrive className="w-3.5 h-3.5 text-cyan-400" />
          <span>Teto Explícito / Preset RTSD</span>
        </div>
        <p className="text-[11px] text-slate-400 mt-1">
          Mantém o clamp ativo (<code>0xC6</code>) e crava <code>MaxProductLBA</code> no preset oficial (ex.: 250 GB
          = <code>0x07466528</code>) ou qualquer GB customizado.
        </p>
      </button>

      <button
        type="button"
        onClick={() => onModeChange('factory_200gb')}
        className={`p-3.5 rounded-xl border text-left transition-all ${
          mode === 'factory_200gb'
            ? 'bg-amber-500/10 border-amber-500/60 text-amber-200 shadow-lg'
            : 'bg-bench-900 border-slate-800 text-slate-400 hover:border-slate-700'
        }`}
      >
        <div className="font-bold flex items-center gap-1.5">
          <ShieldCheck className="w-3.5 h-3.5 text-amber-400" />
          <span>Restaurar Fábrica (200 GB)</span>
        </div>
        <p className="text-[11px] text-slate-400 mt-1">
          Reverte para os parâmetros originais da Panasonic (<code>0x0BA43B74</code> / <code>0x05D21DBA</code> +
          clamp <code>0xC6</code>).
        </p>
      </button>
    </div>

    {mode !== 'factory_200gb' && (
      <div className="space-y-3 pt-2 border-t border-slate-800/80">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <span className="text-[11px] text-slate-400 font-semibold uppercase">
            {mode === 'dynamic'
              ? 'Opcional em Dinâmico: calibrar MaxProductLBA para um disco específico ou deixar Livre (~2.2 TB):'
              : 'Selecione o Preset RTSD ou digite a capacidade alvo em GB:'}
          </span>
          {mode === 'dynamic' && (
            <label className="flex items-center gap-2 text-[11px] text-cyan-300 cursor-pointer">
              <input
                type="checkbox"
                checked={limitInDynamicMode}
                onChange={e => onLimitInDynamicChange(e.target.checked)}
                className="rounded border-slate-700 bg-slate-900"
              />
              <span>Gravar também MaxProductLBA do alvo (GB)</span>
            </label>
          )}
        </div>

        {(mode === 'cap' || limitInDynamicMode) && (
          <div className="flex flex-wrap items-center gap-2">
            {QUICK_CAPACITIES.length === 0 ? (
              <span className="text-[11px] text-slate-500">Sem presets de capacidade carregados.</span>
            ) : (
              QUICK_CAPACITIES.map(gb => (
                <button
                  key={gb}
                  type="button"
                  onClick={() => onTargetGbChange(gb)}
                  className={`px-3 py-1.5 rounded-lg text-xs font-bold border transition-colors ${
                    targetGb === gb
                      ? 'bg-cyan-500 text-slate-950 border-cyan-400'
                      : 'bg-bench-900 text-slate-300 border-slate-800 hover:border-slate-700'
                  }`}
                >
                  {gb >= 1000 ? `${gb / 1000} TB (${gb} GB)` : `${gb} GB`}
                  {gb === 250 ? ' ★' : ''}
                </button>
              ))
            )}

            <div className="flex items-center gap-1.5 ml-auto">
              <span className="text-[11px] text-slate-400">Custom (GB):</span>
              <input
                type="number"
                min={20}
                max={2200}
                value={targetGb}
                onChange={e => onTargetGbChange(Math.max(20, Math.min(2200, Number(e.target.value) || 250)))}
                className="w-24 px-2.5 py-1.5 rounded-lg bg-slate-900 border border-slate-700 text-xs text-cyan-300 focus:outline-none focus:border-cyan-400"
              />
            </div>
          </div>
        )}
      </div>
    )}

    {onPassthroughModeChange && (
      <div className="space-y-2 pt-2 border-t border-slate-800/80 text-xs">
        <span className="text-[11px] text-slate-400 font-semibold uppercase block">
          Co-aplicar Patch AV TBC E-E Passthrough em `MAIN` (Operação sem Drive Óptico):
        </span>
        <div className="flex flex-wrap gap-2">
          {(
            [
              { id: 'unchanged', label: 'Manter TBC Original' },
              { id: 'av_passthrough', label: 'AV TBC Passthrough (av_passthrough)' },
              { id: 'factory_original', label: 'Restaurar ODD Stock (factory_original)' },
            ] as const
          ).map(opt => (
            <button
              key={opt.id}
              type="button"
              onClick={() => onPassthroughModeChange(opt.id)}
              className={`px-3 py-1.5 rounded-lg text-[11px] font-bold border transition-colors ${
                passthroughMode === opt.id
                  ? 'bg-emerald-500/20 text-emerald-200 border-emerald-500/60'
                  : 'bg-bench-900 text-slate-400 border-slate-800 hover:border-slate-700'
              }`}
            >
              {opt.label}
            </button>
          ))}
        </div>
      </div>
    )}

    <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 pt-3 border-t border-slate-800">
      <div className="text-[11px] text-slate-300 space-y-0.5">
        <div>
          Teto calculado em blocos de 2 KB:{' '}
          <code className="text-emerald-300 font-bold">
            0x{previewBlocks.toString(16).toUpperCase().padStart(8, '0')}
          </code>{' '}
          ({previewBlocks.toLocaleString()} blocos)
        </div>
        <div className="text-slate-400">
          Setores LBA48 (512 B): <span className="text-cyan-300">{(previewBlocks * 4).toLocaleString()}</span> •
          Regenera zlib + Adler-32 strided (step=16) + Feistel 16 rodadas automaticamente
        </div>
      </div>

      <button
        type="button"
        onClick={onApply}
        disabled={patching}
        className="px-5 py-2.5 rounded-xl bg-emerald-500 hover:bg-emerald-400 text-slate-950 font-bold text-xs flex items-center gap-2 transition-all disabled:opacity-50 shrink-0"
      >
        <Unlock className="w-4 h-4" />
        <span>{patching ? 'Reempacotando...' : 'Aplicar Patch Dinâmico de Capacidade'}</span>
      </button>
    </div>
  </div>
);
