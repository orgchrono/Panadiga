import { describe, it, expect, beforeEach } from 'vitest';
import { renderHook } from '@testing-library/react';
import { setGlobalLocale, useTranslation } from '@/i18n/useTranslation';
import { buildMockPicoStatus } from '@/test/mocks/mockPicoHandlers';
import { buildCanonicalHardwareSsotSpec } from '@/constants/hardwareSpecs';
import { derivePicoBenchMetricsPresentation } from './metricsPresenter';

describe('derivePicoBenchMetricsPresentation (MVVM strict presenter)', () => {
  beforeEach(() => {
    setGlobalLocale('pt-BR');
  });

  it('derives nominal powered, flash MB, and WDT fault_stage recovery warning cleanly', () => {
    const { result } = renderHook(() => useTranslation());
    const ssot = buildCanonicalHardwareSsotSpec('pt-BR');
    const deviceStatus = {
      ...buildMockPicoStatus('COM10'),
      connected: true,
      vdd_volts: 3.24,
      core_state: 'RUNNING' as const,
      wdt_active: true,
      wdt_timeout_ms: 4000,
      wdt_recoveries: 1,
      wdt_reboot_src: 'WDT_RECOVERY',
      wdt_fault_stage: 'PROBE_SWEEP',
      breakin_mode: 2 as const,
    };

    const cards = derivePicoBenchMetricsPresentation({
      deviceStatus,
      ssot,
      detectedFlashMb: 8,
      t: result.current.t,
    });

    expect(cards).toHaveLength(4);
    expect(cards[0].subtitle).toBe('Trilho de 3.3V Estabilizado (Nominal)');
    expect(cards[0].badge?.label).toBe('POWERED');
    expect(cards[1].subtitle).toContain('Flash NOR Detectada: 8 MB');
    expect(cards[2].variant).toBe('amber');
    expect(cards[2].subtitle).toContain('WDT_RECOVERY [PROBE_SWEEP] (1 rec)');
    expect(cards[3].value).toBe('Modo 2');
    expect(cards[3].badge?.label).toBe('RECOMENDADO');
  });

  it('handles unpowered rail and custom break-in mode when disconnected', () => {
    const { result } = renderHook(() => useTranslation());
    const ssot = buildCanonicalHardwareSsotSpec('pt-BR');
    const cards = derivePicoBenchMetricsPresentation({
      deviceStatus: {
        ...buildMockPicoStatus(),
        connected: false,
        vdd_volts: 0.4,
        core_state: 'DISCONNECTED' as const,
        wdt_active: false,
        breakin_mode: 1 as const,
      },
      ssot,
      detectedFlashMb: null,
      t: result.current.t,
    });

    expect(cards[0].variant).toBe('rose');
    expect(cards[0].badge?.label).toBe('UNPOWERED');
    expect(cards[2].badge).toBeUndefined();
    expect(cards[3].badge?.label).toBe('CUSTOM');
  });
});
