import React from 'react';
import { Globe, Layers, RotateCcw, SlidersHorizontal } from 'lucide-react';
import { useWorkbenchMode } from '@/context/WorkbenchModeContext';
import { useToast } from '@/context/ToastContext';
import { useTranslation } from '@/i18n/useTranslation';
import { Button, Card, CardContent, CardDescription, CardHeader, CardTitle, Switch } from '@/components/ui';

export const WorkflowRecoveryCards: React.FC = () => {
  const { mode, setMode, hudOpen, setHudOpen, resetWorkbenchPreferences, resetPipeline, pipeline } =
    useWorkbenchMode();
  const { t, locale, setLocale } = useTranslation();
  const { showToast } = useToast();

  const handleResetAll = () => {
    resetWorkbenchPreferences();
    resetPipeline();
    showToast({
      type: 'success',
      title: t('settings.workflow.resetAllToastTitle'),
      message: t('settings.workflow.resetAllToastMsg'),
    });
  };

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Globe className="w-4 h-4 text-emerald-400" />
            <span>{t('settings.workflow.cardTitle')}</span>
          </CardTitle>
          <CardDescription>
            {t('settings.workflow.cardDesc')}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-1.5">
            <span className="text-[11px] text-slate-400 uppercase font-bold block">
              {t('settings.workflow.languageLabel')}
            </span>
            <div className="flex items-center gap-2">
              <Button
                type="button"
                size="sm"
                variant={locale === 'pt-BR' ? 'default' : 'outline'}
                onClick={() => setLocale('pt-BR')}
              >
                Português (pt-BR)
              </Button>
              <Button
                type="button"
                size="sm"
                variant={locale === 'en-US' ? 'default' : 'outline'}
                onClick={() => setLocale('en-US')}
              >
                English (en-US)
              </Button>
            </div>
          </div>

          <div className="space-y-1.5 pt-2 border-t border-slate-800">
            <span className="text-[11px] text-slate-400 uppercase font-bold flex items-center gap-1.5">
              <Layers className="w-3.5 h-3.5 text-cyan-400" />
              <span>{t('settings.workflow.complexityLabel')}</span>
            </span>
            <div className="flex items-center gap-2">
              <Button
                type="button"
                size="sm"
                variant={mode === 'basic' ? 'emerald' : 'outline'}
                onClick={() => setMode('basic')}
              >
                {t('settings.workflow.basicMode')}
              </Button>
              <Button
                type="button"
                size="sm"
                variant={mode === 'advanced' ? 'default' : 'outline'}
                onClick={() => setMode('advanced')}
              >
                {t('settings.workflow.advancedMode')}
              </Button>
            </div>
          </div>

          <div className="flex items-center justify-between pt-2 border-t border-slate-800">
            <span className="text-xs text-slate-300 flex items-center gap-1.5">
              <SlidersHorizontal className="w-3.5 h-3.5 text-cyan-400" />
              <span>{t('settings.workflow.hudLabel')}</span>
            </span>
            <Switch
              aria-label={t('settings.workflow.hudAria')}
              checked={hudOpen}
              onCheckedChange={setHudOpen}
            />
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <RotateCcw className="w-4 h-4 text-amber-400" />
            <span>{t('settings.workflow.recoveryTitle')}</span>
          </CardTitle>
          <CardDescription>
            {t('settings.workflow.recoveryDesc')}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="p-3 rounded-xl bg-slate-950 border border-slate-800 text-xs space-y-1">
            <div className="text-slate-400">{t('settings.workflow.pipelineStatusTitle')}</div>
            <div className="text-slate-200">
              Pico: {pipeline.picoReady ? t('settings.workflow.ok') : t('settings.workflow.pending')} • NVRAM:{' '}
              {pipeline.nvramCleaned ? t('settings.workflow.clean') : t('settings.workflow.pending')} • HDD:{' '}
              {pipeline.hddInspected ? t('settings.workflow.inspected') : t('settings.workflow.pending')} • Unbrick:{' '}
              {pipeline.unbrickReady ? t('settings.workflow.ready') : t('settings.workflow.pending')}
            </div>
          </div>

          <div className="flex flex-wrap gap-2">
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => {
                resetPipeline();
                showToast({ type: 'info', title: t('settings.workflow.pipelineResetToast') });
              }}
              className="gap-1.5"
            >
              <RotateCcw className="w-3.5 h-3.5" />
              <span>{t('settings.workflow.resetPipelineBtn')}</span>
            </Button>

            <Button
              type="button"
              variant="destructive"
              size="sm"
              onClick={handleResetAll}
              className="gap-1.5"
            >
              <RotateCcw className="w-3.5 h-3.5" />
              <span>{t('settings.workflow.resetAllBtn')}</span>
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
};
