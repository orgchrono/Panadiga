import React, { useCallback, useMemo, useState } from 'react';
import { Layers } from 'lucide-react';
import { StatusBadge } from '@/components/StatusBadge';
import { RangeDndScrubber } from '@/components/ui';
import { HEX32_PAD_WIDTH, KIB_BYTES } from '@/constants';
import { useWindowPointerUp } from '@/hooks/usePointerDrag';
import { useTranslation } from '@/i18n/useTranslation';
import { FlashSectorInfo } from '@/models';
import { cn } from '@/lib/utils';

interface NvramSectorMapProps {
  sectorMap: FlashSectorInfo[];
  selectedSector: FlashSectorInfo | null;
  filterType: string;
  cleanSuccess: boolean;
  onSelectSector: (sector: FlashSectorInfo) => void;
  onSelectFilter: (filter: string) => void;
}

function getSectorTileClass(sector: FlashSectorInfo, cleanSuccess: boolean): string {
  if (sector.is_corrupted_area) {
    return cleanSuccess
      ? 'bg-emerald-500 hover:bg-emerald-400 text-slate-950 font-bold'
      : 'bg-rose-500 hover:bg-rose-400 text-slate-950 font-bold animate-pulse';
  }
  if (sector.sector_type === 'bootloader') {
    return 'bg-cyan-900/60 hover:bg-cyan-800/80 text-cyan-200';
  }
  return 'bg-slate-800 hover:bg-slate-700 text-slate-300';
}

function getSectorInspectorBadgeLabel(
  sector: FlashSectorInfo,
  cleanSuccess: boolean,
  t: ReturnType<typeof useTranslation>['t'],
): string {
  if (!sector.is_corrupted_area) return t('nvram.sectorMap.badgeIntact');
  return cleanSuccess ? t('nvram.sectorMap.badgeCleanFf') : t('nvram.sectorMap.badgeU99');
}

export const NvramSectorMap: React.FC<NvramSectorMapProps> = ({
  sectorMap,
  selectedSector,
  filterType,
  cleanSuccess,
  onSelectSector,
  onSelectFilter,
}) => {
  const { t } = useTranslation();
  const maxIdx = Math.max(0, sectorMap.length - 1);
  const scopeKey = `${sectorMap.length}:${filterType}`;
  const [rangeState, setRangeState] = useState<{
    readonly scope: string;
    readonly start: number;
    readonly end: number;
  }>(() => ({ scope: scopeKey, start: 0, end: maxIdx }));
  const [dragAnchorIdx, setDragAnchorIdx] = useState<number | null>(null);

  const handleStopDrag = useCallback(() => setDragAnchorIdx(null), []);
  useWindowPointerUp(handleStopDrag);

  const activeRange =
    rangeState.scope === scopeKey ? rangeState : { scope: scopeKey, start: 0, end: maxIdx };
  const lo = Math.min(activeRange.start, activeRange.end);
  const hi = Math.max(activeRange.start, activeRange.end);

  const rangeMetrics = useMemo(() => {
    const slice = sectorMap.slice(lo, hi + 1);
    const totalKb = slice.reduce((acc, s) => acc + s.size_bytes / KIB_BYTES, 0);
    const activeBytes = slice.reduce((acc, s) => acc + (s.active_bytes ?? 0), 0);
    return {
      count: slice.length,
      totalKb,
      activeBytes,
      first: slice[0],
      last: slice[slice.length - 1],
    };
  }, [hi, lo, sectorMap]);

  const presets = useMemo(() => {
    if (sectorMap.length <= 1) return [];
    const bootEnd = sectorMap.reduce(
      (last, s, idx) => (s.sector_type === 'bootloader' ? idx : last),
      -1,
    );
    const nvramStart = sectorMap.findIndex((s) => s.sector_type === 'nvram_microsector');
    return [
      { label: t('nvram.sectorMap.presetAll', { count: sectorMap.length }), start: 0, end: maxIdx },
      ...(bootEnd >= 0 ? [{ label: t('nvram.sectorMap.presetBoot'), start: 0, end: bootEnd }] : []),
      ...(nvramStart >= 0
        ? [{ label: t('nvram.sectorMap.presetNvram'), start: nvramStart, end: maxIdx }]
        : []),
    ];
  }, [maxIdx, sectorMap, t]);

  return (
    <div className="rounded-2xl bg-bench-900 border border-slate-800 p-5 shadow-xl space-y-4 font-mono">
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 border-b border-slate-800 pb-3">
        <div>
          <h3 className="text-xs font-semibold uppercase tracking-wider text-slate-300 flex items-center gap-2">
            <Layers className="w-4 h-4 text-cyan-400" />
            <span>{t('nvram.sectorMap.title')}</span>
          </h3>
          <p className="text-[11px] text-slate-500 font-sans mt-0.5">
            {t('nvram.sectorMap.subtitle')}
          </p>
        </div>

        <div className="flex flex-wrap items-center gap-1.5 p-1 rounded-xl bg-slate-950 border border-slate-800 text-xs">
          <button
            type="button"
            onClick={() => onSelectFilter('all')}
            className={cn(
              'px-2.5 py-1 rounded-lg transition-colors',
              filterType === 'all' ? 'bg-cyan-500/20 text-cyan-300 font-bold' : 'text-slate-400 hover:text-slate-200',
            )}
          >
            {t('nvram.sectorMap.filterAllCount')}
          </button>
          <button
            type="button"
            onClick={() => onSelectFilter('nvram')}
            className={cn(
              'px-2.5 py-1 rounded-lg transition-colors',
              filterType === 'nvram' ? 'bg-amber-500/20 text-amber-300 font-bold' : 'text-slate-400 hover:text-slate-200',
            )}
          >
            {t('nvram.sectorMap.filterNvram')}
          </button>
          <button
            type="button"
            onClick={() => onSelectFilter('bootloader')}
            className={cn(
              'px-2.5 py-1 rounded-lg transition-colors',
              filterType === 'bootloader' ? 'bg-cyan-500/20 text-cyan-300 font-bold' : 'text-slate-400 hover:text-slate-200',
            )}
          >
            {t('nvram.sectorMap.filterBootloader')}
          </button>
        </div>
      </div>

      {sectorMap.length > 1 && (
        <RangeDndScrubber
          label={t('nvram.sectorMap.scrubberLabel')}
          min={0}
          max={maxIdx}
          startValue={lo}
          endValue={hi}
          formatValue={(idx) => sectorMap[idx]?.name ?? `#${idx}`}
          presets={presets}
          onChangeRange={(start, end) => {
            setRangeState({ scope: scopeKey, start, end });
            if (sectorMap[start]) onSelectSector(sectorMap[start]);
          }}
        />
      )}

      <div className="p-3 rounded-xl bg-slate-950 border border-slate-800/80 overflow-hidden">
        <div className="flex flex-wrap items-center justify-between gap-2 text-[11px] text-slate-500 mb-2 px-1">
          <span>{t('nvram.sectorMap.addrStart')}</span>
          <span className="text-amber-400 font-bold">{t('nvram.sectorMap.addrNvram')}</span>
          <span>{t('nvram.sectorMap.addrEnd')}</span>
        </div>

        <div className="grid grid-cols-[repeat(auto-fill,minmax(26px,1fr))] gap-1">
          {sectorMap.length === 0 ? (
            <div className="col-span-full py-8 text-center text-xs text-slate-500 font-mono">
              {t('nvram.sectorMap.empty')}
            </div>
          ) : (
            sectorMap.map((sector, idx) => {
              const isSelected = selectedSector?.name === sector.name;
              const inRange = idx >= lo && idx <= hi;
              const tileBg = getSectorTileClass(sector, cleanSuccess);

              return (
                <button
                  key={sector.name}
                  type="button"
                  onPointerDown={() => {
                    setDragAnchorIdx(idx);
                    setRangeState({ scope: scopeKey, start: idx, end: idx });
                    onSelectSector(sector);
                  }}
                  onPointerEnter={() => {
                    if (dragAnchorIdx !== null) {
                      setRangeState({
                        scope: scopeKey,
                        start: Math.min(dragAnchorIdx, idx),
                        end: Math.max(dragAnchorIdx, idx),
                      });
                      onSelectSector(sector);
                    }
                  }}
                  onClick={() => onSelectSector(sector)}
                  title={t('nvram.sectorMap.tileTitle', {
                    name: sector.name,
                    offset: sector.start_offset.toString(16).toUpperCase(),
                    kb: sector.size_bytes / KIB_BYTES,
                  })}
                  className={cn(
                    'h-7 rounded text-[9px] font-bold flex items-center justify-center transition-all select-none',
                    tileBg,
                    inRange ? 'opacity-100' : 'opacity-35',
                    isSelected && 'ring-2 ring-cyan-400 scale-105 z-10',
                  )}
                >
                  {sector.index}
                </button>
              );
            })
          )}
        </div>

        <div className="flex flex-wrap items-center justify-between gap-3 mt-3 pt-3 border-t border-slate-800 text-[11px] text-slate-400">
          <div className="flex flex-wrap items-center gap-4">
            <div className="flex items-center gap-1.5">
              <span className="w-3 h-3 rounded bg-cyan-900/60 border border-cyan-700/50" />
              <span>{t('nvram.sectorMap.legendBoot')}</span>
            </div>
            <div className="flex items-center gap-1.5">
              <span className="w-3 h-3 rounded bg-slate-800" />
              <span>{t('nvram.sectorMap.legendOs')}</span>
            </div>
            <div className="flex items-center gap-1.5">
              <span className={cn('w-3 h-3 rounded', cleanSuccess ? 'bg-emerald-500' : 'bg-rose-500 animate-pulse')} />
              <span>
                {cleanSuccess
                  ? t('nvram.sectorMap.legendClean')
                  : t('nvram.sectorMap.legendCorrupted')}
              </span>
            </div>
          </div>

          {rangeMetrics.first && rangeMetrics.last && (
            <span className="text-cyan-300 font-semibold">
              {t('nvram.sectorMap.rangeSummary', {
                first: rangeMetrics.first.name,
                last: rangeMetrics.last.name,
                count: rangeMetrics.count,
                kb: rangeMetrics.totalKb,
              })}
            </span>
          )}
        </div>
      </div>

      {selectedSector && (
        <div className="p-4 rounded-xl bg-bench-950 border border-slate-800 flex flex-col md:flex-row items-start md:items-center justify-between gap-4">
          <div className="space-y-1">
            <div className="flex items-center gap-2">
              <span className="text-sm font-bold text-slate-100">
                {t('nvram.sectorMap.selectedPrefix', { name: selectedSector.name })}
              </span>
              <StatusBadge
                variant={selectedSector.is_corrupted_area ? (cleanSuccess ? 'success' : 'error') : 'info'}
                label={getSectorInspectorBadgeLabel(selectedSector, cleanSuccess, t)}
              />
            </div>
            <p className="text-xs text-slate-400 font-sans">{selectedSector.notes}</p>
          </div>

          <div className="flex flex-wrap items-center gap-4 text-xs font-mono">
            <div className="text-slate-400">
              <span className="text-slate-500 block text-[10px]">{t('nvram.sectorMap.fieldOffset')}</span>
              <span>
                0x{selectedSector.start_offset.toString(16).padStart(HEX32_PAD_WIDTH, '0').toUpperCase()}
              </span>
            </div>
            <div className="text-slate-400">
              <span className="text-slate-500 block text-[10px]">{t('nvram.sectorMap.fieldSize')}</span>
              <span>{t('nvram.sectorMap.sizeKb', { kb: selectedSector.size_bytes / KIB_BYTES })}</span>
            </div>
            <div className="text-slate-400">
              <span className="text-slate-500 block text-[10px]">{t('nvram.sectorMap.fieldActive')}</span>
              <span className={selectedSector.is_corrupted_area && !cleanSuccess ? 'text-rose-400 font-bold' : 'text-slate-200'}>
                {t('nvram.sectorMap.bytesUnit', { bytes: selectedSector.active_bytes ?? 0 })}
              </span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
