import React, { useState } from 'react';
import { Play, ShieldAlert, Wrench } from 'lucide-react';
import { useTranslation } from '@/i18n/useTranslation';
import { withUniqueKeys } from '@/utils/formatters';
import {
  Alert,
  AlertDescription,
  AlertTitle,
  Badge,
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  EmptyState,
} from '@/components/ui';
import type { IrMacroRiskLevel, VerifiedIrMacro } from '@/models/ir';

interface IrServiceRecoveryTabProps {
  macros: VerifiedIrMacro[];
  isExecutingMacro: boolean;
  onExecuteMacro: (macro: VerifiedIrMacro) => void;
}

const RISK_BADGE_CLASSES: Record<IrMacroRiskLevel, string> = {
  SAFE: 'border-emerald-500/50 text-emerald-300 bg-emerald-500/10',
  MODERATE: 'border-amber-500/50 text-amber-300 bg-amber-500/10',
  DESTRUCTIVE: 'border-rose-500/60 text-rose-300 bg-rose-500/15',
};

export const IrServiceRecoveryTab: React.FC<IrServiceRecoveryTabProps> = ({
  macros,
  isExecutingMacro,
  onExecuteMacro,
}) => {
  const { t, locale } = useTranslation();
  const [pendingConfirm, setPendingConfirm] = useState<VerifiedIrMacro | null>(null);
  const isPt = locale === 'pt-BR';

  const handleRunClick = (macro: VerifiedIrMacro) => {
    if (macro.risk === 'SAFE') {
      onExecuteMacro(macro);
      return;
    }
    setPendingConfirm(macro);
  };

  return (
    <div className="space-y-4">
      <Alert variant="warning">
        <AlertTitle>{t('irCommander.service.guardTitle')}</AlertTitle>
        <AlertDescription className="text-xs">
          {t('irCommander.service.guardBody')}
        </AlertDescription>
      </Alert>

      {macros.length === 0 ? (
        <EmptyState
          compact
          title={t('irCommander.service.emptyTitle')}
          description={t('irCommander.service.emptyDesc')}
        />
      ) : (
        <div className="space-y-3">
          {macros.map(macro => {
            const title = isPt ? macro.title_pt : macro.title_en;
            const desc = isPt ? macro.description_pt : macro.description_en;
            return (
              <Card key={macro.id} data-testid={`ir-macro-card-${macro.id}`}>
                <CardHeader className="pb-2">
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <CardTitle className="text-sm flex items-center gap-2">
                      <Wrench className="w-4 h-4 text-cyan-400" />
                      <span>{title}</span>
                    </CardTitle>
                    <div className="flex items-center gap-1.5">
                      <Badge variant="outline" className="text-[10px] font-mono text-slate-300">
                        {macro.verification_source}
                      </Badge>
                      <Badge
                        variant="outline"
                        className={`text-[10px] font-mono ${RISK_BADGE_CLASSES[macro.risk]}`}
                      >
                        {macro.risk}
                      </Badge>
                    </div>
                  </div>
                </CardHeader>
                <CardContent className="space-y-3">
                  <p className="text-xs text-slate-300">{desc}</p>
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <div className="flex flex-wrap gap-1 font-mono text-[10px] text-slate-400">
                      {withUniqueKeys(
                        macro.steps,
                        (s) => `${macro.id}-${s.device}-${s.subdevice}-${s.obc}-${s.post_delay_ms}-${s.label}`,
                      ).map(({ item: s, key }, idx) => (
                        <Badge key={key} variant="outline" className="text-[10px]">
                          {idx + 1}. {s.label} ({s.device.toString(16).toUpperCase()}:
                          {s.subdevice.toString(16).toUpperCase()}:
                          {s.obc.toString(16).toUpperCase()} · +{s.post_delay_ms}ms)
                        </Badge>
                      ))}
                    </div>
                    <Button
                      type="button"
                      size="sm"
                      variant={macro.risk === 'DESTRUCTIVE' ? 'destructive' : 'default'}
                      disabled={isExecutingMacro}
                      onClick={() => handleRunClick(macro)}
                      data-testid={`ir-run-macro-${macro.id}`}
                      className="gap-1.5 shrink-0"
                    >
                      <Play className="w-3.5 h-3.5" />
                      {t('irCommander.service.runBtn')}
                    </Button>
                  </div>
                </CardContent>
              </Card>
            );
          })}
        </div>
      )}

      <Dialog open={pendingConfirm !== null} onOpenChange={open => !open && setPendingConfirm(null)}>
        <DialogContent className="max-w-md font-mono">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2 text-rose-300">
              <ShieldAlert className="w-5 h-5" />
              <span>{t('irCommander.service.confirmTitle')}</span>
            </DialogTitle>
            <DialogDescription>
              {pendingConfirm
                ? isPt
                  ? pendingConfirm.description_pt
                  : pendingConfirm.description_en
                : ''}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => setPendingConfirm(null)}
            >
              {t('common.cancel')}
            </Button>
            <Button
              type="button"
              variant="destructive"
              size="sm"
              onClick={() => {
                if (pendingConfirm) {
                  onExecuteMacro(pendingConfirm);
                }
                setPendingConfirm(null);
              }}
              data-testid="ir-confirm-destructive-macro-btn"
            >
              {t('irCommander.service.confirmRunBtn')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
};
