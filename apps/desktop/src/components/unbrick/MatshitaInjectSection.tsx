import React, { useState } from 'react';
import { Loader2, ShieldAlert } from 'lucide-react';
import type { MatshitaInjectWriteResult } from '@/models';
import { Button, Input } from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import { MatshitaErrorBanner } from './MatshitaErrorBanner';
import type { MatshitaAsyncSlice } from './matshitaTypes';

interface MatshitaInjectSectionProps {
  injectState: MatshitaAsyncSlice<MatshitaInjectWriteResult>;
  onInjectWrite: (params: {
    targetVa?: string;
    offset?: number;
    mode?: number;
    useVendorEd?: boolean;
    payloadHex: string;
  }) => Promise<MatshitaInjectWriteResult | null>;
}

export const MatshitaInjectSection: React.FC<MatshitaInjectSectionProps> = ({
  injectState,
  onInjectWrite,
}) => {
  const { t } = useTranslation();
  const [targetVa, setTargetVa] = useState('0x51800000');
  const [payloadHex, setPayloadHex] = useState('DEADBEEF');
  const [useVendorEd, setUseVendorEd] = useState(false);

  return (
    <section className="space-y-3 rounded-xl border border-slate-800 bg-slate-950/60 p-4">
      <div className="flex items-center gap-2 text-xs font-mono font-semibold text-slate-200">
        <ShieldAlert className="w-3.5 h-3.5 text-amber-400" />
        <span>{t('unbrick.matshita.injectTitle')}</span>
      </div>
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
        <div className="space-y-1">
          <label htmlFor="matshita-target-va" className="text-[11px] font-mono text-slate-400">
            {t('unbrick.matshita.targetVaLabel')}
          </label>
          <Input
            id="matshita-target-va"
            value={targetVa}
            onChange={e => setTargetVa(e.target.value)}
            placeholder="0x51800000"
          />
        </div>
        <div className="space-y-1">
          <label htmlFor="matshita-payload-hex" className="text-[11px] font-mono text-slate-400">
            {t('unbrick.matshita.payloadHexLabel')}
          </label>
          <Input
            id="matshita-payload-hex"
            value={payloadHex}
            onChange={e => setPayloadHex(e.target.value)}
            placeholder="DEADBEEF"
          />
        </div>
      </div>
      <label className="flex items-center gap-2 text-xs font-mono text-slate-300 cursor-pointer">
        <input
          id="matshita-vendor-ed"
          type="checkbox"
          checked={useVendorEd}
          onChange={e => setUseVendorEd(e.target.checked)}
          className="rounded border-slate-700 bg-slate-900"
        />
        <span>{t('unbrick.matshita.vendorEdToggle')}</span>
      </label>
      <Button
        type="button"
        size="sm"
        variant="outline"
        disabled={injectState.loading}
        onClick={() =>
          void onInjectWrite({
            targetVa,
            useVendorEd,
            payloadHex,
          })
        }
        data-testid="matshita-inject-btn"
      >
        {injectState.loading && <Loader2 className="w-3.5 h-3.5 mr-1.5 animate-spin" />}
        {t('unbrick.matshita.injectBtn')}
      </Button>
      <MatshitaErrorBanner message={injectState.error} />
      {injectState.data && (
        <div className="rounded-lg border border-slate-800 bg-slate-900/70 p-2.5 text-[11px] font-mono text-emerald-300 space-y-0.5">
          <div>
            {injectState.data.record.source} ({injectState.data.record.cdb0_or_cmd}) -&gt;{' '}
            {injectState.data.record.status}
          </div>
          <div className="text-slate-400 break-all">{injectState.data.record.note}</div>
        </div>
      )}
    </section>
  );
};
