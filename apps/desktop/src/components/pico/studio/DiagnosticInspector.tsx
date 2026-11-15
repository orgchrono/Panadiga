import React from 'react';
import type { PicoDiagnosticResult } from '@/models';
import { Badge, EmptyState, ScrollArea } from '@/components/ui';
import { History } from 'lucide-react';
import { DiagnosticResultTables } from './DiagnosticResultTables';

export interface DiagnosticInspectorProps {
  activeResult: PicoDiagnosticResult | null;
  history: PicoDiagnosticResult[];
  onInspect: (item: PicoDiagnosticResult) => void;
}

export const DiagnosticInspector: React.FC<DiagnosticInspectorProps> = ({
  activeResult,
  history,
  onInspect,
}) => (
  <div className="rounded-xl bg-slate-950 border border-slate-800 p-4 space-y-3">
    <div className="flex flex-wrap items-center justify-between gap-2 border-b border-slate-800 pb-2.5">
      <div className="flex items-center gap-2">
        <History className="w-4 h-4 text-cyan-400" />
        <span className="text-xs font-bold text-slate-200">
          Inspetor Estruturado de Diagnóstico
        </span>
      </div>

      {history.length > 0 && (
        <div className="flex items-center gap-1.5 flex-wrap">
          {history.slice(0, 6).map((item, idx) => (
            <button
              key={`${item.raw_command}-${idx}`}
              type="button"
              onClick={() => onInspect(item)}
              className={`px-2 py-0.5 rounded text-[10px] border transition-colors ${
                activeResult === item
                  ? 'bg-cyan-500/20 text-cyan-300 border-cyan-500/40 font-bold'
                  : 'bg-slate-900 text-slate-400 border-slate-800 hover:text-slate-200'
              }`}
            >
              {item.command_id} ({item.elapsed_ms}ms)
            </button>
          ))}
        </div>
      )}
    </div>

    {!activeResult ? (
      <EmptyState
        title="Nenhum diagnóstico executado nesta sessão"
        description="Selecione uma aba acima (ex: Elétrica P7501 -> PINCHECK) para auditar os 6 pinos, capturar boot UART ou executar probes OCD."
        className="min-h-[160px] p-4 border-slate-800/70"
      />
    ) : (
      <div className="space-y-3">
        <div className="flex flex-wrap items-center gap-2">
          <Badge variant={activeResult.success ? 'success' : 'destructive'}>
            {activeResult.success ? 'OK' : 'FALHA / ALERTA'}
          </Badge>
          <code className="text-xs text-cyan-300 font-bold">{activeResult.raw_command}</code>
          <Badge variant="secondary">{activeResult.elapsed_ms} ms</Badge>
          {activeResult.vdd_volts !== null && activeResult.vdd_volts !== undefined && (
            <Badge variant="default">VDD {activeResult.vdd_volts.toFixed(2)}V</Badge>
          )}
          {activeResult.detected_flash_mb && (
            <Badge variant="success">Flash {activeResult.detected_flash_mb} MB</Badge>
          )}
          {activeResult.xreset_rise_us && (
            <Badge variant="secondary">XRESET Rise: {activeResult.xreset_rise_us}</Badge>
          )}
          {activeResult.captured_bytes !== null && activeResult.captured_bytes !== undefined && (
            <Badge variant="secondary">{activeResult.captured_bytes} bytes UART/FMU</Badge>
          )}
        </div>

        <DiagnosticResultTables result={activeResult} />

        <ScrollArea className="max-h-44 rounded-lg bg-slate-900/90 border border-slate-800 p-3">
          <pre className="text-[11px] text-slate-300 whitespace-pre-wrap break-words font-mono">
            {activeResult.raw_output}
          </pre>
        </ScrollArea>
      </div>
    )}
  </div>
);
