/**
 * @file viewmodels/picoDevice/metricsPresenter.ts
 * @brief Pure functional presenter deriving telemetry MetricCard view specs for PicoBenchMetrics.
 */

import type { HardwareSsotSpec, PicoDeviceStatus } from '@/models';
import type { StatusVariant } from '@/components/StatusBadge';
import type { TranslationKey } from '@/i18n/useTranslation';
import {
  formatVoltage,
  isTargetPowered,
  isTargetVoltageNominal,
} from '@/utils/formatters';

export type MetricCardVariant = 'emerald' | 'amber' | 'cyan' | 'rose' | 'slate';

export type TranslateFn = (
  key: TranslationKey,
  fallbackOrOptions?: string | Record<string, unknown>
) => string;

export interface PicoBenchMetricCardSpec {
  key: 'rail' | 'core' | 'bus_wdt' | 'breakin';
  title: string;
  value: string;
  unit?: string;
  subtitle: string;
  variant: MetricCardVariant;
  badge?: {
    variant: StatusVariant;
    label: string;
  };
}

export interface PicoBenchMetricsInput {
  deviceStatus: PicoDeviceStatus | null;
  ssot: HardwareSsotSpec;
  detectedFlashMb?: number | null;
  t: TranslateFn;
}

function buildRailMetricCard({
  deviceStatus,
  ssot,
  t,
}: PicoBenchMetricsInput): PicoBenchMetricCardSpec {
  const { thresholds } = ssot;
  const vdd = deviceStatus?.vdd_volts ?? 0;
  const isPowered = isTargetPowered(vdd, thresholds.min_vdd_threshold_volts);
  const isNominal = isTargetVoltageNominal(
    vdd,
    thresholds.safe_operating_min_volts,
    thresholds.max_vdd_ceiling_volts
  );
  const voltageStr = formatVoltage(vdd);
  const thresholdStr = thresholds.min_vdd_threshold_volts.toFixed(1);

  if (isNominal) {
    return {
      key: 'rail',
      title: t('pico.metrics.vddTitle'),
      value: voltageStr,
      unit: t('pico.metrics.vddUnit'),
      subtitle: t('pico.metrics.railNominal'),
      variant: 'emerald',
      badge: { variant: 'success', label: t('pico.metrics.badgePowered') },
    };
  }
  if (isPowered) {
    return {
      key: 'rail',
      title: t('pico.metrics.vddTitle'),
      value: voltageStr,
      unit: t('pico.metrics.vddUnit'),
      subtitle: t('pico.metrics.railActive', { voltage: voltageStr, threshold: thresholdStr }),
      variant: 'amber',
      badge: { variant: 'warning', label: t('pico.metrics.badgeLowRail') },
    };
  }
  return {
    key: 'rail',
    title: t('pico.metrics.vddTitle'),
    value: voltageStr,
    unit: t('pico.metrics.vddUnit'),
    subtitle: t('pico.metrics.railLow', { threshold: thresholdStr }),
    variant: 'rose',
    badge: { variant: 'error', label: t('pico.metrics.badgeUnpowered') },
  };
}

function buildCoreMetricCard({
  deviceStatus,
  detectedFlashMb,
  t,
}: PicoBenchMetricsInput): PicoBenchMetricCardSpec {
  const coreState = deviceStatus?.core_state ?? 'DISCONNECTED';
  const badge =
    coreState === 'HALTED'
      ? { variant: 'info' as const, label: t('pico.metrics.badgeOcdActive') }
      : coreState === 'RUNNING'
        ? { variant: 'success' as const, label: t('pico.metrics.badgeReady') }
        : { variant: 'neutral' as const, label: t('pico.metrics.badgeStandby') };

  return {
    key: 'core',
    title: t('pico.metrics.coreTitle'),
    value: coreState,
    subtitle: detectedFlashMb
      ? t('pico.metrics.flashDetected', { mb: detectedFlashMb })
      : t('pico.metrics.uartChannel'),
    variant: coreState === 'HALTED' ? 'cyan' : 'slate',
    badge,
  };
}

function buildBusWdtMetricCard({
  deviceStatus,
  ssot,
  t,
}: PicoBenchMetricsInput): PicoBenchMetricCardSpec {
  const mhzStr = (ssot.thresholds.bus_freq_hz / 1_000_000).toFixed(2);
  if (!deviceStatus?.wdt_active) {
    return {
      key: 'bus_wdt',
      title: t('pico.metrics.busFreqTitle'),
      value: mhzStr,
      unit: t('pico.metrics.busFreqUnit'),
      subtitle: t('pico.metrics.legacyChannel'),
      variant: 'slate',
    };
  }

  const timeout = deviceStatus.wdt_timeout_ms ?? 4000;
  const recoveries = deviceStatus.wdt_recoveries ?? 0;
  const src = deviceStatus.wdt_reboot_src ?? 'CLEAN';
  const faultStage = deviceStatus.wdt_fault_stage;
  const isRecovered = recoveries > 0 && src === 'WDT_RECOVERY';
  const showStage = Boolean(faultStage && (faultStage !== 'IDLE' || isRecovered));

  const subtitle = showStage
    ? t('pico.metrics.wdtStatusWithStage', {
        timeout,
        src,
        stage: faultStage,
        rec: recoveries,
      })
    : t('pico.metrics.wdtStatus', {
        timeout,
        src,
        rec: recoveries,
      });

  return {
    key: 'bus_wdt',
    title: t('pico.metrics.busFreqTitle'),
    value: mhzStr,
    unit: t('pico.metrics.busFreqUnit'),
    subtitle,
    variant: isRecovered ? 'amber' : 'slate',
    badge: {
      variant: isRecovered ? 'warning' : 'success',
      label: t('pico.metrics.wdtBadge', { timeout }),
    },
  };
}

function buildBreakinMetricCard({
  deviceStatus,
  t,
}: PicoBenchMetricsInput): PicoBenchMetricCardSpec {
  const mode = deviceStatus?.breakin_mode ?? 2;
  const isRecommended = mode === 2;
  return {
    key: 'breakin',
    title: t('pico.metrics.breakinTitle'),
    value: t('pico.metrics.breakinValue', { mode }),
    subtitle: t('pico.metrics.breakinSubtitle'),
    variant: 'emerald',
    badge: {
      variant: isRecommended ? 'success' : 'info',
      label: isRecommended
        ? t('pico.metrics.badgeRecommended')
        : t('pico.metrics.badgeCustom'),
    },
  };
}

export function derivePicoBenchMetricsPresentation(
  input: PicoBenchMetricsInput
): readonly PicoBenchMetricCardSpec[] {
  return [
    buildRailMetricCard(input),
    buildCoreMetricCard(input),
    buildBusWdtMetricCard(input),
    buildBreakinMetricCard(input),
  ];
}
