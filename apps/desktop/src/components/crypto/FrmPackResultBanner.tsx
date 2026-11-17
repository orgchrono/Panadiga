/**
 * @file components/crypto/FrmPackResultBanner.tsx
 * @brief Status, validation error, idle state, and module summary panel for FrmPackCard.
 */

import React from 'react';
import { AlertCircle, CheckCircle2 } from 'lucide-react';
import { HEX16_PAD_WIDTH } from '@/constants';
import type { FrmPackReport } from '@/models';
import { useTranslation } from '@/i18n/useTranslation';

interface FrmPackResultBannerProps {
  packLoading: boolean;
  packError: string | null;
  packResult: FrmPackReport | null;
  success: string | null;
}

export const FrmPackResultBanner: React.FC<FrmPackResultBannerProps> = ({
  packLoading,
  packError,
  packResult,
  success,
}) => {
  const { t } = useTranslation();

  return (
    <>
      {success && !packError && <p className="text-[11px] text-emerald-300 font-mono">{success}</p>}

      {packError && (
        <div className="p-3 rounded-lg bg-rose-500/10 border border-rose-500/30 text-xs text-rose-300 flex items-start gap-2">
          <AlertCircle className="w-4 h-4 shrink-0 mt-0.5" />
          <span>{packError}</span>
        </div>
      )}

      {!packLoading && !packError && !packResult && (
        <p className="text-[11px] text-slate-500 bg-slate-950 p-3 rounded-lg border border-slate-800">
          {t('crypto.packager.packIdle')}
        </p>
      )}

      {packResult && (
        <div className="p-3.5 rounded-xl bg-emerald-500/10 border border-emerald-500/30 space-y-2 text-xs">
          <div className="flex items-center justify-between">
            <span className="text-emerald-300 font-bold flex items-center gap-1.5">
              <CheckCircle2 className="w-4 h-4" />
              <span>{t('crypto.packager.packSuccess')}</span>
            </span>
            <span className="text-slate-300">
              {t('crypto.packager.totalSummary', {
                bytes: packResult.total_bytes.toLocaleString(),
                tocSize: packResult.toc_region_size,
              })}
            </span>
          </div>
          <div className="space-y-1 text-[11px] text-slate-300">
            {packResult.modules.length === 0 ? (
              <p className="text-slate-500">{t('crypto.packager.noModules')}</p>
            ) : (
              packResult.modules.map(m => (
                <div key={m.index} className="flex justify-between bg-slate-950/70 px-2.5 py-1 rounded">
                  <span>
                    [{m.index}] {m.name} @ 0x{m.module_offset.toString(16).toUpperCase()}
                  </span>
                  <span className="text-cyan-300">
                    {m.module_size.toLocaleString()} B • FVU 0x{m.fvu_checksum_u32.toString(16).toUpperCase()} • sum 0x
                    {m.list_sum_u16.toString(16).toUpperCase().padStart(HEX16_PAD_WIDTH, '0')}
                  </span>
                </div>
              ))
            )}
          </div>
        </div>
      )}
    </>
  );
};
