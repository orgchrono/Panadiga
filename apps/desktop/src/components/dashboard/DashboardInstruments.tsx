import React, { useState } from 'react';
import { Reorder } from 'framer-motion';
import { ViewKey } from '@/components/Sidebar';
import { HardDrive, Cpu, Radio, ArrowRight, GripHorizontal } from 'lucide-react';
import { cn } from '@/lib/utils';

interface DashboardInstrumentsProps {
  onNavigate: (view: ViewKey) => void;
}

interface InstrumentCardDef {
  readonly view: ViewKey;
  readonly ariaLabel: string;
  readonly title: string;
  readonly description: string;
  readonly footerLeft: string;
  readonly footerRight: string;
  readonly borderHover: string;
  readonly iconTone: string;
  readonly titleHover: string;
  readonly footerTone: string;
  readonly Icon: React.ComponentType<{ className?: string }>;
}

const DEFAULT_INSTRUMENTS: readonly InstrumentCardDef[] = [
  {
    view: 'nvram',
    ariaLabel: 'Abrir Cirurgia de NVRAM U99',
    title: 'Cirurgia de NVRAM (U99)',
    description:
      'Auditoria de microsetores SA0..SA141, limpeza cirúrgica dos 64KB de EEPROM emulada (0xFF) mantendo o bootloader imutável.',
    footerLeft: '8 Microsetores de 8KB',
    footerRight: '1-Clique Fix',
    borderHover: 'hover:border-amber-500/50 focus:ring-amber-500/50',
    iconTone: 'bg-amber-500/10 text-amber-400 border-amber-500/20',
    titleHover: 'group-hover:text-amber-300',
    footerTone: 'text-amber-400',
    Icon: Cpu,
  },
  {
    view: 'hdd',
    ariaLabel: 'Abrir Forensics do Disco Rígido',
    title: 'Forensics do Disco Rígido (100MB)',
    description:
      'Validação MBR "SH" vs Espelho LBA 57.344, decodificação MEIHDFS (XOR 0xFF), histórico do Journal WAL (10 blocos) e extração com manifesto.',
    footerLeft: '256 Registros MEIHDFS',
    footerRight: 'SHA256SUMS',
    borderHover: 'hover:border-cyan-500/50 focus:ring-cyan-500/50',
    iconTone: 'bg-cyan-500/10 text-cyan-400 border-cyan-500/20',
    titleHover: 'group-hover:text-cyan-300',
    footerTone: 'text-cyan-400',
    Icon: HardDrive,
  },
  {
    view: 'picobench',
    ariaLabel: 'Abrir Bancada PicoBench P7501 UART',
    title: 'Bancada PicoBench (P7501 UART / OCD)',
    description:
      'Captura UART P7501 (115200 8N1), matriz elétrica de 6 pinos, monitoramento ADC0 SW_UART e modo legado Cold Break-In / RAM Loader.',
    footerLeft: 'P7501 Async UART',
    footerRight: 'Boot & Recovery',
    borderHover: 'hover:border-emerald-500/50 focus:ring-emerald-500/50',
    iconTone: 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20',
    titleHover: 'group-hover:text-emerald-300',
    footerTone: 'text-emerald-400',
    Icon: Radio,
  },
];

export const DashboardInstruments: React.FC<DashboardInstrumentsProps> = ({ onNavigate }) => {
  const [cards, setCards] = useState<InstrumentCardDef[]>(() => [...DEFAULT_INSTRUMENTS]);

  return (
    <div>
      <div className="flex flex-wrap items-center justify-between gap-2 mb-3">
        <h3 className="text-xs font-mono font-semibold uppercase tracking-wider text-slate-400 flex items-center gap-2">
          <span>Instrumentos de Bancada & Recuperação</span>
        </h3>
        <span className="text-[10px] font-mono text-slate-500">
          Arraste os cartões para reorganizar a prioridade
        </span>
      </div>

      {cards.length === 0 ? (
        <div className="p-4 rounded-xl border border-slate-800 bg-bench-900 text-xs text-slate-500 font-mono">
          Nenhum instrumento configurado.
        </div>
      ) : (
        <Reorder.Group
          axis="x"
          values={cards}
          onReorder={setCards}
          className="grid grid-cols-1 md:grid-cols-3 gap-4 list-none p-0 m-0"
        >
          {cards.map(card => {
            const Icon = card.Icon;
            return (
              <Reorder.Item
                key={card.view}
                value={card}
                whileDrag={{ scale: 1.02 }}
                className="list-none"
              >
                <div
                  role="button"
                  tabIndex={0}
                  aria-label={card.ariaLabel}
                  onClick={() => onNavigate(card.view)}
                  onKeyDown={e => (e.key === 'Enter' || e.key === ' ') && onNavigate(card.view)}
                  className={cn(
                    'group cursor-pointer h-full p-4 rounded-xl bg-bench-900 border border-slate-800 hover:bg-bench-850 transition-all duration-150 shadow-lg flex flex-col justify-between focus:outline-none focus:ring-1',
                    card.borderHover
                  )}
                >
                  <div>
                    <div className="flex items-center justify-between mb-2">
                      <div
                        className={cn(
                          'p-2 rounded-lg border group-hover:scale-110 transition-transform',
                          card.iconTone
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
                      {card.title}
                    </h4>
                    <p className="text-[11px] text-slate-400 mt-1 leading-relaxed">
                      {card.description}
                    </p>
                  </div>
                  <div
                    className={cn(
                      'mt-4 pt-3 border-t border-slate-800 flex items-center justify-between text-[11px] font-mono',
                      card.footerTone
                    )}
                  >
                    <span>{card.footerLeft}</span>
                    <span className="text-slate-500">{card.footerRight}</span>
                  </div>
                </div>
              </Reorder.Item>
            );
          })}
        </Reorder.Group>
      )}
    </div>
  );
};
