import React from 'react';
import { useTranslation } from '@/i18n/useTranslation';
import type { HardwareSsotSpec } from '@/models';

export interface MemoryMapPanelProps {
  ssot: HardwareSsotSpec;
}

export const MemoryMapPanel: React.FC<MemoryMapPanelProps> = ({ ssot }) => {
  const { t } = useTranslation();

  return (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
      <div className="rounded-xl bg-slate-950 border border-slate-800 p-3.5 space-y-2">
        <h4 className="text-xs font-bold text-cyan-300">{t('pico.memoryMap.title')}</h4>
        <div className="overflow-x-auto">
          <table className="w-full text-[11px] font-mono">
            <thead>
              <tr className="text-left text-slate-500 border-b border-slate-800">
                <th className="py-1.5 pr-2">{t('pico.memoryMap.colRegion')}</th>
                <th className="py-1.5 pr-2">{t('pico.memoryMap.colStart')}</th>
                <th className="py-1.5 pr-2">{t('pico.memoryMap.colEnd')}</th>
                <th className="py-1.5">{t('pico.memoryMap.colPerms')}</th>
              </tr>
            </thead>
            <tbody>
              {ssot.memory_regions.length === 0 ? (
                <tr>
                  <td colSpan={4} className="py-2 text-slate-500">
                    {t('pico.memoryMap.empty')}
                  </td>
                </tr>
              ) : (
                ssot.memory_regions.map((reg) => (
                  <tr key={reg.name} className="border-b border-slate-900 text-slate-300">
                    <td className="py-1.5 pr-2">{reg.name}</td>
                    <td className="py-1.5 pr-2 text-cyan-400">{reg.start}</td>
                    <td className="py-1.5 pr-2 text-slate-400">{reg.end}</td>
                    <td className="py-1.5 text-amber-300">{reg.perms}</td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </div>

      <div className="rounded-xl bg-slate-950 border border-slate-800 p-3.5 space-y-2">
        <h4 className="text-xs font-bold text-emerald-300">{t('pico.memoryMap.geoTitle')}</h4>
        <div className="overflow-x-auto">
          <table className="w-full text-[11px] font-mono">
            <thead>
              <tr className="text-left text-slate-500 border-b border-slate-800">
                <th className="py-1.5 pr-2">{t('pico.memoryMap.colChip')}</th>
                <th className="py-1.5 pr-2">{t('pico.memoryMap.colNvramBase')}</th>
                <th className="py-1.5">{t('pico.memoryMap.colMicrosectors')}</th>
              </tr>
            </thead>
            <tbody>
              {ssot.flash_geometries.map((geo) => (
                <tr key={geo.size_mb} className="border-b border-slate-900 text-slate-300">
                  <td className="py-1.5 pr-2">{geo.chip_name}</td>
                  <td className="py-1.5 pr-2 text-cyan-400">{geo.nvram_base_hex}</td>
                  <td className="py-1.5 text-emerald-300">{geo.microsector_range}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
};
