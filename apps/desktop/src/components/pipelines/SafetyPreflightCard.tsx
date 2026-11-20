import React from 'react';
import { AlertTriangle, Info, Lock, ShieldAlert } from 'lucide-react';
import { Badge, EmptyState } from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import type { PreflightCheckItem, SafetySeverity } from '@/models';
import { cn } from '@/lib/utils';

export interface SafetyPreflightCardProps {
  readonly checks: readonly PreflightCheckItem[];
  readonly checkedIds: readonly string[];
  readonly onToggleCheck: (id: string) => void;
  readonly severity?: SafetySeverity;
  readonly title?: string;
}

const CONTAINER_STYLE: Readonly<Record<SafetySeverity, string>> = {
  critical: 'bg-rose-950/20 border-rose-500/40',
  warning: 'bg-amber-950/20 border-amber-500/40',
  info: 'bg-cyan-950/20 border-cyan-500/40',
};

const SEVERITY_BADGE: Readonly<Record<SafetySeverity, 'destructive' | 'warning' | 'secondary'>> = {
  critical: 'destructive',
  warning: 'warning',
  info: 'secondary',
};

export const SafetyPreflightCard: React.FC<SafetyPreflightCardProps> = ({
  checks,
  checkedIds,
  onToggleCheck,
  severity = 'warning',
  title,
}) => {
  const { t } = useTranslation();
  const checkedCount = checks.filter(c => checkedIds.includes(c.id) && !c.lockedReason).length;
  const allChecked = checks.length > 0 && checkedCount === checks.length;

  return (
    <div className={cn('rounded-xl border p-4 space-y-3 font-mono', CONTAINER_STYLE[severity])}>
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex items-center gap-2 text-xs font-bold text-slate-100">
          {severity === 'critical' ? (
            <ShieldAlert className="w-4 h-4 text-rose-400 shrink-0" />
          ) : severity === 'warning' ? (
            <AlertTriangle className="w-4 h-4 text-amber-400 shrink-0" />
          ) : (
            <Info className="w-4 h-4 text-cyan-400 shrink-0" />
          )}
          <span>{title ?? t('pipelines.primitives.preflight.title')}</span>
        </div>
        <Badge variant={allChecked ? 'success' : SEVERITY_BADGE[severity]}>
          {t('pipelines.primitives.preflight.counter', {
            checked: checkedCount,
            total: checks.length,
          })}
        </Badge>
      </div>

      {checks.length === 0 ? (
        <EmptyState compact title={t('pipelines.primitives.preflight.empty')} />
      ) : (
        <div className="space-y-2">
          {checks.map(item => {
            const locked = Boolean(item.lockedReason);
            const checked = !locked && checkedIds.includes(item.id);
            const sevLabel =
              item.severity === 'critical'
                ? t('pipelines.primitives.preflight.severityCritical')
                : item.severity === 'warning'
                  ? t('pipelines.primitives.preflight.severityWarning')
                  : t('pipelines.primitives.preflight.severityInfo');

            return (
              <label
                key={item.id}
                className={cn(
                  'flex items-start gap-2.5 p-2.5 rounded-lg border text-xs transition-colors',
                  locked
                    ? 'bg-slate-950/90 border-rose-500/40 opacity-80 cursor-not-allowed'
                    : checked
                      ? 'bg-emerald-950/20 border-emerald-500/40 cursor-pointer'
                      : 'bg-slate-950/70 border-slate-800 hover:border-slate-700 cursor-pointer',
                )}
              >
                <input
                  type="checkbox"
                  data-testid={`preflight-check-${item.id}`}
                  checked={checked}
                  disabled={locked}
                  onChange={() => onToggleCheck(item.id)}
                  className="mt-0.5 rounded border-slate-700 bg-slate-900 text-emerald-500"
                />
                <div className="space-y-1 flex-1 min-w-0">
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="font-bold text-slate-100">{item.title}</span>
                    <Badge variant={SEVERITY_BADGE[item.severity]} className="text-[10px]">
                      {sevLabel}
                    </Badge>
                  </div>
                  <p className="text-[11px] text-slate-300 font-sans">{item.detail}</p>
                  {item.lockedReason && (
                    <div className="flex items-center gap-1.5 text-[11px] text-rose-300 font-bold">
                      <Lock className="w-3 h-3 shrink-0" />
                      <span>
                        {t('pipelines.primitives.preflight.lockedPrefix')} {item.lockedReason}
                      </span>
                    </div>
                  )}
                </div>
              </label>
            );
          })}
        </div>
      )}
    </div>
  );
};
