import { describe, it, expect, beforeEach } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { resetHardwareSsotCacheForTests, useHardwareSsot } from './useHardwareSsot';
import { getInstalledTestIpcBridge, installTestIpcBridge } from '@/test/mocks/testHostBridges';

async function renderSettledSsotHook() {
  const rendered = renderHook(() => useHardwareSsot());
  await waitFor(() => {
    expect(rendered.result.current.isLoading).toBe(false);
  });
  return rendered.result;
}

describe('useHardwareSsot ViewModel Test', () => {
  beforeEach(() => {
    resetHardwareSsotCacheForTests();
  });

  it('loads canonical SSOT spec when backend IPC succeeds', async () => {
    const result = await renderSettledSsotHook();

    expect(result.current.isLoaded).toBe(true);
    expect(result.current.error).toBeNull();
    expect(result.current.ssot).not.toBeNull();
    expect(result.current.ssot?.pico_commands.length).toBeGreaterThanOrEqual(25);
  });

  it('surfaces explicit error and keeps ssot null without silent fallback when IPC fails', async () => {
    const previousBridge = getInstalledTestIpcBridge();
    installTestIpcBridge({
      invoke: async () => {
        throw new Error('SSOT IPC unreachable');
      },
      listen: () => () => {},
    });

    try {
      const result = await renderSettledSsotHook();

      expect(result.current.isLoaded).toBe(false);
      expect(result.current.ssot).toBeNull();
      expect(result.current.error).toContain('SSOT IPC unreachable');
    } finally {
      installTestIpcBridge(previousBridge);
    }
  });
});
