import React, {
  createContext,
  useContext,
  useState,
  useCallback,
  useEffect,
  useRef,
  ReactNode,
} from 'react';
import { Toaster, toast as sonnerToast } from 'sonner';
import { nextUniqueId } from '@/utils/formatters';

export type ToastType = 'success' | 'error' | 'warning' | 'info';

export interface ToastItem {
  id: string;
  type: ToastType;
  title: string;
  message?: string;
  durationMs?: number;
}

interface ToastContextType {
  toasts: readonly ToastItem[];
  showToast: (toast: Omit<ToastItem, 'id'>) => void;
  removeToast: (id: string) => void;
}

const ToastContext = createContext<ToastContextType | undefined>(undefined);

const SONNER_DISPATCH: Record<
  ToastType,
  (title: string, opts: { id: string; description?: string; duration: number }) => string | number
> = {
  success: (title, opts) => sonnerToast.success(title, opts),
  error: (title, opts) => sonnerToast.error(title, opts),
  warning: (title, opts) => sonnerToast.warning(title, opts),
  info: (title, opts) => sonnerToast.info(title, opts),
};

export const ToastProvider: React.FC<{ children: ReactNode }> = ({ children }) => {
  const [toasts, setToasts] = useState<readonly ToastItem[]>([]);
  const timersRef = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map());

  useEffect(() => {
    const timers = timersRef.current;
    return () => {
      timers.forEach(handle => clearTimeout(handle));
      timers.clear();
    };
  }, []);

  const removeToast = useCallback((id: string) => {
    const existing = timersRef.current.get(id);
    if (existing !== undefined) {
      clearTimeout(existing);
      timersRef.current.delete(id);
    }
    sonnerToast.dismiss(id);
    setToasts(prev => prev.filter(t => t.id !== id));
  }, []);

  const showToast = useCallback(
    ({ type, title, message, durationMs = 4000 }: Omit<ToastItem, 'id'>) => {
      const id = nextUniqueId('toast');
      const newToast: ToastItem = { id, type, title, message, durationMs };

      setToasts(prev => [...prev, newToast]);

      SONNER_DISPATCH[type](title, {
        id,
        description: message,
        duration: durationMs > 0 ? durationMs : Infinity,
      });

      if (durationMs > 0) {
        const handle = setTimeout(() => {
          timersRef.current.delete(id);
          setToasts(prev => prev.filter(t => t.id !== id));
        }, durationMs);
        timersRef.current.set(id, handle);
      }
    },
    []
  );

  return (
    <ToastContext.Provider value={{ toasts, showToast, removeToast }}>
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
    </ToastContext.Provider>
  );
};

const noopToastAction = (): void => {};

const EMPTY_TOASTS: readonly ToastItem[] = Object.freeze([]);

const NOOP_TOAST_CONTEXT: ToastContextType = Object.freeze({
  toasts: EMPTY_TOASTS,
  showToast: noopToastAction,
  removeToast: noopToastAction,
});

export function useToast(): ToastContextType {
  const ctx = useContext(ToastContext);
  return ctx ?? NOOP_TOAST_CONTEXT;
}
