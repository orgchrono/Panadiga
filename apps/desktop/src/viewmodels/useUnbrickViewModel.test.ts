import { renderHook, act, waitFor } from '@testing-library/react';
import { vi, describe, it, expect, beforeEach } from 'vitest';
import { useUnbrickViewModel } from './useUnbrickViewModel';
import * as ipcService from '@/services/ipc';
import { handleMockUnbrickCommand } from '@/test/mocks/mockUnbrickHandlers';
import type { SdUnbrickKitReport, UnbrickWorkspaceReport } from '@/models';

const mockShowToast = vi.fn();

vi.mock('@/services/ipc', () => ({
  invokeCommand: vi.fn(),
  emitLocalEvent: vi.fn(),
}));

vi.mock('@/context/ToastContext', () => ({
  useToast: () => ({ showToast: mockShowToast }),
}));

async function renderWorkbenchWithQueue(...queued: unknown[]) {
  const mockWorkbench = handleMockUnbrickCommand(
    'inspect_unbrick_workbench',
  ) as UnbrickWorkspaceReport;
  const invokeMock = vi.mocked(ipcService.invokeCommand).mockResolvedValueOnce(mockWorkbench);
  for (const item of queued) {
    invokeMock.mockResolvedValueOnce(item);
  }
  const rendered = renderHook(() => useUnbrickViewModel());
  await waitFor(() => expect(rendered.result.current.workbench.loading).toBe(false));
  return rendered;
}

describe('useUnbrickViewModel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('loads unbrick workbench on mount and hydrates TBC + RSP + SD verify state', async () => {
    const { result } = await renderWorkbenchWithQueue();

    expect(ipcService.invokeCommand).toHaveBeenCalledWith('inspect_unbrick_workbench', undefined);
    expect(result.current.workbench.data?.discovered_pana_dvd).toBe(
      'firmware_and_images/PANA_DVD.FRM'
    );
    expect(result.current.tbc.data?.detected_mode).toBe('FACTORY_ORIGINAL');
    expect(result.current.rspPlan.data?.handshake_hex).toBe('0e0402');
    expect(result.current.verify.data?.overall_status).toBe('READY_WITH_HW_WARNINGS');
  });

  it('builds SD recovery kit and syncs post-verification state', async () => {
    const mockKit = handleMockUnbrickCommand('prepare_sd_unbrick_kit_command', {
      request: { output_dir: 'sd_out', bgo_mode: 'bypass' },
    }) as SdUnbrickKitReport;
    const { result } = await renderWorkbenchWithQueue(mockKit);

    await act(async () => {
      await result.current.buildSdKit({
        output_dir: 'sd_out',
        bgo_mode: 'bypass',
        main_passthrough_mode: 'av_passthrough',
        include_pana_sd: true,
        unlock_hdd_dynamic: true,
      });
    });

    expect(ipcService.invokeCommand).toHaveBeenCalledWith('prepare_sd_unbrick_kit_command', {
      request: expect.objectContaining({ output_dir: 'sd_out', bgo_mode: 'bypass' }),
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

  it('defaults DIGADIAG wrapped NOR dumper loadAddr to 0x50400000 and drives Matshita emulator', async () => {
    const mockStub = handleMockUnbrickCommand('build_nor_dumper_stub_command', {
      wrapAsDigadiag: true,
    });
    const mockBoot = handleMockUnbrickCommand('run_matshita_boot_emu', {
      scenario: 'unlock_disc_frm',
    });
    const mockInject = handleMockUnbrickCommand('inject_matshita_atapi_write', {
      targetVa: '0x51800000',
      payloadHex: 'DEADBEEF',
      useVendorEd: true,
    });
    const { result } = await renderWorkbenchWithQueue(mockStub, mockBoot, mockInject);

    await act(async () => {
      await result.current.buildNorStub({ wrapAsDigadiag: true });
      await result.current.runMatshitaBoot('unlock_disc_frm');
      await result.current.injectMatshitaWrite({
        targetVa: '0x51800000',
        payloadHex: 'DEADBEEF',
        useVendorEd: true,
      });
    });

    expect(ipcService.invokeCommand).toHaveBeenCalledWith(
      'build_nor_dumper_stub_command',
      expect.objectContaining({
        wrapAsDigadiag: true,
        loadAddr: '0x50400000',
      }),
    );
    expect(result.current.norStub.data?.load_addr).toBe('0x50400000');
    expect(result.current.matshitaBoot.data?.jump_target_va_hex).toBe('0x50400000');
    expect(result.current.matshitaInject.data?.record.cdb0_or_cmd).toBe('0xED');
  });
});
