import React from 'react';
import type { HardwareSsotSpec, PicoDeviceStatus } from '@/models';
import { MetricCard } from '@/components/MetricCard';
import { StatusBadge } from '@/components/StatusBadge';
import { EmptyState } from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import { derivePicoBenchMetricsPresentation } from '@/viewmodels/picoDevice/metricsPresenter';

export interface PicoBenchMetricsProps {
  deviceStatus: PicoDeviceStatus | null;
  ssot: HardwareSsotSpec;
  detectedFlashMb?: number | null;
}

/**
 * Pure Passive MVVM View: Renders pre-computed hardware telemetry cards
 * produced by `derivePicoBenchMetricsPresentation`.
 */
export const PicoBenchMetrics: React.FC<PicoBenchMetricsProps> = ({
  deviceStatus,
  ssot,
  detectedFlashMb,
}) => {
  const { t } = useTranslation();
  const cards = derivePicoBenchMetricsPresentation({
    deviceStatus,
    ssot,
    detectedFlashMb,
    t,
  });

  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
      {cards.length === 0 ? (
        <EmptyState title={t('pico.metrics.empty')} className="sm:col-span-2 lg:col-span-4 p-4" />
      ) : (
        cards.map(card => (
          <MetricCard
            key={card.key}
            title={card.title}
            value={card.value}
            unit={card.unit}
            subtitle={card.subtitle}
            variant={card.variant}
            badge={
              card.badge ? (
                <StatusBadge variant={card.badge.variant} label={card.badge.label} />
              ) : undefined
            }
          />
        ))
      )}
    </div>
  );
};
