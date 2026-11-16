import React, { useState } from 'react';
import { FileCheck2, Loader2, Terminal } from 'lucide-react';
import { Button, Input } from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import { MatshitaErrorBanner } from './MatshitaErrorBanner';
import { MATSHITA_SELECT_CLS, type MatshitaSerialFrmProps } from './matshitaTypes';

export const MatshitaSerialFrmSection: React.FC<MatshitaSerialFrmProps> = ({
  serialState,
  frmVerifyState,
  frmPackState,
  onRunSerial,
  onVerifyFrm,
  onPackFrm,
}) => {
  const { t } = useTranslation();
  const [serialCmd, setSerialCmd] = useState('WW 0x51800000 0xDEADBEEF');
  const [unlockPass, setUnlockPass] = useState(true);
  const [containerKind, setContainerKind] = useState('sd');
  const [frmPath, setFrmPath] = useState('sd_card_eh55/DIGADIAG.FRM');
  const [outPath, setOutPath] = useState('');

  const frmSummary = frmPackState.data?.verification ?? frmVerifyState.data;

  return (
    <>
      <section className="space-y-3 rounded-xl border border-slate-800 bg-slate-950/60 p-4">
        <div className="flex items-center gap-2 text-xs font-mono font-semibold text-slate-200">
          <Terminal className="w-3.5 h-3.5 text-cyan-400" />
          <span>{t('unbrick.matshita.serialTitle')}</span>
        </div>
        <div className="space-y-1">
          <label htmlFor="matshita-serial-cmd" className="text-[11px] font-mono text-slate-400">
            {t('unbrick.matshita.serialCmdLabel')}
          </label>
          <Input
            id="matshita-serial-cmd"
            value={serialCmd}
            onChange={e => setSerialCmd(e.target.value)}
            placeholder="WW 0x51800000 0xDEADBEEF"
          />
        </div>
        <label className="flex items-center gap-2 text-xs font-mono text-slate-300 cursor-pointer">
          <input
            id="matshita-unlock-pass"
            type="checkbox"
            checked={unlockPass}
            onChange={e => setUnlockPass(e.target.checked)}
            className="rounded border-slate-700 bg-slate-900"
          />
          <span>{t('unbrick.matshita.unlockPassToggle')}</span>
        </label>
        <Button
          type="button"
          size="sm"
          variant="outline"
          disabled={serialState.loading}
          onClick={() => void onRunSerial(serialCmd, unlockPass)}
          data-testid="matshita-serial-btn"
        >
          {serialState.loading && <Loader2 className="w-3.5 h-3.5 mr-1.5 animate-spin" />}
          {t('unbrick.matshita.sendSerialBtn')}
        </Button>
        <MatshitaErrorBanner message={serialState.error} />
        {serialState.data && (
          <pre className="rounded-lg border border-slate-800 bg-slate-900/70 p-2.5 text-[11px] font-mono text-slate-200 overflow-x-auto">
            {serialState.data.output_lines.join('\n')}
          </pre>
        )}
      </section>

      <section className="space-y-3 rounded-xl border border-slate-800 bg-slate-950/60 p-4">
        <div className="flex items-center gap-2 text-xs font-mono font-semibold text-slate-200">
          <FileCheck2 className="w-3.5 h-3.5 text-emerald-400" />
          <span>{t('unbrick.matshita.frmTitle')}</span>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
          <div className="space-y-1">
            <label htmlFor="matshita-frm-kind" className="text-[11px] font-mono text-slate-400">
              {t('unbrick.matshita.containerKindLabel')}
            </label>
            <select
              id="matshita-frm-kind"
              className={MATSHITA_SELECT_CLS}
              value={containerKind}
              onChange={e => setContainerKind(e.target.value)}
            >
              <option value="sd">sd (EXTRHEAD 128K)</option>
              <option value="sd48">sd48 (PANA__SD)</option>
              <option value="cd">cd (PANA_DVD.FRM)</option>
              <option value="dvd">dvd (MATSHITA)</option>
            </select>
          </div>
          <div className="space-y-1">
            <label htmlFor="matshita-frm-out" className="text-[11px] font-mono text-slate-400">
              {t('unbrick.matshita.outPathLabel')}
            </label>
            <Input
              id="matshita-frm-out"
              value={outPath}
              onChange={e => setOutPath(e.target.value)}
              placeholder="out/UNLOCK.FRM"
            />
          </div>
        </div>
        <div className="space-y-1">
          <label htmlFor="matshita-frm-path" className="text-[11px] font-mono text-slate-400">
            {t('unbrick.matshita.frmPathLabel')}
          </label>
          <Input
            id="matshita-frm-path"
            value={frmPath}
            onChange={e => setFrmPath(e.target.value)}
            placeholder="sd_card_eh55/DIGADIAG.FRM"
          />
        </div>
        <div className="flex flex-wrap gap-2">
          <Button
            type="button"
            size="sm"
            variant="outline"
            disabled={frmVerifyState.loading}
            onClick={() => void onVerifyFrm(containerKind, frmPath)}
            data-testid="matshita-verify-frm-btn"
          >
            {frmVerifyState.loading && <Loader2 className="w-3.5 h-3.5 mr-1.5 animate-spin" />}
            {t('unbrick.matshita.verifyFrmBtn')}
          </Button>
          <Button
            type="button"
            size="sm"
            disabled={frmPackState.loading}
            onClick={() => void onPackFrm({ containerKind, outPath })}
            data-testid="matshita-pack-frm-btn"
          >
            {frmPackState.loading && <Loader2 className="w-3.5 h-3.5 mr-1.5 animate-spin" />}
            {t('unbrick.matshita.packFrmBtn')}
          </Button>
        </div>
        <MatshitaErrorBanner message={frmVerifyState.error ?? frmPackState.error} />
        {frmSummary && (
          <div className="rounded-lg border border-slate-800 bg-slate-900/70 p-2.5 text-[11px] font-mono text-slate-300 space-y-0.5">
            <div className="text-emerald-300">
              {frmSummary.container_kind} | {frmSummary.container_bytes} B | JMP{' '}
              {frmSummary.bootloader_jump_va_hex}
            </div>
            <div className="text-slate-400 break-all">SHA256: {frmSummary.sha256}</div>
          </div>
        )}
      </section>
    </>
  );
};
