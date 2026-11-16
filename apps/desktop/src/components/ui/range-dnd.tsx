import React, { useCallback, useRef, useState } from 'react';
import { MoveHorizontal } from 'lucide-react';
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

type DragTarget = 'start' | 'end' | 'window' | null;

export const RangeDndScrubber: React.FC<RangeDndScrubberProps> = ({
  label,
  min,
  max,
  step = 1,
  startValue,
  endValue,
  onChangeRange,
  formatValue = val => String(val),
  presets = [],
  className,
}) => {
  const trackRef = useRef<HTMLDivElement | null>(null);
  const dragTargetRef = useRef<DragTarget>(null);
  const windowAnchorRef = useRef<{ pointerRatio: number; span: number; origStart: number }>({
    pointerRatio: 0,
    span: 0,
    origStart: min,
  });
  const [activeTarget, setActiveTarget] = useState<DragTarget>(null);

  const safeSpan = Math.max(step, max - min);
  const clampedStart = Math.max(min, Math.min(startValue, endValue));
  const clampedEnd = Math.min(max, Math.max(endValue, clampedStart));

  const startPct = ((clampedStart - min) / safeSpan) * 100;
  const endPct = ((clampedEnd - min) / safeSpan) * 100;

  const snapValue = useCallback(
    (raw: number): number => {
      const stepped = Math.round((raw - min) / step) * step + min;
      return Math.max(min, Math.min(max, stepped));
    },
    [max, min, step]
  );

  const clientXToRatio = useCallback((clientX: number): number => {
    const rect = trackRef.current?.getBoundingClientRect();
    if (!rect || rect.width <= 0) return 0;
    return Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
  }, []);

  const handlePointerMove = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      const target = dragTargetRef.current;
      if (!target) return;
      const ratio = clientXToRatio(event.clientX);
      const val = snapValue(min + ratio * safeSpan);

      if (target === 'start') {
        onChangeRange(Math.min(val, clampedEnd), clampedEnd);
      } else if (target === 'end') {
        onChangeRange(clampedStart, Math.max(val, clampedStart));
      } else if (target === 'window') {
        const { pointerRatio, span, origStart } = windowAnchorRef.current;
        const deltaVal = Math.round(((ratio - pointerRatio) * safeSpan) / step) * step;
        const boundedStart = Math.max(min, Math.min(max - span, origStart + deltaVal));
        const nextStart = snapValue(boundedStart);
        onChangeRange(nextStart, Math.min(max, nextStart + span));
      }
    },
    [clampedEnd, clampedStart, clientXToRatio, max, min, onChangeRange, safeSpan, snapValue, step]
  );

  const beginDrag = (target: DragTarget, event: React.PointerEvent<HTMLElement>) => {
    event.stopPropagation();
    event.preventDefault();
    dragTargetRef.current = target;
    setActiveTarget(target);
    if (target === 'window') {
      windowAnchorRef.current = {
        pointerRatio: clientXToRatio(event.clientX),
        span: clampedEnd - clampedStart,
        origStart: clampedStart,
      };
    }
    trackRef.current?.setPointerCapture?.(event.pointerId);
  };

  const endDrag = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!dragTargetRef.current) return;
    dragTargetRef.current = null;
    setActiveTarget(null);
    trackRef.current?.releasePointerCapture?.(event.pointerId);
  };

  return (
    <div className={cn('p-3.5 rounded-xl bg-slate-950/90 border border-slate-800 space-y-3 font-mono', className)}>
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

      <div
        ref={trackRef}
        onPointerMove={handlePointerMove}
        onPointerUp={endDrag}
        onPointerCancel={endDrag}
        className="relative h-7 flex items-center select-none touch-none"
      >
        <div className="w-full h-2 rounded-full bg-slate-800 overflow-hidden" />

        <div
          role="presentation"
          onPointerDown={e => beginDrag('window', e)}
          style={{ left: `${startPct}%`, width: `${Math.max(2, endPct - startPct)}%` }}
          title="Arraste a faixa selecionada para mover a janela"
          className={cn(
            'absolute h-3.5 rounded-full cursor-grab active:cursor-grabbing transition-colors border',
            activeTarget === 'window'
              ? 'bg-cyan-400/50 border-cyan-300'
              : 'bg-cyan-500/35 border-cyan-500/70 hover:bg-cyan-500/50'
          )}
        />

        <button
          type="button"
          role="slider"
          aria-label={`${label} início`}
          aria-valuemin={min}
          aria-valuemax={clampedEnd}
          aria-valuenow={clampedStart}
          onPointerDown={e => beginDrag('start', e)}
          onKeyDown={e => {
            if (e.key === 'ArrowLeft') onChangeRange(Math.max(min, clampedStart - step), clampedEnd);
            if (e.key === 'ArrowRight') onChangeRange(Math.min(clampedEnd, clampedStart + step), clampedEnd);
          }}
          style={{ left: `calc(${startPct}% - 9px)` }}
          className="absolute w-4.5 h-4.5 min-w-[18px] min-h-[18px] rounded-full bg-cyan-400 border-2 border-slate-950 shadow-lg cursor-ew-resize focus:outline-none focus:ring-2 focus:ring-cyan-300"
        />

        <button
          type="button"
          role="slider"
          aria-label={`${label} fim`}
          aria-valuemin={clampedStart}
          aria-valuemax={max}
          aria-valuenow={clampedEnd}
          onPointerDown={e => beginDrag('end', e)}
          onKeyDown={e => {
            if (e.key === 'ArrowLeft') onChangeRange(clampedStart, Math.max(clampedStart, clampedEnd - step));
            if (e.key === 'ArrowRight') onChangeRange(clampedStart, Math.min(max, clampedEnd + step));
          }}
          style={{ left: `calc(${endPct}% - 9px)` }}
          className="absolute w-4.5 h-4.5 min-w-[18px] min-h-[18px] rounded-full bg-emerald-400 border-2 border-slate-950 shadow-lg cursor-ew-resize focus:outline-none focus:ring-2 focus:ring-emerald-300"
        />
      </div>

      {presets.length === 0 ? null : (
        <div className="flex flex-wrap items-center gap-1.5">
          {presets.map(preset => {
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
                    : 'bg-bench-900 border-slate-800 text-slate-400 hover:text-slate-200'
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
