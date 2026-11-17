import type { ReactNode } from 'react';
import { act, fireEvent, render, renderHook, screen, waitFor } from '@testing-library/react';
import { Provider } from 'react-redux';
import { beforeEach, describe, expect, it } from 'vitest';
import { ToastProvider } from '@/context/ToastContext';
import { WorkbenchModeProvider } from '@/context/WorkbenchModeContext';
import { createAppStore } from '@/store';
import { SettingsView } from '@/views/SettingsView';
import { useRuntimeSettingsViewModel } from './useRuntimeSettingsViewModel';

function createWrapper() {
  const store = createAppStore();
  return function SettingsTestWrapper({ children }: { readonly children: ReactNode }) {
    return (
      <Provider store={store}>
        <WorkbenchModeProvider>
          <ToastProvider>{children}</ToastProvider>
        </WorkbenchModeProvider>
      </Provider>
    );
  };
}

describe('useRuntimeSettingsViewModel & SettingsView', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it('validates hardware GPIO safety and persists valid runtime defaults via store & Zod schema', async () => {
    const wrapper = createWrapper();
    const { result } = renderHook(() => useRuntimeSettingsViewModel(), { wrapper });

    expect(result.current.runtimeSettings.serial.default_baud_rate).toBe(115200);
    expect(result.current.runtimeSettings.ir.default_ir_pico_gpio).toBe(6);

    act(() => {
      result.current.updateIrField('default_ir_pico_gpio', 3);
    });

    await act(async () => {
      await result.current.saveRuntimeConfig();
    });

    expect(result.current.error).toBeTruthy();
    expect(result.current.runtimeSettings.ir.default_ir_pico_gpio).toBe(6);

    act(() => {
      result.current.updateIrField('default_ir_pico_gpio', 16);
      result.current.updateSerialField('default_baud_rate', 57600);
      result.current.updateWorkbenchField('log_level', 'debug');
    });

    await act(async () => {
      await result.current.saveRuntimeConfig();
    });

    expect(result.current.error).toBeNull();
    expect(result.current.runtimeSettings.ir.default_ir_pico_gpio).toBe(16);
    expect(result.current.runtimeSettings.serial.default_baud_rate).toBe(57600);
    expect(result.current.runtimeSettings.workbench.log_level).toBe('debug');

    await act(async () => {
      await result.current.resetRuntimeDefaults();
    });

    expect(result.current.runtimeSettings.serial.default_baud_rate).toBe(115200);
    expect(result.current.runtimeSettings.ir.default_ir_pico_gpio).toBe(6);
  });

  it('renders RuntimeDefaultsCard inside SettingsView and saves modified baud rate cleanly', async () => {
    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <SettingsView />
      </Wrapper>,
    );

    const baudSelect = screen.getByLabelText(/Baud Rate/i);
    fireEvent.change(baudSelect, { target: { value: '57600' } });

    const saveBtn = screen.getByTestId('runtime-settings-save-btn');
    expect(saveBtn).toHaveProperty('disabled', false);
    await act(async () => {
      fireEvent.click(saveBtn);
    });

    await waitFor(() => {
      expect(screen.getByTestId('runtime-settings-save-btn')).toHaveProperty('disabled', true);
      expect(screen.getByTestId('runtime-settings-reset-btn')).toHaveProperty('disabled', false);
      expect(screen.getByLabelText(/Baud Rate/i)).toHaveProperty('value', '57600');
      expect(screen.getByRole('status').textContent).toMatch(
        /validated and saved atomically|validados e gravados atomicamente/i,
      );
    });
  });
});
