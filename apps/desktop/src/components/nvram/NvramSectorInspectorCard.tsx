import React from 'react';
import { StatusBadge } from '@/components/StatusBadge';
import { HEX32_PAD_WIDTH, KIB_BYTES } from '@/constants';
import { useTranslation } from '@/i18n/useTranslation';
import type { FlashSectorInfo } from '@/models';

interface NvramSectorInspectorCardProps {
  readonly sector: FlashSectorInfo;
  readonly cleanSuccess: boolean;
}

export const NvramSectorInspectorCard: React.FC<NvramSectorInspectorCardProps> = ({
  sector,
  cleanSuccess,
}) => {
  const { t } = useTranslation();
  const badgeLabel = !sector.is_corrupted_area
    ? t('nvram.sectorMap.badgeIntact')
    : cleanSuccess
      ? t('nvram.sectorMap.badgeCleanFf')
      : t('nvram.sectorMap.badgeU99');

  return (
    <div className="p-4 rounded-xl bg-bench-950 border border-slate-800 flex flex-col md:flex-row items-start md:items-center justify-between gap-4">
      <div className="space-y-1">
        <div className="flex items-center gap-2">
          <span className="text-sm font-bold text-slate-100">
            {t('nvram.sectorMap.selectedPrefix', { name: sector.name })}
          </span>
          <StatusBadge
            variant={sector.is_corrupted_area ? (cleanSuccess ? 'success' : 'error') : 'info'}
            label={badgeLabel}
          />
        </div>
        <p className="text-xs text-slate-400 font-sans">{sector.notes}</p>
      </div>

      <div className="flex flex-wrap items-center gap-4 text-xs font-mono">
        <div className="text-slate-400">
          <span className="text-slate-500 block text-[10px]">{t('nvram.sectorMap.fieldOffset')}</span>
          <span>
            0x{sector.start_offset.toString(16).padStart(HEX32_PAD_WIDTH, '0').toUpperCase()}
          </span>
        </div>
        <div className="text-slate-400">
          <span className="text-slate-500 block text-[10px]">{t('nvram.sectorMap.fieldSize')}</span>
          <span>{t('nvram.sectorMap.sizeKb', { kb: sector.size_bytes / KIB_BYTES })}</span>
        </div>
        <div className="text-slate-400">
          <span className="text-slate-500 block text-[10px]">{t('nvram.sectorMap.fieldActive')}</span>
          <span className={sector.is_corrupted_area && !cleanSuccess ? 'text-rose-400 font-bold' : 'text-slate-200'}>
            {t('nvram.sectorMap.bytesUnit', { bytes: sector.active_bytes ?? 0 })}
          </span>
        </div>
      </div>
    </div>
  );
};
