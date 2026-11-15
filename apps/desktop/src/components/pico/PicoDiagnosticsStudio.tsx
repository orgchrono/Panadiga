import React, { useState } from 'react';
import type {
  HardwareSsotSpec,
  PicoCommandCategory,
  PicoCommandSpec,
  PicoDiagnosticResult,
} from '@/models';
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
  label: string;
  Icon: React.ComponentType<{ className?: string }>;
}> = [
  { value: 'electrical', label: 'Elétrica P7501', Icon: Activity },
  { value: 'ocd_flash', label: 'OCD & NOR Flash', Icon: Cpu },
  { value: 'uart_fmu', label: 'UART & FMU Bootrom', Icon: Radio },
  { value: 'probe', label: 'Controle Sonda', Icon: Wrench },
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
            <h3 className="text-sm font-bold text-slate-100">
              Estúdio Granular P7501 Async UART, Elétrica &amp; Probes NOR Flash
            </h3>
            <Badge variant="default">{ssot.pico_commands.length} COMANDOS FIRMWARE</Badge>
          </div>
          <p className="text-xs text-slate-400 font-sans mt-0.5">
            Espelha 100% do CLI embarcado (`p7501_cli.c` — P7501 UART 115200 8N1) via USB CDC, Android OTG e BLE NUS.
          </p>
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
            <span>Limpar Histórico</span>
          </Button>
        )}
      </div>

      <Tabs
        value={activeTab}
        onValueChange={val => {
          setActiveTab(val);
          if (val !== 'memory') {
            onSelectCategory(val as PicoCommandCategory);
          }
        }}
      >
        <TabsList className="w-full h-auto flex-wrap justify-start gap-1">
          {COMMAND_TABS.map(({ value, label, Icon }) => (
            <TabsTrigger key={value} value={value} className="gap-1.5">
              <Icon className="w-3.5 h-3.5" />
              <span>{label}</span>
            </TabsTrigger>
          ))}
          <TabsTrigger value="memory" className="gap-1.5">
            <Layers className="w-3.5 h-3.5" />
            <span>Mapa Memória AM34</span>
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
            <AlertTitle>Erro na Execução Diagnóstica</AlertTitle>
            <AlertDescription>{error}</AlertDescription>
          </div>
        </Alert>
      )}

      {isBusy && (
        <Alert variant="info">
          <Loader2 className="w-4 h-4 text-cyan-400 animate-spin shrink-0" />
          <div>
            <AlertTitle>Sonda Pico Executando {runningCommandId}</AlertTitle>
            <AlertDescription>
              Aguardando janela de captura / resposta determinística no barramento...
            </AlertDescription>
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
