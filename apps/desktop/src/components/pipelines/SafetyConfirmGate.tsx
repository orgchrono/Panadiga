import React, { useState } from 'react';
import { Loader2, ShieldCheck } from 'lucide-react';
import { Alert, AlertDescription, Button, Input } from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';

export interface SafetyConfirmGateProps {
  readonly expectedToken: string;
  readonly typedValue: string;
  readonly onChange: (value: string) => void;
  readonly onConfirm: () => void;
  readonly isBusy: boolean;
  readonly successMessage?: string | null;
  readonly preflightReady?: boolean;
  readonly actionLabel?: string;
  readonly title?: string;
  readonly description?: string;
  readonly inputTestId?: string;
  readonly submitTestId?: string;
}

export const SafetyConfirmGate: React.FC<SafetyConfirmGateProps> = ({
  expectedToken,
  typedValue,
  onChange,
  onConfirm,
  isBusy,
  successMessage = null,
  preflightReady = true,
  actionLabel,
  title,
  description,
  inputTestId,
  submitTestId,
}) => {
  const { t } = useTranslation();
  const [success, setSuccess] = useState<string | null>(null);
  const trimmed = typedValue.trim();
  const tokenMissing = expectedToken.trim().length === 0;
  const isMismatch = !tokenMissing && trimmed.length > 0 && trimmed !== expectedToken;
  const isTokenValid = !tokenMissing && trimmed === expectedToken;
  const canSubmit = !isBusy && preflightReady && isTokenValid;
  const validationMessage =
    trimmed.length === 0
      ? t('pipelines.primitives.confirmGate.requiredError')
      : isMismatch
        ? t('pipelines.primitives.confirmGate.mismatchError', { token: expectedToken })
        : null;

  const submitGate = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (canSubmit) {
      setSuccess(actionLabel ?? t('pipelines.primitives.confirmGate.confirmBtn'));
      onConfirm();
    } else {
      setSuccess(null);
    }
  };

  return (
    <form
      onSubmit={submitGate}
      noValidate
      className="rounded-xl bg-slate-950 border border-slate-800 p-4 space-y-3 font-mono"
    >
      <div className="space-y-1">
        <div className="flex items-center gap-2 text-xs font-bold text-amber-300">
          <ShieldCheck className="w-4 h-4 shrink-0" />
          <span>{title ?? t('pipelines.primitives.confirmGate.title')}</span>
        </div>
        {description && <p className="text-[11px] text-slate-400 font-sans">{description}</p>}
      </div>

      <div className="space-y-1.5 text-xs">
        <div className="flex flex-wrap items-center gap-2 text-[11px] text-slate-400">
          <label htmlFor={`gate-input-${expectedToken}`}>
            {t('pipelines.primitives.confirmGate.expectedLabel')}
          </label>
          <code className="px-2 py-0.5 rounded bg-bench-900 border border-amber-500/40 text-amber-300 font-bold">
            {expectedToken}
          </code>
        </div>
        <Input
          id={`gate-input-${expectedToken}`}
          data-testid={inputTestId}
          type="text"
          value={typedValue}
          aria-label={t('pipelines.primitives.confirmGate.inputLabel')}
          aria-invalid={Boolean(validationMessage)}
          placeholder={t('pipelines.primitives.confirmGate.inputPlaceholder', { token: expectedToken })}
          onChange={e => {
            setSuccess(null);
            onChange(e.target.value);
          }}
        />
        {validationMessage && (
          <output className="block text-[11px] text-amber-300">
            {validationMessage}
          </output>
        )}
      </div>

      <Button
        type="submit"
        variant="emerald"
        size="sm"
        data-testid={submitTestId}
        disabled={!canSubmit}
        className="gap-1.5"
      >
        {isBusy ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <ShieldCheck className="w-3.5 h-3.5" />}
        <span>
          {isBusy
            ? t('pipelines.primitives.confirmGate.busyBtn')
            : actionLabel ?? t('pipelines.primitives.confirmGate.confirmBtn')}
        </span>
      </Button>
      {(successMessage ?? (!isBusy ? success : null)) && (
        <Alert variant="success">
          <AlertDescription>{successMessage ?? success}</AlertDescription>
        </Alert>
      )}
    </form>
  );
};
