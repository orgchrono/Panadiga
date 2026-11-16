import React from 'react';
import { Card, CardContent } from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import type { BridgeStreamLine } from '@/viewmodels/useSc0Bridge';
import type { Sc0BridgeInputMode, Sc0BridgeStatus } from '@/models';
import { BridgeNoticeBanner } from './BridgeNoticeBanner';
import type { Sc0BridgeProbeHandlers } from './sc0BridgeTypes';
import { Sc0BridgeHeader } from './Sc0BridgeHeader';
import { Sc0BridgeQuickProbes } from './Sc0BridgeQuickProbes';
import { Sc0BridgeLogConsole } from './Sc0BridgeLogConsole';

export interface Sc0BridgePanelProps extends Sc0BridgeProbeHandlers {
  readonly status: Sc0BridgeStatus;
  readonly logs: readonly BridgeStreamLine[];
  readonly baudRate: string;
  readonly onChangeBaudRate: (baud: string) => void;
  readonly bootMode: string;
  readonly onChangeBootMode: (mode: string) => void;
  readonly inputMode: Sc0BridgeInputMode;
  readonly onChangeInputMode: (mode: Sc0BridgeInputMode) => void;
  readonly txInput: string;
  readonly onChangeTxInput: (value: string) => void;
  readonly isBusy: boolean;
  readonly error: string | null;
  readonly onStart: () => void;
  readonly onStop: () => void;
  readonly onSubmitTx: () => void;
  readonly onClearLogs: () => void;
}

export const Sc0BridgePanel: React.FC<Sc0BridgePanelProps> = ({
  status,
  logs,
  baudRate,
  onChangeBaudRate,
  bootMode,
  onChangeBootMode,
  inputMode,
  onChangeInputMode,
  txInput,
  onChangeTxInput,
  isBusy,
  error,
  onStart,
  onStop,
  onSubmitTx,
  onClearLogs,
  onTriggerWaittx,
  onSendRspPreset,
  onSendDriveCommand,
  onRunDiagnostic,
}) => {
  const { t } = useTranslation();

  return (
    <Card className="border-slate-800 bg-bench-900/90 shadow-xl">
      <Sc0BridgeHeader
        status={status}
        baudRate={baudRate}
        onChangeBaudRate={onChangeBaudRate}
        bootMode={bootMode}
        onChangeBootMode={onChangeBootMode}
        isBusy={isBusy}
        onStart={onStart}
        onStop={onStop}
      />

      <CardContent className="pt-4 space-y-4">
        <BridgeNoticeBanner
          role="note"
          toneClass="border-sky-500/30 bg-sky-500/10 text-[11px] text-sky-200"
          titleClass="text-sky-100"
          bodyClass="text-sky-200/90"
          title={t('pico.bridge.busArchitectureTitle')}
          body={t('pico.bridge.busArchitectureSummary')}
        />
        {logs.some(entry => /BGO\s+driver\s+init\s+failed/i.test(entry.text)) && (
          <BridgeNoticeBanner
            role="alert"
            toneClass="border-amber-500/40 bg-amber-500/10 text-xs text-amber-200"
            titleClass="text-amber-100"
            bodyClass="text-amber-200/90"
            title={t('pico.bridge.bgoHaltWarningTitle')}
            body={t('pico.bridge.bgoHaltWarningBody')}
          />
        )}
        {error && (
          <div
            role="alert"
            className="rounded-lg border border-rose-500/40 bg-rose-500/10 px-3 py-2 text-xs text-rose-300"
          >
            {error}
          </div>
        )}

        <Sc0BridgeQuickProbes
          bridgeRunning={status.running}
          isBusy={isBusy}
          onTriggerWaittx={onTriggerWaittx}
          onSendRspPreset={onSendRspPreset}
          onSendDriveCommand={onSendDriveCommand}
          onRunDiagnostic={onRunDiagnostic}
        />

        <Sc0BridgeLogConsole
          logs={logs}
          lastEvent={status.last_event}
          bridgeRunning={status.running}
          inputMode={inputMode}
          onChangeInputMode={onChangeInputMode}
          txInput={txInput}
          onChangeTxInput={onChangeTxInput}
          isBusy={isBusy}
          onSubmitTx={onSubmitTx}
          onClearLogs={onClearLogs}
        />
      </CardContent>
    </Card>
  );
};
