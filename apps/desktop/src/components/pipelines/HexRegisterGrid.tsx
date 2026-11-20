import React from 'react';
import { StatusBadge, type StatusVariant } from '@/components/StatusBadge';
import { Button, EmptyState } from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import type { HardwareRegisterSnapshot } from '@/models';

export interface HexRegisterGridProps {
  readonly title?: string;
  readonly registers: readonly HardwareRegisterSnapshot[];
  readonly emptyMessage?: string;
  readonly onInspectRegister?: (reg: HardwareRegisterSnapshot) => void;
}

const SEVERITY_VARIANT: Readonly<Record<HardwareRegisterSnapshot['severity'], StatusVariant>> = {
  critical: 'error',
  warning: 'warning',
  info: 'info',
  ok: 'success',
};

export const HexRegisterGrid: React.FC<HexRegisterGridProps> = ({
  title,
  registers,
  emptyMessage,
  onInspectRegister,
}) => {
  const { t } = useTranslation();

  return (
    <div className="rounded-xl bg-slate-950 border border-slate-800 p-4 space-y-3 font-mono">
      {title && <h4 className="text-xs font-bold text-slate-200">{title}</h4>}

      {registers.length === 0 ? (
        <EmptyState compact title={emptyMessage ?? t('pipelines.primitives.registerGrid.empty')} />
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-left text-[11px] border-collapse">
            <thead>
              <tr className="border-b border-slate-800 text-slate-400">
                <th className="py-1.5 pr-3">{t('pipelines.primitives.registerGrid.colRegister')}</th>
                <th className="py-1.5 pr-3">{t('pipelines.primitives.registerGrid.colAddress')}</th>
                <th className="py-1.5 pr-3">{t('pipelines.primitives.registerGrid.colValue')}</th>
                <th className="py-1.5 pr-3">{t('pipelines.primitives.registerGrid.colStatus')}</th>
                <th className="py-1.5">{t('pipelines.primitives.registerGrid.colNotes')}</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-800/70">
              {registers.map(reg => (
                <tr key={reg.id} className="hover:bg-slate-900/60">
                  <td className="py-2 pr-3 font-bold text-slate-100">{reg.name}</td>
                  <td className="py-2 pr-3 text-cyan-300">{reg.addressHex}</td>
                  <td className="py-2 pr-3 text-emerald-300 font-bold">
                    {reg.valueHex} ({reg.widthBits}b)
                  </td>
                  <td className="py-2 pr-3">
                    <StatusBadge variant={SEVERITY_VARIANT[reg.severity]} label={reg.statusLabel} />
                  </td>
                  <td className="py-2 text-slate-400 font-sans">
                    <div className="flex flex-wrap items-center justify-between gap-2">
                      <span>
                        {reg.flagBreakdown ? `${reg.flagBreakdown} — ` : ''}
                        {reg.note ?? ''}
                      </span>
                      {onInspectRegister && (
                        <Button
                          type="button"
                          size="sm"
                          variant="ghost"
                          onClick={() => onInspectRegister(reg)}
                          className="h-6 px-2 text-[10px]"
                        >
                          {t('pipelines.primitives.registerGrid.inspectBtn')}
                        </Button>
                      )}
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
};
