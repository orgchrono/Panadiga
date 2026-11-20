import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import { ToastProvider } from '@/context/ToastContext';
import i18n from '@/i18n/i18n';
import { usePicoBgoDavisViewModel } from '@/viewmodels/pipelines';
import { BgoFtlBenchPanel } from './BgoFtlBenchPanel';
import { DavisIdeProbePanel } from './DavisIdeProbePanel';

afterEach(async () => {
  await act(async () => {
    await i18n.changeLanguage('en-US');
  });
});

function PicoBgoDavisHarness({ onOpenHdd }: { readonly onOpenHdd: () => void }) {
  const { bgo, davis } = usePicoBgoDavisViewModel('COM10', onOpenHdd);
  return (
    <div className="space-y-4">
      <BgoFtlBenchPanel vm={bgo} />
      <DavisIdeProbePanel vm={davis} />
    </div>
  );
}

describe('BgoFtlBenchPanel & DavisIdeProbePanel Components', () => {
  it('completes 1-click NOR dump, BGO 40/820 audit, dry-run/live repair, and DAVIS CH1 probe with HDD navigation', async () => {
    const onOpenHdd = vi.fn();
    render(
      <ToastProvider>
        <PicoBgoDavisHarness onOpenHdd={onOpenHdd} />
      </ToastProvider>,
    );

    fireEvent.click(screen.getByTestId('bgo-nor-dump-btn'));
    expect(await screen.findByText(/eh55_nor_bgo_cow\.bin/)).toBeTruthy();

    fireEvent.click(screen.getByTestId('bgo-audit-btn'));
    expect(await screen.findByText(/40\/40.*820\/820/)).toBeTruthy();

    fireEvent.click(screen.getByTestId('bgo-dry-run-btn'));
    expect(await screen.findByText(/PURE 1->0 TRANSITION VERIFIED/i)).toBeTruthy();

    fireEvent.click(screen.getByTestId('preflight-check-cowBackupSession'));
    fireEvent.click(screen.getByTestId('preflight-check-upperNorUnlock'));
    fireEvent.click(screen.getByTestId('preflight-check-noBootInterrupt'));
    fireEvent.change(screen.getByTestId('bgo-confirm-input'), {
      target: { value: 'CONFIRM_UPPER_NOR' },
    });
    fireEvent.click(screen.getByTestId('bgo-live-repair-btn'));
    expect(await screen.findByText(/BOOTTRACE 15000 NOJIG -> BOOT NORMAL/)).toBeTruthy();

    fireEvent.click(screen.getByTestId('davis-probe-btn'));
    expect(await screen.findByText('ST3250820ACE')).toBeTruthy();
    expect(screen.getByText('EH55 DAVIS COMPATIBLE')).toBeTruthy();

    fireEvent.click(screen.getByTestId('davis-open-hdd-btn'));
    expect(onOpenHdd).toHaveBeenCalledTimes(1);
  });
});
