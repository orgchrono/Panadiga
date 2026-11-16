import React from 'react';
import { Globe, Layers, RotateCcw, SlidersHorizontal } from 'lucide-react';
import { useWorkbenchMode } from '@/context/WorkbenchModeContext';
import { useToast } from '@/context/ToastContext';
import { useTranslation } from '@/i18n/useTranslation';
import { Button, Card, CardContent, CardDescription, CardHeader, CardTitle, Switch } from '@/components/ui';

export const WorkflowRecoveryCards: React.FC = () => {
  const { mode, setMode, hudOpen, setHudOpen, resetWorkbenchPreferences, resetPipeline, pipeline } =
    useWorkbenchMode();
  const { locale, setLocale } = useTranslation();
  const { showToast } = useToast();

  const handleResetAll = () => {
    resetWorkbenchPreferences();
    resetPipeline();
    showToast({
      type: 'success',
      title: 'Layout e Preferências Restaurados',
      message: 'Proporções de divisores, tema e modo foram redefinidos com segurança.',
    });
  };

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Globe className="w-4 h-4 text-emerald-400" />
            <span>Idioma & Fluxo de Trabalho</span>
          </CardTitle>
          <CardDescription>
            Alterne o idioma do dicionário i18n e o nível de complexidade das ferramentas.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-1.5">
            <span className="text-[11px] text-slate-400 uppercase font-bold block">Idioma (i18n)</span>
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
              <span>Padrão de Complexidade</span>
            </span>
            <div className="flex items-center gap-2">
              <Button
                type="button"
                size="sm"
                variant={mode === 'basic' ? 'emerald' : 'outline'}
                onClick={() => setMode('basic')}
              >
                Básico (Guiado)
              </Button>
              <Button
                type="button"
                size="sm"
                variant={mode === 'advanced' ? 'default' : 'outline'}
                onClick={() => setMode('advanced')}
              >
                Avançado (Engenharia)
              </Button>
            </div>
          </div>

          <div className="flex items-center justify-between pt-2 border-t border-slate-800">
            <span className="text-xs text-slate-300 flex items-center gap-1.5">
              <SlidersHorizontal className="w-3.5 h-3.5 text-cyan-400" />
              <span>HUD Flutuante Arrastável P7501</span>
            </span>
            <Switch
              aria-label="Alternar HUD Flutuante P7501"
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
            <span>Recuperação & Limpeza de Estado</span>
          </CardTitle>
          <CardDescription>
            Saída segura caso algum painel redimensionável ou estado de sessão fique desalinhado.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="p-3 rounded-xl bg-slate-950 border border-slate-800 text-xs space-y-1">
            <div className="text-slate-400">Status atual do Pipeline:</div>
            <div className="text-slate-200">
              Pico: {pipeline.picoReady ? 'OK' : 'Pendente'} • NVRAM:{' '}
              {pipeline.nvramCleaned ? 'Limpa' : 'Pendente'} • HDD:{' '}
              {pipeline.hddInspected ? 'Inspecionado' : 'Pendente'} • Unbrick:{' '}
              {pipeline.unbrickReady ? 'Pronto' : 'Pendente'}
            </div>
          </div>

          <div className="flex flex-wrap gap-2">
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => {
                resetPipeline();
                showToast({ type: 'info', title: 'Pipeline Reiniciado' });
              }}
              className="gap-1.5"
            >
              <RotateCcw className="w-3.5 h-3.5" />
              <span>Zerar Progresso do Pipeline</span>
            </Button>

            <Button
              type="button"
              variant="destructive"
              size="sm"
              onClick={handleResetAll}
              className="gap-1.5"
            >
              <RotateCcw className="w-3.5 h-3.5" />
              <span>Restaurar Layout & Padrões</span>
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
};
