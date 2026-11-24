import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import React from 'react';
import { toast as sonnerToast } from 'sonner';
import { ToastProvider, useToast } from '@/context/ToastContext';
import { WorkbenchModeProvider, useWorkbenchMode } from '@/context/WorkbenchModeContext';
import { SettingsView } from '@/views/SettingsView';
import { emitLocalEvent } from '@/services/ipc';
import { store } from '@/store';
import { resetPipeline, resetWorkbenchPreferences } from '@/store/slices/workbenchSlice';
import { clearToasts } from '@/store/slices/toastSlice';
import { setGlobalLocale } from '@/i18n/useTranslation';
import {
  BottomConsoleDrawer,
  ComplianceDialog,
  DraggableBenchHud,
  GuidedPipelineBar,
  LeftCommandDock,
  RangeDndScrubber,
  ResizableSplitPane,
  RightTelemetryDock,
  WorkbenchStatusBar,
} from './index';
import type { PicoDiagnosticResult, SerialLogEntry } from '@/models';

describe('React UI Ecosystem Primitives (Resizable, RangeDnD, Pipeline, HUD, Sonner)', () => {
  beforeEach(() => {
    window.localStorage.clear();
    setGlobalLocale('en-US');
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
    const WorkbenchHarness: React.FC = () => {
      const { mode, setMode, hudOpen, toggleHud, setHudOpen, pipeline } = useWorkbenchMode();
      return (
        <>
          <GuidedPipelineBar
            currentView="dashboard"
            onNavigate={onNavigate}
            mode={mode}
            onModeChange={setMode}
            hudOpen={hudOpen}
            onToggleHud={toggleHud}
            pipeline={pipeline}
          />
          <DraggableBenchHud
            vddVolts={3.29}
            hardwareConnected={true}
            hudOpen={hudOpen}
            onCloseHud={() => setHudOpen(false)}
            onNavigate={onNavigate}
          />
        </>
      );
    };

    render(
      <WorkbenchModeProvider>
        <WorkbenchHarness />
      </WorkbenchModeProvider>
    );

    const expressBtn = screen.getByRole('button', { name: 'Express (Guided)' });
    fireEvent.click(expressBtn);
    expect(expressBtn.getAttribute('aria-pressed')).toBe('true');
    expect(window.localStorage.getItem('panadiga.workbench.mode')).toBe('express');

    expect(screen.queryByText(/P7501 HUD/i)).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Pinout HUD' }));
    expect(screen.getByText(/P7501 HUD/i)).toBeTruthy();
    expect(screen.getByText(/VDD 3\.29V/)).toBeTruthy();

    act(() => {
      emitLocalEvent('nvram://cleaned', true);
      emitLocalEvent('hdd://inspected', true);
    });
    expect(screen.getByText(/\(2\/4 steps completed\)/)).toBeTruthy();
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

    vi.mocked(sonnerToast.success).mockClear();
    vi.mocked(sonnerToast.dismiss).mockClear();

    fireEvent.click(screen.getByRole('button', { name: 'Disparar' }));
    expect(screen.getByTestId('toast-count').textContent).toBe('1');
    const createdId = store.getState().toast.items[0]?.id;
    expect(createdId).toBeTruthy();
    expect(sonnerToast.success).toHaveBeenCalledTimes(1);
    expect(sonnerToast.success).toHaveBeenCalledWith(
      'Operação OK',
      expect.objectContaining({ id: createdId, description: 'Sonner ativo', duration: Infinity }),
    );

    fireEvent.click(screen.getByRole('button', { name: 'Remover' }));
    expect(screen.getByTestId('toast-count').textContent).toBe('0');
    expect(sonnerToast.dismiss).toHaveBeenCalledTimes(1);
    expect(sonnerToast.dismiss).toHaveBeenCalledWith(createdId);
  });

  it('useWorkbenchMode returns safe defaults without provider', () => {
    const Probe: React.FC = () => {
      const { mode, isAdvanced } = useWorkbenchMode();
      return <span>{`${mode}:${isAdvanced}`}</span>;
    };
    render(<Probe />);
    expect(screen.getByText('express:false')).toBeTruthy();
  });

  it('SettingsView switches accessible themes, colorblind mode, font scale, locale, and restores defaults', () => {
    window.localStorage.setItem('panadiga.split.hex', '50');
    render(
      <WorkbenchModeProvider>
        <ToastProvider>
          <SettingsView />
        </ToastProvider>
      </WorkbenchModeProvider>
    );

    fireEvent.click(screen.getByRole('button', { name: /High Contrast/i }));
    expect(document.documentElement.getAttribute('data-theme')).toBe('high-contrast');

    fireEvent.click(screen.getByRole('switch', { name: 'Enable colorblind mode' }));
    expect(document.documentElement.getAttribute('data-colorblind')).toBe('true');

    fireEvent.click(screen.getByRole('switch', { name: 'Enable enlarged typography scale' }));
    expect(document.documentElement.getAttribute('data-font-scale')).toBe('large');

    fireEvent.click(screen.getByRole('button', { name: 'Português (pt-BR)' }));
    expect(screen.getByText('Tema & Acessibilidade Visual')).toBeTruthy();

    fireEvent.click(screen.getByRole('button', { name: /Restaurar Layout & Padrões/i }));
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark-lab');
    expect(document.documentElement.getAttribute('data-colorblind')).toBe('false');
    expect(window.localStorage.getItem('panadiga.split.hex')).toBeNull();
  });

  it('ComplianceDialog renders checklist & invariants and enforces checkbox before accept', () => {
    const onOpenChange = vi.fn();
    const onAccept = vi.fn();
    render(<ComplianceDialog open={true} onOpenChange={onOpenChange} onAccept={onAccept} />);

    expect(screen.getByTestId('compliance-dialog')).toBeTruthy();
    expect(screen.getByText('4-Stage Recovery Workflow')).toBeTruthy();
    expect(screen.getByText(/3\.3V LVTTL Only & NSW5V Hi-Z \(GP3\)/)).toBeTruthy();
    expect(screen.getByText(/Open-Drain XRESET \(Pin 2 \/ GP5\)/)).toBeTruthy();

    const acceptBtn = screen.getByTestId('compliance-accept-btn');
    expect(acceptBtn).toHaveProperty('disabled', true);
    fireEvent.click(acceptBtn);
    expect(onAccept).not.toHaveBeenCalled();

    fireEvent.click(screen.getByTestId('compliance-ack-checkbox'));
    expect(acceptBtn).toHaveProperty('disabled', false);
    fireEvent.click(acceptBtn);
    expect(onAccept).toHaveBeenCalledTimes(1);
  });

  it('LeftCommandDock, RightTelemetryDock, BottomConsoleDrawer, and WorkbenchStatusBar operate together with form validation', () => {
    const onRunCommand = vi.fn();
    const onOpenPicoBench = vi.fn();
    const onClearLogs = vi.fn();
    const onSendCommand = vi.fn();
    const onToggleLeftDock = vi.fn();
    const onToggleBottomDock = vi.fn();
    const onToggleRightDock = vi.fn();
    const onToggleExpand = vi.fn();
    const onOpenCompliance = vi.fn();

    const sampleLogs: SerialLogEntry[] = [
      { id: 'l1', timestamp: '12:00:01', source: 'HOST', level: 'tx', message: 'PINCHECK' },
      { id: 'l2', timestamp: '12:00:02', source: 'PICO', level: 'rx', message: 'OK P7501' },
      { id: 'l3', timestamp: '12:00:03', source: 'TARGET', level: 'error', message: 'ERR U99' },
    ];
    const latestDiagnostic: PicoDiagnosticResult = {
      command_id: 'PINCHECK',
      raw_command: 'PINCHECK',
      success: true,
      elapsed_ms: 18,
      summary: 'P7501 lines verified',
      raw_output: 'OK',
      vdd_volts: 3.29,
      pincheck_rows: [],
      transition_rows: [],
      dump_rows: [],
    };

    const DockHarness: React.FC = () => {
      const [commandInput, setCommandInput] = React.useState('');
      return (
        <>
          <LeftCommandDock
            open={true}
            onClose={onToggleLeftDock}
            onRunCommand={onRunCommand}
            onOpenPicoBench={onOpenPicoBench}
            hardwareConnected={true}
            breakinMode={2}
          />
          <RightTelemetryDock
            open={true}
            onClose={onToggleRightDock}
            hardwareConnected={true}
            vddVolts={3.29}
            portName="COM10"
            modelId="RP2040"
            gdbRunning={true}
            gdbListenAddr="127.0.0.1:3333"
            sc0Running={true}
            latestDiagnostic={latestDiagnostic}
          />
          <BottomConsoleDrawer
            open={true}
            expanded={false}
            onToggleExpand={onToggleExpand}
            onClose={onToggleBottomDock}
            logs={sampleLogs}
            onClearLogs={onClearLogs}
            commandInput={commandInput}
            onCommandInputChange={setCommandInput}
            onSendCommand={onSendCommand}
            hardwareConnected={true}
          />
          <WorkbenchStatusBar
            currentStageLabel="Pico Bench"
            hardwareConnected={true}
            vddVolts={3.29}
            portName="COM10"
            logCount={sampleLogs.length}
            docks={{ leftOpen: true, rightOpen: true, bottomOpen: true, bottomExpanded: false }}
            onToggleLeftDock={onToggleLeftDock}
            onToggleBottomDock={onToggleBottomDock}
            onToggleRightDock={onToggleRightDock}
            onOpenCompliance={onOpenCompliance}
          />
        </>
      );
    };

    render(<DockHarness />);

    expect(screen.getByTestId('left-command-dock')).toBeTruthy();
    expect(screen.getByTestId('right-telemetry-dock')).toBeTruthy();
    expect(screen.getByTestId('right-dock-latest-diagnostic')).toBeTruthy();
    expect(screen.getByTestId('bottom-console-drawer')).toBeTruthy();
    expect(screen.getByTestId('workbench-status-bar')).toBeTruthy();

    fireEvent.click(screen.getByTestId('left-dock-cmd-PINCHECK'));
    expect(onRunCommand).toHaveBeenCalledWith('PINCHECK', undefined);

    fireEvent.click(screen.getByTestId('status-toggle-left-dock'));
    fireEvent.click(screen.getByTestId('status-toggle-bottom-dock'));
    fireEvent.click(screen.getByTestId('status-toggle-right-dock'));
    fireEvent.click(screen.getByTestId('status-open-compliance-btn'));
    fireEvent.click(screen.getByTestId('bottom-console-expand-btn'));
    expect(onToggleLeftDock).toHaveBeenCalledTimes(1);
    expect(onToggleBottomDock).toHaveBeenCalledTimes(1);
    expect(onToggleRightDock).toHaveBeenCalledTimes(1);
    expect(onOpenCompliance).toHaveBeenCalledTimes(1);
    expect(onToggleExpand).toHaveBeenCalledTimes(1);

    expect(screen.getByText('3 events')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'RX / TX' }));
    expect(screen.getByText('2 events')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Errors' }));
    expect(screen.getByText('1 events')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'All' }));
    expect(screen.getByText('3 events')).toBeTruthy();

    const input = screen.getByTestId('bottom-console-input');
    const form = input.closest('form');
    expect(form).not.toBeNull();
    if (form) {
      fireEvent.submit(form);
    }
    expect(screen.getByRole('alert')).toBeTruthy();
    expect(input.getAttribute('aria-invalid')).toBe('true');
    expect(onSendCommand).not.toHaveBeenCalled();

    fireEvent.change(input, { target: { value: 'STATUS' } });
    expect(input.getAttribute('aria-invalid')).toBe('false');
    expect(screen.queryByRole('alert')).toBeNull();
    if (form) {
      fireEvent.submit(form);
    }
    expect(onSendCommand).toHaveBeenCalledTimes(1);
  });

  it('GuidedPipelineBar updates contextual guidance across Probe Offline, Standby, Active, and Executing states', () => {
    const baseProps = {
      currentView: 'picobench' as const,
      onNavigate: vi.fn(),
      mode: 'engineering' as const,
      onModeChange: vi.fn(),
      hudOpen: false,
      onToggleHud: vi.fn(),
      pipeline: { picoReady: false, nvramCleaned: false, hddInspected: false, unbrickReady: false },
    };

    const { rerender } = render(
      <GuidedPipelineBar {...baseProps} hardwareConnected={false} vddVolts={0} runningCommandId={null} />,
    );
    expect(screen.getByTestId('pipeline-step-guide').textContent).toContain('Probe Offline');

    rerender(
      <GuidedPipelineBar {...baseProps} hardwareConnected={true} vddVolts={0} runningCommandId={null} />,
    );
    expect(screen.getByTestId('pipeline-step-guide').textContent).toContain('Target Standby');

    rerender(
      <GuidedPipelineBar {...baseProps} hardwareConnected={true} vddVolts={3.29} runningCommandId={null} />,
    );
    expect(screen.getByTestId('pipeline-step-guide').textContent).toContain('Bench Active');

    rerender(
      <GuidedPipelineBar {...baseProps} hardwareConnected={true} vddVolts={3.29} runningCommandId="PINCHECK" />,
    );
    expect(screen.getByTestId('pipeline-step-guide').textContent).toContain('Executing PINCHECK');
  });
});
