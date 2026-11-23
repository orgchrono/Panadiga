import React, { useId, useState } from 'react';
import { ChevronDown, ChevronUp, Send, Terminal, Trash2, X } from 'lucide-react';
import type { TranslationKey } from '@/i18n/translations';
import { useTranslation } from '@/i18n/useTranslation';
import { cn } from '@/lib/utils';
import type { SerialLogEntry } from '@/models';
import { Badge } from './badge';
import { Button } from './button';
import { Input } from './input';

export interface BottomConsoleDrawerProps {
  readonly open: boolean;
  readonly expanded: boolean;
  readonly onToggleExpand: () => void;
  readonly onClose: () => void;
  readonly logs: readonly SerialLogEntry[];
  readonly onClearLogs: () => void;
  readonly commandInput: string;
  readonly onCommandInputChange: (value: string) => void;
  readonly onSendCommand: () => void;
  readonly hardwareConnected?: boolean;
}

type StreamFilterMode = 'all' | 'rxtx' | 'errors';

const STREAM_FILTERS: ReadonlyArray<{ readonly id: StreamFilterMode; readonly labelKey: TranslationKey }> = [
  { id: 'all', labelKey: 'shell.docks.filterAll' },
  { id: 'rxtx', labelKey: 'shell.docks.filterRxTx' },
  { id: 'errors', labelKey: 'shell.docks.filterErrors' },
];

const LEVEL_COLOR_MAP: Readonly<Record<SerialLogEntry['level'], string>> = {
  success: 'text-emerald-400',
  warn: 'text-amber-400',
  error: 'text-rose-400',
  rx: 'text-cyan-400',
  tx: 'text-sky-300',
  info: 'text-slate-300',
};

const SOURCE_STYLE_MAP: Readonly<Record<SerialLogEntry['source'], string>> = {
  PICO: 'bg-cyan-950/70 text-cyan-300 border-cyan-800/80',
  TARGET: 'bg-purple-950/70 text-purple-300 border-purple-800/80',
  HOST: 'bg-slate-900 text-slate-300 border-slate-700',
};

function matchesStreamFilter(entry: SerialLogEntry, mode: StreamFilterMode): boolean {
  if (mode === 'rxtx') return entry.level === 'rx' || entry.level === 'tx';
  if (mode === 'errors') return entry.level === 'error' || entry.level === 'warn';
  return true;
}

export const BottomConsoleDrawer: React.FC<BottomConsoleDrawerProps> = ({
  open, expanded, onToggleExpand, onClose, logs, onClearLogs,
  commandInput, onCommandInputChange, onSendCommand, hardwareConnected = false,
}) => {
  const { t } = useTranslation();
  const consoleErrorId = useId();
  const [filterMode, setFilterMode] = useState<StreamFilterMode>('all');
  const [inputError, setInputError] = useState<string | null>(null);
  if (!open) return null;

  const visibleLogs = logs.filter((entry) => matchesStreamFilter(entry, filterMode));
  const expandLabel = expanded ? t('shell.docks.consoleCollapse') : t('shell.docks.consoleExpand');

  return (
    <section
      data-testid="bottom-console-drawer"
      aria-label={t('shell.docks.consoleTitle')}
      className={cn('flex flex-col border-t border-slate-800 bg-bench-950/95 font-mono shrink-0 transition-all', expanded ? 'h-80 max-h-[55dvh]' : 'h-52 max-h-[45dvh]')}
    >
      <div className="flex flex-wrap items-center justify-between gap-2 px-3 py-1.5 bg-bench-900/90 border-b border-slate-800">
        <div className="flex items-center gap-2 min-w-0">
          <Terminal className="w-3.5 h-3.5 text-cyan-400 shrink-0" />
          <span className="text-xs font-bold text-slate-100 truncate">{t('shell.docks.consoleTitle')}</span>
          <Badge variant={hardwareConnected ? 'success' : 'secondary'} className="text-[9px] px-1.5 py-0">
            {hardwareConnected ? t('pico.connected') : t('pico.disconnected')}
          </Badge>
          <span className="text-[10px] text-slate-500">{t('pico.terminal.eventsCount', { count: visibleLogs.length })}</span>
        </div>

        <div className="flex items-center gap-1.5">
          <div className="inline-flex rounded-lg bg-slate-950 border border-slate-800 p-0.5">
            {STREAM_FILTERS.map((item) => (
              <button
                key={item.id}
                type="button"
                onClick={() => setFilterMode(item.id)}
                aria-pressed={filterMode === item.id}
                className={cn('px-2 py-0.5 rounded text-[10px] transition-colors', filterMode === item.id ? 'bg-cyan-500/20 text-cyan-300 font-bold' : 'text-slate-400 hover:text-slate-200')}
              >
                {t(item.labelKey)}
              </button>
            ))}
          </div>
          <Button type="button" variant="ghost" size="icon" data-testid="bottom-console-expand-btn" onClick={onToggleExpand} title={expandLabel} aria-label={expandLabel} className="h-7 w-7">
            {expanded ? <ChevronDown className="w-3.5 h-3.5" /> : <ChevronUp className="w-3.5 h-3.5" />}
          </Button>
          <Button type="button" variant="ghost" size="sm" onClick={onClearLogs} title={t('pico.terminal.clearTitle')} aria-label={t('pico.terminal.clearAria')} className="h-7 px-2 text-[10px] gap-1 text-slate-400 hover:text-slate-100">
            <Trash2 className="w-3 h-3" />
            <span>{t('pico.terminal.clearBtn')}</span>
          </Button>
          <Button type="button" variant="ghost" size="icon" onClick={onClose} title={t('shell.docks.closeDock')} aria-label={t('shell.docks.closeDock')} className="h-7 w-7">
            <X className="w-3.5 h-3.5" />
          </Button>
        </div>
      </div>

      <div role="log" aria-live="polite" aria-label={t('pico.terminal.streamAria')} className="flex-1 overflow-y-auto px-3 py-2 space-y-1 text-xs select-text scrollbar-thin scrollbar-thumb-slate-800">
        {visibleLogs.length === 0 ? (
          <div className="h-full flex items-center justify-center text-slate-500 text-xs italic font-sans">{t('pico.terminal.empty')}</div>
        ) : (
          visibleLogs.map((entry) => (
            <div key={entry.id} className="flex items-start gap-2 px-1.5 py-0.5 rounded hover:bg-slate-900/60 leading-relaxed">
              <span className="text-[10px] text-slate-500 shrink-0">{entry.timestamp}</span>
              <span className={cn('text-[10px] px-1 rounded border shrink-0', SOURCE_STYLE_MAP[entry.source])}>{entry.source}</span>
              <span className={cn('flex-1 break-all', LEVEL_COLOR_MAP[entry.level])}>{entry.message}</span>
            </div>
          ))
        )}
      </div>

      <form
        onSubmit={(event) => {
          event.preventDefault();
          if (!commandInput.trim()) {
            setInputError(t('shell.docks.emptyCommandError'));
            return;
          }
          setInputError(null);
          onSendCommand();
        }}
        className="border-t border-slate-800 bg-bench-900/60"
      >
        <div className="flex items-center gap-2 px-3 py-1.5">
          <Input
            data-testid="bottom-console-input"
            value={commandInput}
            onChange={(event) => {
              if (inputError && event.target.value.trim()) {
                setInputError(null);
              }
              onCommandInputChange(event.target.value);
            }}
            placeholder={t('pico.terminalPlaceholder')}
            aria-label={t('pico.terminalPlaceholder')}
            aria-invalid={Boolean(inputError)}
            aria-describedby={inputError ? consoleErrorId : undefined}
            className="h-8 flex-1 text-xs"
          />
          <Button
            type="submit"
            size="sm"
            data-testid="bottom-console-send-btn"
            disabled={Boolean(inputError)}
            className="h-8 px-3 gap-1.5 text-xs shrink-0"
          >
            <Send className="w-3 h-3" />
            <span>{t('pico.sendButton')}</span>
          </Button>
        </div>
        {inputError && (
          <p id={consoleErrorId} role="alert" className="px-3 pb-1.5 text-[11px] text-rose-400">
            {inputError}
          </p>
        )}
      </form>
    </section>
  );
};
