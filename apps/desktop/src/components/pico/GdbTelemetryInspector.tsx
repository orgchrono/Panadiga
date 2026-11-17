import React, { useState } from 'react';
import { GDB_TELEMETRY_PREVIEW_LIMIT } from '@/constants';
import { useTranslation } from '@/i18n/useTranslation';
import { GdbPacketTelemetryEvent } from '@/models';
import { withUniqueKeys } from '@/utils/formatters';
import { Activity, ChevronDown, ChevronUp } from 'lucide-react';

export interface GdbTelemetryInspectorProps {
  isRunning?: boolean;
  telemetry?: GdbPacketTelemetryEvent[];
}

export const GdbTelemetryInspector: React.FC<GdbTelemetryInspectorProps> = ({
  telemetry = [],
}) => {
  const { t } = useTranslation();
  const [showTelemetry, setShowTelemetry] = useState<boolean>(false);
  const recentTelemetry = withUniqueKeys(
    telemetry
      .slice(Math.max(0, telemetry.length - GDB_TELEMETRY_PREVIEW_LIMIT))
      .map((_, idx, list) => list[list.length - 1 - idx]),
    (pkt) => `${pkt.timestamp_ms}-${pkt.direction}-${pkt.command_type}-${pkt.raw_packet}`,
  );

  return (
    <div className="border-t border-slate-800 pt-3">
      <button
        type="button"
        onClick={() => setShowTelemetry(!showTelemetry)}
        className="flex items-center justify-between w-full text-xs font-semibold text-slate-300 hover:text-cyan-400 py-1 transition-colors"
      >
        <div className="flex items-center gap-2">
          <Activity className="w-4 h-4 text-cyan-400" />
          <span>{t('pico.telemetry.title', { count: telemetry.length })}</span>
        </div>
        {showTelemetry ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
      </button>

      {showTelemetry && (
        <div className="mt-3 p-3 rounded-xl bg-slate-950 border border-slate-800 max-h-48 overflow-y-auto font-mono text-[11px] space-y-1.5">
          {recentTelemetry.length === 0 ? (
            <p className="text-slate-500 italic">{t('pico.telemetry.empty')}</p>
          ) : (
            recentTelemetry.map(({ item: pkt, key }) => (
              <div
                key={key}
                className="flex items-center justify-between gap-2 p-1.5 rounded bg-slate-900 border border-slate-800/60"
              >
                <div className="flex items-center gap-2 overflow-hidden">
                  <span
                    className={`px-1.5 py-0.5 rounded text-[10px] font-bold ${
                      pkt.direction === 'INBOUND'
                        ? 'bg-blue-500/20 text-blue-400 border border-blue-500/30'
                        : 'bg-emerald-500/20 text-emerald-400 border border-emerald-500/30'
                    }`}
                  >
                    {pkt.direction === 'INBOUND'
                      ? t('pico.telemetry.hostToPico')
                      : t('pico.telemetry.picoToHost')}
                  </span>
                  <span className="text-slate-400 font-bold">[{pkt.command_type}]</span>
                  <span className="text-slate-300 truncate max-w-md">{pkt.raw_packet}</span>
                </div>
                <div className="shrink-0">
                  {pkt.checksum_valid ? (
                    <span className="text-emerald-400 text-[10px] font-bold">
                      {t('pico.telemetry.csumOk')}
                    </span>
                  ) : (
                    <span className="text-rose-400 text-[10px] font-bold">
                      {t('pico.telemetry.csumErr')}
                    </span>
                  )}
                </div>
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
};
