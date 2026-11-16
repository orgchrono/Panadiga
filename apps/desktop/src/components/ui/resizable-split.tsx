import React, { useCallback, useEffect, useRef, useState, type ReactNode } from 'react';
import { GripVertical } from 'lucide-react';
import { cn } from '@/lib/utils';

export interface ResizableSplitPaneProps {
  readonly left: ReactNode;
  readonly right: ReactNode;
  readonly initialLeftPercent?: number;
  readonly minLeftPercent?: number;
  readonly maxLeftPercent?: number;
  readonly storageKey?: string;
  readonly className?: string;
  readonly ariaLabel?: string;
}

function clampPercent(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, Math.round(value * 10) / 10));
}

export const ResizableSplitPane: React.FC<ResizableSplitPaneProps> = ({
  left,
  right,
  initialLeftPercent = 40,
  minLeftPercent = 24,
  maxLeftPercent = 74,
  storageKey,
  className,
  ariaLabel = 'Divisor redimensionável de painéis',
}) => {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [dragging, setDragging] = useState(false);
  const [leftPercent, setLeftPercent] = useState<number>(() => {
    if (storageKey && typeof window !== 'undefined') {
      try {
        const raw = window.localStorage.getItem(storageKey);
        const parsed = raw ? Number.parseFloat(raw) : NaN;
        if (Number.isFinite(parsed)) {
          return clampPercent(parsed, minLeftPercent, maxLeftPercent);
        }
      } catch {
        // ignore storage errors
      }
    }
    return clampPercent(initialLeftPercent, minLeftPercent, maxLeftPercent);
  });

  const persistPercent = useCallback(
    (next: number) => {
      const clamped = clampPercent(next, minLeftPercent, maxLeftPercent);
      setLeftPercent(clamped);
      if (storageKey && typeof window !== 'undefined') {
        try {
          window.localStorage.setItem(storageKey, String(clamped));
        } catch {
          // ignore storage errors
        }
      }
    },
    [maxLeftPercent, minLeftPercent, storageKey]
  );

  useEffect(() => {
    if (!dragging) return;

    const handleMove = (event: PointerEvent) => {
      const rect = containerRef.current?.getBoundingClientRect();
      if (!rect || rect.width <= 0) return;
      const pct = ((event.clientX - rect.left) / rect.width) * 100;
      persistPercent(pct);
    };

    const handleStop = () => setDragging(false);

    window.addEventListener('pointermove', handleMove);
    window.addEventListener('pointerup', handleStop);
    window.addEventListener('pointercancel', handleStop);

    return () => {
      window.removeEventListener('pointermove', handleMove);
      window.removeEventListener('pointerup', handleStop);
      window.removeEventListener('pointercancel', handleStop);
    };
  }, [dragging, persistPercent]);

  const handleKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (event.key === 'ArrowLeft') {
      event.preventDefault();
      persistPercent(leftPercent - 2);
    } else if (event.key === 'ArrowRight') {
      event.preventDefault();
      persistPercent(leftPercent + 2);
    } else if (event.key === 'Home') {
      event.preventDefault();
      persistPercent(initialLeftPercent);
    }
  };

  return (
    <div
      ref={containerRef}
      style={{ '--split-left': `${leftPercent}%` } as React.CSSProperties}
      className={cn(
        'flex flex-col lg:flex-row items-stretch gap-4 lg:gap-0 w-full min-w-0',
        dragging && 'select-none',
        className
      )}
    >
      <div className="w-full lg:w-[var(--split-left)] min-w-0 flex flex-col shrink-0">
        {left}
      </div>

      <div
        role="separator"
        tabIndex={0}
        aria-label={ariaLabel}
        aria-orientation="vertical"
        aria-valuemin={minLeftPercent}
        aria-valuemax={maxLeftPercent}
        aria-valuenow={Math.round(leftPercent)}
        title="Arraste para redimensionar ou clique duplo para restaurar"
        onPointerDown={e => {
          e.preventDefault();
          setDragging(true);
        }}
        onDoubleClick={() => persistPercent(initialLeftPercent)}
        onKeyDown={handleKeyDown}
        className={cn(
          'hidden lg:flex w-4 mx-1 shrink-0 cursor-col-resize items-center justify-center rounded-lg transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-cyan-400 group',
          dragging ? 'bg-cyan-500/20' : 'hover:bg-slate-800/70'
        )}
      >
        <div
          className={cn(
            'w-1 h-14 rounded-full flex items-center justify-center transition-colors',
            dragging ? 'bg-cyan-400' : 'bg-slate-700 group-hover:bg-cyan-500/70'
          )}
        >
          <GripVertical className="w-3 h-3 text-slate-950 opacity-80" />
        </div>
      </div>

      <div className="w-full lg:flex-1 min-w-0 flex flex-col">
        {right}
      </div>
    </div>
  );
};
