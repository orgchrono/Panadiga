import React, { useEffect, useRef, useState } from 'react';
import { Bug, FileCode, Binary, Terminal, RefreshCw, Cpu, type LucideIcon } from 'lucide-react';
import { Button } from '@/components/ui';
import { GDB_DEFAULT_TCP_PORT } from '@/constants/hardwareSpecs/physicalConstants';

export type ExternalToolName = 'gdb' | 'ghidra' | 'ghidra_sync' | 'ghidra_mcp' | 'hxd' | 'terminal';

export interface GdbExternalToolLaunchersProps {
  onLaunchTool?: (tool: ExternalToolName) => Promise<void>;
  isAndroidHost?: boolean;
}

const TOOL_LAUNCHERS: Array<{
  id: ExternalToolName;
  label: string;
  title: string;
  className: string;
  Icon: LucideIcon;
}> = [
  {
    id: 'gdb',
    label: 'GDB Multiarch',
    title: `Abre terminal interativo com gdb-multiarch conectado na porta ${GDB_DEFAULT_TCP_PORT}`,
    className: 'bg-cyan-500/10 hover:bg-cyan-500/20 text-cyan-300 border-cyan-500/30',
    Icon: Bug,
  },
  {
    id: 'ghidra',
    label: 'Scripts Ghidra',
    title:
      'Sincroniza scripts Jython SSOT em ~/ghidra_scripts e abre o Ghidra GUI diretamente em Panadiga_MN103S.gpr sem janelas CMD extras',
    className: 'bg-emerald-500/10 hover:bg-emerald-500/20 text-emerald-300 border-emerald-500/30',
    Icon: FileCode,
  },
  {
    id: 'ghidra_sync',
    label: 'Sync Jython (Headless)',
    title:
      'Executa analyzeHeadless.bat com mapa AM34, símbolos SSOT, fix MN103S e auditoria U99 gravando panadiga_jython_sync.log',
    className: 'bg-teal-500/10 hover:bg-teal-500/20 text-teal-300 border-teal-500/30',
    Icon: RefreshCw,
  },
  {
    id: 'ghidra_mcp',
    label: 'Ghidra MCP :8081',
    title: 'Inicia a ponte streamable-http ghidra-mcp.exe em http://127.0.0.1:8081 conectada ao Ghidra (:8080)',
    className: 'bg-indigo-500/10 hover:bg-indigo-500/20 text-indigo-300 border-indigo-500/30',
    Icon: Cpu,
  },
  {
    id: 'hxd',
    label: 'Editor Hexadecimal',
    title: 'Abre o editor hexadecimal configurado no sistema com os binários da bancada',
    className: 'bg-slate-800 hover:bg-slate-700 text-slate-200 border-slate-700',
    Icon: Binary,
  },
  {
    id: 'terminal',
    label: 'Terminal Host',
    title: 'Abre o shell nativo do sistema operacional no diretório do projeto',
    className: 'bg-slate-800 hover:bg-slate-700 text-slate-300 border-slate-700',
    Icon: Terminal,
  },
];

export const GdbExternalToolLaunchers: React.FC<GdbExternalToolLaunchersProps> = ({
  onLaunchTool,
  isAndroidHost = false,
}) => {
  const [activeTool, setActiveTool] = useState<string | null>(null);
  const resetTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const androidHost = Boolean(isAndroidHost);

  useEffect(() => {
    return () => {
      if (resetTimerRef.current !== null) {
        clearTimeout(resetTimerRef.current);
      }
    };
  }, []);

  if (androidHost) {
    return (
      <div className="bg-slate-950/60 p-3 rounded-xl border border-slate-800/80 text-xs font-mono text-slate-400">
        Ferramentas desktop externas (GDB CLI / Ghidra GUI) ficam disponíveis no host Desktop (Windows / Linux / macOS).
      </div>
    );
  }

  const handleLaunch = async (tool: ExternalToolName) => {
    if (!onLaunchTool) return;
    setActiveTool(tool);
    try {
      await onLaunchTool(tool);
    } finally {
      if (resetTimerRef.current !== null) {
        clearTimeout(resetTimerRef.current);
      }
      resetTimerRef.current = setTimeout(() => setActiveTool(null), 1200);
    }
  };

  return (
    <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 bg-slate-950/60 p-3 rounded-xl border border-slate-800/80">
      <span className="text-xs font-semibold text-slate-300">
        Disparo Rápido de Ferramentas de Engenharia Reversa:
      </span>
      <div className="flex items-center gap-2 flex-wrap">
        {TOOL_LAUNCHERS.length === 0 ? (
          <span className="text-xs text-slate-500">Nenhuma ferramenta externa configurada.</span>
        ) : (
          TOOL_LAUNCHERS.map(({ id, label, title, className, Icon }) => (
            <Button
              key={id}
              type="button"
              variant="outline"
              size="sm"
              onClick={() => handleLaunch(id)}
              disabled={activeTool === id}
              title={title}
              className={`font-mono font-semibold text-xs gap-1.5 h-8 ${className}`}
            >
              <Icon className="w-3.5 h-3.5" />
              <span>{label}</span>
            </Button>
          ))
        )}
      </div>
    </div>
  );
};
