import React, { useMemo, useState } from 'react';
import { Reorder } from 'framer-motion';
import { ViewKey } from '@/components/Sidebar';
import type { TranslationKey } from '@/i18n/translations';
import { useTranslation } from '@/i18n/useTranslation';
import { HardDrive, Cpu, Radio, ArrowRight, GripHorizontal } from 'lucide-react';
import { cn } from '@/lib/utils';

interface DashboardInstrumentsProps {
  onNavigate: (view: ViewKey) => void;
}

interface InstrumentStyleDef {
  readonly view: ViewKey;
  readonly ariaKey: TranslationKey;
  readonly titleKey: TranslationKey;
  readonly descKey: TranslationKey;
  readonly footerLeftKey: TranslationKey;
  readonly footerRightKey: TranslationKey;
  readonly borderHover: string;
  readonly iconTone: string;
  readonly titleHover: string;
  readonly footerTone: string;
  readonly Icon: React.ComponentType<{ className?: string }>;
}

const DEFAULT_INSTRUMENTS: readonly InstrumentStyleDef[] = [
  {
    view: 'nvram',
    ariaKey: 'dashboard.instruments.nvram.aria',
    titleKey: 'dashboard.instruments.nvram.title',
    descKey: 'dashboard.instruments.nvram.desc',
    footerLeftKey: 'dashboard.instruments.nvram.footerLeft',
    footerRightKey: 'dashboard.instruments.nvram.footerRight',
    borderHover: 'hover:border-amber-500/50 focus:ring-amber-500/50',
    iconTone: 'bg-amber-500/10 text-amber-400 border-amber-500/20',
    titleHover: 'group-hover:text-amber-300',
    footerTone: 'text-amber-400',
    Icon: Cpu,
  },
  {
    view: 'hdd',
    ariaKey: 'dashboard.instruments.hdd.aria',
    titleKey: 'dashboard.instruments.hdd.title',
    descKey: 'dashboard.instruments.hdd.desc',
    footerLeftKey: 'dashboard.instruments.hdd.footerLeft',
    footerRightKey: 'dashboard.instruments.hdd.footerRight',
    borderHover: 'hover:border-cyan-500/50 focus:ring-cyan-500/50',
    iconTone: 'bg-cyan-500/10 text-cyan-400 border-cyan-500/20',
    titleHover: 'group-hover:text-cyan-300',
    footerTone: 'text-cyan-400',
    Icon: HardDrive,
  },
  {
    view: 'picobench',
    ariaKey: 'dashboard.instruments.picobench.aria',
    titleKey: 'dashboard.instruments.picobench.title',
    descKey: 'dashboard.instruments.picobench.desc',
    footerLeftKey: 'dashboard.instruments.picobench.footerLeft',
    footerRightKey: 'dashboard.instruments.picobench.footerRight',
    borderHover: 'hover:border-emerald-500/50 focus:ring-emerald-500/50',
    iconTone: 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20',
    titleHover: 'group-hover:text-emerald-300',
    footerTone: 'text-emerald-400',
    Icon: Radio,
  },
];

export const DashboardInstruments: React.FC<DashboardInstrumentsProps> = ({ onNavigate }) => {
  const { t } = useTranslation();
  const [order, setOrder] = useState<ViewKey[]>(() => DEFAULT_INSTRUMENTS.map((d) => d.view));

  const orderedCards = useMemo(
    () =>
      order
        .map((view) => DEFAULT_INSTRUMENTS.find((item) => item.view === view))
        .filter((item): item is InstrumentStyleDef => item !== undefined),
    [order],
  );

  return (
    <div>
      <div className="flex flex-wrap items-center justify-between gap-2 mb-3">
        <h3 className="text-xs font-mono font-semibold uppercase tracking-wider text-slate-400 flex items-center gap-2">
          <span>{t('dashboard.instruments.heading')}</span>
        </h3>
        <span className="text-[10px] font-mono text-slate-500">
          {t('dashboard.instruments.dragHint')}
        </span>
      </div>

      {orderedCards.length === 0 ? (
        <div className="p-4 rounded-xl border border-slate-800 bg-bench-900 text-xs text-slate-500 font-mono">
          {t('dashboard.instruments.empty')}
        </div>
      ) : (
        <Reorder.Group
          axis="x"
          values={order}
          onReorder={setOrder}
          className="grid grid-cols-1 md:grid-cols-3 gap-4 list-none p-0 m-0"
        >
          {orderedCards.map((card) => {
            const Icon = card.Icon;
            return (
              <Reorder.Item
                key={card.view}
                value={card.view}
                whileDrag={{ scale: 1.02 }}
                className="list-none"
              >
                <button
                  type="button"
                  aria-label={t(card.ariaKey)}
                  onClick={() => onNavigate(card.view)}
                  className={cn(
                    'w-full text-left group cursor-pointer h-full p-4 rounded-xl bg-bench-900 border border-slate-800 hover:bg-bench-850 transition-all duration-150 shadow-lg flex flex-col justify-between focus:outline-none focus:ring-1',
                    card.borderHover,
                  )}
                >
                  <div>
                    <div className="flex items-center justify-between mb-2">
                      <div
                        className={cn(
                          'p-2 rounded-lg border group-hover:scale-110 transition-transform',
                          card.iconTone,
                        )}
                      >
                        <Icon className="w-5 h-5" />
                      </div>
                      <div className="flex items-center gap-1.5 text-slate-600">
                        <GripHorizontal className="w-4 h-4 cursor-grab active:cursor-grabbing" />
                        <ArrowRight className="w-4 h-4 group-hover:translate-x-0.5 transition-all" />
                      </div>
                    </div>
                    <h4 className={cn('text-xs font-bold font-mono text-slate-200', card.titleHover)}>
                      {t(card.titleKey)}
                    </h4>
                    <p className="text-[11px] text-slate-400 mt-1 leading-relaxed">
                      {t(card.descKey)}
                    </p>
                  </div>
                  <div
                    className={cn(
                      'mt-4 pt-3 border-t border-slate-800 flex items-center justify-between text-[11px] font-mono',
                      card.footerTone,
                    )}
                  >
                    <span>{t(card.footerLeftKey)}</span>
                    <span className="text-slate-500">{t(card.footerRightKey)}</span>
                  </div>
                </button>
              </Reorder.Item>
            );
          })}
        </Reorder.Group>
      )}
    </div>
  );
};
