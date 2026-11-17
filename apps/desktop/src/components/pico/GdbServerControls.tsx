import React, { useState } from 'react';
import type { GdbBridgeStatus } from '@/models';
import {
  CLIPBOARD_FEEDBACK_MS,
  GDB_DEFAULT_LISTEN_ADDR,
  GDB_DEFAULT_TCP_PORT,
} from '@/constants';
import { StatusBadge } from '@/components/StatusBadge';
import { Button } from '@/components/ui';
import { useTransientFlag } from '@/hooks/useManagedTimeouts';
import { useTranslation } from '@/i18n/useTranslation';
import { Server, Play, RefreshCw, Copy, Check, AlertCircle } from 'lucide-react';

export interface GdbServerControlsProps {
  gdbStatus: GdbBridgeStatus | null;
  isStarting: boolean;
  error?: string | null;
  onStart: (listenAddr?: string) => Promise<void>;
  onStop: () => Promise<void>;
  onCopyCommand?: (command: string) => void;
}

export const GdbServerControls: React.FC<GdbServerControlsProps> = ({
  gdbStatus,
  isStarting,
  error,
  onStart,
  onStop,
  onCopyCommand,
}) => {
  const { t } = useTranslation();
  const { active: copied, trigger: triggerCopied } = useTransientFlag(CLIPBOARD_FEEDBACK_MS);
  const [listenAddrInput, setListenAddrInput] = useState<string>(
    gdbStatus?.listen_addr || GDB_DEFAULT_LISTEN_ADDR,
  );
  const listenAddr = gdbStatus?.running
    ? gdbStatus.listen_addr || listenAddrInput
    : listenAddrInput;
  const activePortLabel = listenAddr.split(':').at(-1) || String(GDB_DEFAULT_TCP_PORT);
  const connectionCommand = `target remote ${listenAddr}`;

  const handleCopy = () => {
    onCopyCommand?.(connectionCommand);
    triggerCopied();
  };

  return (
    <div className="space-y-3 border-b border-slate-800 pb-4">
      <div className="flex flex-col md:flex-row items-start md:items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <div className="p-2.5 rounded-xl bg-purple-500/10 text-purple-400 border border-purple-500/30">
            <Server className="w-5 h-5" />
          </div>
          <div>
            <div className="flex items-center gap-2 flex-wrap">
              <h3 className="text-sm font-bold text-slate-100">
                {t('pico.gdbPortTitle', { port: activePortLabel })}
              </h3>
              {gdbStatus?.running ? (
                (gdbStatus.connected_clients ?? 0) > 0 ? (
                  <StatusBadge
                    variant="success"
                    label={t('pico.gdbClientActive', { count: gdbStatus.connected_clients })}
                  />
                ) : (
                  <StatusBadge
                    variant="warning"
                    label={t('pico.gdbListeningPort', { port: activePortLabel })}
                  />
                )
              ) : (
                <StatusBadge variant="neutral" label={t('pico.gdbOffline')} />
              )}
            </div>
            <p className="text-xs text-slate-400 mt-0.5 font-sans">
              {t('pico.gdbSocketHelp')}{' '}
              <code className="text-purple-300 font-mono">{listenAddr}</code>
              {gdbStatus?.port_name
                ? t('pico.gdbTargetPrefix', { port: gdbStatus.port_name })
                : t('pico.gdbLocalStubHint')}
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2.5 shrink-0 flex-wrap">
          <label className="flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-slate-950 border border-slate-800 text-[11px] text-slate-300">
            <span className="text-slate-500">{t('pico.gdbBindLabel')}</span>
            <input
              type="text"
              aria-label={t('pico.gdbBindAria')}
              value={listenAddr}
              onChange={e => setListenAddrInput(e.target.value)}
              disabled={Boolean(gdbStatus?.running)}
              className="w-32 bg-transparent font-mono text-[11px] text-purple-200 focus:outline-none disabled:opacity-60"
            />
          </label>

          <div className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-slate-950 border border-slate-800 text-[11px] text-slate-300">
            <code>(gdb) {connectionCommand}</code>
            <button
              type="button"
              onClick={handleCopy}
              className="text-slate-400 hover:text-cyan-400 p-1 transition-colors"
              title={t('pico.gdbCopyTitle')}
            >
              {copied ? (
                <Check className="w-3.5 h-3.5 text-emerald-400" />
              ) : (
                <Copy className="w-3.5 h-3.5" />
              )}
            </button>
          </div>

          {gdbStatus?.running ? (
            <Button
              type="button"
              variant="destructive"
              size="sm"
              onClick={() => onStop()}
              className="font-bold text-xs"
            >
              {t('pico.gdbStop')}
            </Button>
          ) : (
            <Button
              type="button"
              variant="default"
              size="sm"
              onClick={() => onStart(listenAddr.trim() || GDB_DEFAULT_LISTEN_ADDR)}
              disabled={isStarting}
              className="bg-purple-600 hover:bg-purple-500 text-white font-bold text-xs gap-2 shadow-lg shadow-purple-950/50"
            >
              {isStarting ? (
                <>
                  <RefreshCw className="w-3.5 h-3.5 animate-spin" />
                  <span>{t('pico.gdbStarting')}</span>
                </>
              ) : (
                <>
                  <Play className="w-3.5 h-3.5 fill-current" />
                  <span>{t('pico.gdbStart')}</span>
                </>
              )}
            </Button>
          )}
        </div>
      </div>

      {error && (
        <div
          role="alert"
          className="flex items-center gap-2 rounded-lg bg-rose-500/10 border border-rose-500/40 px-3 py-2 text-xs text-rose-300 font-mono"
        >
          <AlertCircle className="w-4 h-4 shrink-0" />
          <span>{error}</span>
        </div>
      )}
    </div>
  );
};
