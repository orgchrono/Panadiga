import React from 'react';
import { useHexInspectorViewModel } from '@/viewmodels/useHexInspectorViewModel';
import { MetricCard } from '@/components/MetricCard';
import { StatusBadge } from '@/components/StatusBadge';
import { RangeDndScrubber, ResizableSplitPane } from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import {
  Binary,
  ChevronLeft,
  ChevronRight,
  Compass,
} from 'lucide-react';

export const HexInspectorView: React.FC = () => {
  const { t } = useTranslation();
  const {
    currentOffset,
    currentLba,
    hexRows,
    bookmarks,
    selectedBookmark,
    jumpInput,
    setJumpInput,
    loading,
    feedbackMessage,
    selectBookmark,
    jumpToOffset,
    jumpToInputOffset,
    nextPage,
    prevPage,
  } = useHexInspectorViewModel();

  return (
    <div className="space-y-6 max-w-7xl mx-auto pb-12 font-mono">
      {/* Top Header */}
      <div className="flex flex-col md:flex-row items-start md:items-center justify-between gap-4 p-5 rounded-2xl bg-bench-900 border border-slate-800 shadow-xl">
        <div className="flex items-center gap-3.5">
          <div className="p-3 rounded-xl bg-cyan-500/10 text-cyan-400 border border-cyan-500/30">
            <Binary className="w-6 h-6" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h2 className="text-base font-bold text-slate-100">
                {t('hex.title')}
              </h2>
              <StatusBadge variant="info" label={t('hex.badge')} />
            </div>
            <p className="text-xs text-slate-400 mt-0.5">
              {t('hex.subtitle')}
            </p>
          </div>
        </div>

        {/* Jump & Navigation Controls */}
        <div className="flex items-center gap-2">
          <div className="relative">
            <input
              type="text"
              placeholder={t('hex.jumpPlaceholder')}
              value={jumpInput}
              onChange={e => setJumpInput(e.target.value)}
              onKeyDown={e => e.key === 'Enter' && jumpToInputOffset()}
              aria-label={t('hex.inputAria')}
              className="px-3 py-2 rounded-xl bg-slate-950 border border-slate-800 text-xs text-cyan-400 focus:outline-none focus:border-cyan-500 w-36 font-bold focus:ring-1 focus:ring-cyan-500"
            />
          </div>
          <button
            onClick={jumpToInputOffset}
            className="px-3 py-2 rounded-xl bg-slate-800 hover:bg-slate-750 text-slate-200 text-xs font-semibold border border-slate-700 transition-colors focus:outline-none focus:ring-2 focus:ring-slate-400"
          >
            {t('hex.jumpButton')}
          </button>
        </div>
      </div>

      {feedbackMessage && (
        <div className="p-3 rounded-xl bg-emerald-950/80 border border-emerald-500/40 text-emerald-300 text-xs font-mono flex items-center justify-between animate-in fade-in duration-200">
          <span>{feedbackMessage}</span>
        </div>
      )}

      {/* Telemetry Metrics */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <MetricCard
          title={t('hex.metrics.offsetTitle')}
          value={`0x${currentOffset.toString(16).padStart(8, '0').toUpperCase()}`}
          subtitle={t('hex.metrics.offsetSub', { bytes: currentOffset.toLocaleString() })}
          variant="cyan"
        />
        <MetricCard
          title={t('hex.metrics.lbaTitle')}
          value={`LBA ${currentLba.toLocaleString()}`}
          subtitle={t('hex.metrics.lbaSub')}
          variant="emerald"
        />
        <MetricCard
          title={t('hex.metrics.bookmarkTitle')}
          value={selectedBookmark ? selectedBookmark.category : t('hex.metrics.bookmarkFree')}
          subtitle={selectedBookmark ? selectedBookmark.label : t('hex.metrics.bookmarkManual')}
          variant="amber"
        />
        <MetricCard
          title={t('hex.metrics.windowTitle')}
          value={`${hexRows.length * 16}`}
          unit={t('hex.metrics.windowUnit')}
          subtitle={t('hex.metrics.windowSub', { rows: hexRows.length })}
          variant="slate"
        />
      </div>

      {/* RangeDnD Scrubbing Window across 100 MB Disk / LBA Space */}
      <RangeDndScrubber
        label={t('hex.scrubberLabel')}
        min={0}
        max={104857600}
        step={512}
        startValue={currentOffset}
        endValue={Math.min(104857600, currentOffset + hexRows.length * 16)}
        formatValue={val => `0x${val.toString(16).toUpperCase().padStart(8, '0')} (LBA ${Math.floor(val / 512)})`}
        presets={[
          { label: t('hex.presets.mbrLba0'), start: 0, end: 384 },
          { label: t('hex.presets.journal8mb'), start: 8388608, end: 8388992 },
          { label: t('hex.presets.mirror28mb'), start: 29360128, end: 29360512 },
          { label: t('hex.presets.part1_32mb'), start: 33554432, end: 33554816 },
        ]}
        onChangeRange={start => jumpToOffset(start)}
      />

      {/* Main Hex Workspace: Resizable Split (Bookmarks vs Hex Dump Grid) */}
      <ResizableSplitPane
        initialLeftPercent={34}
        minLeftPercent={22}
        maxLeftPercent={55}
        storageKey="panadiga.split.hex"
        ariaLabel={t('hex.splitAria')}
        left={
          <div className="flex flex-col h-[520px] lg:h-[680px]">
            <div className="rounded-xl bg-bench-900 border border-slate-800 p-5 shadow-xl space-y-3 flex flex-col h-full">
              <h3 className="text-xs font-semibold uppercase tracking-wider text-slate-300 flex items-center gap-2 shrink-0">
                <Compass className="w-4 h-4 text-cyan-400" />
                <span>{t('hex.criticalBookmarks')}</span>
              </h3>

              <div className="space-y-2 flex-1 min-h-0 overflow-y-auto pr-1 scrollbar-thin scrollbar-thumb-slate-800">
                {bookmarks.length === 0 ? (
                  <div className="p-6 text-center text-xs text-slate-500 font-mono">
                    {t('hex.noBookmarks')}
                  </div>
                ) : (
                  bookmarks.map(bm => {
                    const isSelected = selectedBookmark?.id === bm.id;
                    return (
                      <button
                        key={bm.id}
                        type="button"
                        onClick={() => selectBookmark(bm)}
                        className={`w-full text-left p-3 rounded-xl border cursor-pointer transition-all ${
                          isSelected
                            ? 'bg-cyan-500/10 border-cyan-500/50 shadow-md shadow-cyan-950/20'
                            : 'bg-slate-950 border-slate-800 hover:border-slate-700'
                        }`}
                      >
                        <div className="flex items-center justify-between mb-1">
                          <span className={`text-xs font-bold ${isSelected ? 'text-cyan-400' : 'text-slate-200'}`}>
                            {bm.label}
                          </span>
                          <span className="text-[10px] text-slate-500 font-mono">
                            {bm.offset_hex}
                          </span>
                        </div>
                        <p className="text-[11px] text-slate-400 font-sans">
                          {bm.description}
                        </p>
                      </button>
                    );
                  })
                )}
              </div>
            </div>
          </div>
        }
        right={
          <div className="flex flex-col h-[520px] lg:h-[680px]">
            <div className="rounded-xl bg-bench-900 border border-slate-800 shadow-xl overflow-hidden flex flex-col h-full">
            {/* Hex Header Bar */}
            <div className="px-4 py-2.5 bg-bench-950 border-b border-slate-800 flex items-center justify-between text-xs text-slate-400 shrink-0">
              <div className="flex items-center gap-6">
                <span className="w-24 font-bold text-slate-500">OFFSET</span>
                <span className="w-52 font-bold tracking-wider text-slate-400">
                  00 01 02 03 04 05 06 07  08 09 0A 0B 0C 0D 0E 0F
                </span>
                <span className="font-bold text-slate-500">ASCII DUMP</span>
              </div>

              {/* Pagination controls */}
              <div className="flex items-center gap-1">
                <button
                  onClick={prevPage}
                  className="p-1 rounded-lg hover:bg-slate-800 text-slate-400 hover:text-slate-200 transition-colors focus:outline-none focus:ring-2 focus:ring-slate-400"
                  title={t('hex.prevPageTitle')}
                  aria-label={t('hex.prevPageTitle')}
                >
                  <ChevronLeft className="w-4 h-4" />
                </button>
                <button
                  onClick={nextPage}
                  className="p-1 rounded-lg hover:bg-slate-800 text-slate-400 hover:text-slate-200 transition-colors focus:outline-none focus:ring-2 focus:ring-slate-400"
                  title={t('hex.nextPageTitle')}
                  aria-label={t('hex.nextPageTitle')}
                >
                  <ChevronRight className="w-4 h-4" />
                </button>
              </div>
            </div>

            {/* Rows */}
            <div className="p-4 bg-slate-950 overflow-x-auto overflow-y-auto space-y-1 text-xs select-text flex-1 min-h-0">
              {loading ? (
                <div className="py-12 text-center text-slate-500">
                  {t('hex.loadingBytes')}
                </div>
              ) : (
                hexRows.map(row => (
                  <div
                    key={row.offset}
                    className="flex items-center gap-6 hover:bg-slate-900/60 px-2 py-0.5 rounded transition-colors group"
                  >
                    {/* Offset */}
                    <span className="w-24 text-cyan-400 font-bold shrink-0">
                      {row.offset_hex}
                    </span>

                    {/* 16 Hex bytes split 8 / 8 */}
                    <div className="w-52 flex items-center gap-3 shrink-0">
                      <span className="text-slate-200 tracking-wider">
                        {row.hex_part1}
                      </span>
                      <span className="text-slate-300 tracking-wider">
                        {row.hex_part2}
                      </span>
                    </div>

                    {/* ASCII */}
                    <span className="text-emerald-400 tracking-wider shrink-0 font-bold border-l border-slate-800 pl-4 group-hover:text-emerald-300">
                      |{row.ascii}|
                    </span>
                  </div>
                ))
              )}
            </div>

            {/* Hex Footer */}
            <div className="px-4 py-2 bg-bench-950 border-t border-slate-800 flex items-center justify-between text-[11px] text-slate-500">
              <span>{t('hex.footerMode')}</span>
              <div className="flex items-center gap-3">
                <button
                  onClick={prevPage}
                  className="text-slate-400 hover:text-slate-200 underline"
                >
                  {t('hex.footerPrev')}
                </button>
                <span>•</span>
                <button
                  onClick={nextPage}
                  className="text-slate-400 hover:text-slate-200 underline"
                >
                  {t('hex.footerNext')}
                </button>
              </div>
            </div>
          </div>
        </div>
        }
      />
    </div>
  );
};
