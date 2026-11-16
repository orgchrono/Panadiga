import React from 'react';
import { CheckCircle2 } from 'lucide-react';
import type { MainCapacityPatchReport, RtsdCapacityPreset } from '@/models';

export interface PatchOutputBannerProps {
  patchReport: MainCapacityPatchReport;
}

export const PatchOutputBanner: React.FC<PatchOutputBannerProps> = ({ patchReport }) => (
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
        {patchReport.passthrough_mode && ` • TBC: ${patchReport.passthrough_mode}`}
      </div>
    </div>
  </div>
);

export interface RtsdPresetsTableProps {
  presets: RtsdCapacityPreset[];
}

export const RtsdPresetsTable: React.FC<RtsdPresetsTableProps> = ({ presets }) => (
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
          {presets.length === 0 ? (
            <tr>
              <td colSpan={5} className="py-3 text-center text-slate-500">
                Nenhum preset RTSD carregado.
              </td>
            </tr>
          ) : (
            presets.map(p => (
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
            ))
          )}
        </tbody>
      </table>
    </div>
  </div>
);
