import React from 'react';
import type { MainCapacityInspectionReport } from '@/models';

export interface CapacityInspectionGridProps {
  inspection: MainCapacityInspectionReport;
}

export const CapacityInspectionGrid: React.FC<CapacityInspectionGridProps> = ({ inspection }) => (
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
);
