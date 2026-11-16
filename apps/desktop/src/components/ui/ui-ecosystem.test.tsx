import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import React from 'react';
import { ToastProvider, useToast } from '@/context/ToastContext';
import { WorkbenchModeProvider, useWorkbenchMode } from '@/context/WorkbenchModeContext';
import { SettingsView } from '@/views/SettingsView';
import { emitLocalEvent } from '@/services/ipc';
import { store } from '@/store';
import { resetPipeline, resetWorkbenchPreferences } from '@/store/slices/workbenchSlice';
import { clearToasts } from '@/store/slices/toastSlice';
import {
  DraggableBenchHud,
  GuidedPipelineBar,
  RangeDndScrubber,
  ResizableSplitPane,
} from './index';

describe('React UI Ecosystem Primitives (Resizable, RangeDnD, Pipeline, HUD, Sonner)', () => {
  beforeEach(() => {
    window.localStorage.clear();
    store.dispatch(resetWorkbenchPreferences());
    store.dispatch(resetPipeline());
    store.dispatch(clearToasts());
  });

  it('ResizableSplitPane adjusts ratio via keyboard and resets on double-click', () => {
    render(
      <ResizableSplitPane
        initialLeftPercent={40}
        minLeftPercent={20}
        maxLeftPercent={70}
        storageKey="test.split"
        left={<div>Left Pane</div>}
        right={<div>Right Pane</div>}
      />
    );

    const separator = screen.getByRole('separator');
    expect(separator.getAttribute('aria-valuenow')).toBe('40');

    fireEvent.keyDown(separator, { key: 'ArrowRight' });
    expect(separator.getAttribute('aria-valuenow')).toBe('42');
    expect(window.localStorage.getItem('test.split')).toBe('42');

    fireEvent.doubleClick(separator);
    expect(separator.getAttribute('aria-valuenow')).toBe('40');
  });

  it('RangeDndScrubber applies presets and keyboard step bounds', () => {
    const onChangeRange = vi.fn();
    render(
      <RangeDndScrubber
        label="Setores Flash"
        min={0}
        max={70}
        step={1}
        startValue={10}
        endValue={30}
        presets={[{ label: 'NVRAM SA134..SA141', start: 63, end: 70 }]}
        onChangeRange={onChangeRange}
      />
    );

    fireEvent.click(screen.getByRole('button', { name: 'NVRAM SA134..SA141' }));
    expect(onChangeRange).toHaveBeenCalledWith(63, 70);

    const startSlider = screen.getByRole('slider', { name: 'Setores Flash início' });
    fireEvent.keyDown(startSlider, { key: 'ArrowRight' });
    expect(onChangeRange).toHaveBeenCalledWith(11, 30);
  });

  it('WorkbenchModeProvider toggles Basic/Advanced, HUD, and tracks reactive pipeline events', () => {
    const onNavigate = vi.fn();
    render(
      <WorkbenchModeProvider>
        <GuidedPipelineBar currentView="dashboard" onNavigate={onNavigate} />
        <DraggableBenchHud vddVolts={3.29} hardwareConnected={true} onNavigate={onNavigate} />
      </WorkbenchModeProvider>
    );

    const basicBtn = screen.getByRole('button', { name: 'Básico (Guiado)' });
    fireEvent.click(basicBtn);
    expect(basicBtn.getAttribute('aria-pressed')).toBe('true');
    expect(window.localStorage.getItem('panadiga.workbench.mode')).toBe('basic');

    expect(screen.queryByText(/HUD P7501/i)).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'HUD Pinout' }));
    expect(screen.getByText(/HUD P7501/i)).toBeTruthy();
    expect(screen.getByText(/VDD 3\.29V/)).toBeTruthy();

    act(() => {
      emitLocalEvent('nvram://cleaned', true);
      emitLocalEvent('hdd://inspected', true);
    });
    expect(screen.getByText(/\(2\/4 etapas concluídas\)/)).toBeTruthy();
  });

  it('ToastProvider exposes showToast and removeToast cleanly with sonner', () => {
    const Trigger: React.FC = () => {
      const { toasts, showToast, removeToast } = useToast();
      return (
        <div>
          <button
            type="button"
            onClick={() => showToast({ type: 'success', title: 'Operação OK', message: 'Sonner ativo', durationMs: 0 })}
          >
            Disparar
          </button>
          <button
            type="button"
            onClick={() => toasts[0] && removeToast(toasts[0].id)}
          >
            Remover
          </button>
          <span data-testid="toast-count">{toasts.length}</span>
        </div>
      );
    };

    render(
      <ToastProvider>
        <Trigger />
      </ToastProvider>
    );

    fireEvent.click(screen.getByRole('button', { name: 'Disparar' }));
    expect(screen.getByTestId('toast-count').textContent).toBe('1');
    fireEvent.click(screen.getByRole('button', { name: 'Remover' }));
    expect(screen.getByTestId('toast-count').textContent).toBe('0');
  });

  it('useWorkbenchMode returns safe defaults without provider', () => {
    const Probe: React.FC = () => {
      const { mode, isAdvanced } = useWorkbenchMode();
      return <span>{`${mode}:${isAdvanced}`}</span>;
    };
    render(<Probe />);
    expect(screen.getByText('advanced:true')).toBeTruthy();
  });

  it('SettingsView switches accessible themes, colorblind mode, font scale, and restores defaults', () => {
    window.localStorage.setItem('panadiga.split.hex', '50');
    render(
      <WorkbenchModeProvider>
        <ToastProvider>
          <SettingsView />
        </ToastProvider>
      </WorkbenchModeProvider>
    );

    fireEvent.click(screen.getByRole('button', { name: /Alto Contraste/i }));
    expect(document.documentElement.getAttribute('data-theme')).toBe('high-contrast');

    fireEvent.click(screen.getByRole('switch', { name: 'Ativar modo daltônico' }));
    expect(document.documentElement.getAttribute('data-colorblind')).toBe('true');

    fireEvent.click(screen.getByRole('switch', { name: 'Alternar escala tipográfica ampliada' }));
    expect(document.documentElement.getAttribute('data-font-scale')).toBe('large');

    fireEvent.click(screen.getByRole('button', { name: /Restaurar Layout & Padrões/i }));
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark-lab');
    expect(document.documentElement.getAttribute('data-colorblind')).toBe('false');
    expect(window.localStorage.getItem('panadiga.split.hex')).toBeNull();
  });
});
