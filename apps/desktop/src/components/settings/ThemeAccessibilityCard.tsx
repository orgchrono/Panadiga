import React from 'react';
import { Contrast, Eye, Moon, Sparkles, Sun, Tv, Type } from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle, Switch } from '@/components/ui';
import type { TranslationKey } from '@/i18n/translations';
import { useTranslation } from '@/i18n/useTranslation';
import { cn } from '@/lib/utils';
import type {
  AccessibilitySettings,
  FontScaleMode,
  ToastMessage,
  WorkbenchTheme,
} from '@/models';

const THEME_OPTIONS: ReadonlyArray<{
  readonly id: WorkbenchTheme;
  readonly labelKey: TranslationKey;
  readonly descKey: TranslationKey;
  readonly swatch: string;
  readonly Icon: React.ComponentType<{ className?: string }>;
}> = [
  {
    id: 'dark-lab',
    labelKey: 'settings.themes.darkLab.label',
    descKey: 'settings.themes.darkLab.desc',
    swatch: 'bg-[#090d16] border-cyan-500/60',
    Icon: Moon,
  },
  {
    id: 'high-contrast',
    labelKey: 'settings.themes.highContrast.label',
    descKey: 'settings.themes.highContrast.desc',
    swatch: 'bg-black border-white',
    Icon: Contrast,
  },
  {
    id: 'daylight-bench',
    labelKey: 'settings.themes.daylightBench.label',
    descKey: 'settings.themes.daylightBench.desc',
    swatch: 'bg-slate-100 border-slate-500',
    Icon: Sun,
  },
  {
    id: 'amber-crt',
    labelKey: 'settings.themes.amberCrt.label',
    descKey: 'settings.themes.amberCrt.desc',
    swatch: 'bg-[#171108] border-amber-400',
    Icon: Tv,
  },
];

export interface ThemeAccessibilityCardProps {
  readonly accessibility: AccessibilitySettings;
  readonly onSetTheme: (theme: WorkbenchTheme) => void;
  readonly onSetColorblindSafe: (enabled: boolean) => void;
  readonly onSetReducedMotion: (enabled: boolean) => void;
  readonly onSetFontScale: (scale: FontScaleMode) => void;
  readonly onNotify: (toast: Omit<ToastMessage, 'id'>) => void;
}

export const ThemeAccessibilityCard: React.FC<ThemeAccessibilityCardProps> = ({
  accessibility,
  onSetTheme,
  onSetColorblindSafe,
  onSetReducedMotion,
  onSetFontScale,
  onNotify,
}) => {
  const { t } = useTranslation();

  const handleThemeSelect = (nextTheme: WorkbenchTheme) => {
    onSetTheme(nextTheme);
    const opt = THEME_OPTIONS.find((item) => item.id === nextTheme);
    onNotify({
      type: 'info',
      title: t('settings.themes.toastThemeTitle'),
      message: opt ? t(opt.labelKey) : nextTheme,
    });
  };

  const handleFontScale = (scale: FontScaleMode) => {
    onSetFontScale(scale);
    onNotify({
      type: 'info',
      title: t('settings.themes.toastScaleTitle'),
      message:
        scale === 'large'
          ? t('settings.themes.scaleLargeEnabled')
          : t('settings.themes.scaleDefaultRestored'),
    });
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Contrast className="w-4 h-4 text-cyan-400" />
          <span>{t('settings.themes.cardTitle')}</span>
        </CardTitle>
        <CardDescription>{t('settings.themes.cardDesc')}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-5">
        {THEME_OPTIONS.length === 0 ? (
          <span className="text-xs text-slate-500">{t('settings.themes.emptyThemes')}</span>
        ) : (
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            {THEME_OPTIONS.map((opt) => {
              const Icon = opt.Icon;
              const active = accessibility.theme === opt.id;
              return (
                <button
                  key={opt.id}
                  type="button"
                  aria-pressed={active}
                  onClick={() => handleThemeSelect(opt.id)}
                  className={cn(
                    'flex items-start gap-3 p-3.5 rounded-xl border text-left transition-colors',
                    active
                      ? 'bg-cyan-500/10 border-cyan-500/60 text-slate-100'
                      : 'bg-slate-950/80 border-slate-800 text-slate-300 hover:border-slate-700',
                  )}
                >
                  <div
                    className={cn(
                      'w-9 h-9 rounded-lg border-2 flex items-center justify-center shrink-0',
                      opt.swatch,
                    )}
                  >
                    <Icon className="w-4 h-4 text-cyan-400" />
                  </div>
                  <div className="min-w-0">
                    <div className="text-xs font-bold">{t(opt.labelKey)}</div>
                    <p className="text-[11px] text-slate-400 font-sans mt-0.5 leading-snug">
                      {t(opt.descKey)}
                    </p>
                  </div>
                </button>
              );
            })}
          </div>
        )}

        <div className="grid grid-cols-1 md:grid-cols-3 gap-3 pt-2 border-t border-slate-800">
          <div className="flex items-center justify-between gap-3 p-3 rounded-xl bg-slate-950 border border-slate-800">
            <div className="min-w-0">
              <div className="text-xs font-bold text-slate-200 flex items-center gap-1.5">
                <Eye className="w-3.5 h-3.5 text-cyan-400 shrink-0" />
                <span>{t('settings.themes.colorblindTitle')}</span>
              </div>
              <p className="text-[10px] text-slate-400 font-sans mt-0.5">
                {t('settings.themes.colorblindDesc')}
              </p>
            </div>
            <Switch
              aria-label={t('settings.themes.colorblindAria')}
              checked={accessibility.colorblindSafe}
              onCheckedChange={onSetColorblindSafe}
            />
          </div>

          <div className="flex items-center justify-between gap-3 p-3 rounded-xl bg-slate-950 border border-slate-800">
            <div className="min-w-0">
              <div className="text-xs font-bold text-slate-200 flex items-center gap-1.5">
                <Sparkles className="w-3.5 h-3.5 text-cyan-400 shrink-0" />
                <span>{t('settings.themes.reducedMotionTitle')}</span>
              </div>
              <p className="text-[10px] text-slate-400 font-sans mt-0.5">
                {t('settings.themes.reducedMotionDesc')}
              </p>
            </div>
            <Switch
              aria-label={t('settings.themes.reducedMotionAria')}
              checked={accessibility.reducedMotion}
              onCheckedChange={onSetReducedMotion}
            />
          </div>

          <div className="flex items-center justify-between gap-3 p-3 rounded-xl bg-slate-950 border border-slate-800">
            <div className="min-w-0">
              <div className="text-xs font-bold text-slate-200 flex items-center gap-1.5">
                <Type className="w-3.5 h-3.5 text-cyan-400 shrink-0" />
                <span>{t('settings.themes.fontScaleTitle')}</span>
              </div>
              <p className="text-[10px] text-slate-400 font-sans mt-0.5">
                {t('settings.themes.fontScaleDesc')}
              </p>
            </div>
            <Switch
              aria-label={t('settings.themes.fontScaleAria')}
              checked={accessibility.fontScale === 'large'}
              onCheckedChange={(checked) => handleFontScale(checked ? 'large' : 'normal')}
            />
          </div>
        </div>
      </CardContent>
    </Card>
  );
};
