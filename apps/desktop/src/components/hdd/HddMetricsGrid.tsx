import React from 'react';
import { MetricCard } from '@/components/MetricCard';
import { StatusBadge } from '@/components/StatusBadge';
import {
  HDD_METRIC_ENTROPY_PRECISION,
  KIB_BYTES,
  LBA_SECTOR_SIZE_BYTES,
  MIB_BYTES,
} from '@/constants';
import { useTranslation } from '@/i18n/useTranslation';
import { HddAnalysisReport } from '@/models';

export interface HddMetricsGridProps {
  report?: HddAnalysisReport | null;
}

export const HddMetricsGrid: React.FC<HddMetricsGridProps> = ({ report }) => {
  const { t } = useTranslation();

  if (!report) {
    return (
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <MetricCard
          title={t('hdd.metrics.imageSize')}
          value="--"
          unit={t('hdd.metrics.mbUnit')}
          subtitle={t('hdd.metrics.noFileSub')}
          variant="slate"
        />
        <MetricCard
          title={t('hdd.metrics.mirrorMbr')}
          value="--"
          unit=""
          subtitle={t('hdd.metrics.awaitingSub')}
          variant="slate"
          badge={<StatusBadge variant="neutral" label={t('hdd.offlineBadge')} />}
        />
        <MetricCard
          title={t('hdd.metrics.part1Default')}
          value="--"
          unit=""
          subtitle={t('hdd.metrics.meihdfsUnanalyzedSub')}
          variant="slate"
        />
        <MetricCard
          title={t('hdd.metrics.part2Default')}
          value="--"
          unit=""
          subtitle={t('hdd.metrics.meihdfsUnanalyzedSub')}
          variant="slate"
        />
      </div>
    );
  }

  const isGb = report.file_size_mb >= KIB_BYTES;
  const sizeVal = isGb ? (report.file_size_mb / KIB_BYTES).toFixed(2) : report.file_size_mb.toFixed(2);
  const sizeUnit = isGb ? t('hdd.metrics.gbUnit') : t('hdd.metrics.mbUnit');
  const mirrorLbaMb = Math.round((report.mbr_tables.mirror_lba * LBA_SECTOR_SIZE_BYTES) / MIB_BYTES);
  const isMirrorOk = report.mbr_tables.mirror_is_bit_identical;

  const part1 = report.partitions[0];
  const part2 = report.partitions[1];

  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
      <MetricCard
        title={t('hdd.metrics.imageSize')}
        value={sizeVal}
        unit={sizeUnit}
        subtitle={t('hdd.metrics.sizeMetaSub', {
          bytes: report.file_size_bytes.toLocaleString(),
          sectors: report.total_sectors.toLocaleString(),
        })}
        variant="cyan"
      />
      <MetricCard
        title={t('hdd.metrics.mirrorMbr')}
        value={t('hdd.metrics.lbaPrefix', { lba: report.mbr_tables.mirror_lba.toLocaleString() })}
        unit={t('hdd.metrics.mbUnitBracket', { mb: mirrorLbaMb })}
        subtitle={t('hdd.metrics.guidSub', {
          guid: report.mbr_tables.disk_volume_guid || t('hdd.metrics.guidNa'),
        })}
        variant={isMirrorOk ? 'emerald' : 'rose'}
        badge={
          isMirrorOk ? (
            <StatusBadge variant="success" label={t('hdd.metrics.matchBadge')} />
          ) : (
            <StatusBadge variant="error" label={t('hdd.metrics.mismatchBadge')} />
          )
        }
      />
      <MetricCard
        title={part1 ? part1.name : t('hdd.metrics.part1Default')}
        value={part1 ? part1.exact_payload_size.toLocaleString() : '--'}
        unit={part1 ? t('hdd.metrics.bytesUnit') : ''}
        subtitle={
          part1
            ? t('hdd.metrics.entropySub', {
                lba: part1.lba_start.toLocaleString(),
                entropy: part1.entropy.toFixed(HDD_METRIC_ENTROPY_PRECISION),
              })
            : t('hdd.metrics.notIdentified')
        }
        variant="slate"
      />
      <MetricCard
        title={part2 ? part2.name : t('hdd.metrics.part2Default')}
        value={part2 ? part2.exact_payload_size.toLocaleString() : '--'}
        unit={part2 ? t('hdd.metrics.bytesUnit') : ''}
        subtitle={
          part2
            ? t('hdd.metrics.sectorsSub', {
                lba: part2.lba_start.toLocaleString(),
                sectors: part2.sectors_count.toLocaleString(),
              })
            : t('hdd.metrics.notIdentified')
        }
        variant="slate"
      />
    </div>
  );
};
