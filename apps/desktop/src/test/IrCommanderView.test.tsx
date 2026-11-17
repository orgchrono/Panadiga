import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { Provider } from 'react-redux';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ToastProvider } from '@/context/ToastContext';
import { WorkbenchModeProvider } from '@/context/WorkbenchModeContext';
import { createAppStore } from '@/store';
import {
  IS_ANDROID_APK_BUILD,
  isAndroidCommanderEnabled,
} from '@/config/platform';
import {
  installAndroidIrTestBridge,
  setAndroidRuntimeTestOverride,
} from '@/test/mocks/testHostBridges';
import { IrCommanderView } from '@/views/IrCommanderView';

function renderCommander() {
  const store = createAppStore();
  return render(
    <Provider store={store}>
      <WorkbenchModeProvider>
        <ToastProvider>
          <IrCommanderView />
        </ToastProvider>
      </WorkbenchModeProvider>
    </Provider>,
  );
}

describe('IrCommanderView & Platform Gating', () => {
  afterEach(() => {
    setAndroidRuntimeTestOverride(false);
    installAndroidIrTestBridge(undefined);
  });

  it('keeps DIGA Commander disabled on non-Android desktop builds by default', () => {
    expect(IS_ANDROID_APK_BUILD).toBe(false);
    expect(isAndroidCommanderEnabled()).toBe(false);
    setAndroidRuntimeTestOverride(true);
    expect(isAndroidCommanderEnabled()).toBe(true);
  });

  it('renders React/Radix DIGA Commander tabs, transmits keys, and guards editing', async () => {
    const txSpy = vi.fn(() => JSON.stringify({ ok: true, backendUsed: 'Android CIR' }));
    installAndroidIrTestBridge({
      isAvailable: () => true,
      getCapabilitiesJson: () =>
        JSON.stringify({
          hasEmitter: true,
          serviceFound: true,
          hasSecIrNode: false,
          hasFpgaNode: false,
          suAvailable: false,
          ranges: [{ min_hz: 36000, max_hz: 40000 }],
          device: {
            manufacturer: 'Test',
            model: 'Unit',
            hardware: 'qcom',
            board: 'generic',
            product: 'generic',
            device: 'generic',
            android_release: '14',
            sdk_int: 34,
          },
        }),
      getSettingsJson: () =>
        JSON.stringify({ timingUnitOverride: 'AUTO', repeatCount: 1, customProfilesJson: '[]' }),
      saveSettings: () => JSON.stringify({ ok: true }),
      saveCustomProfilesJson: () => true,
      transmitPreparedJson: txSpy,
    });

    renderCommander();

    await waitFor(() => {
      expect(screen.getByTestId('ir-key-curated_diga_eh55_eur7659_0')).toBeTruthy();
    });

    fireEvent.click(screen.getByTestId('ir-key-curated_diga_eh55_eur7659_0'));
    await waitFor(() => {
      expect(txSpy).toHaveBeenCalledTimes(1);
    });

    fireEvent.click(screen.getByTestId('ir-remote-edit-switch'));
    fireEvent.click(screen.getByTestId('ir-key-curated_diga_eh55_eur7659_0'));

    await waitFor(() => {
      expect(screen.getByTestId('ir-edit-label-input')).toBeTruthy();
    });
    fireEvent.change(screen.getByTestId('ir-edit-label-input'), {
      target: { value: 'Power (Renamed)' },
    });
    fireEvent.click(screen.getByTestId('ir-edit-save-btn'));

    await waitFor(() => {
      expect(screen.getByTestId('ir-key-curated_diga_eh55_eur7659_0').textContent).toContain(
        'Power (Renamed)',
      );
    });
  });
});
