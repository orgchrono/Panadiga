import React from 'react';
import { MIB_BYTES } from '@/constants';
import { useTranslation } from '@/i18n/useTranslation';
import type { MainCapacityInspectionReport } from '@/models';

export interface CapacityInspectionGridProps {
  inspection: MainCapacityInspectionReport;
}

export const CapacityInspectionGrid: React.FC<CapacityInspectionGridProps> = ({ inspection }) => {
  const { t } = useTranslation();

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-3 text-xs">
      <div className="p-3.5 rounded-xl bg-slate-950 border border-slate-800 space-y-1">
        <span className="text-[10px] text-slate-500 uppercase font-semibold">
          {t('hdd.capacity.inspection.containerLabel')}
        </span>
        <div className="text-slate-100 font-bold truncate">{inspection.container_format}</div>
        <div className="text-[11px] text-cyan-400">
          {t('hdd.capacity.inspection.buildMeta', {
            build: inspection.build_name || t('hdd.capacity.inspection.rawMainFallback'),
            mb: (inspection.raw_main_size / MIB_BYTES).toFixed(2),
          })}
        </div>
      </div>

      <div className="p-3.5 rounded-xl bg-slate-950 border border-slate-800 space-y-1">
        <span className="text-[10px] text-slate-500 uppercase font-semibold">
          {t('hdd.capacity.inspection.patchSiteLabel')}
        </span>
        <div className="text-emerald-300 font-bold">{inspection.patch_site_va_hex}</div>
        <div className="text-[11px] text-slate-400">
          {t('hdd.capacity.inspection.offsetMeta', {
            offset: inspection.patch_site_file_offset.toString(16).toUpperCase(),
            lba: inspection.max_product_lba_var_va_hex,
          })}
        </div>
      </div>

      <div className="p-3.5 rounded-xl bg-slate-950 border border-slate-800 space-y-1">
        <span className="text-[10px] text-slate-500 uppercase font-semibold">
          {t('hdd.capacity.inspection.clampLabel')}
        </span>
        <div className="text-amber-300 font-bold">
          {t(
            inspection.is_dynamic_unclamped
              ? 'hdd.capacity.inspection.opcodeUnclamped'
              : 'hdd.capacity.inspection.opcodeClamped',
            { opcode: inspection.clamp_branch_opcode_hex },
          )}
        </div>
        <div className="text-[11px] text-slate-400">
          {t('hdd.capacity.inspection.physBlocksRam', {
            addr: inspection.physical_blocks_var_va_hex,
          })}
        </div>
      </div>

      <div className="p-3.5 rounded-xl bg-slate-950 border border-slate-800 space-y-1">
        <span className="text-[10px] text-slate-500 uppercase font-semibold">
          {t('hdd.capacity.inspection.slotsLabel')}
        </span>
        <div className="text-slate-100 font-bold">
          {t('hdd.capacity.inspection.slot200', {
            hex: inspection.slot_200gb_hex,
            gb: inspection.slot_200gb_gb.toFixed(1),
          })}
        </div>
        <div className="text-[11px] text-slate-400">
          {t('hdd.capacity.inspection.slotsOther', {
            s400: inspection.slot_400gb_hex,
            sfb: inspection.slot_fallback_hex,
          })}
        </div>
      </div>
    </div>
  );
};
