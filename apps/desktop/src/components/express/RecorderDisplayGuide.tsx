import React from 'react';
import { MonitorCheck } from 'lucide-react';
import { EmptyState } from '@/components/ui';
import type { TranslationKey } from '@/i18n/translations';
import { useTranslation } from '@/i18n/useTranslation';

interface VfdStageDef {
  readonly codeKey: TranslationKey;
  readonly titleKey: TranslationKey;
  readonly descKey: TranslationKey;
  readonly toneClass: string;
}

const VFD_STAGES: readonly VfdStageDef[] = [
  {
    codeKey: 'express.vfdGuide.steps.sdCode',
    titleKey: 'express.vfdGuide.steps.sdTitle',
    descKey: 'express.vfdGuide.steps.sdDesc',
    toneClass: 'border-cyan-500/40 text-cyan-300',
  },
  {
    codeKey: 'express.vfdGuide.steps.readCode',
    titleKey: 'express.vfdGuide.steps.readTitle',
    descKey: 'express.vfdGuide.steps.readDesc',
    toneClass: 'border-cyan-500/40 text-cyan-200',
  },
  {
    codeKey: 'express.vfdGuide.steps.writeCode',
    titleKey: 'express.vfdGuide.steps.writeTitle',
    descKey: 'express.vfdGuide.steps.writeDesc',
    toneClass: 'border-amber-500/50 text-amber-300',
  },
  {
    codeKey: 'express.vfdGuide.steps.completeCode',
    titleKey: 'express.vfdGuide.steps.completeTitle',
    descKey: 'express.vfdGuide.steps.completeDesc',
    toneClass: 'border-emerald-500/50 text-emerald-300',
  },
  {
    codeKey: 'express.vfdGuide.steps.f99Code',
    titleKey: 'express.vfdGuide.steps.f99Title',
    descKey: 'express.vfdGuide.steps.f99Desc',
    toneClass: 'border-emerald-500/40 text-emerald-200',
  },
  {
    codeKey: 'express.vfdGuide.steps.swoffCode',
    titleKey: 'express.vfdGuide.steps.swoffTitle',
    descKey: 'express.vfdGuide.steps.swoffDesc',
    toneClass: 'border-slate-600 text-slate-200',
  },
];

export const RecorderDisplayGuide: React.FC = () => {
  const { t } = useTranslation();

  return (
    <section
      data-testid="recorder-display-guide"
      className="rounded-xl border border-slate-800 bg-slate-950/90 p-4 space-y-3"
    >
      <div className="space-y-0.5">
        <h4 className="text-xs font-mono font-bold text-slate-100 flex items-center gap-2">
          <MonitorCheck className="w-4 h-4 text-emerald-400 shrink-0" />
          <span>{t('express.vfdGuide.title')}</span>
        </h4>
        <p className="text-[11px] text-slate-400 font-sans">{t('express.vfdGuide.subtitle')}</p>
      </div>

      {VFD_STAGES.length === 0 ? (
        <EmptyState compact title={t('express.vfdGuide.emptySteps')} />
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2.5">
          {VFD_STAGES.map((stage, idx) => (
            <div
              key={stage.codeKey}
              className="rounded-lg border border-slate-800 bg-bench-900/80 p-2.5 space-y-1.5"
            >
              <div className="flex items-center justify-between gap-2">
                <span className="text-[10px] font-mono text-slate-500">
                  {String(idx + 1).padStart(2, '0')}
                </span>
                <span
                  className={`px-2 py-0.5 rounded bg-black border font-mono text-xs font-bold tracking-widest ${stage.toneClass}`}
                >
                  {t(stage.codeKey)}
                </span>
              </div>
              <div className="text-xs font-semibold text-slate-200">{t(stage.titleKey)}</div>
              <p className="text-[11px] text-slate-400 leading-relaxed font-sans">
                {t(stage.descKey)}
              </p>
            </div>
          ))}
        </div>
      )}
    </section>
  );
};
