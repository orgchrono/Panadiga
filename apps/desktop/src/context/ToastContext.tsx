/**
 * @file context/ToastContext.tsx
 * @brief Redux-backed notification facade bridging Redux `toastSlice` with Sonner toasts.
 */

import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  type ReactNode,
} from 'react';
import { Toaster, toast as sonnerToast } from 'sonner';
import { useAppDispatch, useAppSelector } from '@/store/hooks';
import { dismissToast, enqueueToast } from '@/store/slices/toastSlice';
import type { ToastItem, ToastType } from '@/types/workbench';
import { nextUniqueId } from '@/utils/formatters';

export type { ToastItem, ToastType };

interface ToastContextType {
  toasts: readonly ToastItem[];
  showToast: (toast: Omit<ToastItem, 'id'>) => void;
  removeToast: (id: string) => void;
}

interface ToastTimerRegistry {
  schedule: (id: string, durationMs: number, onExpire: () => void) => void;
  cancel: (id: string) => void;
}

const SONNER_DISPATCH: Record<
  ToastType,
  (title: string, opts: { id: string; description?: string; duration: number }) => string | number
> = {
  success: (title, opts) => sonnerToast.success(title, opts),
  error: (title, opts) => sonnerToast.error(title, opts),
  warning: (title, opts) => sonnerToast.warning(title, opts),
  info: (title, opts) => sonnerToast.info(title, opts),
};

const ToastTimerContext = createContext<ToastTimerRegistry | null>(null);

export const ToastProvider: React.FC<{ children: ReactNode }> = ({ children }) => {
  const activeTimersRef = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map());

  const cancel = useCallback((id: string) => {
    const existing = activeTimersRef.current.get(id);
    if (existing !== undefined) {
      clearTimeout(existing);
      activeTimersRef.current.delete(id);
    }
  }, []);

  const schedule = useCallback(
    (id: string, durationMs: number, onExpire: () => void) => {
      cancel(id);
      const handle = setTimeout(() => {
        activeTimersRef.current.delete(id);
        onExpire();
      }, durationMs);
      activeTimersRef.current.set(id, handle);
    },
    [cancel],
  );

  useEffect(
    () => () => {
      activeTimersRef.current.forEach((handle, id) => {
        clearTimeout(handle);
        sonnerToast.dismiss(id);
      });
      activeTimersRef.current.clear();
    },
    [],
  );

  const registry = useMemo<ToastTimerRegistry>(
    () => ({ schedule, cancel }),
    [schedule, cancel],
  );

  return (
    <ToastTimerContext.Provider value={registry}>
      {children}
      <Toaster
        theme="dark"
        position="bottom-right"
        richColors
        closeButton
        toastOptions={{
          className: 'font-mono text-xs border border-slate-800 bg-slate-950 text-slate-100 shadow-2xl',
        }}
      />
    </ToastTimerContext.Provider>
  );
};

export function useToast(): ToastContextType {
  const dispatch = useAppDispatch();
  const toasts = useAppSelector(state => state.toast.items);
  const timerRegistry = useContext(ToastTimerContext);

  const removeToast = useCallback(
    (id: string) => {
      timerRegistry?.cancel(id);
      sonnerToast.dismiss(id);
      dispatch(dismissToast(id));
    },
    [dispatch, timerRegistry],
  );

  const showToast = useCallback(
    ({ type, title, message, durationMs = 4000 }: Omit<ToastItem, 'id'>) => {
      const id = nextUniqueId('toast');
      const newToast: ToastItem = { id, type, title, message, durationMs };
      dispatch(enqueueToast(newToast));

      SONNER_DISPATCH[type](title, {
        id,
        description: message,
        duration: durationMs > 0 ? durationMs : Infinity,
      });

      if (durationMs > 0) {
        timerRegistry?.schedule(id, durationMs, () => {
          dispatch(dismissToast(id));
        });
      }
    },
    [dispatch, timerRegistry],
  );

  return { toasts, showToast, removeToast };
}
