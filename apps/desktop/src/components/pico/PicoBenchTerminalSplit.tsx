import React, { type RefObject } from 'react';
import { Send } from 'lucide-react';
import { TerminalView } from '@/components/TerminalView';
import { Button, ResizableSplitPane } from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import type { ColdBreakinMode, HardwareSsotSpec, SerialLogEntry } from '@/models';
import { ColdBreakinSelector } from './ColdBreakinSelector';
import { P7501PinoutCard } from './P7501PinoutCard';

export interface PicoBenchTerminalSplitProps {
  readonly ssot: HardwareSsotSpec;
  readonly currentMode: ColdBreakinMode;
  readonly onSelectMode: (mode: ColdBreakinMode) => void;
  readonly logs: SerialLogEntry[];
  readonly clearLogs: () => void;
  readonly logContainerRef: RefObject<HTMLDivElement>;
  readonly commandInput: string;
  readonly setCommandInput: (val: string) => void;
  readonly sendCommand: () => void;
}

export const PicoBenchTerminalSplit: React.FC<PicoBenchTerminalSplitProps> = ({
  ssot,
  currentMode,
  onSelectMode,
  logs,
  clearLogs,
  logContainerRef,
  commandInput,
  setCommandInput,
  sendCommand,
}) => {
  const { t } = useTranslation();
  return (
    <ResizableSplitPane
      initialLeftPercent={42}
      minLeftPercent={28}
      maxLeftPercent={62}
      storageKey="panadiga.split.picobench"
      ariaLabel={t('pico.splitAria')}
      left={
        <div className="flex flex-col space-y-4 lg:min-h-[720px]">
          <ColdBreakinSelector
            currentMode={currentMode}
            onSelectMode={onSelectMode}
            breakinModes={ssot.breakin_modes}
          />
          <div className="flex-1 min-h-0 flex flex-col">
            <P7501PinoutCard ssot={ssot} />
          </div>
        </div>
      }
      right={
        <div className="flex flex-col space-y-3 min-h-[420px] lg:min-h-[720px]">
          <div className="flex-1 min-h-0 flex flex-col">
            <TerminalView logs={logs} onClear={clearLogs} containerRef={logContainerRef} />
          </div>
          <div className="flex items-center gap-2 p-2 rounded-xl bg-bench-900 border border-slate-800 shrink-0">
            <span className="text-slate-500 text-xs font-mono pl-2">{'>'}</span>
            <input
              type="text"
              aria-label={t('pico.title')}
              placeholder={t('pico.terminalPlaceholder')}
              value={commandInput}
              onChange={e => setCommandInput(e.target.value)}
              onKeyDown={e => e.key === 'Enter' && sendCommand()}
              className="flex-1 bg-transparent border-none text-xs text-slate-100 placeholder-slate-500 focus:outline-none font-mono"
            />
            <Button
              type="button"
              variant="default"
              size="sm"
              data-testid="pico-serial-send-btn"
              onClick={sendCommand}
              className="font-bold text-xs gap-1.5"
            >
              <Send className="w-3.5 h-3.5" />
              <span>{t('pico.sendButton')}</span>
            </Button>
          </div>
        </div>
      }
    />
  );
};
