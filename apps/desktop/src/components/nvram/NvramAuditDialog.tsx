import React from 'react';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { useTranslation } from '@/i18n/useTranslation';
import { NvramCleanReport } from '@/models';

interface NvramAuditDialogProps {
  isOpen: boolean;
  onClose: () => void;
  report: NvramCleanReport | null;
}

export const NvramAuditDialog: React.FC<NvramAuditDialogProps> = ({
  isOpen,
  onClose,
  report,
}) => {
  const { t } = useTranslation();

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="max-w-2xl font-mono">
        <DialogHeader>
          <DialogTitle>{t('nvram.auditDialog.title')}</DialogTitle>
          <DialogDescription>
            {t('nvram.auditDialog.desc')}
          </DialogDescription>
        </DialogHeader>

        {report && (
          <div className="space-y-4 text-xs">
            <div className="p-3.5 rounded-xl bg-slate-950 border border-slate-800 space-y-2">
              <span className="text-[10px] text-slate-500 uppercase block font-semibold">
                {t('nvram.auditDialog.corruptedTitle')}
              </span>
              <div className="space-y-1">
                <div className="flex justify-between text-slate-400">
                  <span>{t('nvram.auditDialog.fileLabel')}</span>
                  <span className="text-slate-200 truncate max-w-sm">{report.input_file}</span>
                </div>
                <div className="flex justify-between text-slate-400">
                  <span>{t('nvram.auditDialog.sha256Label')}</span>
                  <span className="text-amber-400 truncate max-w-sm">{report.original_sha256}</span>
                </div>
                <div className="flex justify-between text-slate-400">
                  <span>{t('nvram.auditDialog.md5Label')}</span>
                  <span className="text-slate-300">{report.original_md5}</span>
                </div>
              </div>
            </div>

            <div className="p-3.5 rounded-xl bg-emerald-950/20 border border-emerald-500/30 space-y-2">
              <span className="text-[10px] text-emerald-400 uppercase block font-semibold">
                {t('nvram.auditDialog.cleanTitle')}
              </span>
              <div className="space-y-1">
                <div className="flex justify-between text-slate-400">
                  <span>{t('nvram.auditDialog.fileLabel')}</span>
                  <span className="text-slate-200 truncate max-w-sm">{report.output_file}</span>
                </div>
                <div className="flex justify-between text-slate-400">
                  <span>{t('nvram.auditDialog.sha256Label')}</span>
                  <span className="text-emerald-400 truncate max-w-sm">{report.clean_sha256}</span>
                </div>
                <div className="flex justify-between text-slate-400">
                  <span>{t('nvram.auditDialog.md5Label')}</span>
                  <span className="text-slate-300">{report.clean_md5}</span>
                </div>
              </div>
            </div>

            <div className="p-3.5 rounded-xl bg-slate-950 border border-slate-800 space-y-2">
              <span className="text-[10px] text-slate-500 uppercase block font-semibold">
                {t('nvram.auditDialog.backupTitle')}
              </span>
              <div className="flex justify-between text-slate-400">
                <span>{t('nvram.auditDialog.backupLocalLabel')}</span>
                <span className="text-slate-200">{report.backup_file}</span>
              </div>
              <div className="flex justify-between text-slate-400">
                <span>{t('nvram.auditDialog.sizeLabel')}</span>
                <span className="text-slate-200">{t('nvram.auditDialog.sizeValue')}</span>
              </div>
              <div className="flex justify-between text-slate-400">
                <span>{t('nvram.auditDialog.bootloaderLabel')}</span>
                <span className="text-emerald-400 font-bold">{t('nvram.auditDialog.bootloaderOk')}</span>
              </div>
            </div>
          </div>
        )}

        <DialogFooter>
          <Button variant="secondary" onClick={onClose}>
            {t('nvram.auditDialog.closeBtn')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};
