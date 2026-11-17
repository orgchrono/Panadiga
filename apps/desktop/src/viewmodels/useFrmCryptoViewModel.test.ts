import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { useFrmCryptoViewModel } from './useFrmCryptoViewModel';
import { useFrmPackager } from './useFrmPackager';
import {
  mockCodebookReport,
  mockCryptanalysisReport,
  mockFrmTocDumpReport,
} from '@/test/mocks/mockCrypto';
import { getInstalledTestIpcBridge, installTestIpcBridge } from '@/test/mocks/testHostBridges';

describe('useFrmCryptoViewModel & useFrmPackager Test', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('passes selected frm_path across analyze_frm, dump_frm_toc, and get_codebook', async () => {
    const previousBridge = getInstalledTestIpcBridge();
    const calls: Array<{ cmd: string; args?: Record<string, unknown> }> = [];
    const mockBridge: {
      invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
      listen: () => () => void;
    } = {
      invoke: async (cmd, args) => {
        calls.push({ cmd, args });
        if (cmd === 'analyze_frm') return mockCryptanalysisReport;
        if (cmd === 'dump_frm_toc') return mockFrmTocDumpReport;
        if (cmd === 'get_codebook') return mockCodebookReport;
        return [];
      },
      listen: () => () => {},
    };
    installTestIpcBridge(mockBridge);

    try {
      const { result } = renderHook(() => useFrmCryptoViewModel());
      await waitFor(() => expect(result.current.loading).toBe(false));
      expect(result.current.tocReport?.decrypted_toc).toHaveLength(3);

      await act(async () => {
        result.current.setFrmPath('firmware_and_images/PANA__SD.FRM');
      });

      await waitFor(() =>
        expect(
          calls.some(
            c => c.cmd === 'dump_frm_toc' && c.args?.frmPath === 'firmware_and_images/PANA__SD.FRM'
          )
        ).toBe(true)
      );
    } finally {
      installTestIpcBridge(previousBridge);
    }
  });

  it('packs .FRM container with Tauri request wrapper and unpacks EXTRHEAD payload', async () => {
    const { result } = renderHook(() => useFrmPackager());

    await waitFor(() => expect(result.current.containers.length).toBeGreaterThan(0));

    let packed = null;
    await act(async () => {
      packed = await result.current.packFrm({
        mode: 'sd',
        out_path: 'firmware_and_images/out/PANA__SD.FRM',
        modules: [{ name: 'DIAG01', path: 'diag.bin' }],
      });
    });

    expect(packed).not.toBeNull();
    expect(result.current.packError).toBeNull();
    expect(result.current.packResult?.mode).toBe('sd');

    let unpacked = null;
    await act(async () => {
      unpacked = await result.current.unpackExtrhead({
        inputPath: 'firmware_and_images/DIGADIAG.FRM',
        mode: 'auto',
      });
    });

    expect(unpacked).not.toBeNull();
    expect(result.current.unpackError).toBeNull();
    expect(result.current.unpackResult?.checksum_valid).toBe(true);
  });
});
