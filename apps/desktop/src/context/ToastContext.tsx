/**
 * @file context/ToastContext.tsx
 * @brief Redux-backed notification facade bridging Redux `toastSlice` with Sonner toasts.
 */

import React, { useCallback, useEffect, type ReactNode } from 'react';
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

const SONNER_DISPATCH: Record<
  ToastType,
  (title: string, opts: { id: string; description?: string; duration: number }) => string | number
> = {
  success: (title, opts) => sonnerToast.success(title, opts),
  error: (title, opts) => sonnerToast.error(title, opts),
  warning: (title, opts) => sonnerToast.warning(title, opts),
  info: (title, opts) => sonnerToast.info(title, opts),
};

const ACTIVE_TIMERS = new Map<string, ReturnType<typeof setTimeout>>();

export const ToastProvider: React.FC<{ children: ReactNode }> = ({ children }) => {
  useEffect(
    () => () => {
      ACTIVE_TIMERS.forEach(handle => clearTimeout(handle));
      ACTIVE_TIMERS.clear();
    },
    [],
  );

  return (
    <>
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
    </>
  );
};

export function useToast(): ToastContextType {
  const dispatch = useAppDispatch();
  const toasts = useAppSelector(state => state.toast.items);

  const removeToast = useCallback(
    (id: string) => {
      const existing = ACTIVE_TIMERS.get(id);
      if (existing !== undefined) {
        clearTimeout(existing);
        ACTIVE_TIMERS.delete(id);
      }
      sonnerToast.dismiss(id);
      dispatch(dismissToast(id));
    },
    [dispatch],
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
        const handle = setTimeout(() => {
          ACTIVE_TIMERS.delete(id);
          dispatch(dismissToast(id));
        }, durationMs);
        ACTIVE_TIMERS.set(id, handle);
      }
    },
    [dispatch],
  );

  return { toasts, showToast, removeToast };
}
