import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { Sc0BridgePanel, type Sc0BridgePanelProps } from './Sc0BridgePanel';
import type { BridgeStreamLine } from '@/viewmodels/useSc0Bridge';

function renderBridgePanel(overrides: Partial<Sc0BridgePanelProps> = {}) {
  return render(
    <Sc0BridgePanel
      status={{
        running: false,
        port_name: 'COM10',
        baud_rate: 115200,
        boot_mode: 'JIG:350',
        bytes_rx: 0,
        bytes_tx: 0,
        last_event: 'Standby',
      }}
      logs={[]}
      baudRate="115200"
      onChangeBaudRate={vi.fn()}
      bootMode="JIG:350"
      onChangeBootMode={vi.fn()}
      inputMode="line"
      onChangeInputMode={vi.fn()}
      txInput=""
      onChangeTxInput={vi.fn()}
      isBusy={false}
      error={null}
      onStart={vi.fn()}
      onStop={vi.fn()}
      onSubmitTx={vi.fn()}
      onClearLogs={vi.fn()}
      onTriggerWaittx={vi.fn()}
      onSendRspPreset={vi.fn()}
      onSendDriveCommand={vi.fn()}
      {...overrides}
    />
  );
}

describe('Sc0BridgePanel Component', () => {
  it('renders quick PANARSP, $g REGS, $x binary, DIAGWATCH Read NVRAM, WAITTX, drive buttons, and two-bus banner cleanly', () => {
    const onSendRsp = vi.fn();
    const onSendDrive = vi.fn();
    const onWaittx = vi.fn();
    const onRunDiag = vi.fn();

    renderBridgePanel({
      onTriggerWaittx: onWaittx,
      onSendRspPreset: onSendRsp,
      onSendDriveCommand: onSendDrive,
      onRunDiagnostic: onRunDiag,
    });

    const archBanner = screen.getByRole('note');
    expect(archBanner.textContent).toContain('0x880000C0');
    expect(archBanner.textContent).toContain('0xB002..0xB004');
    expect(archBanner.textContent).toContain('PASS 0x4d415453 0x48495441');
    expect(screen.queryByRole('alert')).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: '0x15 NAK' }));
    expect(onSendRsp).toHaveBeenCalledWith('hex', '15', '4000 JIG:350 NAK');

    fireEvent.click(screen.getByRole('button', { name: '$g REGS' }));
    expect(onSendRsp).toHaveBeenCalledWith('rsp', 'g', '5000 JIG:350 REGS');

    fireEvent.click(screen.getByRole('button', { name: '$x#.. Bin' }));
    expect(onSendRsp).toHaveBeenCalledWith('rsp', 'x84000000,40', '4000 XMEM:84000000,40');

    fireEvent.click(screen.getByRole('button', { name: 'F/E Unlock' }));
    expect(onSendDrive).toHaveBeenCalledWith('PASS 0x4d415453 0x48495441');

    fireEvent.click(screen.getByRole('button', { name: 'PASS' }));
    expect(onSendDrive).toHaveBeenCalledWith('PASS');

    fireEvent.click(screen.getByRole('button', { name: /WAITTX wait 1/i }));
    expect(onWaittx).toHaveBeenCalledWith('8000 JIG:350 "wait 1" 0D 4000');

    fireEvent.click(screen.getByRole('button', { name: /DIAGWATCH/i }));
    expect(onRunDiag).toHaveBeenCalledWith(
      'DIAGWATCH',
      '25000 JIG:2500 READ:847F0000,20 RELEASE'
    );
  });

  it('shows BGO halt warning alert when live logs report BGO driver init failed', () => {
    const bgoLog: BridgeStreamLine = {
      id: 'bgo-1',
      stream: 'rx',
      text: 'BGO driver init failed[1].',
      hex: '42474F',
      bytes: [0x42, 0x47, 0x4f],
      timestamp: '00:00:01',
    };
    renderBridgePanel({ logs: [bgoLog] });

    const alert = screen.getByRole('alert');
    expect(alert.textContent).toContain('boot_panic_or_halt_7812');
    expect(alert.textContent).toContain('FUN_4400836f');
    expect(alert.textContent).toContain('PANARSP');
  });
});
