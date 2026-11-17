import React from 'react';
import { StatusBadge } from '@/components/StatusBadge';
import { useTranslation } from '@/i18n/useTranslation';
import { HddAnalysisReport } from '@/models';
import { MbrSectorDetailsCard } from './MbrSectorDetailsCard';

export interface HddMbrComparisonProps {
  hddReport: HddAnalysisReport;
}

export const HddMbrComparison: React.FC<HddMbrComparisonProps> = ({ hddReport }) => {
  const { t } = useTranslation();

  return (
    <div className="rounded-xl bg-bench-900 border border-slate-800 p-5 shadow-xl space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-sm font-bold text-slate-100 flex items-center gap-2">
            <span>{t('hdd.mbrSection.comparisonTitle')}</span>
          </h3>
          <p className="text-xs text-slate-400 mt-0.5">
            {t('hdd.mbrSection.comparisonSubtitle')}
          </p>
        </div>
        <StatusBadge variant="success" label={t('hdd.mbrSection.replicaBadge')} />
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-xs font-mono">
        <MbrSectorDetailsCard
          title={t('hdd.mbrSection.primaryCardTitle')}
          titleColor="text-cyan-400"
          badgeLabel={t('hdd.mbrSection.sectorZeroBadge')}
          badgeVariant="info"
          rows={[
            { label: t('hdd.mbrSection.hexSignature'), value: hddReport.mbr_tables.lba0_hex_signature },
            { label: t('hdd.mbrSection.magicAscii'), value: hddReport.mbr_tables.magic_ascii, valueClassName: 'text-emerald-400 font-bold' },
            { label: t('hdd.mbrSection.diskGuid'), value: hddReport.mbr_tables.disk_volume_guid },
            { label: t('hdd.mbrSection.ibmBoot'), value: t('hdd.mbrSection.ibmBootVal'), valueClassName: 'text-slate-400' },
            { label: t('hdd.mbrSection.sha256'), value: hddReport.mbr_tables.lba0_sha256, valueClassName: 'text-slate-300 truncate max-w-[200px]', title: hddReport.mbr_tables.lba0_sha256 },
          ]}
        />

        <MbrSectorDetailsCard
          title={t('hdd.mbrSection.mirrorTitle')}
          titleColor="text-emerald-400"
          badgeLabel={t('hdd.mbrSection.mirrorBadge')}
          badgeVariant="success"
          rows={[
            { label: t('hdd.mbrSection.physicalOffset'), value: t('hdd.mbrSection.mirrorPhysicalOffsetVal') },
            { label: t('hdd.mbrSection.bitComparison'), value: t('hdd.mbrSection.bitComparisonVal'), valueClassName: 'text-emerald-400 font-bold' },
            { label: t('hdd.mbrSection.diskGuid'), value: hddReport.mbr_tables.disk_volume_guid },
            { label: t('hdd.mbrSection.md5'), value: hddReport.mbr_tables.mirror_md5, valueClassName: 'text-slate-300' },
            { label: t('hdd.mbrSection.sha256'), value: hddReport.mbr_tables.mirror_sha256, valueClassName: 'text-slate-300 truncate max-w-[200px]', title: hddReport.mbr_tables.mirror_sha256 },
          ]}
        />
      </div>
    </div>
  );
};
