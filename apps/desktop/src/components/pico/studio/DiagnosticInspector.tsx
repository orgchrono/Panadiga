import React from 'react';
import { DIAGNOSTIC_HISTORY_PREVIEW_LIMIT } from '@/constants';
import { useTranslation } from '@/i18n/useTranslation';
import type { PicoDiagnosticResult } from '@/models';
import { Badge, EmptyState, ScrollArea } from '@/components/ui';
import { withUniqueKeys } from '@/utils/formatters';
import { History } from 'lucide-react';
import { DiagnosticResultTables } from './DiagnosticResultTables';

export interface DiagnosticInspectorProps {
  activeResult: PicoDiagnosticResult | null;
  history: PicoDiagnosticResult[];
  onInspect: (item: PicoDiagnosticResult) => void;
}

type BadgeVariant = 'default' | 'secondary' | 'destructive' | 'success' | 'warning';

interface MetaBadgeItem {
  readonly key: string;
  readonly variant: BadgeVariant;
  readonly label: string;
}

const DiagnosticMetaBadges: React.FC<{ activeResult: PicoDiagnosticResult }> = ({
  activeResult,
}) => {
  const { t } = useTranslation();
  const resIsOk =
    activeResult.protocol_code === '0' || activeResult.protocol_code?.toUpperCase() === 'OK';
  const resVariant: BadgeVariant = resIsOk ? 'success' : 'destructive';

  const optionalBadges: ReadonlyArray<MetaBadgeItem> = [
    ...(activeResult.vdd_volts !== null && activeResult.vdd_volts !== undefined
      ? [{ key: 'vdd', variant: 'default' as const, label: t('pico.studio.vddBadge', { volts: activeResult.vdd_volts.toFixed(2) }) }]
      : []),
    ...(activeResult.detected_flash_mb
      ? [{ key: 'flash', variant: 'success' as const, label: t('pico.studio.flashBadge', { mb: activeResult.detected_flash_mb }) }]
      : []),
    ...(activeResult.xreset_rise_us
      ? [{ key: 'rise', variant: 'secondary' as const, label: t('pico.studio.xresetRisePrefix', { value: activeResult.xreset_rise_us }) }]
      : []),
    ...(activeResult.captured_bytes !== null && activeResult.captured_bytes !== undefined
      ? [{ key: 'cap', variant: 'secondary' as const, label: t('pico.studio.capturedBytesSuffix', { bytes: activeResult.captured_bytes }) }]
      : []),
    ...(activeResult.ident_model
      ? [{ key: 'model', variant: 'default' as const, label: t('pico.studio.modelBadge', { model: activeResult.ident_model }) }]
      : []),
    ...(activeResult.ident_flash
      ? [{ key: 'chip', variant: 'secondary' as const, label: t('pico.studio.flashChipBadge', { chip: activeResult.ident_flash }) }]
      : []),
    ...(activeResult.ident_loader
      ? [{ key: 'loader', variant: 'secondary' as const, label: t('pico.studio.loaderBadge', { loader: activeResult.ident_loader }) }]
      : []),
    ...(activeResult.protocol_code
      ? [{ key: 'res', variant: resVariant, label: t('pico.studio.resCodeBadge', { code: activeResult.protocol_code }) }]
      : []),
  ];

  return (
    <div className="flex flex-wrap items-center gap-2">
      <Badge variant={activeResult.success ? 'success' : 'destructive'}>
        {activeResult.success ? t('pico.studio.statusOk') : t('pico.studio.statusFail')}
      </Badge>
      <code className="text-xs text-cyan-300 font-bold">{activeResult.raw_command}</code>
      <Badge variant="secondary">{activeResult.elapsed_ms} ms</Badge>
      {optionalBadges.map((b) => (
        <Badge key={b.key} variant={b.variant}>
          {b.label}
        </Badge>
      ))}
    </div>
  );
};

export const DiagnosticInspector: React.FC<DiagnosticInspectorProps> = ({
  activeResult,
  history,
  onInspect,
}) => {
  const { t } = useTranslation();
  const previewHistory = withUniqueKeys(
    history.slice(0, DIAGNOSTIC_HISTORY_PREVIEW_LIMIT),
    (item) => `${item.raw_command}-${item.elapsed_ms}`,
  );
  const keyedFindings = activeResult?.findings
    ? withUniqueKeys(activeResult.findings, (f) => `${f.code}-${f.summary}`)
    : [];
  const wireStreamItems = activeResult
    ? withUniqueKeys(
        [
          ...(activeResult.progress_steps ?? []).map((s) => `@PROG ${s}`),
          ...(activeResult.wire_events ?? []).map((e) => `@EVT ${e}`),
        ],
        (item) => item,
      )
    : [];

  return (
    <div className="rounded-xl bg-slate-950 border border-slate-800 p-4 space-y-3">
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-slate-800 pb-2.5">
        <div className="flex items-center gap-2">
          <History className="w-4 h-4 text-cyan-400" />
          <span className="text-xs font-bold text-slate-200">
            {t('pico.studio.inspectorTitle')}
          </span>
        </div>

        {previewHistory.length > 0 && (
          <div className="flex items-center gap-1.5 flex-wrap">
            {previewHistory.map(({ item, key }) => (
              <button
                key={key}
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
          title={t('pico.studio.emptyHistoryTitle')}
          description={t('pico.studio.emptyHistoryDesc')}
          className="min-h-[160px] p-4 border-slate-800/70"
        />
      ) : (
        <div className="space-y-3">
          <DiagnosticMetaBadges activeResult={activeResult} />

          <DiagnosticResultTables result={activeResult} />

          {wireStreamItems.length > 0 && (
            <div
              className="rounded-lg border border-slate-800 bg-slate-900/60 px-3 py-2 space-y-1"
              data-testid="diagnostic-wire-events"
            >
              <div className="text-[10px] font-bold uppercase tracking-wider text-cyan-300">
                {t('pico.studio.wireEventsTitle')}
              </div>
              <ul className="space-y-0.5 text-[11px] text-slate-300">
                {wireStreamItems.map(({ item, key }) => (
                  <li key={key} className="truncate">
                    {item}
                  </li>
                ))}
              </ul>
            </div>
          )}

          {keyedFindings.length > 0 && (
            <div className="space-y-1.5" data-testid="diagnostic-findings">
              {keyedFindings.map(({ item: finding, key }) => (
                <div
                  key={key}
                  className="rounded-lg border border-slate-800 bg-slate-900/70 px-3 py-2 text-[11px] flex flex-wrap items-start gap-2"
                >
                  <Badge
                    variant={
                      finding.severity === 'error'
                        ? 'destructive'
                        : finding.severity === 'warning'
                          ? 'warning'
                          : 'secondary'
                    }
                  >
                    {finding.code}
                  </Badge>
                  <div className="min-w-0 flex-1 space-y-0.5">
                    <div className="font-semibold text-slate-200">{finding.summary}</div>
                    <div className="text-[10px] text-slate-400 font-sans break-words">
                      {finding.detail}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}

          <ScrollArea className="max-h-44 rounded-lg bg-slate-900/90 border border-slate-800 p-3">
            <pre className="text-[11px] text-slate-300 whitespace-pre-wrap break-words font-mono">
              {activeResult.raw_output}
            </pre>
          </ScrollArea>
        </div>
      )}
    </div>
  );
};
