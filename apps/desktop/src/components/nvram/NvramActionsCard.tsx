import React from 'react';
import { Cpu, ShieldCheck, RefreshCw, FileCode, Radio } from 'lucide-react';
import { StatusBadge } from '@/components/StatusBadge';
import { Button } from '@/components/ui/button';
import { useTranslation } from '@/i18n/useTranslation';
import { LiveNvramCaptureEvent, NvramCleanReport } from '@/models';

interface NvramActionsCardProps {
  report: NvramCleanReport | null;
  cleaning: boolean;
  cleanSuccess: boolean;
  liveCapture?: LiveNvramCaptureEvent | null;
  capturingLive?: boolean;
  onExecuteClean: () => void;
  onCaptureLive?: () => void;
  onClearLiveCapture?: () => void;
  onRefresh: () => void;
}

export const NvramActionsCard: React.FC<NvramActionsCardProps> = ({
  report,
  cleaning,
  cleanSuccess,
  liveCapture = null,
  capturingLive = false,
  onExecuteClean,
  onCaptureLive,
  onClearLiveCapture,
  onRefresh,
}) => {
  const { t } = useTranslation();
  const flashBytes = report?.total_flash_bytes ?? 0;

  const startHex = report
    ? `0x${report.nvram_offset_start.toString(16).toUpperCase().padStart(8, '0')}`
    : '0x007F0000';
  const endHex = report
    ? `0x${(report.nvram_offset_end - 1).toString(16).toUpperCase().padStart(8, '0')}`
    : '0x007FFFFF';

  const flashName =
    flashBytes === 0x0020_0000
      ? 'Spansion S29AL016J NOR Flash (2 MB / 16 Mbit)'
      : flashBytes === 0x0040_0000
        ? 'Spansion S29JL032H NOR Flash (4 MB / 32 Mbit)'
        : flashBytes === 0x0100_0000
          ? 'Spansion S29GL128P NOR Flash (16 MB / 128 Mbit)'
          : 'Spansion S29JL064H NOR Flash (8 MB / 64 Mbit)';

  return (
    <div className="flex flex-col md:flex-row items-start md:items-center justify-between gap-4 p-5 rounded-2xl bg-bench-900 border border-slate-800 shadow-xl">
      <div className="flex items-center gap-3.5">
        <div className="p-3 rounded-xl bg-amber-500/10 text-amber-400 border border-amber-500/30">
          <Cpu className="w-6 h-6" />
        </div>
        <div>
          <div className="flex items-center gap-2 flex-wrap">
            <h2 className="text-base font-bold text-slate-100">{flashName}</h2>
            <StatusBadge variant="info" label="TOP-BOOT" />
            <StatusBadge variant="neutral" label="TSOP-48 / BGA" />
            {liveCapture && (
              <StatusBadge
                variant="success"
                label={t('nvram.actions.livePicoBadge', {
                  port: liveCapture.port,
                  addr: liveCapture.base_bus_address,
                })}
              />
            )}
          </div>
          <p className="text-xs text-slate-400 mt-1">
            {t('nvram.actions.descPrefix')}{' '}
            <code className="text-cyan-400">{startHex}..{endHex}</code>{' '}
            {t('nvram.actions.descMiddle', { count: report?.microsectors.length ?? 8 })}
            {liveCapture && (
              <span className="text-emerald-400 ml-2">
                {t('nvram.actions.liveCaptureInfo', {
                  source: liveCapture.source,
                  bytes: liveCapture.captured_bytes,
                  time: liveCapture.captured_at,
                })}
              </span>
            )}
          </p>
        </div>
      </div>

      <div className="flex items-center gap-2.5 flex-wrap shrink-0">
        {liveCapture && onClearLiveCapture && (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            data-testid="nvram-back-to-disk-btn"
            onClick={onClearLiveCapture}
            className="text-xs text-slate-400 hover:text-slate-200"
          >
            {t('nvram.actions.backToDisk')}
          </Button>
        )}

        {onCaptureLive && (
          <Button
            type="button"
            variant="outline"
            data-testid="nvram-capture-live-btn"
            onClick={onCaptureLive}
            disabled={capturingLive || cleaning}
            className="gap-2 bg-cyan-500/10 hover:bg-cyan-500/20 text-cyan-300 border-cyan-500/40"
          >
            <Radio className={`w-4 h-4 ${capturingLive ? 'animate-pulse' : ''}`} />
            <span>
              {capturingLive ? t('nvram.actions.capturingPico') : t('nvram.actions.captureLiveBtn')}
            </span>
          </Button>
        )}

        <Button
          variant="ghost"
          size="icon"
          onClick={onRefresh}
          title={t('nvram.actions.reloadTooltip')}
          aria-label={t('nvram.actions.reloadTooltip')}
        >
          <RefreshCw className="w-4 h-4 text-slate-400" />
        </Button>

        <Button
          variant="emerald"
          data-testid="nvram-clean-btn"
          onClick={onExecuteClean}
          disabled={cleaning || cleanSuccess || !report || Boolean(liveCapture)}
          className="gap-2"
        >
          {cleaning ? (
            <>
              <RefreshCw className="w-4 h-4 animate-spin" />
              <span>{t('nvram.actions.cleaningBtn')}</span>
            </>
          ) : cleanSuccess ? (
            <>
              <ShieldCheck className="w-4 h-4" />
              <span>{t('nvram.actions.cleanedBadge')}</span>
            </>
          ) : (
            <>
              <FileCode className="w-4 h-4" />
              <span>{t('nvram.actions.surgicalCleanBtn')}</span>
            </>
          )}
        </Button>
      </div>
    </div>
  );
};
