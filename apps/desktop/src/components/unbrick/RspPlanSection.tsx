import React, { useState } from 'react';
import { Loader2, Terminal } from 'lucide-react';
import type { MainPassthroughRspPlan, NorDumperStubResult } from '@/models';
import { Badge, Button, EmptyState, Input } from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';

interface RspPlanSectionProps {
  rspLoading: boolean;
  rspPlan: MainPassthroughRspPlan | null;
  norStubLoading: boolean;
  norStubResult: NorDumperStubResult | null;
  onRefreshRspPlan: (callbackTargetVa?: string) => Promise<MainPassthroughRspPlan | null>;
  onBuildNorStub: (params: {
    wrapAsDigadiag: boolean;
    outPath?: string;
  }) => Promise<NorDumperStubResult | null>;
}

export const RspPlanSection: React.FC<RspPlanSectionProps> = ({
  rspLoading,
  rspPlan,
  norStubLoading,
  norStubResult,
  onRefreshRspPlan,
  onBuildNorStub,
}) => {
  const { t } = useTranslation();
  const [callbackVa, setCallbackVa] = useState('0x50401F3C');

  const rspFrames = rspPlan?.passthrough_frames ?? [];
  const picoShortcuts = rspPlan?.pico_cli_shortcuts ?? [];

  return (
    <div className="border-t border-slate-800/80 pt-3 space-y-2.5 text-xs">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <span className="font-mono font-bold text-slate-200 flex items-center gap-1.5">
          <Terminal className="w-3.5 h-3.5 text-purple-400" />
          <span>{t('unbrick.rsp.title')}</span>
        </span>
        <Badge variant="outline">MAGIC {rspPlan?.handshake_hex ?? '0e0402'}</Badge>
      </div>

      <div className="flex gap-2">
        <Input
          value={callbackVa}
          aria-label={t('unbrick.rsp.callbackLabel')}
          onChange={e => setCallbackVa(e.target.value)}
          placeholder="0x50401F3C"
        />
        <Button
          type="button"
          size="sm"
          variant="purple"
          disabled={rspLoading}
          onClick={() => void onRefreshRspPlan(callbackVa)}
        >
          {rspLoading && <Loader2 className="w-3.5 h-3.5 animate-spin mr-1" />}
          <span>{t('unbrick.rsp.refreshBtn')}</span>
        </Button>
      </div>

      {rspFrames.length === 0 ? (
        <EmptyState compact title={t('unbrick.rsp.empty')} />
      ) : (
        <div className="rounded-xl bg-slate-950/90 border border-slate-800 p-2.5 font-mono text-[11px] space-y-1 text-slate-300 break-all">
          <div>
            cb@{rspPlan?.serial_callback_va}: {rspPlan?.callback_write_frame ?? 'none'} | isr_pc_off:{' '}
            {rspPlan?.isr_saved_pc_offset}
          </div>
          {picoShortcuts.map(cmd => (
            <div key={cmd} className="text-emerald-300">
              &gt; {cmd}
            </div>
          ))}
        </div>
      )}

      <div className="flex flex-wrap items-center justify-between gap-2 pt-1">
        <Button
          type="button"
          size="sm"
          variant="outline"
          disabled={norStubLoading}
          onClick={() =>
            void onBuildNorStub({
              wrapAsDigadiag: true,
              outPath: 'unbrick_out/DIGADIAG_NOR_DUMPER.FRM',
            })
          }
        >
          {norStubLoading && <Loader2 className="w-3.5 h-3.5 animate-spin mr-1.5" />}
          <span>{t('unbrick.norStub.buildBtn')}</span>
        </Button>
        {norStubResult && (
          <span className="font-mono text-[11px] text-cyan-300">
            {norStubResult.raw_stub_bytes}B stub -&gt; {norStubResult.output_bytes}B (
            {norStubResult.load_addr})
          </span>
        )}
      </div>
    </div>
  );
};
