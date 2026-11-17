import React, { useState } from 'react';
import { useTranslation } from '@/i18n/useTranslation';
import {
  Button,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
} from '@/components/ui';

interface IrImportProfileDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onConfirmImport: (format: 'irplus' | 'flipper', content: string, name: string) => void;
}

export const IrImportProfileDialog: React.FC<IrImportProfileDialogProps> = ({
  open,
  onOpenChange,
  onConfirmImport,
}) => {
  const { t } = useTranslation();
  const [importFormat, setImportFormat] = useState<'flipper' | 'irplus'>('flipper');
  const [importName, setImportName] = useState('Imported Panasonic Profile');
  const [importBody, setImportBody] = useState('');

  const handleConfirm = () => {
    if (!importBody.trim()) return;
    onConfirmImport(importFormat, importBody, importName.trim() || 'Custom Profile');
    onOpenChange(false);
    setImportBody('');
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg font-mono">
        <DialogHeader>
          <DialogTitle>{t('irCommander.catalog.importDialogTitle')}</DialogTitle>
          <DialogDescription>{t('irCommander.catalog.importDialogDesc')}</DialogDescription>
        </DialogHeader>
        <div className="space-y-3 text-xs">
          <div className="flex gap-2">
            <Button
              type="button"
              size="sm"
              variant={importFormat === 'flipper' ? 'default' : 'outline'}
              onClick={() => setImportFormat('flipper')}
            >
              Flipper (.ir)
            </Button>
            <Button
              type="button"
              size="sm"
              variant={importFormat === 'irplus' ? 'default' : 'outline'}
              onClick={() => setImportFormat('irplus')}
            >
              irplus (.irplus XML)
            </Button>
          </div>
          <Input
            value={importName}
            onChange={e => setImportName(e.target.value)}
            placeholder="Profile Name"
          />
          <textarea
            rows={8}
            value={importBody}
            onChange={e => setImportBody(e.target.value)}
            data-testid="ir-import-textarea"
            placeholder="Paste Filetype: IR signals file or <irplus> XML here..."
            className="w-full rounded-lg bg-bench-950 border border-slate-700 p-2.5 text-xs text-slate-100 font-mono"
          />
        </div>
        <DialogFooter>
          <Button type="button" variant="outline" size="sm" onClick={() => onOpenChange(false)}>
            {t('common.cancel')}
          </Button>
          <Button
            type="button"
            size="sm"
            disabled={!importBody.trim()}
            onClick={handleConfirm}
            data-testid="ir-import-confirm-btn"
          >
            {t('irCommander.catalog.importConfirmBtn')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

interface IrExportXmlDialogProps {
  profileName: string | undefined;
  exportedIrplusXml: string | null;
  onClose: () => void;
}

export const IrExportXmlDialog: React.FC<IrExportXmlDialogProps> = ({
  profileName,
  exportedIrplusXml,
  onClose,
}) => {
  const { t } = useTranslation();

  return (
    <Dialog open={exportedIrplusXml !== null} onOpenChange={open => !open && onClose()}>
      <DialogContent className="max-w-lg font-mono">
        <DialogHeader>
          <DialogTitle>Exported .irplus XML</DialogTitle>
          <DialogDescription>{profileName ?? 'Panadiga IR Profile'}</DialogDescription>
        </DialogHeader>
        <textarea
          readOnly
          rows={10}
          value={exportedIrplusXml ?? ''}
          data-testid="ir-export-xml-output"
          className="w-full rounded-lg bg-bench-950 border border-slate-700 p-2.5 text-[11px] text-emerald-300 font-mono"
        />
        <DialogFooter>
          <Button type="button" size="sm" onClick={onClose}>
            {t('common.cancel')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};
