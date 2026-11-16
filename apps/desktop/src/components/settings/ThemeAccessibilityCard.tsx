import React from 'react';
import { Contrast, Eye, Moon, Sparkles, Sun, Tv, Type } from 'lucide-react';
import {
  useWorkbenchMode,
  type FontScaleMode,
  type WorkbenchTheme,
} from '@/context/WorkbenchModeContext';
import { useToast } from '@/context/ToastContext';
import { Card, CardContent, CardDescription, CardHeader, CardTitle, Switch } from '@/components/ui';
import { cn } from '@/lib/utils';

const THEME_OPTIONS: ReadonlyArray<{
  readonly id: WorkbenchTheme;
  readonly label: string;
  readonly description: string;
  readonly swatch: string;
  readonly Icon: React.ComponentType<{ className?: string }>;
}> = [
  {
    id: 'dark-lab',
    label: 'Dark Lab (Padrão)',
    description: 'Tema escuro de bancada com contraste balanceado para engenharia.',
    swatch: 'bg-[#090d16] border-cyan-500/60',
    Icon: Moon,
  },
  {
    id: 'high-contrast',
    label: 'Alto Contraste (WCAG AAA)',
    description: 'Fundo preto puro, bordas nítidas e leitura máxima de texto/hex.',
    swatch: 'bg-black border-white',
    Icon: Contrast,
  },
  {
    id: 'daylight-bench',
    label: 'Daylight Bench (Claro)',
    description: 'Modo claro otimizado para bancadas sob iluminação intensa.',
    swatch: 'bg-slate-100 border-slate-500',
    Icon: Sun,
  },
  {
    id: 'amber-crt',
    label: 'Amber CRT (Baixa Luz Azul)',
    description: 'Fósforo âmbar estilo osciloscópio para sessões prolongadas.',
    swatch: 'bg-[#171108] border-amber-400',
    Icon: Tv,
  },
];

export const ThemeAccessibilityCard: React.FC = () => {
  const { accessibility, setTheme, setColorblindSafe, setReducedMotion, setFontScale } =
    useWorkbenchMode();
  const { showToast } = useToast();

  const handleThemeSelect = (nextTheme: WorkbenchTheme) => {
    setTheme(nextTheme);
    const label = THEME_OPTIONS.find(item => item.id === nextTheme)?.label ?? nextTheme;
    showToast({ type: 'info', title: 'Tema Atualizado', message: label });
  };

  const handleFontScale = (scale: FontScaleMode) => {
    setFontScale(scale);
    showToast({
      type: 'info',
      title: 'Escala Tipográfica',
      message: scale === 'large' ? 'Fonte Ampliada ativada' : 'Fonte Padrão restaurada',
    });
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Contrast className="w-4 h-4 text-cyan-400" />
          <span>Tema & Acessibilidade Visual</span>
        </CardTitle>
        <CardDescription>
          Escolha o perfil cromático que melhor se adapta à iluminação e conforto visual do laboratório.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-5">
        {THEME_OPTIONS.length === 0 ? (
          <span className="text-xs text-slate-500">Nenhum tema disponível.</span>
        ) : (
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            {THEME_OPTIONS.map(opt => {
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
                      : 'bg-slate-950/80 border-slate-800 text-slate-300 hover:border-slate-700'
                  )}
                >
                  <div
                    className={cn(
                      'w-9 h-9 rounded-lg border-2 flex items-center justify-center shrink-0',
                      opt.swatch
                    )}
                  >
                    <Icon className="w-4 h-4 text-cyan-400" />
                  </div>
                  <div className="min-w-0">
                    <div className="text-xs font-bold">{opt.label}</div>
                    <p className="text-[11px] text-slate-400 font-sans mt-0.5 leading-snug">
                      {opt.description}
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
                <span>Modo Daltônico</span>
              </div>
              <p className="text-[10px] text-slate-400 font-sans mt-0.5">
                Substitui verde/vermelho por azul/laranja de alta distinção.
              </p>
            </div>
            <Switch
              aria-label="Ativar modo daltônico"
              checked={accessibility.colorblindSafe}
              onCheckedChange={checked => setColorblindSafe(checked)}
            />
          </div>

          <div className="flex items-center justify-between gap-3 p-3 rounded-xl bg-slate-950 border border-slate-800">
            <div className="min-w-0">
              <div className="text-xs font-bold text-slate-200 flex items-center gap-1.5">
                <Sparkles className="w-3.5 h-3.5 text-cyan-400 shrink-0" />
                <span>Reduzir Movimento</span>
              </div>
              <p className="text-[10px] text-slate-400 font-sans mt-0.5">
                Minimiza transições e animações do Framer Motion/CSS.
              </p>
            </div>
            <Switch
              aria-label="Ativar redução de movimento"
              checked={accessibility.reducedMotion}
              onCheckedChange={checked => setReducedMotion(checked)}
            />
          </div>

          <div className="flex items-center justify-between gap-3 p-3 rounded-xl bg-slate-950 border border-slate-800">
            <div className="min-w-0">
              <div className="text-xs font-bold text-slate-200 flex items-center gap-1.5">
                <Type className="w-3.5 h-3.5 text-cyan-400 shrink-0" />
                <span>Texto Ampliado</span>
              </div>
              <p className="text-[10px] text-slate-400 font-sans mt-0.5">
                Escala legível para endereços hex e tabelas técnicas.
              </p>
            </div>
            <Switch
              aria-label="Alternar escala tipográfica ampliada"
              checked={accessibility.fontScale === 'large'}
              onCheckedChange={checked => handleFontScale(checked ? 'large' : 'normal')}
            />
          </div>
        </div>
      </CardContent>
    </Card>
  );
};
