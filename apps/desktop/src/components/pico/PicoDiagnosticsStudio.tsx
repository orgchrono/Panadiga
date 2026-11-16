import React, { useState } from 'react';
import type {
  HardwareSsotSpec,
  PicoCommandCategory,
  PicoCommandSpec,
  PicoDiagnosticResult,
} from '@/models';
import { isPicoCommandCategory } from '@/utils/typeGuards';
import {
  Alert,
  AlertDescription,
  AlertTitle,
  Badge,
  Button,
  Card,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from '@/components/ui';
import {
  Activity,
  Cpu,
  Layers,
  Loader2,
  Radio,
  ShieldAlert,
  Trash2,
  Wrench,
} from 'lucide-react';
import { DiagnosticCommandCard } from './studio/DiagnosticCommandCard';
import { MemoryMapPanel } from './studio/MemoryMapPanel';
import { DiagnosticInspector } from './studio/DiagnosticInspector';
import { useTranslation } from '@/i18n/useTranslation';

export interface PicoDiagnosticsStudioProps {
  ssot: HardwareSsotSpec;
  latestResult: PicoDiagnosticResult | null;
  history: PicoDiagnosticResult[];
  runningCommandId: string | null;
  error: string | null;
  selectedCategory: PicoCommandCategory;
  onSelectCategory: (cat: PicoCommandCategory) => void;
  onExecute: (commandId: string, arg?: string) => Promise<PicoDiagnosticResult | null>;
  onClearHistory: () => void;
}

const COMMAND_TABS: Array<{
  value: PicoCommandCategory;
  labelKey:
    | 'pico.studio.tabElectrical'
    | 'pico.studio.tabOcdFlash'
    | 'pico.studio.tabUartFmu'
    | 'pico.studio.tabProbe';
  Icon: React.ComponentType<{ className?: string }>;
}> = [
  { value: 'electrical', labelKey: 'pico.studio.tabElectrical', Icon: Activity },
  { value: 'ocd_flash', labelKey: 'pico.studio.tabOcdFlash', Icon: Cpu },
  { value: 'uart_fmu', labelKey: 'pico.studio.tabUartFmu', Icon: Radio },
  { value: 'probe', labelKey: 'pico.studio.tabProbe', Icon: Wrench },
];

export const PicoDiagnosticsStudio: React.FC<PicoDiagnosticsStudioProps> = ({
  ssot,
  latestResult,
  history,
  runningCommandId,
  error,
  selectedCategory,
  onSelectCategory,
  onExecute,
  onClearHistory,
}) => {
  const { t } = useTranslation();
  const [argMap, setArgMap] = useState<Record<string, string>>({});
  const [activeTab, setActiveTab] = useState<string>(selectedCategory);
  const [inspectedResult, setInspectedResult] = useState<PicoDiagnosticResult | null>(null);

  const activeResult = inspectedResult ?? latestResult;
  const isBusy = runningCommandId !== null;

  const getArgForCmd = (cmd: PicoCommandSpec): string =>
    argMap[cmd.id] !== undefined ? argMap[cmd.id] : cmd.default_arg ?? '';

  return (
    <Card className="p-4 sm:p-5 shadow-xl space-y-4 font-mono">
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-2 border-b border-slate-800 pb-3">
        <div>
          <div className="flex items-center gap-2 flex-wrap">
            <h3 data-testid="pico-studio-title" className="text-sm font-bold text-slate-100">
              {t('pico.studio.title')}
            </h3>
            <Badge variant="default">
              {t('pico.studio.badgeCount', { count: ssot.pico_commands.length })}
            </Badge>
          </div>
          <p className="text-xs text-slate-400 font-sans mt-0.5">{t('pico.studio.subtitle')}</p>
        </div>

        {history.length > 0 && (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => {
              setInspectedResult(null);
              onClearHistory();
            }}
            className="text-xs text-slate-400 hover:text-rose-400 gap-1.5"
          >
            <Trash2 className="w-3.5 h-3.5" />
            <span>{t('pico.studio.clearHistory')}</span>
          </Button>
        )}
      </div>

      <Tabs
        value={activeTab}
        onValueChange={val => {
          setActiveTab(val);
          if (isPicoCommandCategory(val)) {
            onSelectCategory(val);
          }
        }}
      >
        <TabsList className="w-full h-auto flex-wrap justify-start gap-1">
          {COMMAND_TABS.length === 0 ? (
            <span className="text-xs text-slate-500 px-2 py-1">{t('pico.studio.noCategory')}</span>
          ) : (
            COMMAND_TABS.map(({ value, labelKey, Icon }) => (
              <TabsTrigger
                key={value}
                value={value}
                data-testid={`pico-tab-${value}`}
                className="gap-1.5"
              >
                <Icon className="w-3.5 h-3.5" />
                <span>{t(labelKey)}</span>
              </TabsTrigger>
            ))
          )}
          <TabsTrigger value="memory" data-testid="pico-tab-memory" className="gap-1.5">
            <Layers className="w-3.5 h-3.5" />
            <span>{t('pico.studio.memoryMapTab')}</span>
          </TabsTrigger>
        </TabsList>

        {COMMAND_TABS.map(({ value }) => {
          const cmds = ssot.pico_commands.filter(c => c.category === value);
          return (
            <TabsContent key={value} value={value}>
              <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-3">
                {cmds.map(cmd => (
                  <DiagnosticCommandCard
                    key={cmd.id}
                    cmd={cmd}
                    argValue={getArgForCmd(cmd)}
                    onArgChange={val => setArgMap(prev => ({ ...prev, [cmd.id]: val }))}
                    isBusy={isBusy}
                    isCurrent={runningCommandId === cmd.id}
                    onRun={() => {
                      setInspectedResult(null);
                      void onExecute(cmd.id, getArgForCmd(cmd));
                    }}
                  />
                ))}
              </div>
            </TabsContent>
          );
        })}

        <TabsContent value="memory">
          <MemoryMapPanel ssot={ssot} />
        </TabsContent>
      </Tabs>

      {error && (
        <Alert variant="destructive">
          <ShieldAlert className="w-4 h-4 text-rose-400 shrink-0" />
          <div>
            <AlertTitle>{t('pico.studio.errorTitle')}</AlertTitle>
            <AlertDescription>{error}</AlertDescription>
          </div>
        </Alert>
      )}

      {isBusy && (
        <Alert variant="info">
          <Loader2 className="w-4 h-4 text-cyan-400 animate-spin shrink-0" />
          <div>
            <AlertTitle>
              {t('pico.studio.runningTitle', { id: runningCommandId ?? '' })}
            </AlertTitle>
            <AlertDescription>{t('pico.studio.runningDesc')}</AlertDescription>
          </div>
        </Alert>
      )}

      <DiagnosticInspector
        activeResult={activeResult}
        history={history}
        onInspect={setInspectedResult}
      />
    </Card>
  );
};
