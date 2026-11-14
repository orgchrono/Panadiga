import React, { useEffect, useState } from 'react';
import { Cpu, Loader2, Radio, Terminal, Wrench } from 'lucide-react';
import type {
  MainPassthroughReport,
  MainPassthroughRspPlan,
  NorDumperStubResult,
} from '@/models';
import {
  Alert,
  AlertDescription,
  AlertTitle,
  Badge,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  EmptyState,
  Input,
  Select,
} from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';

interface TbcPassthroughCardProps {
  discoveredMainPath: string | null;
  tbcLoading: boolean;
  tbcReport: MainPassthroughReport | null;
  tbcError: string | null;
  rspLoading: boolean;
  rspPlan: MainPassthroughRspPlan | null;
  norStubLoading: boolean;
  norStubResult: NorDumperStubResult | null;
  onRunTbc: (params: {
    action: 'inspect' | 'patch';
    inputPath: string;
    outPath?: string;
    mode?: 'av_passthrough' | 'factory_original';
  }) => Promise<MainPassthroughReport | null>;
  onRefreshRspPlan: (callbackTargetVa?: string) => Promise<MainPassthroughRspPlan | null>;
  onBuildNorStub: (params: { wrapAsDigadiag: boolean; outPath?: string }) => Promise<NorDumperStubResult | null>;
}

export const TbcPassthroughCard: React.FC<TbcPassthroughCardProps> = ({
  discoveredMainPath,
  tbcLoading,
  tbcReport,
  tbcError,
  rspLoading,
  rspPlan,
  norStubLoading,
  norStubResult,
  onRunTbc,
  onRefreshRspPlan,
  onBuildNorStub,
}) => {
  const { t } = useTranslation();
  const [mainInput, setMainInput] = useState(
    discoveredMainPath ?? 'firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin'
  );
  const [mainOutput, setMainOutput] = useState('unbrick_out/MAIN3410PQ_TBC_PASSTHROUGH.bin');
  const [mode, setMode] = useState<'av_passthrough' | 'factory_original'>('av_passthrough');
  const [callbackVa, setCallbackVa] = useState('0x50401F3C');

  const [inErr, setInErr] = useState<string | null>(null);
  const [outErr, setOutErr] = useState<string | null>(null);

  useEffect(() => {
    if (discoveredMainPath) setMainInput(discoveredMainPath);
  }, [discoveredMainPath]);

  const handleInspectOrPatch = async (action: 'inspect' | 'patch') => {
    if (!mainInput.trim()) {
      setInErr(t('unbrick.errors.mainPathRequired'));
      return;
    }
    setInErr(null);

    if (action === 'patch' && !mainOutput.trim()) {
      setOutErr(t('unbrick.errors.mainOutRequired'));
      return;
    }
    setOutErr(null);

    await onRunTbc({
      action,
      inputPath: mainInput.trim(),
      outPath: action === 'patch' ? mainOutput.trim() : undefined,
      mode,
    });
  };

  const sites = tbcReport?.sites ?? [];
  const rspFrames = rspPlan?.passthrough_frames ?? [];
  const picoShortcuts = rspPlan?.pico_cli_shortcuts ?? [];

  return (
    <Card className="h-full flex flex-col">
      <CardHeader>
        <div className="flex items-start justify-between gap-2">
          <div>
            <CardTitle className="flex items-center gap-2">
              <Radio className="w-4 h-4 text-cyan-400" />
              <span>{t('unbrick.tbc.title')}</span>
            </CardTitle>
            <CardDescription className="mt-1">{t('unbrick.tbc.desc')}</CardDescription>
          </div>
          <Badge variant={tbcReport?.detected_mode === 'AV_PASSTHROUGH' ? 'success' : 'secondary'}>
            {tbcReport?.detected_mode ?? 'MAIN 3410PQ'}
          </Badge>
        </div>
      </CardHeader>

      <CardContent className="space-y-4 flex-1">
        <div className="space-y-2.5 text-xs">
          <div className="space-y-1">
            <label htmlFor="tbc-main-in" className="text-slate-400 block font-mono">
              {t('unbrick.tbc.inputPath')}
            </label>
            <Input
              id="tbc-main-in"
              value={mainInput}
              aria-invalid={Boolean(inErr)}
              onChange={e => {
                setMainInput(e.target.value);
                if (inErr) setInErr(null);
              }}
            />
            {inErr && <p className="text-[11px] text-rose-400 font-mono">{inErr}</p>}
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
            <div className="space-y-1">
              <label htmlFor="tbc-main-out" className="text-slate-400 block font-mono">
                {t('unbrick.tbc.outputPath')}
              </label>
              <Input
                id="tbc-main-out"
                value={mainOutput}
                aria-invalid={Boolean(outErr)}
                onChange={e => {
                  setOutOutputSafe(e.target.value);
                }}
              />
              {outErr && <p className="text-[11px] text-rose-400 font-mono">{outErr}</p>}
            </div>

            <div className="space-y-1">
              <label htmlFor="tbc-patch-mode" className="text-slate-400 block font-mono">
                {t('unbrick.tbc.modeLabel')}
              </label>
              <Select
                id="tbc-patch-mode"
                value={mode}
                onChange={e => setMode(e.target.value as typeof mode)}
              >
                <option value="av_passthrough">AV_PASSTHROUGH (Bypass FPC + ODD)</option>
                <option value="factory_original">FACTORY_ORIGINAL (Restore Stock)</option>
              </Select>
            </div>
          </div>

          <div className="flex flex-wrap gap-2">
            <Button
              type="button"
              size="sm"
              variant="outline"
              disabled={tbcLoading}
              onClick={() => void handleInspectOrPatch('inspect')}
            >
              {tbcLoading ? <Loader2 className="w-3.5 h-3.5 animate-spin mr-1.5" /> : <Cpu className="w-3.5 h-3.5 mr-1.5" />}
              <span>{t('unbrick.tbc.inspectBtn')}</span>
            </Button>
            <Button
              type="button"
              size="sm"
              variant="default"
              disabled={tbcLoading}
              onClick={() => void handleInspectOrPatch('patch')}
            >
              {tbcLoading ? <Loader2 className="w-3.5 h-3.5 animate-spin mr-1.5" /> : <Wrench className="w-3.5 h-3.5 mr-1.5" />}
              <span>{t('unbrick.tbc.patchBtn')}</span>
            </Button>
          </div>
        </div>

        {tbcError && (
          <Alert variant="destructive">
            <AlertDescription>{tbcError}</AlertDescription>
          </Alert>
        )}

        {tbcReport?.output_path && (
          <Alert variant="success">
            <AlertTitle>{t('unbrick.tbc.patchedSuccess')}</AlertTitle>
            <AlertDescription className="font-mono text-[11px] break-all">
              {tbcReport.output_path} ({tbcReport.applied_mode})
            </AlertDescription>
          </Alert>
        )}

        {sites.length === 0 ? (
          <EmptyState compact title={t('unbrick.tbc.sitesEmpty')} />
        ) : (
          <div className="space-y-1.5">
            {sites.map(site => (
              <div
                key={site.va_hex}
                className="rounded-xl border border-slate-800 bg-slate-950/70 p-2.5 text-[11px] font-mono space-y-1"
              >
                <div className="flex items-center justify-between gap-2">
                  <span className="text-cyan-300 font-bold">
                    {site.name} ({site.va_hex})
                  </span>
                  <Badge variant={site.state === 'AV_PASSTHROUGH' ? 'success' : 'secondary'}>
                    {site.state}
                  </Badge>
                </div>
                <div className="text-slate-400">{site.description}</div>
                <div className="text-slate-300 break-all">RSP: {site.rsp_passthrough_frame}</div>
              </div>
            ))}
          </div>
        )}

        <div className="border-t border-slate-800/80 pt-3 space-y-2.5 text-xs">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <span className="font-mono font-bold text-slate-200 flex items-center gap-1.5">
              <Terminal className="w-3.5 h-3.5 text-purple-400" />
              <span>{t('unbrick.rsp.title')}</span>
            </span>
            <Badge variant="outline">MAGIC {rspPlan?.handshake_hex ?? '0e020204'}</Badge>
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
      </CardContent>
    </Card>
  );

  function setOutOutputSafe(val: string) {
    setMainOutput(val);
    if (outErr) setOutErr(null);
  }
};
