import React from 'react';
import { MetricCard, type MetricVariant } from '@/components/MetricCard';
import { StatusBadge, type StatusVariant } from '@/components/StatusBadge';
import {
  BITS_PER_BYTE,
  HEX32_PAD_WIDTH,
  KIB_BYTES,
  MIB_BYTES,
  SPANSION_MICROSECTOR_COUNT,
} from '@/constants';
import { useTranslation } from '@/i18n/useTranslation';
import { ShieldCheck, AlertOctagon, Layers, ArrowRight } from 'lucide-react';
import { NvramCleanReport, FlashSectorInfo } from '@/models';

interface NvramMetricsGridProps {
  report: NvramCleanReport | null;
  selectedSector: FlashSectorInfo | null;
  cleanSuccess: boolean;
}

interface NvramStatusDisplay {
  readonly value: string;
  readonly subtitle: string;
  readonly variant: MetricVariant;
  readonly badgeVariant: StatusVariant;
  readonly badgeLabel: string;
}

function getNvramStatusDisplay(
  report: NvramCleanReport | null,
  cleanSuccess: boolean,
  isCorrupted: boolean,
  corruptedCount: number,
  t: ReturnType<typeof useTranslation>['t'],
): NvramStatusDisplay {
  if (cleanSuccess) {
    return {
      value: t('nvram.metrics.valVirgin'),
      subtitle: t('nvram.metrics.subVirgin'),
      variant: 'emerald',
      badgeVariant: 'success',
      badgeLabel: t('nvram.metrics.badgeCleanFf'),
    };
  }
  if (!report) {
    return {
      value: t('nvram.metrics.valWaiting'),
      subtitle: t('nvram.metrics.subNoDump'),
      variant: 'cyan',
      badgeVariant: 'neutral',
      badgeLabel: t('nvram.metrics.badgeStatusOk'),
    };
  }
  if (isCorrupted) {
    return {
      value: t('nvram.metrics.valCorrupted'),
      subtitle: t('nvram.metrics.subCorrupted', { count: corruptedCount }),
      variant: 'rose',
      badgeVariant: 'error',
      badgeLabel: t('nvram.metrics.badgeU99Loop'),
    };
  }
  return {
    value: t('nvram.metrics.valClean'),
    subtitle: t('nvram.metrics.subAlreadyClean'),
    variant: 'cyan',
    badgeVariant: 'neutral',
    badgeLabel: t('nvram.metrics.badgeStatusOk'),
  };
}

function getCorruptedCardSubtitle(
  report: NvramCleanReport | null,
  cleanSuccess: boolean,
  isCorrupted: boolean,
  t: ReturnType<typeof useTranslation>['t'],
): string {
  if (cleanSuccess) return t('nvram.metrics.corruptedSubVirgin');
  if (!report) return t('nvram.metrics.corruptedSubWaiting');
  return isCorrupted
    ? t('nvram.metrics.corruptedSubActive', { bytes: report.nvram_non_ff_bytes.toLocaleString() })
    : t('nvram.metrics.corruptedSubClean');
}

function getCorruptedBadgeLabel(
  report: NvramCleanReport | null,
  cleanSuccess: boolean,
  isCorrupted: boolean,
  t: ReturnType<typeof useTranslation>['t'],
): string {
  if (cleanSuccess) return t('nvram.metrics.badgeZeroCorrupted');
  if (isCorrupted) {
    return t('nvram.metrics.badgeBytesU99', { bytes: report?.nvram_non_ff_bytes ?? 0 });
  }
  return t('nvram.metrics.badgeNoCorruption');
}

function getFlashSizeDisplay(
  report: NvramCleanReport | null,
  flashBytes: number,
  flashKb: number,
  t: ReturnType<typeof useTranslation>['t'],
) {
  const flashMb = (flashBytes / MIB_BYTES).toFixed(0);
  const flashMbit = flashBytes > 0 ? (flashBytes * BITS_PER_BYTE) / MIB_BYTES : 0;
  return {
    value: flashKb > 0 ? flashKb.toLocaleString() : '---',
    unit: flashKb > 0 ? t('nvram.metrics.unitKb') : undefined,
    subtitle:
      flashBytes > 0
        ? t('nvram.metrics.flashSizeSub', {
            mb: flashMb,
            addr: (report?.nvram_offset_start ?? 0).toString(16).toUpperCase(),
          })
        : t('nvram.metrics.flashWaitingSub'),
    badgeLabel:
      flashMbit > 0
        ? t('nvram.metrics.flashMbitBadge', { mbit: flashMbit })
        : t('nvram.metrics.flashDetectingBadge'),
  };
}

function getInspectedSectorDisplay(
  selectedSector: FlashSectorInfo | null,
  t: ReturnType<typeof useTranslation>['t'],
): {
  readonly value: string;
  readonly subtitle: string;
  readonly badgeVariant: StatusVariant;
  readonly badgeLabel: string;
} {
  const badgeVariant: StatusVariant = selectedSector?.is_corrupted_area ? 'warning' : 'neutral';
  return {
    value: selectedSector?.name ?? '---',
    subtitle: selectedSector
      ? t('nvram.metrics.inspectedSub', {
          offset: selectedSector.start_offset
            .toString(16)
            .toUpperCase()
            .padStart(HEX32_PAD_WIDTH, '0'),
          kb: (selectedSector.size_bytes / KIB_BYTES).toFixed(0),
        })
      : t('nvram.metrics.inspectedPrompt'),
    badgeVariant,
    badgeLabel: selectedSector?.sector_type.toUpperCase() ?? t('nvram.metrics.badgeNone'),
  };
}

export const NvramMetricsGrid: React.FC<NvramMetricsGridProps> = ({
  report,
  selectedSector,
  cleanSuccess,
}) => {
  const { t } = useTranslation();
  const flashBytes = report?.total_flash_bytes ?? 0;
  const flashKb = Math.round(flashBytes / KIB_BYTES);
  const totalMicrosectors = report?.microsectors.length ?? SPANSION_MICROSECTOR_COUNT;
  const corruptedCount = cleanSuccess
    ? 0
    : (report?.microsectors.filter((m) => m.active_bytes > 0).length ?? 0);
  const isCorrupted = !cleanSuccess && corruptedCount > 0;

  const statusCard = getNvramStatusDisplay(report, cleanSuccess, isCorrupted, corruptedCount, t);
  const flashCard = getFlashSizeDisplay(report, flashBytes, flashKb, t);
  const sectorCard = getInspectedSectorDisplay(selectedSector, t);
  const activeTone: MetricVariant = cleanSuccess ? 'emerald' : isCorrupted ? 'rose' : 'cyan';
  const activeBadgeTone: StatusVariant = cleanSuccess ? 'success' : isCorrupted ? 'error' : 'neutral';

  const statusIcon = cleanSuccess ? (
    <ShieldCheck className="w-5 h-5 text-emerald-400" />
  ) : (
    <AlertOctagon className={`w-5 h-5 ${isCorrupted ? 'text-rose-400' : 'text-cyan-400'}`} />
  );

  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
      <MetricCard
        title={t('nvram.metrics.statusTitle')}
        value={statusCard.value}
        subtitle={statusCard.subtitle}
        variant={statusCard.variant}
        icon={statusIcon}
        badge={<StatusBadge variant={statusCard.badgeVariant} label={statusCard.badgeLabel} />}
      />

      <MetricCard
        title={t('nvram.metrics.flashSizeTitle')}
        value={flashCard.value}
        unit={flashCard.unit}
        subtitle={flashCard.subtitle}
        variant="amber"
        icon={<Layers className="w-5 h-5 text-amber-400" />}
        badge={<StatusBadge variant="warning" label={flashCard.badgeLabel} />}
      />

      <MetricCard
        title={t('nvram.metrics.corruptedTitle')}
        value={report ? corruptedCount : '---'}
        unit={t('nvram.metrics.unitMicrosectors', { count: totalMicrosectors })}
        subtitle={getCorruptedCardSubtitle(report, cleanSuccess, isCorrupted, t)}
        variant={activeTone}
        badge={
          <StatusBadge
            variant={activeBadgeTone}
            label={getCorruptedBadgeLabel(report, cleanSuccess, isCorrupted, t)}
          />
        }
      />

      <MetricCard
        title={t('nvram.metrics.inspectedTitle')}
        value={sectorCard.value}
        subtitle={sectorCard.subtitle}
        variant="cyan"
        icon={<ArrowRight className="w-5 h-5 text-cyan-400" />}
        badge={<StatusBadge variant={sectorCard.badgeVariant} label={sectorCard.badgeLabel} />}
      />
    </div>
  );
};
