import React from 'react';
import { ArrowRight, Globe, Layers, ShieldCheck, Wrench } from 'lucide-react';
import { Badge, Button } from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';

export interface ExpressHeaderProps {
  readonly onEnterEngineering: () => void;
}

export const ExpressHeader: React.FC<ExpressHeaderProps> = ({ onEnterEngineering }) => {
  const { t, locale, setLocale } = useTranslation();

  return (
    <header
      data-testid="express-header"
      className="px-4 sm:px-6 py-3 bg-bench-900/95 backdrop-blur border-b border-slate-800 flex flex-wrap items-center justify-between gap-3 shrink-0"
    >
      <div className="flex items-center gap-3 min-w-0">
        <div className="w-9 h-9 rounded-xl bg-gradient-to-br from-emerald-500 to-cyan-600 flex items-center justify-center shadow text-white shrink-0">
          <ShieldCheck className="w-5 h-5" />
        </div>
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <h1 className="text-sm sm:text-base font-bold font-mono text-slate-100 tracking-tight truncate">
              PANADIGA // DMR-EH55
            </h1>
            <Badge variant="success" className="text-[10px]">
              {t('express.header.badge')}
            </Badge>
          </div>
          <p className="text-[11px] text-slate-400 font-sans truncate">
            {t('express.header.subtitle')}
          </p>
        </div>
      </div>

      <div className="flex items-center gap-2 shrink-0">
        <Button
          type="button"
          variant="outline"
          size="sm"
          data-testid="express-lang-toggle"
          onClick={() => setLocale(locale === 'pt-BR' ? 'en-US' : 'pt-BR')}
          className="h-8 px-2.5 text-xs font-mono gap-1.5 border-slate-700 bg-bench-950"
          title={t('express.header.toggleLanguage')}
        >
          <Globe className="w-3.5 h-3.5 text-cyan-400" />
          <span>{locale}</span>
        </Button>

        <div className="inline-flex items-center rounded-xl border border-slate-800 bg-bench-950 p-0.5 text-xs font-mono">
          <span
            data-testid="express-mode-pill"
            className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-emerald-500/20 text-emerald-300 font-bold"
          >
            <Layers className="w-3.5 h-3.5" />
            <span>{t('express.header.modePillActive')}</span>
          </span>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            data-testid="express-to-engineering-btn"
            onClick={onEnterEngineering}
            className="h-7 px-2.5 text-xs font-mono text-cyan-300 hover:text-cyan-200 gap-1"
          >
            <Wrench className="w-3.5 h-3.5" />
            <span>{t('express.header.switchToEngineering')}</span>
            <ArrowRight className="w-3 h-3" />
          </Button>
        </div>
      </div>
    </header>
  );
};
