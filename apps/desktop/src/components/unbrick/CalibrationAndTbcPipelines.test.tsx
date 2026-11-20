import { describe, it, expect, afterEach } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import { ToastProvider } from '@/context/ToastContext';
import i18n from '@/i18n/i18n';
import { useCalibrationViewModel, useTbcSdHardenViewModel } from '@/viewmodels/pipelines';
import { CalibrationRegionPipeline } from './CalibrationRegionPipeline';
import { TbcLabAndSdHardenPipeline } from './TbcLabAndSdHardenPipeline';

afterEach(async () => {
  await act(async () => {
    await i18n.changeLanguage('en-US');
  });
});

function CalibrationHarness() {
  const vm = useCalibrationViewModel();
  return <CalibrationRegionPipeline vm={vm} />;
}

function TbcSdHarness() {
  const vm = useTbcSdHardenViewModel();
  return <TbcLabAndSdHardenPipeline vm={vm} />;
}

describe('Calibration & TBC/SD Hardening Pipelines', () => {
  it('inspects region calibration bundle and applies CoW patch behind confirm gate', async () => {
    render(
      <ToastProvider>
        <CalibrationHarness />
      </ToastProvider>,
    );

    expect(screen.getAllByText(/METADATA.*DEV.*OEM/i).length).toBeGreaterThanOrEqual(1);
    fireEvent.click(screen.getByTestId('calibration-inspect-btn'));
    expect(await screen.findByText('AIOF_VMODESEL')).toBeTruthy();
    expect(screen.getAllByText(/0x8021/).length).toBeGreaterThanOrEqual(1);

    fireEvent.click(screen.getByTestId('preflight-check-distinctCow'));
    fireEvent.click(screen.getByTestId('preflight-check-backupReady'));
    fireEvent.click(screen.getByTestId('preflight-check-timerMcuAck'));

    fireEvent.change(screen.getByTestId('calibration-confirm-input'), {
      target: { value: 'CALIBRATE_REGION_COW' },
    });
    fireEvent.click(screen.getByTestId('calibration-apply-btn'));

    expect(await screen.findByText(/ifodev2_calibrated\.bin/)).toBeTruthy();
  });

  it('applies standalone 5 TBC bypasses and builds hardened ver0.61 SD kit', async () => {
    render(
      <ToastProvider>
        <TbcSdHarness />
      </ToastProvider>,
    );

    fireEvent.click(screen.getByTestId('tbc-standalone-preset-btn'));
    fireEvent.click(screen.getByTestId('tbc-rsp-plan-btn'));
    expect(await screen.findByText(/MAGIC 0e0402/)).toBeTruthy();

    fireEvent.change(screen.getByTestId('tbc-confirm-input'), {
      target: { value: 'APPLY_MAIN_TBC_COW' },
    });
    fireEvent.click(screen.getByTestId('tbc-apply-btn'));
    expect(await screen.findByText(/MAIN3410PQ_STANDALONE_TBC\.bin/)).toBeTruthy();

    fireEvent.click(screen.getByTestId('sd-audit-disk-btn'));
    expect((await screen.findAllByText(/FAT16/)).length).toBeGreaterThanOrEqual(1);

    fireEvent.click(screen.getByTestId('preflight-check-autoFormatD4D5'));
    fireEvent.click(screen.getByTestId('preflight-check-sd91006Fat16'));
    fireEvent.click(screen.getByTestId('preflight-check-p59001P7402'));
    fireEvent.click(screen.getByTestId('sd-harden-build-btn'));

    expect(await screen.findByText('DUAL PARITY OK')).toBeTruthy();
  });
});
