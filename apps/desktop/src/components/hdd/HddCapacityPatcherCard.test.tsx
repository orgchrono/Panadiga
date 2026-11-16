import { describe, it, expect } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ToastProvider } from '@/context/ToastContext';
import { HddCapacityPatcherCard } from './HddCapacityPatcherCard';

function renderCapacityPatcher() {
  return render(
    <ToastProvider>
      <HddCapacityPatcherCard />
    </ToastProvider>
  );
}

describe('HddCapacityPatcherCard Component', () => {
  it('inspects factory clamp, applies dynamic LBA48 unlock (0xCA), and supports 250 GB RTSD cap', async () => {
    renderCapacityPatcher();

    // Wait for initial auto-inspect IPC call to resolve
    expect(await screen.findByText('0x574DD1FC')).toBeTruthy();
    expect(screen.getByText('TRAVA ATIVA: 200 GB (0xC6)')).toBeTruthy();
    expect(screen.getByText('Opcode 0xC6 (bls +12 Travado)')).toBeTruthy();
    expect(screen.getAllByText('0x07466528').length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('0x3FFFFFFF')).toBeTruthy();

    // Default mode is 'dynamic' -> select AV TBC passthrough co-application
    fireEvent.click(screen.getByRole('button', { name: /AV TBC Passthrough \(av_passthrough\)/ }));
    fireEvent.click(screen.getByRole('button', { name: 'Aplicar Patch Dinâmico de Capacidade' }));

    expect(
      await screen.findByText('Firmware Patcheado & Verificado (Adler-32 / EXTRFOOT OK)!')
    ).toBeTruthy();
    expect(screen.getByText(/TBC: av_passthrough/)).toBeTruthy();
    expect(screen.getByText('DINÂMICO / UNCLAMPED (0xCA)')).toBeTruthy();
    expect(screen.getByText('Opcode 0xCA (bra +12 Livre)')).toBeTruthy();

    // Switch to explicit cap mode and select official 250 GB RTSD preset
    fireEvent.click(screen.getByRole('button', { name: /Teto Explícito \/ Preset RTSD/ }));
    fireEvent.click(screen.getByRole('button', { name: '250 GB ★' }));

    expect(screen.getAllByText(/488[.,]215[.,]712/).length).toBeGreaterThanOrEqual(2);

    fireEvent.click(screen.getByRole('button', { name: 'Aplicar Patch Dinâmico de Capacidade' }));

    expect(await screen.findByText('TRAVA ATIVA: 250 GB (0xC6)')).toBeTruthy();
    expect(screen.getByText('Opcode 0xC6 (bls +12 Travado)')).toBeTruthy();
  });
});
