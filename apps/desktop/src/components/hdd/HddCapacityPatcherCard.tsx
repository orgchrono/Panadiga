import React from 'react';
import { AlertTriangle, RefreshCw, Unlock } from 'lucide-react';
import { StatusBadge } from '@/components/StatusBadge';
import { useHddCapacityViewModel } from '@/viewmodels/useHddCapacityViewModel';
import { CapacityInspectionGrid } from './capacity/CapacityInspectionGrid';
import { CapacityStrategyPanel } from './capacity/CapacityStrategyPanel';
import { PatchOutputBanner, RtsdPresetsTable } from './capacity/RtsdPresetsTable';

export const HddCapacityPatcherCard: React.FC = () => {
  const {
    inputPath,
    setInputPath,
    outputPath,
    setOutputPath,
    mode,
    setMode,
    targetGb,
    setTargetGb,
    limitInDynamicMode,
    setLimitInDynamicMode,
    inspection,
    patchReport,
    inspecting,
    patching,
    inspectError,
    previewBlocks,
    runInspect,
    handleApplyPatch,
  } = useHddCapacityViewModel();

  return (
    <div className="space-y-6 font-mono">
      <div className="rounded-xl bg-bench-900 border border-slate-800 p-6 shadow-xl space-y-5">
        <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 border-b border-slate-800 pb-4">
          <div>
            <h3 className="text-sm font-bold text-slate-100 flex items-center gap-2">
              <Unlock className="w-5 h-5 text-emerald-400" />
              <span>Desbloqueio Dinâmico de Capacidade de HDD (src/hdd_cmd.c + MEIHDFS1.0)</span>
            </h3>
            <p className="text-xs text-slate-400 mt-1">
              Remove o clamp artificial de 200 GB (<code>bls +12</code> em{' '}
              <code className="text-cyan-300">MaxProductLBA</code>) e libera detecção automática LBA48 para discos de
              250 GB, 320 GB, 500 GB, 1 TB ou 2 TB+ direto no módulo <code>MAIN</code> (raw, <code>EXTRHEAD</code> ou
              imagem HDD).
            </p>
          </div>
          <div className="flex items-center gap-2">
            {inspection ? (
              inspection.is_dynamic_unclamped ? (
                <StatusBadge variant="success" label="DINÂMICO / UNCLAMPED (0xCA)" />
              ) : (
                <StatusBadge
                  variant="warning"
                  label={`TRAVA ATIVA: ${inspection.effective_default_limit_gb.toFixed(0)} GB (0xC6)`}
                />
              )
            ) : (
              <StatusBadge variant="neutral" label="AGUARDANDO BINÁRIO" />
            )}
          </div>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 text-xs">
          <div className="space-y-1.5">
            <label className="text-[11px] text-slate-400 font-semibold uppercase block">
              Arquivo de Origem (Partição 1 0x02000000, MAIN raw ou hdd_100.bin)
            </label>
            <div className="flex gap-2">
              <input
                type="text"
                value={inputPath}
                onChange={e => setInputPath(e.target.value)}
                placeholder="firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin"
                className="flex-1 px-3 py-2 rounded-xl bg-slate-950 border border-slate-800 text-xs text-slate-200 focus:outline-none focus:border-cyan-500"
              />
              <button
                type="button"
                onClick={() => void runInspect()}
                disabled={inspecting}
                className="px-3 py-2 rounded-xl bg-slate-800 hover:bg-slate-700 text-cyan-300 font-bold text-xs flex items-center gap-1.5 transition-colors disabled:opacity-50"
              >
                <RefreshCw className={`w-3.5 h-3.5 ${inspecting ? 'animate-spin' : ''}`} />
                <span>Inspecionar</span>
              </button>
            </div>
          </div>

          <div className="space-y-1.5">
            <label className="text-[11px] text-slate-400 font-semibold uppercase block">
              Arquivo de Saída Patcheado (Opcional — gera automático se vazio)
            </label>
            <input
              type="text"
              value={outputPath}
              onChange={e => setOutputPath(e.target.value)}
              placeholder="Ex: extracted_partitions/hdd_partition_firmware_0x02000000_PATCHED_DYNAMIC.bin"
              className="w-full px-3 py-2 rounded-xl bg-slate-950 border border-slate-800 text-xs text-slate-200 focus:outline-none focus:border-emerald-500"
            />
          </div>
        </div>

        {inspectError && (
          <div className="p-3 rounded-xl bg-rose-950/30 border border-rose-800/60 text-xs text-rose-300 flex items-center gap-2">
            <AlertTriangle className="w-4 h-4 shrink-0" />
            <span>{inspectError}</span>
          </div>
        )}

        {inspection && <CapacityInspectionGrid inspection={inspection} />}

        <CapacityStrategyPanel
          mode={mode}
          onModeChange={setMode}
          targetGb={targetGb}
          onTargetGbChange={setTargetGb}
          limitInDynamicMode={limitInDynamicMode}
          onLimitInDynamicChange={setLimitInDynamicMode}
          previewBlocks={previewBlocks}
          patching={patching}
          onApply={() => void handleApplyPatch()}
        />

        {patchReport && <PatchOutputBanner patchReport={patchReport} />}
      </div>

      {inspection && inspection.rtsd_presets.length > 0 && (
        <RtsdPresetsTable presets={inspection.rtsd_presets} />
      )}
    </div>
  );
};
