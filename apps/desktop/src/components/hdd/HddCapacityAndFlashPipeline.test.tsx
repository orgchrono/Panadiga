import { describe, it, expect, afterEach } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import { ToastProvider } from '@/context/ToastContext';
import i18n from '@/i18n/i18n';
import { useHddSafeFlashViewModel } from '@/viewmodels/pipelines';
import { HddCapacityAndFlashPipeline } from './HddCapacityAndFlashPipeline';

afterEach(async () => {
  await act(async () => {
    await i18n.changeLanguage('en-US');
  });
});

function HddSafeFlashHarness() {
  const vm = useHddSafeFlashViewModel();
  return <HddCapacityAndFlashPipeline vm={vm} />;
}

describe('HddCapacityAndFlashPipeline Component', () => {
  it('supports 5 GB SAMSUNG SV0511D preset, locks Disk 0 / NVMe, and flashes PhysicalDrive2 with SHA-256 read-back', async () => {
    render(
      <ToastProvider>
        <HddSafeFlashHarness />
      </ToastProvider>,
    );

    const disk0Btn = await screen.findByTestId('hdd-safe-disk-0');
    const disk1Btn = screen.getByTestId('hdd-safe-disk-1');
    const disk2Btn = screen.getByTestId('hdd-safe-disk-2');
    expect(disk0Btn instanceof HTMLButtonElement && disk0Btn.disabled).toBe(true);
    expect(disk1Btn instanceof HTMLButtonElement && disk1Btn.disabled).toBe(true);
    expect(disk2Btn instanceof HTMLButtonElement && disk2Btn.disabled).toBe(false);

    fireEvent.click(screen.getByTestId('hdd-safe-cap-5'));
    expect(screen.getByTestId('hdd-safe-5gb-badge')).toBeTruthy();
    expect(screen.getByText(/0x24C388/)).toBeTruthy();

    fireEvent.click(screen.getByTestId('preflight-check-backup64k'));
    fireEvent.click(screen.getByTestId('preflight-check-wipeLayout'));
    fireEvent.click(screen.getByTestId('preflight-check-capacityRange'));

    fireEvent.change(screen.getByTestId('hdd-safe-confirm-input'), {
      target: { value: 'CONFIRM-PhysicalDrive2' },
    });
    fireEvent.click(screen.getByTestId('hdd-safe-flash-btn'));

    expect(await screen.findByText('READ-BACK SHA-256 VERIFIED')).toBeTruthy();
    expect(screen.getByText(/pre_flash_64k_backup\.bin/)).toBeTruthy();
  });
});
