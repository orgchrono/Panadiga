import React from 'react';
import { act, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ToastProvider } from '@/context/ToastContext';
import { Provider } from 'react-redux';
import { createAppStore } from '@/store';
import {
  installAndroidIrTestBridge,
  installAndroidNativeTestBridge,
} from '@/test/mocks/testHostBridges';
import { useIrCommanderViewModel } from './useIrCommanderViewModel';

const Wrapper: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const store = React.useMemo(() => createAppStore(), []);
  return (
    <Provider store={store}>
      <ToastProvider>{children}</ToastProvider>
    </Provider>
  );
};

describe('useIrCommanderViewModel', () => {
  const transmitSpy = vi.fn(() =>
    JSON.stringify({ ok: true, backendUsed: 'Exynos sec_ir (PERIOD_SCALED)' }),
  );
  const usbCmdSpy = vi.fn(() => JSON.stringify({ ok: true, output: 'IRTX: OK' }));
  const connectUsbSpy = vi.fn(() =>
    JSON.stringify({ connected: true, deviceName: '/dev/bus/usb/001/002', message: 'Connected' }),
  );

  beforeEach(() => {
    transmitSpy.mockClear();
    usbCmdSpy.mockClear();
    connectUsbSpy.mockClear();

    installAndroidIrTestBridge({
      isAvailable: () => true,
      getCapabilitiesJson: () =>
        JSON.stringify({
          hasEmitter: true,
          serviceFound: true,
          hasSecIrNode: true,
          hasFpgaNode: true,
          suAvailable: true,
          ranges: [{ min_hz: 36000, max_hz: 40000 }],
          device: {
            manufacturer: 'samsung',
            model: 'SM-T805M',
            hardware: 'universal5420',
            board: 'chagalllte',
            product: 'chagalllteub',
            device: 'chagalllte',
            android_release: '13',
            sdk_int: 33,
          },
        }),
      getSettingsJson: () =>
        JSON.stringify({
          timingUnitOverride: 'AUTO',
          repeatCount: 2,
          customProfilesJson: '[]',
        }),
      saveSettings: () => JSON.stringify({ ok: true }),
      saveCustomProfilesJson: () => true,
      transmitPreparedJson: transmitSpy,
    });

    installAndroidNativeTestBridge({
      isAvailable: () => true,
      getCapabilitiesJson: () =>
        JSON.stringify({
          platform: 'android',
          usbHostSupported: true,
          bleSupported: true,
          bluetoothEnabled: true,
        }),
      requestPermissions: () => true,
      listUsbOtgDevicesJson: () => '[]',
      connectUsbOtg: connectUsbSpy,
      disconnectUsbOtg: () => undefined,
      sendUsbOtgCommand: usbCmdSpy,
      connectBleNus: () => JSON.stringify({ connected: false, message: 'No BLE' }),
      sendBleCommand: () => JSON.stringify({ ok: false, output: '' }),
      disconnectBleNus: () => undefined,
    });
  });

  afterEach(() => {
    installAndroidIrTestBridge(undefined);
    installAndroidNativeTestBridge(undefined);
  });

  it('loads offline catalog stats, deck profile, verified macros and transmits via Android HAL', async () => {
    const { result } = renderHook(() => useIrCommanderViewModel(), { wrapper: Wrapper });

    await waitFor(() => {
      expect(result.current.totalCodesets).toBe(1763);
      expect(result.current.activeDeckProfile?.keys.length).toBeGreaterThan(0);
      expect(result.current.verifiedMacros.length).toBeGreaterThan(0);
    });

    const firstKey = result.current.activeDeckProfile?.keys[0];
    expect(firstKey).toBeDefined();
    if (!firstKey) return;

    await act(async () => {
      await result.current.triggerKey(firstKey);
    });

    expect(transmitSpy).toHaveBeenCalledTimes(1);
    expect(result.current.state.lastWireHex).toBe('02 20 B0 00 3D 8D');
    expect(result.current.state.lastBackendUsed).toContain('Exynos sec_ir');
  });

  it('routes IRTX command through Pico USB-OTG when PICO_USB_OTG transport is selected', async () => {
    const { result } = renderHook(() => useIrCommanderViewModel(), { wrapper: Wrapper });

    await waitFor(() => {
      expect(result.current.activeDeckProfile?.keys.length).toBeGreaterThan(0);
    });

    act(() => {
      result.current.changeTransport('PICO_USB_OTG');
      result.current.changePicoGpio(16);
      result.current.changeRepeatCount(1);
    });

    const firstKey = result.current.activeDeckProfile?.keys[0];
    if (!firstKey) return;

    await act(async () => {
      await result.current.triggerKey(firstKey);
    });

    expect(connectUsbSpy).toHaveBeenCalled();
    expect(usbCmdSpy).toHaveBeenCalledWith('IRTX B000:3D GP16 1', 2500);
  });

  it('imports external Flipper .ir text and executes verified macro', async () => {
    const { result } = renderHook(() => useIrCommanderViewModel(), { wrapper: Wrapper });

    await waitFor(() => {
      expect(result.current.verifiedMacros.length).toBeGreaterThan(0);
    });

    await act(async () => {
      await result.current.importProfileText('flipper', 'name: Power', 'Flipper Test');
    });

    expect(result.current.state.activeProfile?.name).toBe('Flipper Test');

    await act(async () => {
      await result.current.exportCurrentIrplus();
    });

    expect(result.current.exportedXml).toContain('<irplus>');

    const macro = result.current.verifiedMacros[0];
    await act(async () => {
      await result.current.executeMacro(macro);
    });

    expect(transmitSpy).toHaveBeenCalled();
  });
});
