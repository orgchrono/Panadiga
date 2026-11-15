import React from 'react';
import { Button } from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import type { BridgeStreamLine } from '@/viewmodels/useSc0Bridge';
import type { Sc0BridgeInputMode } from '@/models';
import { Send, Terminal, Trash2 } from 'lucide-react';

export interface Sc0BridgeLogConsoleProps {
  readonly logs: readonly BridgeStreamLine[];
  readonly lastEvent: string;
  readonly bridgeRunning: boolean;
  readonly inputMode: Sc0BridgeInputMode;
  readonly onChangeInputMode: (mode: Sc0BridgeInputMode) => void;
  readonly txInput: string;
  readonly onChangeTxInput: (value: string) => void;
  readonly isBusy: boolean;
  readonly onSubmitTx: () => void;
  readonly onClearLogs: () => void;
}

export const Sc0BridgeLogConsole: React.FC<Sc0BridgeLogConsoleProps> = ({
  logs,
  lastEvent,
  bridgeRunning,
  inputMode,
  onChangeInputMode,
  txInput,
  onChangeTxInput,
  isBusy,
  onSubmitTx,
  onClearLogs,
}) => {
  const { t } = useTranslation();

  return (
    <div className="rounded-xl border border-slate-800 bg-slate-950 p-3 space-y-2">
      <div className="flex items-center justify-between text-[11px] text-slate-400">
        <span className="flex items-center gap-1.5">
          <Terminal className="w-3.5 h-3.5 text-cyan-400" />
          <span>{lastEvent}</span>
        </span>
        <button
          type="button"
          onClick={onClearLogs}
          className="flex items-center gap-1 text-[11px] text-slate-400 hover:text-slate-200 px-2 py-0.5 rounded hover:bg-slate-800"
        >
          <Trash2 className="w-3 h-3" />
          <span>{t('pico.bridge.clearStream')}</span>
        </button>
      </div>

      <div
        role="log"
        aria-label={t('pico.bridge.title')}
        className="h-36 overflow-y-auto rounded-lg bg-slate-900/60 border border-slate-800/80 p-2 space-y-1 font-mono text-[11px]"
      >
        {logs.length === 0 ? (
          <div className="h-full flex items-center justify-center text-slate-500 text-center px-4">
            {t('pico.bridge.emptyStream')}
          </div>
        ) : (
          logs.map(entry => (
            <div key={entry.id} className="flex items-start gap-2 leading-snug">
              <span className="text-slate-500 shrink-0">{entry.timestamp}</span>
              <span
                className={
                  entry.stream === 'tx'
                    ? 'text-amber-300 shrink-0 font-bold'
                    : entry.stream === 'sys'
                    ? 'text-emerald-400 shrink-0 font-bold'
                    : 'text-cyan-400 shrink-0 font-bold'
                }
              >
                [{entry.stream.toUpperCase()}]
              </span>
              <span className="text-slate-200 break-all">{entry.text}</span>
              {entry.hex && <span className="text-slate-500 shrink-0">[{entry.hex}]</span>}
            </div>
          ))
        )}
      </div>

      <div className="flex flex-col sm:flex-row items-stretch sm:items-center gap-2 pt-1">
        <select
          aria-label={t('pico.bridge.inputModeLabel')}
          value={inputMode}
          onChange={e => onChangeInputMode(e.target.value as Sc0BridgeInputMode)}
          className="rounded-lg bg-slate-900 border border-slate-700 px-2.5 py-1.5 text-xs text-slate-100"
        >
          <option value="line">{t('pico.bridge.modeLine')}</option>
          <option value="hex">{t('pico.bridge.modeHex')}</option>
          <option value="rsp">{t('pico.bridge.modeRsp')}</option>
          <option value="text">{t('pico.bridge.modeText')}</option>
        </select>

        <input
          type="text"
          aria-label={t('pico.bridge.inputPlaceholder')}
          placeholder={t('pico.bridge.inputPlaceholder')}
          value={txInput}
          disabled={!bridgeRunning || isBusy}
          onChange={e => onChangeTxInput(e.target.value)}
          onKeyDown={e => e.key === 'Enter' && onSubmitTx()}
          className="flex-1 rounded-lg bg-slate-900 border border-slate-700 px-3 py-1.5 text-xs text-slate-100 placeholder-slate-500 focus:outline-none focus:ring-1 focus:ring-cyan-500"
        />

        <Button
          type="button"
          size="sm"
          disabled={!bridgeRunning || isBusy || !txInput.trim()}
          onClick={onSubmitTx}
          className="gap-1.5 text-xs font-bold"
        >
          <Send className="w-3.5 h-3.5" />
          <span>{t('pico.bridge.send')}</span>
        </Button>
      </div>
    </div>
  );
};
