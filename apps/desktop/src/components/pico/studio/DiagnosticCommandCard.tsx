import React, { useState } from 'react';
import type { PicoCommandSpec } from '@/models';
import { Badge, Button, Input } from '@/components/ui';
import { Loader2, Play, ShieldAlert } from 'lucide-react';

export interface DiagnosticCommandCardProps {
  cmd: PicoCommandSpec;
  argValue: string;
  onArgChange: (val: string) => void;
  isBusy: boolean;
  isCurrent: boolean;
  onRun: () => void;
}

export const DiagnosticCommandCard: React.FC<DiagnosticCommandCardProps> = ({
  cmd,
  argValue,
  onArgChange,
  isBusy,
  isCurrent,
  onRun,
}) => {
  const [confirmArmed, setConfirmArmed] = useState(false);

  const handleTrigger = () => {
    if (cmd.dangerous && !confirmArmed) {
      setConfirmArmed(true);
      setTimeout(() => setConfirmArmed(false), 4000);
      return;
    }
    setConfirmArmed(false);
    onRun();
  };

  return (
    <div className="rounded-xl bg-slate-950/90 border border-slate-800 p-3.5 flex flex-col justify-between gap-2.5">
      <div className="space-y-1">
        <div className="flex items-start justify-between gap-2">
          <div className="min-w-0">
            <h4 className="text-xs font-bold text-slate-100 leading-snug">{cmd.label}</h4>
            <code className="text-[10px] text-cyan-400 font-mono block">{cmd.syntax}</code>
          </div>
          {cmd.dangerous && (
            <Badge variant="destructive" className="shrink-0 text-[9px]">
              CRÍTICO
            </Badge>
          )}
        </div>
        <p className="text-[11px] text-slate-400 font-sans leading-relaxed">{cmd.description}</p>
      </div>

      <div className="space-y-2 pt-1">
        {cmd.arg_label && (
          <div className="space-y-1.5">
            {cmd.arg_options.length === 0 ? (
              <p className="text-[10px] text-slate-500 font-mono">Parâmetro livre para este comando</p>
            ) : (
              <div className="flex flex-wrap gap-1">
                {cmd.arg_options.map(opt => (
                  <button
                    key={opt.value}
                    type="button"
                    onClick={() => onArgChange(opt.value)}
                    className={`px-2 py-0.5 rounded text-[10px] font-mono border transition-colors ${
                      argValue === opt.value
                        ? 'bg-cyan-500/20 text-cyan-300 border-cyan-500/40 font-bold'
                        : 'bg-slate-900 text-slate-400 border-slate-800 hover:text-slate-200'
                    }`}
                  >
                    {opt.label}
                  </button>
                ))}
              </div>
            )}
            <Input
              value={argValue}
              onChange={e => onArgChange(e.target.value)}
              placeholder={cmd.arg_placeholder ?? cmd.arg_label}
              aria-label={`${cmd.id} argumento`}
              className="h-8 text-[11px]"
            />
          </div>
        )}

        <Button
          type="button"
          size="sm"
          variant={confirmArmed ? 'destructive' : cmd.dangerous ? 'outline' : 'default'}
          disabled={isBusy}
          onClick={handleTrigger}
          className="w-full h-8 text-xs font-bold gap-1.5"
        >
          {isCurrent ? (
            <>
              <Loader2 className="w-3.5 h-3.5 animate-spin" />
              <span>Executando {cmd.id}...</span>
            </>
          ) : confirmArmed ? (
            <>
              <ShieldAlert className="w-3.5 h-3.5" />
              <span>Confirmar {cmd.id}?</span>
            </>
          ) : (
            <>
              <Play className="w-3 h-3" />
              <span>Executar {cmd.id}</span>
            </>
          )}
        </Button>
      </div>
    </div>
  );
};
