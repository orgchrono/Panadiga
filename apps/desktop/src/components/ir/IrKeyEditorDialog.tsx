import React, { useEffect, useState } from 'react';
import { Trash2 } from 'lucide-react';
import { useTranslation } from '@/i18n/useTranslation';
import { useToast } from '@/context/ToastContext';
import {
  Alert,
  AlertDescription,
  Button,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
  Select,
  Switch,
} from '@/components/ui';
import type { DynamicRemoteKey, KeyProtocol, KeySection, KeyTone } from '@/models/ir';
import { KEY_SECTION_ORDER } from './irToneStyles';

interface IrKeyEditorDialogProps {
  keyItem: DynamicRemoteKey | null;
  onClose: () => void;
  onSave: (updated: DynamicRemoteKey) => void;
  onDelete: (keyId: string) => void;
}

const KEY_TONES: readonly KeyTone[] = [
  'NEUTRAL',
  'PRIMARY',
  'INFO',
  'SUCCESS',
  'WARNING',
  'DANGER',
  'DANGER_DARK',
  'ACCENT',
];

function parseHexByte(raw: string, fallback: number): number {
  const n = Number.parseInt(raw.trim().replace(/^0x/i, ''), 16);
  return Number.isFinite(n) ? Math.min(255, Math.max(0, n)) : fallback;
}

export const IrKeyEditorDialog: React.FC<IrKeyEditorDialogProps> = ({
  keyItem,
  onClose,
  onSave,
  onDelete,
}) => {
  const { t } = useTranslation();
  const { showToast: toast } = useToast();
  const [label, setLabel] = useState('');
  const [section, setSection] = useState<KeySection>('CUSTOM');
  const [protocol, setProtocol] = useState<KeyProtocol>('KASEIKYO');
  const [devHex, setDevHex] = useState('B0');
  const [subHex, setSubHex] = useState('00');
  const [obcHex, setObcHex] = useState('3D');
  const [prontoHex, setProntoHex] = useState('');
  const [tone, setTone] = useState<KeyTone>('NEUTRAL');
  const [confirmRequired, setConfirmRequired] = useState(false);
  const [saveFeedback, setSaveFeedback] = useState(false);

  useEffect(() => {
    if (!keyItem) return;
    setSaveFeedback(false);
    setLabel(keyItem.label);
    setSection(keyItem.section);
    setProtocol(keyItem.protocol);
    setDevHex(keyItem.device.toString(16).toUpperCase().padStart(2, '0'));
    setSubHex(keyItem.subdevice.toString(16).toUpperCase().padStart(2, '0'));
    setObcHex(keyItem.obc.toString(16).toUpperCase().padStart(2, '0'));
    setProntoHex(keyItem.pronto_hex ?? '');
    setTone(keyItem.tone);
    setConfirmRequired(keyItem.confirm_required);
  }, [keyItem]);

  if (!keyItem) return null;

  const applyKeyChanges = () => {
    const dev = parseHexByte(devHex, keyItem.device);
    const sub = parseHexByte(subHex, keyItem.subdevice);
    const obc = parseHexByte(obcHex, keyItem.obc);
    const wire =
      protocol === 'KASEIKYO'
        ? [0x02, 0x20, dev, sub, obc, dev ^ sub ^ obc]
            .map(b => b.toString(16).toUpperCase().padStart(2, '0'))
            .join(' ')
        : 'PRONTO RAW';

    onSave({
      ...keyItem,
      label: label.trim() || keyItem.label,
      section,
      protocol,
      device: dev,
      subdevice: sub,
      obc,
      pronto_hex: protocol === 'PRONTO_RAW' ? prontoHex.trim() : null,
      tone,
      confirm_required: confirmRequired,
      wire_preview: wire,
    });
    setSaveFeedback(true);
    toast({
      type: 'success',
      title: label.trim() || keyItem.label,
      message: t('irCommander.editor.title'),
    });
    onClose();
  };

  return (
    <Dialog open={keyItem !== null} onOpenChange={open => !open && onClose()}>
      <DialogContent className="max-w-lg font-mono">
        <DialogHeader>
          <DialogTitle>{t('irCommander.editor.title')}</DialogTitle>
          <DialogDescription>{t('irCommander.editor.subtitle')}</DialogDescription>
        </DialogHeader>

        <div className="space-y-3 text-xs">
          <div>
            <label htmlFor="ir-edit-label" className="text-slate-400 block mb-1">
              {t('irCommander.editor.label')}
            </label>
            <Input
              id="ir-edit-label"
              value={label}
              onChange={e => setLabel(e.target.value)}
              data-testid="ir-edit-label-input"
            />
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div>
              <label htmlFor="ir-edit-section" className="text-slate-400 block mb-1">
                {t('irCommander.editor.section')}
              </label>
              <Select
                id="ir-edit-section"
                value={section}
                onChange={e => setSection(e.target.value as KeySection)}
              >
                {KEY_SECTION_ORDER.length === 0 ? (
                  <option value="CUSTOM">CUSTOM</option>
                ) : (
                  KEY_SECTION_ORDER.map(s => (
                    <option key={s} value={s}>
                      {s}
                    </option>
                  ))
                )}
              </Select>
            </div>
            <div>
              <label htmlFor="ir-edit-protocol" className="text-slate-400 block mb-1">
                {t('irCommander.editor.protocol')}
              </label>
              <Select
                id="ir-edit-protocol"
                value={protocol}
                onChange={e => setProtocol(e.target.value as KeyProtocol)}
              >
                <option value="KASEIKYO">KASEIKYO (Panasonic 48-bit)</option>
                <option value="PRONTO_RAW">PRONTO_RAW (0000 ...)</option>
              </Select>
            </div>
          </div>

          {protocol === 'KASEIKYO' ? (
            <div className="grid grid-cols-3 gap-2">
              <div>
                <label htmlFor="ir-edit-dev" className="text-slate-400 block mb-1">
                  Device (hex)
                </label>
                <Input id="ir-edit-dev" value={devHex} onChange={e => setDevHex(e.target.value)} />
              </div>
              <div>
                <label htmlFor="ir-edit-sub" className="text-slate-400 block mb-1">
                  Subdevice (hex)
                </label>
                <Input id="ir-edit-sub" value={subHex} onChange={e => setSubHex(e.target.value)} />
              </div>
              <div>
                <label htmlFor="ir-edit-obc" className="text-slate-400 block mb-1">
                  OBC (hex)
                </label>
                <Input id="ir-edit-obc" value={obcHex} onChange={e => setObcHex(e.target.value)} />
              </div>
            </div>
          ) : (
            <div>
              <label htmlFor="ir-edit-pronto" className="text-slate-400 block mb-1">
                Pronto Hex
              </label>
              <Input
                id="ir-edit-pronto"
                value={prontoHex}
                onChange={e => setProntoHex(e.target.value)}
                placeholder="0000 0070 0000 0032 ..."
              />
            </div>
          )}

          <div className="grid grid-cols-2 gap-3 items-center">
            <div>
              <label htmlFor="ir-edit-tone" className="text-slate-400 block mb-1">
                {t('irCommander.editor.tone')}
              </label>
              <Select
                id="ir-edit-tone"
                value={tone}
                onChange={e => setTone(e.target.value as KeyTone)}
              >
                {KEY_TONES.map(k => (
                  <option key={k} value={k}>
                    {k}
                  </option>
                ))}
              </Select>
            </div>
            <label className="flex items-center justify-between gap-2 pt-4 cursor-pointer">
              <span className="text-slate-300">{t('irCommander.editor.confirmGuard')}</span>
              <Switch checked={confirmRequired} onCheckedChange={setConfirmRequired} />
            </label>
          </div>
        </div>

        <DialogFooter className="gap-2 sm:justify-between">
          <Button
            type="button"
            variant="destructive"
            size="sm"
            onClick={() => {
              onDelete(keyItem.id);
              onClose();
            }}
            className="gap-1.5"
          >
            <Trash2 className="w-3.5 h-3.5" />
            {t('irCommander.editor.deleteBtn')}
          </Button>
          <div className="flex gap-2">
            <Button type="button" variant="outline" size="sm" onClick={onClose}>
              {t('common.cancel')}
            </Button>
            <Button type="button" size="sm" onClick={applyKeyChanges} data-testid="ir-edit-save-btn">
              {t('irCommander.editor.saveBtn')}
            </Button>
          </div>
        </DialogFooter>
        {saveFeedback && (
          <Alert variant="success" role="status">
            <AlertDescription>{t('common.success')}</AlertDescription>
          </Alert>
        )}
      </DialogContent>
    </Dialog>
  );
};
