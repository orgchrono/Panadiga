import React, { useCallback, useRef, useState, type ReactNode } from 'react';
import { GripVertical } from 'lucide-react';
import {
  DEFAULT_SPLIT_LEFT_PCT,
  MAX_SPLIT_LEFT_PCT,
  MIN_SPLIT_LEFT_PCT,
  PERCENT_SCALE,
  PERCENT_TENTH_SCALE,
  SPLIT_KEYBOARD_STEP_PCT,
} from '@/constants';
import {
  readPersistedStorageNumber,
  useWindowPointerDrag,
  writePersistedStorageString,
} from '@/hooks/usePointerDrag';
import { useTranslation } from '@/i18n/useTranslation';
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
  return Math.min(max, Math.max(min, Math.round(value * PERCENT_TENTH_SCALE) / PERCENT_TENTH_SCALE));
}

export const ResizableSplitPane: React.FC<ResizableSplitPaneProps> = ({
  left,
  right,
  initialLeftPercent = DEFAULT_SPLIT_LEFT_PCT,
  minLeftPercent = MIN_SPLIT_LEFT_PCT,
  maxLeftPercent = MAX_SPLIT_LEFT_PCT,
  storageKey,
  className,
  ariaLabel,
}) => {
  const { t } = useTranslation();
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [dragging, setDragging] = useState(false);
  const [leftPercent, setLeftPercent] = useState<number>(() => {
    const stored = readPersistedStorageNumber(storageKey);
    if (stored !== null) {
      return clampPercent(stored, minLeftPercent, maxLeftPercent);
    }
    return clampPercent(initialLeftPercent, minLeftPercent, maxLeftPercent);
  });

  const persistPercent = useCallback(
    (next: number) => {
      const clamped = clampPercent(next, minLeftPercent, maxLeftPercent);
      setLeftPercent(clamped);
      writePersistedStorageString(storageKey, String(clamped));
    },
    [maxLeftPercent, minLeftPercent, storageKey],
  );

  const handleMove = useCallback(
    (event: PointerEvent) => {
      const rect = containerRef.current?.getBoundingClientRect();
      if (!rect || rect.width <= 0) return;
      const pct = ((event.clientX - rect.left) / rect.width) * PERCENT_SCALE;
      persistPercent(pct);
    },
    [persistPercent],
  );

  const handleStop = useCallback(() => setDragging(false), []);

  useWindowPointerDrag(dragging, handleMove, handleStop);

  const handleKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (event.key === 'ArrowLeft') {
      event.preventDefault();
      persistPercent(leftPercent - SPLIT_KEYBOARD_STEP_PCT);
    } else if (event.key === 'ArrowRight') {
      event.preventDefault();
      persistPercent(leftPercent + SPLIT_KEYBOARD_STEP_PCT);
    } else if (event.key === 'Home') {
      event.preventDefault();
      persistPercent(initialLeftPercent);
    }
  };

  const containerStyle: React.CSSProperties & Record<'--split-left', string> = {
    '--split-left': `${leftPercent}%`,
  };

  return (
    <div
      ref={containerRef}
      style={containerStyle}
      className={cn(
        'flex flex-col lg:flex-row items-stretch gap-4 lg:gap-0 w-full min-w-0',
        dragging && 'select-none',
        className,
      )}
    >
      <div className="w-full lg:w-[var(--split-left)] min-w-0 flex flex-col shrink-0">
        {left}
      </div>

      <div
        role="separator"
        tabIndex={0}
        aria-label={ariaLabel ?? t('shell.ui.splitPaneAria')}
        aria-orientation="vertical"
        aria-valuemin={minLeftPercent}
        aria-valuemax={maxLeftPercent}
        aria-valuenow={Math.round(leftPercent)}
        title={t('shell.ui.splitPaneTitle')}
        onPointerDown={(e) => {
          e.preventDefault();
          setDragging(true);
        }}
        onDoubleClick={() => persistPercent(initialLeftPercent)}
        onKeyDown={handleKeyDown}
        className={cn(
          'hidden lg:flex w-4 mx-1 shrink-0 cursor-col-resize items-center justify-center rounded-lg transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-cyan-400 group',
          dragging ? 'bg-cyan-500/20' : 'hover:bg-slate-800/70',
        )}
      >
        <div
          className={cn(
            'w-1 h-14 rounded-full flex items-center justify-center transition-colors',
            dragging ? 'bg-cyan-400' : 'bg-slate-700 group-hover:bg-cyan-500/70',
          )}
        >
          <GripVertical className="w-3 h-3 text-slate-950 opacity-80" />
        </div>
      </div>

      <div className="w-full lg:flex-1 min-w-0 flex flex-col">{right}</div>
    </div>
  );
};
