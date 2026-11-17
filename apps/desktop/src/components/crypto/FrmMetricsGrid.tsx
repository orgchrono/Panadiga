import React from 'react';
import { MetricCard } from '@/components/MetricCard';
import { StatusBadge } from '@/components/StatusBadge';
import { FRM_ENTROPY_PRECISION, MIB_BYTES } from '@/constants';
import { CryptanalysisReport, FrmTocDumpReport } from '@/models';
import { useTranslation } from '@/i18n/useTranslation';

interface FrmMetricsGridProps {
  cryptReport: CryptanalysisReport;
  tocReport?: FrmTocDumpReport | null;
}

export const FrmMetricsGrid: React.FC<FrmMetricsGridProps> = ({ cryptReport, tocReport }) => {
  const { t } = useTranslation();
  const topBlockCount = cryptReport.top_blocks?.[0]?.count ?? 0;
  const containerMb = (cryptReport.file_size_bytes / MIB_BYTES).toFixed(2);
  const slotCount =
    tocReport?.decrypted_toc && tocReport.decrypted_toc.length > 0
      ? tocReport.decrypted_toc.length
      : (tocReport?.toc_entries?.length ?? 0);
  const tocBadge =
    tocReport?.toc_format === 'sd-plaintext'
      ? t('crypto.metrics.tocSdBadge')
      : tocReport?.toc_format === 'dvd-feistel'
        ? t('crypto.metrics.tocFeistelBadge')
        : t('crypto.metrics.cpusBadge');

  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 font-mono">
      <MetricCard
        title={t('crypto.metrics.entropy')}
        value={cryptReport.global_entropy.toFixed(FRM_ENTROPY_PRECISION)}
        unit={t('crypto.metrics.unitBitsByte')}
        subtitle={t('crypto.metrics.entropySub')}
        variant="slate"
      />
      <MetricCard
        title={t('crypto.metrics.ecbLeaks')}
        value={topBlockCount.toLocaleString()}
        unit={t('crypto.metrics.unitBlocks')}
        subtitle={t('crypto.metrics.ecbLeaksSub')}
        variant={cryptReport.is_ecb_leak_confirmed ? 'amber' : 'slate'}
        badge={
          <StatusBadge
            variant={cryptReport.is_ecb_leak_confirmed ? 'warning' : 'neutral'}
            label={
              cryptReport.is_ecb_leak_confirmed
                ? t('crypto.metrics.ecbBadge')
                : t('crypto.metrics.ecbCleanBadge')
            }
          />
        }
      />
      <MetricCard
        title={t('crypto.metrics.size')}
        value={cryptReport.file_size_bytes.toLocaleString()}
        unit={t('crypto.metrics.unitBytes')}
        subtitle={t('crypto.metrics.subMeta', { file: cryptReport.frm_file, mb: containerMb })}
        variant="slate"
      />
      <MetricCard
        title={t('crypto.metrics.cpus')}
        value={String(slotCount)}
        unit={t('crypto.metrics.unitModules')}
        subtitle={t('crypto.metrics.cpusSub')}
        variant="cyan"
        badge={<StatusBadge variant="success" label={tocBadge} />}
      />
    </div>
  );
};
