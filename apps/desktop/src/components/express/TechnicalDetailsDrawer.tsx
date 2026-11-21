import React, { useState } from 'react';
import { ChevronDown, ChevronRight, Terminal, Wrench } from 'lucide-react';
import { Button, EmptyState } from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';

export interface TechnicalDetailRow {
  readonly label: string;
  readonly value: string;
}

export interface TechnicalDetailsDrawerProps {
  readonly rows: readonly TechnicalDetailRow[];
  readonly onOpenEngineering: () => void;
  readonly testIdPrefix?: string;
}

export const TechnicalDetailsDrawer: React.FC<TechnicalDetailsDrawerProps> = ({
  rows,
  onOpenEngineering,
  testIdPrefix = 'express-tech',
}) => {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);

  return (
    <div className="border-t border-slate-800/80 pt-3 flex flex-col gap-2">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <Button
          type="button"
          variant="ghost"
          size="sm"
          aria-expanded={open}
          data-testid={`${testIdPrefix}-toggle`}
          onClick={() => setOpen(prev => !prev)}
          className="h-7 px-2 text-xs font-mono text-slate-400 hover:text-slate-200 gap-1.5"
        >
          {open ? <ChevronDown className="w-3.5 h-3.5" /> : <ChevronRight className="w-3.5 h-3.5" />}
          <Terminal className="w-3.5 h-3.5 text-cyan-400" />
          <span>{open ? t('express.technical.toggleHide') : t('express.technical.toggleShow')}</span>
        </Button>

        <Button
          type="button"
          variant="outline"
          size="sm"
          data-testid={`${testIdPrefix}-open-engineering`}
          onClick={onOpenEngineering}
          className="h-7 px-2.5 text-[11px] font-mono gap-1.5 border-slate-700 text-cyan-300"
        >
          <Wrench className="w-3 h-3" />
          <span>{t('express.technical.openEngineeringBtn')}</span>
        </Button>
      </div>

      {open && (
        <div
          data-testid={`${testIdPrefix}-content`}
          className="rounded-xl border border-slate-800 bg-slate-950 p-3 font-mono text-[11px] space-y-1.5"
        >
          {rows.length === 0 ? (
            <EmptyState compact title={t('express.technical.emptyDetails')} />
          ) : (
            rows.map(row => (
              <div
                key={`${row.label}:${row.value}`}
                className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-1 border-b border-slate-900 pb-1 last:border-b-0 last:pb-0"
              >
                <span className="text-slate-500">{row.label}</span>
                <span className="text-slate-200 break-all">{row.value}</span>
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
};
