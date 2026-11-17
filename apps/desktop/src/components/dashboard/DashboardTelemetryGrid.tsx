import React from 'react';
import { MetricCard, type MetricVariant } from '@/components/MetricCard';
import { StatusBadge, type StatusVariant } from '@/components/StatusBadge';
import { MIB_BYTES } from '@/constants';
import type { TranslationKey } from '@/i18n/translations';
import { useTranslation } from '@/i18n/useTranslation';
import { HardDrive, Cpu, Radio, Lock } from 'lucide-react';
import type { BinaryFileStatus, SystemOverview } from '@/models';

export interface DashboardTelemetryGridProps {
  overview?: SystemOverview | null;
}

interface BinaryTelemetrySpec {
  readonly idToken: string;
  readonly digits: number;
  readonly titleKey: TranslationKey;
  readonly fallbackSubKey: TranslationKey;
  readonly presentVariant: MetricVariant;
  readonly presentBadgeVariant: StatusVariant;
  readonly presentBadgeKey: TranslationKey;
  readonly absentBadgeKey: TranslationKey;
  readonly showUnitWhenAbsent: boolean;
  readonly icon: React.ReactNode;
}

const BINARY_METRIC_SPECS: readonly BinaryTelemetrySpec[] = [
  {
    idToken: 'flash',
    digits: 2,
    titleKey: 'dashboard.metricSpansion',
    fallbackSubKey: 'dashboard.telemetry.flashFallbackSub',
    presentVariant: 'amber',
    presentBadgeVariant: 'warning',
    presentBadgeKey: 'dashboard.telemetry.flashPresentBadge',
    absentBadgeKey: 'dashboard.telemetry.missingBadge',
    showUnitWhenAbsent: true,
    icon: <Cpu className="w-4 h-4" />,
  },
  {
    idToken: 'hdd',
    digits: 1,
    titleKey: 'dashboard.metricHdd',
    fallbackSubKey: 'dashboard.telemetry.hddFallbackSub',
    presentVariant: 'cyan',
    presentBadgeVariant: 'success',
    presentBadgeKey: 'dashboard.telemetry.hddPresentBadge',
    absentBadgeKey: 'dashboard.telemetry.offlineBadge',
    showUnitWhenAbsent: true,
    icon: <HardDrive className="w-4 h-4" />,
  },
  {
    idToken: 'frm',
    digits: 2,
    titleKey: 'dashboard.metricFrm',
    fallbackSubKey: 'dashboard.telemetry.frmFallbackSub',
    presentVariant: 'slate',
    presentBadgeVariant: 'info',
    presentBadgeKey: 'dashboard.telemetry.frmPresentBadge',
    absentBadgeKey: 'dashboard.telemetry.missingBadge',
    showUnitWhenAbsent: false,
    icon: <Lock className="w-4 h-4" />,
  },
];

function findBinaryByToken(
  binaries: readonly BinaryFileStatus[] | undefined,
  token: string,
): BinaryFileStatus | undefined {
  return binaries?.find(
    (item) => item.id.includes(token) || item.name.toLowerCase().includes(token),
  );
}

function formatMebibytes(bin: BinaryFileStatus | undefined, digits: number): string {
  return bin?.size_bytes ? (bin.size_bytes / MIB_BYTES).toFixed(digits) : '--';
}

export const DashboardTelemetryGrid: React.FC<DashboardTelemetryGridProps> = ({ overview }) => {
  const { t } = useTranslation();

  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 font-mono">
      {BINARY_METRIC_SPECS.length === 0
        ? null
        : BINARY_METRIC_SPECS.map((spec) => {
            const bin = findBinaryByToken(overview?.binaries, spec.idToken);
            const exists = Boolean(bin?.exists);
            const unitLabel = exists || spec.showUnitWhenAbsent ? t('dashboard.telemetry.unitMb') : '';
            return (
              <MetricCard
                key={spec.idToken}
                title={t(spec.titleKey)}
                value={formatMebibytes(bin, spec.digits)}
                unit={unitLabel}
                subtitle={bin?.exists ? bin.name : t(spec.fallbackSubKey)}
                variant={exists ? spec.presentVariant : 'slate'}
                icon={spec.icon}
                badge={
                  <StatusBadge
                    variant={exists ? spec.presentBadgeVariant : 'neutral'}
                    label={t(exists ? spec.presentBadgeKey : spec.absentBadgeKey)}
                  />
                }
              />
            );
          })}
      <MetricCard
        title={overview?.debug_port || t('dashboard.telemetry.uartFallbackTitle')}
        value={t('dashboard.telemetry.uartBaudValue')}
        unit={t('dashboard.telemetry.unitKbps')}
        subtitle={t('dashboard.telemetry.uartSub')}
        variant="emerald"
        icon={<Radio className="w-4 h-4" />}
        badge={<StatusBadge variant="neutral" label={t('dashboard.telemetry.standbyBadge')} />}
      />
    </div>
  );
};
