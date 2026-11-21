import React from 'react';
import { ShieldAlert, ShieldCheck, Zap } from 'lucide-react';
import {
  Button,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui';
import { useSafetyAckDialogState } from '@/hooks/useSafetyAckDialogState';
import type { TranslationKey } from '@/i18n/translations';
import { useTranslation } from '@/i18n/useTranslation';

export interface PicoConnectionSafetySheetProps {
  readonly open: boolean;
  readonly onCancel: () => void;
  readonly onConfirm: () => void;
}

const SAFETY_RULE_ITEMS: ReadonlyArray<{
  readonly id: string;
  readonly titleKey: TranslationKey;
  readonly descKey: TranslationKey;
}> = [
  { id: '3v3', titleKey: 'express.safetySheet.rule3v3Title', descKey: 'express.safetySheet.rule3v3Desc' },
  { id: 'nsw5v', titleKey: 'express.safetySheet.ruleNsw5vTitle', descKey: 'express.safetySheet.ruleNsw5vDesc' },
  { id: 'xreset', titleKey: 'express.safetySheet.ruleXresetTitle', descKey: 'express.safetySheet.ruleXresetDesc' },
  { id: 'backpower', titleKey: 'express.safetySheet.ruleBackpowerTitle', descKey: 'express.safetySheet.ruleBackpowerDesc' },
];

export const PicoConnectionSafetySheet: React.FC<PicoConnectionSafetySheetProps> = ({
  open,
  onCancel,
  onConfirm,
}) => {
  const { t } = useTranslation();
  const { checkboxId, checked, setChecked, handleOpenChange, handleConfirm } =
    useSafetyAckDialogState(onConfirm, onCancel);

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent
        data-testid="pico-connection-safety-sheet"
        className="max-w-xl bg-bench-950 border-amber-500/40"
      >
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2 text-base text-amber-300">
            <Zap className="w-5 h-5 text-amber-400 shrink-0" />
            <span>{t('express.safetySheet.title')}</span>
          </DialogTitle>
          <DialogDescription>{t('express.safetySheet.subtitle')}</DialogDescription>
        </DialogHeader>

        <div className="space-y-2.5 text-xs font-sans">
          {SAFETY_RULE_ITEMS.length === 0
            ? null
            : SAFETY_RULE_ITEMS.map(rule => (
                <div
                  key={rule.id}
                  className="rounded-xl border border-rose-500/30 bg-rose-950/15 p-3 space-y-0.5"
                >
                  <div className="font-mono font-bold text-rose-200 flex items-center gap-1.5">
                    <ShieldAlert className="w-3.5 h-3.5 text-rose-400 shrink-0" />
                    <span>{t(rule.titleKey)}</span>
                  </div>
                  <p className="text-slate-300 leading-relaxed">{t(rule.descKey)}</p>
                </div>
              ))}

          <label
            htmlFor={checkboxId}
            className="flex items-start gap-2.5 rounded-xl border border-slate-800 bg-slate-900/70 p-3 cursor-pointer select-none text-slate-200"
          >
            <input
              id={checkboxId}
              type="checkbox"
              data-testid="pico-safety-sheet-checkbox"
              checked={checked}
              onChange={e => setChecked(e.target.checked)}
              className="mt-0.5 h-4 w-4 rounded border-slate-700 bg-slate-950 accent-emerald-500 shrink-0"
            />
            <span className="leading-relaxed">{t('express.safetySheet.checkboxLabel')}</span>
          </label>
        </div>

        <DialogFooter className="gap-2">
          <Button type="button" variant="outline" onClick={onCancel}>
            {t('express.safetySheet.cancelBtn')}
          </Button>
          <Button
            type="button"
            variant="emerald"
            data-testid="pico-safety-sheet-confirm-btn"
            disabled={!checked}
            onClick={handleConfirm}
            className="gap-1.5"
          >
            <ShieldCheck className="w-4 h-4" />
            <span>{t('express.safetySheet.confirmBtn')}</span>
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};
