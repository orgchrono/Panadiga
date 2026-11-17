import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { clearLiveNvramCapture, ingestP7501DumpOutput } from '@/store/nvramCaptureBridge';
import { useNvramViewModel } from './useNvramViewModel';

async function renderReadyNvramHook() {
  const rendered = renderHook(() => useNvramViewModel());
  await waitFor(() => {
    expect(rendered.result.current.sectorMap.length).toBeGreaterThan(0);
  });
  return rendered;
}

describe('useNvramViewModel MVVM Test', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    clearLiveNvramCapture();
  });

  it('initializes sector map and provides filtering and surgical clean actions', async () => {
    const { result } = await renderReadyNvramHook();

    expect(Array.isArray(result.current.sectorMap)).toBe(true);
    expect(result.current.filterType).toBe('all');
    expect(typeof result.current.executeSurgicalClean).toBe('function');
    expect(typeof result.current.setFilterType).toBe('function');
    expect(typeof result.current.setSelectedSector).toBe('function');
  });

  it('filters sectors according to category criteria', async () => {
    const { result } = await renderReadyNvramHook();

    await act(async () => {
      result.current.setFilterType('corrupted');
    });

    expect(result.current.filterType).toBe('corrupted');
    expect(result.current.filteredSectors.every(s => s.is_corrupted_area)).toBe(true);
  });

  it('supports selecting a specific microsector for deep inspection', async () => {
    const { result } = await renderReadyNvramHook();

    const targetSector = result.current.sectorMap[0];

    await act(async () => {
      result.current.setSelectedSector(targetSector);
    });

    expect(result.current.selectedSector).toBe(targetSector);
  });

  it('reactively switches sectorMap to live Pico DUMP bytes and captures via COM port', async () => {
    const { result } = await renderReadyNvramHook();

    await act(async () => {
      await ingestP7501DumpOutput(
        '0x807F0000: FFFFFFFF 55AA55AA 00000000 CAFEBABE',
        'CDDIAG_CONSOLE',
        'COM10',
        0x807f0000
      );
    });

    expect(result.current.liveCapture).not.toBeNull();
    expect(result.current.liveCapture?.source).toBe('CDDIAG_CONSOLE');
    expect(result.current.liveCapture?.captured_bytes).toBe(16);

    await act(async () => {
      await result.current.captureFromLivePico('COM10');
    });

    expect(result.current.liveCapture).not.toBeNull();
    expect(result.current.liveCapture?.port).toBe('COM10');

    await act(async () => {
      await result.current.clearLiveCapture();
    });

    expect(result.current.liveCapture).toBeNull();
  });

  it('enforces Copy-on-Write and blocks guarded patch beyond 64 KiB NVRAM / IPL boundary', async () => {
    const { result } = await renderReadyNvramHook();

    await act(async () => {
      const samePath = await result.current.executeGuardedPatch({
        inputPath: 'nor.bin',
        outputPath: 'nor.bin',
        nvramOffset: '0x0',
        patchHex: 'FFFFFFFF',
      });
      expect(samePath).toBeNull();
    });
    expect(result.current.patchError).toContain('Copy-on-Write');

    await act(async () => {
      const iplBlocked = await result.current.executeGuardedPatch({
        inputPath: 'nor.bin',
        outputPath: 'nor.patched.bin',
        nvramOffset: '0x10000',
        patchHex: 'AA55',
      });
      expect(iplBlocked).toBeNull();
    });
    expect(result.current.patchError).toContain('E05');

    await act(async () => {
      const validPatch = await result.current.executeGuardedPatch({
        inputPath: 'nor.bin',
        outputPath: 'nor.patched.bin',
        nvramOffset: '0x0',
        patchHex: 'FFFFFFFF',
      });
      expect(validPatch).not.toBeNull();
    });
    expect(result.current.patchSuccess).toBe(true);
    expect(result.current.report?.output_file).toBe('nor.patched.bin');
  });
});
