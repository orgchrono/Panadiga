import React, { useCallback, useRef, useState } from 'react';
import { MoveHorizontal } from 'lucide-react';
import {
  PERCENT_SCALE,
  RANGE_MIN_VISUAL_WIDTH_PCT,
  RANGE_THUMB_HALF_PX,
} from '@/constants';
import { useWindowPointerDrag } from '@/hooks/usePointerDrag';
import { useTranslation } from '@/i18n/useTranslation';
import { cn } from '@/lib/utils';

export interface RangePreset {
  readonly label: string;
  readonly start: number;
  readonly end: number;
}

export interface RangeDndScrubberProps {
  readonly label: string;
  readonly min: number;
  readonly max: number;
  readonly step?: number;
  readonly startValue: number;
  readonly endValue: number;
  readonly onChangeRange: (start: number, end: number) => void;
  readonly formatValue?: (val: number) => string;
  readonly presets?: readonly RangePreset[];
  readonly className?: string;
}

type DragTarget = 'start' | 'end' | 'window';

interface DragSession {
  readonly target: DragTarget;
  readonly pointerRatio: number;
  readonly span: number;
  readonly origStart: number;
}

export const RangeDndScrubber: React.FC<RangeDndScrubberProps> = ({
  label,
  min,
  max,
  step = 1,
  startValue,
  endValue,
  onChangeRange,
  formatValue = (val) => String(val),
  presets = [],
  className,
}) => {
  const { t } = useTranslation();
  const trackRef = useRef<HTMLDivElement | null>(null);
  const [dragSession, setDragSession] = useState<DragSession | null>(null);

  const safeSpan = Math.max(step, max - min);
  const clampedStart = Math.max(min, Math.min(startValue, endValue));
  const clampedEnd = Math.min(max, Math.max(endValue, clampedStart));

  const startPct = ((clampedStart - min) / safeSpan) * PERCENT_SCALE;
  const endPct = ((clampedEnd - min) / safeSpan) * PERCENT_SCALE;

  const snapValue = useCallback(
    (raw: number): number => {
      const stepped = Math.round((raw - min) / step) * step + min;
      return Math.max(min, Math.min(max, stepped));
    },
    [max, min, step],
  );

  const clientXToRatio = useCallback((clientX: number): number => {
    const rect = trackRef.current?.getBoundingClientRect();
    if (!rect || rect.width <= 0) return 0;
    return Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
  }, []);

  const applyDragAtClientX = useCallback(
    (session: DragSession, clientX: number) => {
      const ratio = clientXToRatio(clientX);
      const val = snapValue(min + ratio * safeSpan);

      if (session.target === 'start') {
        onChangeRange(Math.min(val, clampedEnd), clampedEnd);
      } else if (session.target === 'end') {
        onChangeRange(clampedStart, Math.max(val, clampedStart));
      } else {
        const deltaVal = Math.round(((ratio - session.pointerRatio) * safeSpan) / step) * step;
        const boundedStart = Math.max(min, Math.min(max - session.span, session.origStart + deltaVal));
        const nextStart = snapValue(boundedStart);
        onChangeRange(nextStart, Math.min(max, nextStart + session.span));
      }
    },
    [clampedEnd, clampedStart, clientXToRatio, max, min, onChangeRange, safeSpan, snapValue, step],
  );

  const handleWindowMove = useCallback(
    (event: PointerEvent) => {
      if (dragSession) {
        applyDragAtClientX(dragSession, event.clientX);
      }
    },
    [applyDragAtClientX, dragSession],
  );

  const handleWindowStop = useCallback(() => setDragSession(null), []);

  useWindowPointerDrag(dragSession !== null, handleWindowMove, handleWindowStop);

  const beginDrag = (target: DragTarget, event: React.PointerEvent<HTMLElement>) => {
    event.stopPropagation();
    event.preventDefault();
    setDragSession({
      target,
      pointerRatio: clientXToRatio(event.clientX),
      span: clampedEnd - clampedStart,
      origStart: clampedStart,
    });
  };

  const windowStyle: React.CSSProperties & Record<'--range-start' | '--range-width', string> = {
    '--range-start': `${startPct}%`,
    '--range-width': `${Math.max(RANGE_MIN_VISUAL_WIDTH_PCT, endPct - startPct)}%`,
  };
  const thumbStartStyle: React.CSSProperties & Record<'--thumb-start', string> = {
    '--thumb-start': `calc(${startPct}% - ${RANGE_THUMB_HALF_PX}px)`,
  };
  const thumbEndStyle: React.CSSProperties & Record<'--thumb-end', string> = {
    '--thumb-end': `calc(${endPct}% - ${RANGE_THUMB_HALF_PX}px)`,
  };

  return (
    <div
      className={cn(
        'p-3.5 rounded-xl bg-slate-950/90 border border-slate-800 space-y-3 font-mono',
        className,
      )}
    >
      <div className="flex flex-wrap items-center justify-between gap-2 text-xs">
        <span className="text-slate-300 font-semibold flex items-center gap-1.5">
          <MoveHorizontal className="w-3.5 h-3.5 text-cyan-400" />
          <span>{label}</span>
        </span>
        <div className="flex items-center gap-2 text-[11px] bg-bench-900 px-2.5 py-1 rounded-lg border border-slate-800">
          <span className="text-cyan-300 font-bold">{formatValue(clampedStart)}</span>
          <span className="text-slate-600">→</span>
          <span className="text-emerald-300 font-bold">{formatValue(clampedEnd)}</span>
        </div>
      </div>

      <div ref={trackRef} className="relative h-7 flex items-center select-none touch-none">
        <div className="w-full h-2 rounded-full bg-slate-800 overflow-hidden" />

        <div
          role="presentation"
          onPointerDown={(e) => beginDrag('window', e)}
          style={windowStyle}
          title={t('shell.ui.rangeWindowDragTitle')}
          className={cn(
            'absolute h-3.5 left-[var(--range-start)] w-[var(--range-width)] rounded-full cursor-grab active:cursor-grabbing transition-colors border',
            dragSession?.target === 'window'
              ? 'bg-cyan-400/50 border-cyan-300'
              : 'bg-cyan-500/35 border-cyan-500/70 hover:bg-cyan-500/50',
          )}
        />

        <input
          type="range"
          min={min}
          max={clampedEnd}
          step={step}
          value={clampedStart}
          aria-label={`${label} início`}
          onChange={(e) => onChangeRange(Math.min(Number(e.target.value), clampedEnd), clampedEnd)}
          onPointerDown={(e) => beginDrag('start', e)}
          onKeyDown={(e) => {
            if (e.key === 'ArrowLeft') {
              onChangeRange(Math.max(min, clampedStart - step), clampedEnd);
            }
            if (e.key === 'ArrowRight') {
              onChangeRange(Math.min(clampedEnd, clampedStart + step), clampedEnd);
            }
          }}
          style={thumbStartStyle}
          className="absolute left-[var(--thumb-start)] w-4.5 h-4.5 min-w-[18px] min-h-[18px] appearance-none rounded-full bg-cyan-400 border-2 border-slate-950 shadow-lg cursor-ew-resize focus:outline-none focus:ring-2 focus:ring-cyan-300"
        />

        <input
          type="range"
          min={clampedStart}
          max={max}
          step={step}
          value={clampedEnd}
          aria-label={`${label} fim`}
          onChange={(e) =>
            onChangeRange(clampedStart, Math.max(Number(e.target.value), clampedStart))
          }
          onPointerDown={(e) => beginDrag('end', e)}
          onKeyDown={(e) => {
            if (e.key === 'ArrowLeft') {
              onChangeRange(clampedStart, Math.max(clampedStart, clampedEnd - step));
            }
            if (e.key === 'ArrowRight') {
              onChangeRange(clampedStart, Math.min(max, clampedEnd + step));
            }
          }}
          style={thumbEndStyle}
          className="absolute left-[var(--thumb-end)] w-4.5 h-4.5 min-w-[18px] min-h-[18px] appearance-none rounded-full bg-emerald-400 border-2 border-slate-950 shadow-lg cursor-ew-resize focus:outline-none focus:ring-2 focus:ring-emerald-300"
        />
      </div>

      {presets.length === 0 ? null : (
        <div className="flex flex-wrap items-center gap-1.5">
          {presets.map((preset) => {
            const isActive = clampedStart === preset.start && clampedEnd === preset.end;
            return (
              <button
                key={preset.label}
                type="button"
                onClick={() => onChangeRange(preset.start, preset.end)}
                className={cn(
                  'px-2 py-1 rounded-md text-[10px] border transition-colors',
                  isActive
                    ? 'bg-cyan-500/20 border-cyan-500/50 text-cyan-300 font-bold'
                    : 'bg-bench-900 border-slate-800 text-slate-400 hover:text-slate-200',
                )}
              >
                {preset.label}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
};
