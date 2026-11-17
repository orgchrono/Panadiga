import React from 'react';
import { CheckCircle2 } from 'lucide-react';
import { HDD_DEFAULT_CAPACITY_GB, HDD_FACTORY_200GB_GB } from '@/constants';
import { useTranslation } from '@/i18n/useTranslation';
import type { MainCapacityPatchReport, RtsdCapacityPreset } from '@/models';

export interface PatchOutputBannerProps {
  patchReport: MainCapacityPatchReport;
}

export const PatchOutputBanner: React.FC<PatchOutputBannerProps> = ({ patchReport }) => {
  const { t } = useTranslation();

  return (
    <div className="p-4 rounded-xl bg-emerald-500/10 border border-emerald-500/40 text-xs space-y-2">
      <div className="flex items-center gap-2 text-emerald-300 font-bold">
        <CheckCircle2 className="w-4 h-4" />
        <span>{t('hdd.capacity.patchBanner.title')}</span>
      </div>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-2 text-[11px] text-slate-300">
        <div>
          {t('hdd.capacity.patchBanner.outputLabel')}{' '}
          <code className="text-cyan-300">{patchReport.output_path}</code>
        </div>
        <div>
          {t('hdd.capacity.patchBanner.branchLabel')}{' '}
          <code>{patchReport.inspection_before.clamp_branch_opcode_hex}</code> →{' '}
          <code className="text-emerald-300 font-bold">
            {patchReport.inspection_after.clamp_branch_opcode_hex}
          </code>{' '}
          • {t('hdd.capacity.patchBanner.capLabel')}{' '}
          <code>{patchReport.applied_blocks_2kb_hex}</code>
        </div>
        <div className="truncate">
          {t('hdd.capacity.patchBanner.shaLabel')}{' '}
          <code className="text-emerald-400">{patchReport.patched_sha256}</code>
        </div>
        <div>
          {t('hdd.capacity.patchBanner.containerLabel')}{' '}
          {patchReport.repacked_container_size?.toLocaleString() ??
            t('hdd.capacity.patchBanner.rawMainFallback')}{' '}
          {t('hdd.capacity.patchBanner.mbrSyncLabel')}{' '}
          {patchReport.mbr_superblocks_updated
            ? t('hdd.capacity.patchBanner.mbrSyncUpdated')
            : t('hdd.capacity.patchBanner.mbrSyncNa')}
          {patchReport.passthrough_mode &&
            t('hdd.capacity.patchBanner.tbcSuffix', { mode: patchReport.passthrough_mode })}
        </div>
      </div>
    </div>
  );
};

export interface RtsdPresetsTableProps {
  presets: RtsdCapacityPreset[];
}

export const RtsdPresetsTable: React.FC<RtsdPresetsTableProps> = ({ presets }) => {
  const { t } = useTranslation();

  return (
    <div className="rounded-xl bg-bench-900 border border-slate-800 p-5 shadow-xl space-y-3">
      <h4 className="text-xs font-bold text-slate-200 uppercase">
        {t('hdd.capacity.rtsdTable.title')}
      </h4>
      <div className="overflow-x-auto">
        <table className="w-full text-left text-xs border-collapse">
          <thead>
            <tr className="border-b border-slate-800 text-slate-500 text-[10px] uppercase">
              <th className="py-2 pr-3">{t('hdd.capacity.rtsdTable.colItem')}</th>
              <th className="py-2 pr-3">{t('hdd.capacity.rtsdTable.colCap')}</th>
              <th className="py-2 pr-3">{t('hdd.capacity.rtsdTable.colHex')}</th>
              <th className="py-2 pr-3">{t('hdd.capacity.rtsdTable.colDec')}</th>
              <th className="py-2">{t('hdd.capacity.rtsdTable.colSectors')}</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-800/60">
            {presets.length === 0 ? (
              <tr>
                <td colSpan={5} className="py-3 text-center text-slate-500">
                  {t('hdd.capacity.rtsdTable.empty')}
                </td>
              </tr>
            ) : (
              presets.map((p) => (
                <tr
                  key={p.menu_index}
                  className={
                    p.capacity_gb === HDD_DEFAULT_CAPACITY_GB
                      ? 'bg-cyan-500/10 text-cyan-200 font-semibold'
                      : p.capacity_gb === HDD_FACTORY_200GB_GB
                        ? 'bg-amber-500/5 text-amber-200'
                        : 'text-slate-300'
                  }
                >
                  <td className="py-2 pr-3">
                    {t('hdd.capacity.rtsdTable.rowItem', { idx: p.menu_index, label: p.label })}
                  </td>
                  <td className="py-2 pr-3">
                    {t('hdd.capacity.rtsdTable.gbVal', { gb: p.capacity_gb })}
                  </td>
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
};
