import React, { useId, useState } from 'react';
import {
  CheckCircle2,
  Layers,
  ShieldAlert,
  ShieldCheck,
} from 'lucide-react';
import type { TranslationKey } from '@/i18n/translations';
import { useTranslation } from '@/i18n/useTranslation';
import { Button } from './button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from './dialog';

export interface ComplianceDialogProps {
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
  readonly onAccept: () => void;
}

const FLOW_STEP_KEYS: readonly TranslationKey[] = [
  'shell.compliance.flowStep1',
  'shell.compliance.flowStep2',
  'shell.compliance.flowStep3',
  'shell.compliance.flowStep4',
];

const INVARIANT_KEYS: readonly TranslationKey[] = [
  'shell.compliance.invVoltage',
  'shell.compliance.invXreset',
  'shell.compliance.invBackpower',
  'shell.compliance.invNorHdd',
];

export const ComplianceDialog: React.FC<ComplianceDialogProps> = ({
  open,
  onOpenChange,
  onAccept,
}) => {
  const { t } = useTranslation();
  const checkboxId = useId();
  const [checked, setChecked] = useState(false);

  const handleOpenChange = (nextOpen: boolean) => {
    if (!nextOpen) {
      setChecked(false);
    }
    onOpenChange(nextOpen);
  };

  const handleAccept = () => {
    setChecked(false);
    onAccept();
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent
        data-testid="compliance-dialog"
        className="max-w-2xl max-h-[90vh] overflow-y-auto bg-bench-950 border-slate-800"
      >
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2 text-base text-slate-100">
            <ShieldCheck className="w-5 h-5 text-emerald-400 shrink-0" />
            <span>{t('shell.compliance.title')}</span>
          </DialogTitle>
          <DialogDescription>{t('shell.compliance.subtitle')}</DialogDescription>
        </DialogHeader>

        <div className="space-y-4 text-xs font-sans">
          <section className="rounded-xl border border-slate-800 bg-bench-900/60 p-3.5 space-y-2">
            <h3 className="text-xs font-mono font-bold text-cyan-300 flex items-center gap-1.5">
              <CheckCircle2 className="w-4 h-4" />
              <span>{t('shell.compliance.howItWorksTitle')}</span>
            </h3>
            <ul className="space-y-1.5 text-slate-300 leading-relaxed">
              {FLOW_STEP_KEYS.length === 0 ? null : FLOW_STEP_KEYS.map((key) => (
                <li key={key}>{t(key)}</li>
              ))}
            </ul>
          </section>

          <section className="rounded-xl border border-slate-800 bg-bench-900/40 p-3.5 space-y-1.5">
            <h3 className="text-xs font-mono font-bold text-slate-200 flex items-center gap-1.5">
              <Layers className="w-4 h-4 text-cyan-400" />
              <span>{t('shell.compliance.docksGuideTitle')}</span>
            </h3>
            <p className="text-slate-400 leading-relaxed">
              {t('shell.compliance.docksGuideDesc')}
            </p>
          </section>

          <section className="rounded-xl border border-rose-500/40 bg-rose-950/20 p-3.5 space-y-2">
            <h3 className="text-xs font-mono font-bold text-rose-300 flex items-center gap-1.5">
              <ShieldAlert className="w-4 h-4 text-rose-400" />
              <span>{t('shell.compliance.invariantsTitle')}</span>
            </h3>
            <ul className="space-y-1.5 text-rose-100/90 leading-relaxed list-disc pl-4">
              {INVARIANT_KEYS.length === 0 ? null : INVARIANT_KEYS.map((key) => (
                <li key={key}>{t(key)}</li>
              ))}
            </ul>
          </section>

          <div className="rounded-xl border border-slate-800 bg-slate-950 p-3 space-y-2">
            <label
              htmlFor={checkboxId}
              className="flex items-start gap-2.5 cursor-pointer select-none text-slate-200"
            >
              <input
                id={checkboxId}
                type="checkbox"
                data-testid="compliance-ack-checkbox"
                checked={checked}
                onChange={(event) => setChecked(event.target.checked)}
                className="mt-0.5 h-4 w-4 rounded border-slate-700 bg-slate-900 accent-cyan-500 shrink-0"
              />
              <span className="leading-relaxed">{t('shell.compliance.checkboxLabel')}</span>
            </label>
            <p className="text-[11px] text-slate-500 font-mono pl-6">
              {t('shell.compliance.reopenHint')}
            </p>
          </div>
        </div>

        <DialogFooter>
          <Button
            type="button"
            data-testid="compliance-accept-btn"
            disabled={!checked}
            onClick={handleAccept}
            className="w-full sm:w-auto gap-1.5"
          >
            <ShieldCheck className="w-4 h-4" />
            <span>{t('shell.compliance.acceptBtn')}</span>
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};
