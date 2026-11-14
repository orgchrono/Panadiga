import { describe, it, expect } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { useHddWriterViewModel } from './useHddWriterViewModel';

describe('useHddWriterViewModel MVVM Test', () => {
  it('resolves physical drive sectors by default and overrides totalSectors cleanly for customSizeGb (250 GB)', async () => {
    const { result, unmount } = renderHook(() => useHddWriterViewModel());

    await waitFor(() => {
      expect(result.current.selectedDrive).not.toBeNull();
      expect(result.current.loadingDrives).toBe(false);
    });
    expect(result.current.selectedDrive?.device_path).toBe('\\\\.\\PhysicalDrive1');
    expect(result.current.customSizeGb).toBeNull();
    // 80026361856 / 512 = 156301488
    expect(result.current.resolvedTargetSectors).toBe(156_301_488);

    act(() => {
      result.current.setCustomSizeGb(250);
    });
    // Math.floor((250 * 1_000_000_000) / 512) = 488281250
    expect(result.current.resolvedTargetSectors).toBe(488_281_250);

    act(() => {
      result.current.setConfirmInput('CONFIRM-PhysicalDrive1');
    });
    expect(result.current.isTokenValid).toBe(true);

    await act(async () => {
      await result.current.handleProceduralFormat();
    });

    expect(result.current.writeReport?.success).toBe(true);
    expect(result.current.writeReport?.sectors_written).toBe(488_281_250);
    unmount();
  });
});
