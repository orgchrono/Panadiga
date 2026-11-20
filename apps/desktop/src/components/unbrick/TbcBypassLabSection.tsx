import React from 'react';
import { Cpu, Terminal } from 'lucide-react';
import { Alert, AlertDescription, Badge, Button, Input, Select, EmptyState } from '@/components/ui';
import {
  MAIN_TBC_BYPASS_SPECS,
  type MainBypassId,
  type TvStandardSlug,
} from '@/constants';
import { useTranslation } from '@/i18n/useTranslation';
import type {
  ApplyMainTbcBypassesResponse,
  MainPassthroughMode,
  MainPassthroughRspPlan,
  TbcApplianceMode,
} from '@/models';
import { isMainPassthroughMode, isTbcApplianceMode, isTvStandardSlug } from '@/utils/typeGuards';
import { SafetyConfirmGate } from '@/components/pipelines';

export interface TbcBypassLabSectionProps {
  readonly applianceMode: TbcApplianceMode;
  readonly setApplianceMode: (m: TbcApplianceMode) => void;
  readonly experimentalRamAck: boolean;
  readonly setExperimentalRamAck: (v: boolean) => void;
  readonly tvStandard: TvStandardSlug;
  readonly setTvStandard: (s: TvStandardSlug) => void;
  readonly passthroughMode: MainPassthroughMode;
  readonly setPassthroughMode: (m: MainPassthroughMode) => void;
  readonly enabledBypasses: readonly MainBypassId[];
  readonly toggleBypass: (id: MainBypassId) => void;
  readonly applyStandaloneTbcPreset: () => void;
  readonly inputMainPath: string;
  readonly setInputMainPath: (v: string) => void;
  readonly outputMainPath: string;
  readonly setOutputMainPath: (v: string) => void;
  readonly tbcConfirmToken: string;
  readonly setTbcConfirmToken: (v: string) => void;
  readonly expectedTbcToken: string;
  readonly tbcResult: ApplyMainTbcBypassesResponse | null;
  readonly tbcBusy: boolean;
  readonly tbcError: string | null;
  readonly runTbcBypasses: () => Promise<ApplyMainTbcBypassesResponse | null>;
  readonly rspPlan: MainPassthroughRspPlan | null;
  readonly fetchRspPlan: () => Promise<MainPassthroughRspPlan>;
}

export const TbcBypassLabSection: React.FC<TbcBypassLabSectionProps> = (props) => {
  const { t } = useTranslation();
  const cowOutMissing = Boolean(props.inputMainPath.trim()) && !props.outputMainPath.trim();

  return (
    <div className="rounded-xl bg-slate-950 border border-slate-800 p-4 space-y-4 font-mono">
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-slate-800 pb-2.5">
        <div className="flex items-center gap-2 text-xs font-bold text-slate-100">
          <Cpu className="w-4 h-4 text-cyan-400" />
          <span>{t('pipelines.tbcSd.bypassSectionTitle')}</span>
        </div>
        <Button type="button" size="sm" variant="outline" data-testid="tbc-standalone-preset-btn" onClick={props.applyStandaloneTbcPreset} className="text-[11px]">
          {t('pipelines.tbcSd.standalonePresetBtn')}
        </Button>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-3 text-xs">
        <div className="space-y-1">
          <label htmlFor="tbc-appliance-mode" className="text-slate-400 block">{t('pipelines.tbcSd.applianceModeLabel')}</label>
          <Select id="tbc-appliance-mode" data-testid="tbc-appliance-mode" value={props.applianceMode} onChange={e => { if (isTbcApplianceMode(e.target.value)) props.setApplianceMode(e.target.value); }}>
            <option value="stage15_digadiag_tbc">{t('pipelines.tbcSd.modeStage15')}</option>
            <option value="experimental_sd_ram_chainload">{t('pipelines.tbcSd.modeRamChainload')}</option>
          </Select>
        </div>
        <div className="space-y-1">
          <label htmlFor="tbc-tv-standard" className="text-slate-400 block">{t('pipelines.tbcSd.tvStandardLabel')}</label>
          <Select id="tbc-tv-standard" data-testid="tbc-tv-standard" value={props.tvStandard} onChange={e => { if (isTvStandardSlug(e.target.value)) props.setTvStandard(e.target.value); }}>
            <option value="ntsc">{t('pipelines.calibration.standards.ntsc')}</option>
            <option value="pal_m">{t('pipelines.calibration.standards.pal_m')}</option>
            <option value="pal">{t('pipelines.calibration.standards.pal')}</option>
            <option value="secam">{t('pipelines.calibration.standards.secam')}</option>
          </Select>
        </div>
        <div className="space-y-1">
          <label htmlFor="tbc-passthrough-mode" className="text-slate-400 block">{t('pipelines.tbcSd.passthroughModeLabel')}</label>
          <Select id="tbc-passthrough-mode" data-testid="tbc-passthrough-mode" value={props.passthroughMode} onChange={e => { if (isMainPassthroughMode(e.target.value)) props.setPassthroughMode(e.target.value); }}>
            <option value="av_passthrough">{t('pipelines.tbcSd.modeAv')}</option>
            <option value="factory_original">{t('pipelines.tbcSd.modeFactory')}</option>
          </Select>
        </div>
      </div>

      {props.applianceMode === 'experimental_sd_ram_chainload' && (
        <label className="flex items-center gap-2 text-xs text-amber-300 bg-amber-950/20 border border-amber-500/40 rounded-lg p-2.5 cursor-pointer">
          <input type="checkbox" data-testid="tbc-experimental-ack" checked={props.experimentalRamAck} onChange={e => props.setExperimentalRamAck(e.target.checked)} />
          <span>{t('pipelines.tbcSd.ramChainloadAck')}</span>
        </label>
      )}

      <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
        {MAIN_TBC_BYPASS_SPECS.length === 0 ? (
          <EmptyState compact title={t('pipelines.tbcSd.bypassEmpty')} />
        ) : (
          MAIN_TBC_BYPASS_SPECS.map(spec => {
            const active = props.enabledBypasses.includes(spec.id);
            return (
              <label key={spec.id} className={`flex items-start gap-2.5 p-2.5 rounded-lg border text-xs cursor-pointer ${active ? 'bg-cyan-950/20 border-cyan-500/40' : 'bg-bench-900 border-slate-800'}`}>
                <input
                  type="checkbox"
                  data-testid={`tbc-bypass-${spec.id}`}
                  aria-label={t('pipelines.tbcSd.toggleBypassAria', { id: spec.id })}
                  checked={active}
                  onChange={() => props.toggleBypass(spec.id)}
                  className="mt-0.5"
                />
                <div className="space-y-0.5">
                  <div className="flex items-center gap-1.5 flex-wrap">
                    <span className="font-bold text-slate-100">{t(spec.labelKey)}</span>
                    <Badge variant={active ? 'success' : 'secondary'} className="text-[10px]">{spec.vaHex}</Badge>
                  </div>
                  <p className="text-[11px] text-slate-400 font-sans">{t(spec.descKey)}</p>
                  <p className="text-[10px] text-cyan-300">{t('pipelines.tbcSd.vaOffsetBytes', { va: spec.vaHex, offset: spec.offsetHex, orig: spec.origHex, patched: spec.patchedHex })}</p>
                </div>
              </label>
            );
          })
        )}
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-3 text-xs">
        <div className="space-y-1">
          <label htmlFor="tbc-input-main" className="text-slate-400 block">{t('pipelines.tbcSd.inputMainLabel')}</label>
          <Input id="tbc-input-main" type="text" value={props.inputMainPath} onChange={e => props.setInputMainPath(e.target.value)} />
        </div>
        <div className="space-y-1">
          <label htmlFor="tbc-output-main" className="text-slate-400 block">{t('pipelines.tbcSd.outputMainLabel')}</label>
          <Input id="tbc-output-main" type="text" value={props.outputMainPath} aria-invalid={cowOutMissing} onChange={e => props.setOutputMainPath(e.target.value)} />
          {cowOutMissing && <span className="text-[11px] text-rose-400">{t('pipelines.tbcSd.cowOutRequired')}</span>}
        </div>
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <Button type="button" size="sm" variant="outline" data-testid="tbc-rsp-plan-btn" onClick={() => void props.fetchRspPlan()} className="gap-1.5 text-xs">
          <Terminal className="w-3.5 h-3.5" />
          <span>{t('pipelines.tbcSd.rspPlanBtn')}</span>
        </Button>
      </div>

      {props.rspPlan && (
        <div className="rounded-xl bg-bench-900 border border-slate-800 p-3 space-y-1.5 text-[11px] font-mono">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <span className="font-bold text-slate-200">{t('unbrick.rsp.title')}</span>
            <Badge variant="outline">
              MAGIC {props.rspPlan.handshake_hex} • cb@{props.rspPlan.serial_callback_va}
            </Badge>
          </div>
          {props.rspPlan.pico_cli_shortcuts.length > 0 && (
            <div className="text-emerald-300 space-y-0.5">
              {props.rspPlan.pico_cli_shortcuts.map(cmd => (
                <div key={cmd}>&gt; {cmd}</div>
              ))}
            </div>
          )}
        </div>
      )}

      {props.tbcError && (
        <Alert variant="destructive"><AlertDescription>{props.tbcError}</AlertDescription></Alert>
      )}

      <SafetyConfirmGate
        expectedToken={props.expectedTbcToken}
        typedValue={props.tbcConfirmToken}
        onChange={props.setTbcConfirmToken}
        onConfirm={() => void props.runTbcBypasses()}
        isBusy={props.tbcBusy}
        preflightReady={!cowOutMissing}
        actionLabel={t('pipelines.tbcSd.applyTbcBtn')}
        inputTestId="tbc-confirm-input"
        submitTestId="tbc-apply-btn"
      />

      {props.tbcResult?.mainPatchReport && props.tbcResult.cowOutputPath && (
        <Alert variant={props.tbcResult.appliancePlan.nor_sa0_to_sa3_untouched ? 'success' : 'warning'}>
          <AlertDescription className="space-y-1 text-xs font-mono">
            <div className="font-bold">{props.tbcResult.cowOutputPath}</div>
            <div>
              mode={props.tbcResult.mainPatchReport.applied_mode ?? props.tbcResult.mainPatchReport.detected_mode} •{' '}
              {props.tbcResult.mainPatchReport.file_size} B • sites={props.tbcResult.mainPatchReport.sites.length}
            </div>
          </AlertDescription>
        </Alert>
      )}
    </div>
  );
};
