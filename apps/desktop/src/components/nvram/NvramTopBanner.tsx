import React from 'react';
import { CheckCircle2, FileDown } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useTranslation } from '@/i18n/useTranslation';

interface NvramTopBannerProps {
  onOpenAudit: () => void;
}

export const NvramTopBanner: React.FC<NvramTopBannerProps> = ({ onOpenAudit }) => {
  const { t } = useTranslation();

  return (
    <output
      aria-live="polite"
      className="rounded-xl bg-emerald-500/10 border border-emerald-500/40 p-4 flex items-center justify-between shadow-xl"
    >
      <div className="flex items-center gap-3">
        <CheckCircle2 className="w-5 h-5 text-emerald-400 shrink-0" />
        <div>
          <h4 className="text-xs font-bold text-emerald-300">
            {t('nvram.topBanner.title')}
          </h4>
          <p className="text-[11px] text-slate-300 mt-0.5">
            {t('nvram.topBanner.desc')}
          </p>
        </div>
      </div>
      <Button
        variant="outline"
        size="sm"
        onClick={onOpenAudit}
        className="border-emerald-500/40 text-emerald-300 hover:bg-emerald-500/20 gap-1.5"
      >
        <FileDown className="w-3.5 h-3.5" />
        <span>{t('nvram.topBanner.auditBtn')}</span>
      </Button>
    </output>
  );
};
