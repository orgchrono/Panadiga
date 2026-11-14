import React, { useCallback, useEffect, useMemo, useState } from 'react';
import {
  Cpu,
  HardDrive,
  RefreshCw,
  ShieldCheck,
  Unlock,
  CheckCircle2,
  AlertTriangle,
  Sparkles,
} from 'lucide-react';
import { StatusBadge } from '@/components/StatusBadge';
import { MainCapacityInspectionReport, MainCapacityPatchReport } from '@/models';
import { invokeCommand } from '@/services/ipc';
import { useToast } from '@/context/ToastContext';

type PatchMode = 'dynamic' | 'cap' | 'factory_200gb';

const QUICK_CAPACITIES_GB = [250, 300, 320, 400, 500, 1000, 2000] as const;

const OFFICIAL_RTSD_BLOCKS_BY_GB: Record<number, number> = {
  400: 0x0ba43b74,
  300: 0x08baec97,
  250: 0x07466528,
  200: 0x05d21dba,
  160: 0x04a7d7c8,
  137: 0x03fc7ac3,
  120: 0x037dd1d6,
  100: 0x02e8cedd,
  80: 0x0253cbe4,
};

function previewBlocks2Kb(gb: number): number {
  if (OFFICIAL_RTSD_BLOCKS_BY_GB[gb]) {
    return OFFICIAL_RTSD_BLOCKS_BY_GB[gb];
  }
  const safeGb = Math.max(20, Math.min(2200, Math.floor(gb || 250)));
  return Math.min(0x7fffffff, Math.floor((safeGb * 1_000_000_000) / 2048));
}

export const HddCapacityPatcherCard: React.FC = () => {
  const { showToast } = useToast();
  const [inputPath, setInputPath] = useState<string>('');
  const [outputPath, setOutputPath] = useState<string>('');
  const [mode, setMode] = useState<PatchMode>('dynamic');
  const [targetGb, setTargetGb] = useState<number>(250);
  const [limitInDynamicMode, setLimitInDynamicMode] = useState<boolean>(false);
  const [inspection, setInspection] = useState<MainCapacityInspectionReport | null>(null);
  const [patchReport, setPatchReport] = useState<MainCapacityPatchReport | null>(null);
  const [inspecting, setInspecting] = useState<boolean>(false);
  const [patching, setPatching] = useState<boolean>(false);
  const [inspectError, setInspectError] = useState<string | null>(null);

  const runInspect = useCallback(
    async (customInput?: string) => {
      setInspecting(true);
      setInspectError(null);
      const pathVal = (customInput !== undefined ? customInput : inputPath).trim();
      try {
        const res = await invokeCommand<MainCapacityInspectionReport>(
          'inspect_hdd_capacity_patch',
          pathVal ? { inputPath: pathVal } : {}
        );
        setInspection(res);
        if (!pathVal && res.input_path) {
          setInputPath(res.input_path);
        }
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        setInspectError(msg);
      } finally {
        setInspecting(false);
      }
    },
    [inputPath]
  );

  useEffect(() => {
    void runInspect('');
  }, []);

  const previewBlocks = useMemo(() => {
    if (mode === 'factory_200gb') return 0x05d21dba;
    if (mode === 'dynamic' && !limitInDynamicMode) return 0x3fffffff;
    return previewBlocks2Kb(targetGb);
  }, [mode, targetGb, limitInDynamicMode]);

  const handleApplyPatch = async () => {
    setPatching(true);
    try {
      const effectiveTargetGb =
        mode === 'factory_200gb'
          ? null
          : mode === 'cap'
            ? Math.max(20, Math.floor(targetGb || 250))
            : limitInDynamicMode
              ? Math.max(20, Math.floor(targetGb || 250))
              : null;

      const res = await invokeCommand<MainCapacityPatchReport>('apply_hdd_capacity_patch', {
        inputPath: inputPath.trim() || undefined,
        outputPath: outputPath.trim() || undefined,
        mode,
        targetGb: effectiveTargetGb ?? undefined,
      });
      setPatchReport(res);
      setInspection(res.inspection_after);
      showToast({
        type: 'success',
        title: 'Patch de Capacidade Aplicado!',
        message: `Salvo em ${res.output_path} (${res.applied_blocks_2kb_hex})`,
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: 'Falha ao Aplicar Patch de Capacidade',
        message: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setPatching(false);
    }
  };

  return (
    <div className="space-y-6 font-mono">
      {/* Header Banner */}
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

        {/* Input / Output Paths */}
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

        {/* Forensic Inspection Grid */}
        {inspection && (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-3 text-xs">
            <div className="p-3.5 rounded-xl bg-slate-950 border border-slate-800 space-y-1">
              <span className="text-[10px] text-slate-500 uppercase font-semibold">Contêiner / Build</span>
              <div className="text-slate-100 font-bold truncate">{inspection.container_format}</div>
              <div className="text-[11px] text-cyan-400">
                Build: {inspection.build_name || 'RAW MAIN'} ({(inspection.raw_main_size / (1024 * 1024)).toFixed(2)} MB)
              </div>
            </div>

            <div className="p-3.5 rounded-xl bg-slate-950 border border-slate-800 space-y-1">
              <span className="text-[10px] text-slate-500 uppercase font-semibold">Ponto de Patch (MN103S)</span>
              <div className="text-emerald-300 font-bold">{inspection.patch_site_va_hex}</div>
              <div className="text-[11px] text-slate-400">
                Offset: 0x{inspection.patch_site_file_offset.toString(16).toUpperCase()} • MaxLBA:{' '}
                {inspection.max_product_lba_var_va_hex}
              </div>
            </div>

            <div className="p-3.5 rounded-xl bg-slate-950 border border-slate-800 space-y-1">
              <span className="text-[10px] text-slate-500 uppercase font-semibold">Instrução de Clamp (+0x5B)</span>
              <div className="text-amber-300 font-bold">
                Opcode {inspection.clamp_branch_opcode_hex}{' '}
                {inspection.is_dynamic_unclamped ? '(bra +12 Livre)' : '(bls +12 Travado)'}
              </div>
              <div className="text-[11px] text-slate-400">
                PhysBlocks RAM: {inspection.physical_blocks_var_va_hex}
              </div>
            </div>

            <div className="p-3.5 rounded-xl bg-slate-950 border border-slate-800 space-y-1">
              <span className="text-[10px] text-slate-500 uppercase font-semibold">Slots Atuais (2 KB Blocks)</span>
              <div className="text-slate-100 font-bold">
                200GB Slot: {inspection.slot_200gb_hex} ({inspection.slot_200gb_gb.toFixed(1)} GB)
              </div>
              <div className="text-[11px] text-slate-400">
                400GB: {inspection.slot_400gb_hex} • FB: {inspection.slot_fallback_hex}
              </div>
            </div>
          </div>
        )}

        {/* Mode Selection & Dynamic Capacity Controls */}
        <div className="p-5 rounded-xl bg-slate-950 border border-slate-800 space-y-4">
          <div className="flex items-center gap-2 text-xs font-bold text-slate-200">
            <Cpu className="w-4 h-4 text-cyan-400" />
            <span>Estratégia de Tamanho Dinâmico / Desbloqueio</span>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-3 text-xs">
            <button
              type="button"
              onClick={() => setMode('dynamic')}
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
              onClick={() => setMode('cap')}
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
              onClick={() => setMode('factory_200gb')}
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

          {/* Preset Pills + Custom GB */}
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
                      onChange={e => setLimitInDynamicMode(e.target.checked)}
                      className="rounded border-slate-700 bg-slate-900"
                    />
                    <span>Gravar também MaxProductLBA do alvo (GB)</span>
                  </label>
                )}
              </div>

              {(mode === 'cap' || limitInDynamicMode) && (
                <div className="flex flex-wrap items-center gap-2">
                  {QUICK_CAPACITIES_GB.map(gb => (
                    <button
                      key={gb}
                      type="button"
                      onClick={() => setTargetGb(gb)}
                      className={`px-3 py-1.5 rounded-lg text-xs font-bold border transition-colors ${
                        targetGb === gb
                          ? 'bg-cyan-500 text-slate-950 border-cyan-400'
                          : 'bg-bench-900 text-slate-300 border-slate-800 hover:border-slate-700'
                      }`}
                    >
                      {gb >= 1000 ? `${gb / 1000} TB (${gb} GB)` : `${gb} GB`}
                      {gb === 250 ? ' ★' : ''}
                    </button>
                  ))}

                  <div className="flex items-center gap-1.5 ml-auto">
                    <span className="text-[11px] text-slate-400">Custom (GB):</span>
                    <input
                      type="number"
                      min={20}
                      max={2200}
                      value={targetGb}
                      onChange={e => setTargetGb(Math.max(20, Math.min(2200, Number(e.target.value) || 250)))}
                      className="w-24 px-2.5 py-1.5 rounded-lg bg-slate-900 border border-slate-700 text-xs text-cyan-300 focus:outline-none focus:border-cyan-400"
                    />
                  </div>
                </div>
              )}
            </div>
          )}

          {/* Live Calculation Bar + Execute Button */}
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
              onClick={() => void handleApplyPatch()}
              disabled={patching}
              className="px-5 py-2.5 rounded-xl bg-emerald-500 hover:bg-emerald-400 text-slate-950 font-bold text-xs flex items-center gap-2 transition-all disabled:opacity-50 shrink-0"
            >
              <Unlock className="w-4 h-4" />
              <span>{patching ? 'Reempacotando...' : 'Aplicar Patch Dinâmico de Capacidade'}</span>
            </button>
          </div>
        </div>

        {/* Patch Output Report */}
        {patchReport && (
          <div className="p-4 rounded-xl bg-emerald-500/10 border border-emerald-500/40 text-xs space-y-2">
            <div className="flex items-center gap-2 text-emerald-300 font-bold">
              <CheckCircle2 className="w-4 h-4" />
              <span>Firmware Patcheado & Verificado (Adler-32 / EXTRFOOT OK)!</span>
            </div>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-2 text-[11px] text-slate-300">
              <div>
                Saída: <code className="text-cyan-300">{patchReport.output_path}</code>
              </div>
              <div>
                Opcode Branch: <code>{patchReport.inspection_before.clamp_branch_opcode_hex}</code> →{' '}
                <code className="text-emerald-300 font-bold">{patchReport.inspection_after.clamp_branch_opcode_hex}</code>{' '}
                • Teto: <code>{patchReport.applied_blocks_2kb_hex}</code>
              </div>
              <div className="truncate">
                SHA-256 Patcheado: <code className="text-emerald-400">{patchReport.patched_sha256}</code>
              </div>
              <div>
                Contêiner EXTRHEAD: {patchReport.repacked_container_size?.toLocaleString() ?? 'RAW MAIN'} bytes • MBR
                Sync: {patchReport.mbr_superblocks_updated ? 'Atualizado (LBA 0 + 57344)' : 'N/A'}
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Official RTSD Capacity Lookup Table */}
      {inspection && inspection.rtsd_presets.length > 0 && (
        <div className="rounded-xl bg-bench-900 border border-slate-800 p-5 shadow-xl space-y-3">
          <h4 className="text-xs font-bold text-slate-200 uppercase">
            Tabela Nativa `Change HDD Capacity` (Descoberta no Firmware PANASONIC @ 0x574CA054 / 0x574C267B)
          </h4>
          <div className="overflow-x-auto">
            <table className="w-full text-left text-xs border-collapse">
              <thead>
                <tr className="border-b border-slate-800 text-slate-500 text-[10px] uppercase">
                  <th className="py-2 pr-3">Item RTSD</th>
                  <th className="py-2 pr-3">Capacidade</th>
                  <th className="py-2 pr-3">Blocos 2 KB (Hex)</th>
                  <th className="py-2 pr-3">Blocos 2 KB (Dec)</th>
                  <th className="py-2">Setores LBA (512 B)</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-800/60">
                {inspection.rtsd_presets.map(p => (
                  <tr
                    key={p.menu_index}
                    className={
                      p.capacity_gb === 250
                        ? 'bg-cyan-500/10 text-cyan-200 font-semibold'
                        : p.capacity_gb === 200
                          ? 'bg-amber-500/5 text-amber-200'
                          : 'text-slate-300'
                    }
                  >
                    <td className="py-2 pr-3">#{p.menu_index} — {p.label}</td>
                    <td className="py-2 pr-3">{p.capacity_gb} GB</td>
                    <td className="py-2 pr-3 font-bold text-emerald-300">{p.blocks_2kb_hex}</td>
                    <td className="py-2 pr-3">{p.blocks_2kb.toLocaleString()}</td>
                    <td className="py-2">{p.sectors_512b.toLocaleString()}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
};
