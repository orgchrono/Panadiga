import { renderHook, act, waitFor } from '@testing-library/react';
import { vi, describe, it, expect, beforeEach } from 'vitest';
import { useUnbrickViewModel } from './useUnbrickViewModel';
import * as ipcService from '@/services/ipc';
import { handleMockUnbrickCommand } from '@/test/mocks/mockUnbrickHandlers';
import type { SdUnbrickKitReport, UnbrickWorkspaceReport } from '@/models';

const mockShowToast = vi.fn();

vi.mock('@/services/ipc', () => ({
  invokeCommand: vi.fn(),
}));

vi.mock('@/context/ToastContext', () => ({
  useToast: () => ({ showToast: mockShowToast }),
}));

describe('useUnbrickViewModel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('loads unbrick workbench on mount and hydrates TBC + RSP + SD verify state', async () => {
    const mockReport = handleMockUnbrickCommand(
      'inspect_unbrick_workbench'
    ) as UnbrickWorkspaceReport;
    vi.mocked(ipcService.invokeCommand).mockResolvedValueOnce(mockReport);

    const { result } = renderHook(() => useUnbrickViewModel());
    await waitFor(() => expect(result.current.workbench.loading).toBe(false));

    expect(ipcService.invokeCommand).toHaveBeenCalledWith('inspect_unbrick_workbench', undefined);
    expect(result.current.workbench.data?.discovered_pana_dvd).toBe(
      'firmware_and_images/PANA_DVD.FRM'
    );
    expect(result.current.tbc.data?.detected_mode).toBe('FACTORY_ORIGINAL');
    expect(result.current.rspPlan.data?.handshake_hex).toBe('0e020204');
    expect(result.current.verify.data?.overall_status).toBe('READY_WITH_HW_WARNINGS');
  });

  it('builds SD recovery kit and syncs post-verification state', async () => {
    const mockWorkbench = handleMockUnbrickCommand(
      'inspect_unbrick_workbench'
    ) as UnbrickWorkspaceReport;
    const mockKit = handleMockUnbrickCommand('prepare_sd_unbrick_kit_command', {
      request: { output_dir: 'sd_out', bgo_mode: 'auto_format' },
    }) as SdUnbrickKitReport;

    vi.mocked(ipcService.invokeCommand)
      .mockResolvedValueOnce(mockWorkbench)
      .mockResolvedValueOnce(mockKit);

    const { result } = renderHook(() => useUnbrickViewModel());
    await waitFor(() => expect(result.current.workbench.loading).toBe(false));

    await act(async () => {
      await result.current.buildSdKit({
        output_dir: 'sd_out',
        bgo_mode: 'auto_format',
        main_passthrough_mode: 'av_passthrough',
        include_pana_sd: true,
        unlock_hdd_dynamic: true,
      });
    });

    expect(ipcService.invokeCommand).toHaveBeenCalledWith('prepare_sd_unbrick_kit_command', {
      request: expect.objectContaining({ output_dir: 'sd_out', bgo_mode: 'auto_format' }),
    });
    expect(result.current.sdKit.data?.digadiag_path).toBe('sd_out/DIGADIAG.FRM');
    expect(result.current.verify.data?.sd_root_dir).toBe('sd_out');
    expect(mockShowToast).toHaveBeenCalledWith(expect.objectContaining({ type: 'success' }));
  });

  it('captures error state on IPC rejection without throwing', async () => {
    vi.mocked(ipcService.invokeCommand).mockRejectedValueOnce(new Error('SD dir missing'));

    const { result } = renderHook(() => useUnbrickViewModel());
    await waitFor(() => expect(result.current.workbench.loading).toBe(false));

    expect(result.current.workbench.error).toContain('SD dir missing');
    expect(mockShowToast).toHaveBeenCalledWith(expect.objectContaining({ type: 'error' }));
  });
});
