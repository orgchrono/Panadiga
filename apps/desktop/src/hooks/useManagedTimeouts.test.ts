import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useManagedTimeouts } from './useManagedTimeouts';
import {
  isBgoMode,
  isFrmPackMode,
  isMainPassthroughMode,
  isMatshitaBootScenario,
  isPicoCommandCategory,
  isPicoUf2Variant,
  isSc0BridgeInputMode,
} from '@/utils/typeGuards';

describe('useManagedTimeouts', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('fires scheduled callback after delay and cleans up unmounted pending timers', () => {
    const fired = vi.fn();
    const canceled = vi.fn();

    const { result, unmount } = renderHook(() => useManagedTimeouts());

    act(() => {
      result.current.scheduleTimeout(fired, 150);
      result.current.scheduleTimeout(canceled, 500);
    });

    act(() => {
      vi.advanceTimersByTime(150);
    });
    expect(fired).toHaveBeenCalledTimes(1);
    expect(canceled).not.toHaveBeenCalled();

    unmount();

    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(canceled).not.toHaveBeenCalled();
  });

  it('cancels individual handle when cancelTimeout is invoked', () => {
    const callback = vi.fn();
    const { result } = renderHook(() => useManagedTimeouts());

    let handle: ReturnType<typeof setTimeout> | null = null;
    act(() => {
      handle = result.current.scheduleTimeout(callback, 200);
      result.current.cancelTimeout(handle);
    });

    act(() => {
      vi.advanceTimersByTime(500);
    });
    expect(callback).not.toHaveBeenCalled();
  });
});

describe('union runtime typeGuards', () => {
  it('validates closed union strings accurately without unsafe casts', () => {
    expect(isFrmPackMode('digadiag')).toBe(true);
    expect(isFrmPackMode('invalid')).toBe(false);
    expect(isPicoUf2Variant('rp2350')).toBe(true);
    expect(isPicoUf2Variant('esp32')).toBe(false);
    expect(isPicoCommandCategory('ocd_flash')).toBe(true);
    expect(isPicoCommandCategory('memory')).toBe(false);
    expect(isSc0BridgeInputMode('rsp')).toBe(true);
    expect(isSc0BridgeInputMode('binary')).toBe(false);
    expect(isMatshitaBootScenario('unlock_disc_frm')).toBe(true);
    expect(isMatshitaBootScenario('bogus')).toBe(false);
    expect(isBgoMode('bypass')).toBe(true);
    expect(isBgoMode('erase_all')).toBe(false);
    expect(isMainPassthroughMode('av_passthrough')).toBe(true);
    expect(isMainPassthroughMode('custom')).toBe(false);
  });
});
