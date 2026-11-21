import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { CANONICAL_HARDWARE_SSOT_SPEC } from '@/constants';
import { parseDiagnosticOutput } from '@/services/diagnosticParser';
import { simulateMockDiagnostic } from '@/test/mocks/mockDiagnostics';
import { PicoDiagnosticsStudio } from './PicoDiagnosticsStudio';

describe('PicoDiagnosticsStudio Component', () => {
  it('renders empty state initially and dispatches PINCHECK command', async () => {
    const onExecute = vi.fn().mockResolvedValue(simulateMockDiagnostic('PINCHECK'));

    render(
      <PicoDiagnosticsStudio
        ssot={CANONICAL_HARDWARE_SSOT_SPEC}
        latestResult={null}
        history={[]}
        runningCommandId={null}
        error={null}
        selectedCategory="electrical"
        onSelectCategory={vi.fn()}
        onExecute={onExecute}
        onClearHistory={vi.fn()}
      />
    );

    expect(
      screen.getByText(/No diagnostic executed in this session|Nenhum diagnóstico executado nesta sessão/i),
    ).toBeDefined();
    const runPincheck = screen.getByTestId('pico-cmd-run-PINCHECK');
    fireEvent.click(runPincheck);
    expect(onExecute).toHaveBeenCalledWith('PINCHECK', '');
  });

  it('renders structured PINCHECK matrix and DUMP table when latestResult is populated', () => {
    const pinResult = simulateMockDiagnostic('PINCHECK');

    const { rerender } = render(
      <PicoDiagnosticsStudio
        ssot={CANONICAL_HARDWARE_SSOT_SPEC}
        latestResult={pinResult}
        history={[pinResult]}
        runningCommandId={null}
        error={null}
        selectedCategory="electrical"
        onSelectCategory={vi.fn()}
        onExecute={vi.fn()}
        onClearHistory={vi.fn()}
      />
    );

    expect(screen.getByRole('cell', { name: /GP2 \(Pin6 RXD\s*\)/ })).toBeDefined();
    expect(screen.getByText('XRESET Rise: 40 us')).toBeDefined();

    const dumpResult = simulateMockDiagnostic('DUMP', '0x807F0000 16');
    rerender(
      <PicoDiagnosticsStudio
        ssot={CANONICAL_HARDWARE_SSOT_SPEC}
        latestResult={dumpResult}
        history={[dumpResult, pinResult]}
        runningCommandId={null}
        error={null}
        selectedCategory="ocd_flash"
        onSelectCategory={vi.fn()}
        onExecute={vi.fn()}
        onClearHistory={vi.fn()}
      />
    );

    expect(screen.getByText('0x807F0000')).toBeDefined();
    expect(screen.getByText(/0x55AA0100/)).toBeDefined();
  });

  it('displays @P7501_IDENT hardware badges and @PROG / @EVT stream lines', () => {
    const wireResult = parseDiagnosticOutput(
      'IDENTIFY',
      'IDENTIFY',
      [
        '@P7501_IDENT vdd_mv=3295 nsw5v=1 elec=AWAKE_READY loader="Loader ver0.61" jedec=0x0001227E flash=S29JL064H state=READY model=DMR-EH55',
        '@PROG step=1/2 msg=NVRAM_ERASE_SA134_COUNT_8',
        '@EVT kind=SDS_TRIG trig=91006 clk=12840',
        '@P7501_RES cmd=IDENTIFY status=OK code=0 elapsed_us=1420',
      ].join('\n'),
      42,
    );

    render(
      <PicoDiagnosticsStudio
        ssot={CANONICAL_HARDWARE_SSOT_SPEC}
        latestResult={wireResult}
        history={[wireResult]}
        runningCommandId={null}
        error={null}
        selectedCategory="probe"
        onSelectCategory={vi.fn()}
        onExecute={vi.fn()}
        onClearHistory={vi.fn()}
      />,
    );

    expect(screen.getByText(/Target: DMR-EH55|Alvo: DMR-EH55/)).toBeDefined();
    expect(screen.getByText('NOR: S29JL064H')).toBeDefined();
    expect(screen.getByText('Loader: Loader ver0.61')).toBeDefined();
    expect(screen.getByText('RES 0')).toBeDefined();
    const wireBox = screen.getByTestId('diagnostic-wire-events');
    expect(wireBox.textContent).toContain('@PROG 1/2 NVRAM_ERASE_SA134_COUNT_8');
    expect(wireBox.textContent).toContain('@EVT SDS_TRIG');
  });
});
