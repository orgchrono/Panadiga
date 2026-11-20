import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import i18n from '@/i18n/i18n';
import type { HardwareRegisterSnapshot, PipelineStepDescriptor, PreflightCheckItem } from '@/models';
import {
  HexRegisterGrid,
  PipelineStepRunner,
  SafetyConfirmGate,
  SafetyPreflightCard,
  Sha256ArtifactCard,
} from './index';

afterEach(async () => {
  await act(async () => {
    await i18n.changeLanguage('en-US');
  });
});

describe('Pipeline Shared Primitives', () => {
  it('renders steps, preflight lockouts, confirm gate validation, SHA-256 card, and MMIO grid', () => {
    const steps: readonly PipelineStepDescriptor[] = [
      { id: 'step1', label: '1. Inspect', status: 'verified', description: 'OK' },
      { id: 'step2', label: '2. Flash', status: 'blocked' },
    ];
    const checks: readonly PreflightCheckItem[] = [
      { id: 'unlocked', title: 'Backup Ready', detail: '64 KiB CoW saved', severity: 'critical' },
      { id: 'locked', title: 'Session Dump', detail: 'Run dump first', severity: 'critical', lockedReason: 'Missing CoW backup' },
    ];
    const registers: readonly HardwareRegisterSnapshot[] = [
      {
        id: 'vmodesel',
        name: 'AIOF_VMODESEL',
        addressHex: '0x88000100',
        valueHex: '0x8021',
        widthBits: 16,
        statusLabel: 'PAL_M',
        severity: 'info',
        note: 'Encoder matrix',
      },
    ];
    const onToggleCheck = vi.fn();
    const onConfirm = vi.fn();
    const onCopy = vi.fn();
    const onChangeToken = vi.fn();

    render(
      <div>
        <PipelineStepRunner title="Test Runner" steps={steps} isBusy={false} logs={['line-a']} />
        <SafetyPreflightCard checks={checks} checkedIds={['unlocked']} onToggleCheck={onToggleCheck} severity="critical" />
        <SafetyConfirmGate
          expectedToken="TOKEN_OK"
          typedValue="WRONG"
          onChange={onChangeToken}
          onConfirm={onConfirm}
          isBusy={false}
          preflightReady
          inputTestId="gate-in"
          submitTestId="gate-btn"
        />
        <Sha256ArtifactCard outputPath="out/test.bin" outputSha256="abc123" expectedSha256="abc123" onCopyHash={onCopy} />
        <HexRegisterGrid title="MMIO Snapshot" registers={registers} />
      </div>,
    );

    expect(screen.getByText('Test Runner')).toBeTruthy();
    expect(screen.getByText('line-a')).toBeTruthy();
    const lockedBox = screen.getByTestId('preflight-check-locked');
    expect(lockedBox instanceof HTMLInputElement && lockedBox.disabled).toBe(true);
    fireEvent.click(screen.getByTestId('preflight-check-unlocked'));
    expect(onToggleCheck).toHaveBeenCalledWith('unlocked');

    const submitBtn = screen.getByTestId('gate-btn');
    expect(submitBtn instanceof HTMLButtonElement && submitBtn.disabled).toBe(true);
    fireEvent.change(screen.getByTestId('gate-in'), { target: { value: 'TOKEN_OK' } });
    expect(onChangeToken).toHaveBeenCalledWith('TOKEN_OK');

    expect(screen.getByText('AIOF_VMODESEL')).toBeTruthy();
    expect(screen.getByText(/0x8021/)).toBeTruthy();
  });
});
