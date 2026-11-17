import React, { useState } from 'react';
import { Trash2 } from 'lucide-react';
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
import {
  KASEIKYO_HEADER_BYTE0,
  KASEIKYO_HEADER_BYTE1,
  MAX_BYTE_VALUE,
} from '@/constants';
import { useTranslation } from '@/i18n/useTranslation';
import type { DynamicRemoteKey, KeyProtocol, KeySection, KeyTone } from '@/models/ir';
import { oneOfGuard } from '@/utils/typeGuards';
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

const isKeySection = oneOfGuard(KEY_SECTION_ORDER);
const isKeyProtocol = oneOfGuard(['KASEIKYO', 'PRONTO_RAW'] as const);
const isKeyTone = oneOfGuard(KEY_TONES);

function parseHexByte(raw: string, fallback: number): number {
  const n = Number.parseInt(raw.trim().replace(/^0x/i, ''), 16);
  return Number.isFinite(n) ? Math.min(MAX_BYTE_VALUE, Math.max(0, n)) : fallback;
}

function formatByteHex(value: number): string {
  return value.toString(16).toUpperCase().padStart(2, '0');
}

interface IrKeyEditorFormProps {
  readonly keyItem: DynamicRemoteKey;
  readonly onClose: () => void;
  readonly onSave: (updated: DynamicRemoteKey) => void;
  readonly onDelete: (keyId: string) => void;
}

const IrKeyEditorForm: React.FC<IrKeyEditorFormProps> = ({
  keyItem,
  onClose,
  onSave,
  onDelete,
}) => {
  const { t } = useTranslation();
  const [label, setLabel] = useState(keyItem.label);
  const [section, setSection] = useState<KeySection>(keyItem.section);
  const [protocol, setProtocol] = useState<KeyProtocol>(keyItem.protocol);
  const [devHex, setDevHex] = useState(formatByteHex(keyItem.device));
  const [subHex, setSubHex] = useState(formatByteHex(keyItem.subdevice));
  const [obcHex, setObcHex] = useState(formatByteHex(keyItem.obc));
  const [prontoHex, setProntoHex] = useState(keyItem.pronto_hex ?? '');
  const [tone, setTone] = useState<KeyTone>(keyItem.tone);
  const [confirmRequired, setConfirmRequired] = useState(keyItem.confirm_required);
  const [message, setMessage] = useState<string | null>(null);

  const applyKeyChanges = () => {
    const dev = parseHexByte(devHex, keyItem.device);
    const sub = parseHexByte(subHex, keyItem.subdevice);
    const obc = parseHexByte(obcHex, keyItem.obc);
    const wire =
      protocol === 'KASEIKYO'
        ? [KASEIKYO_HEADER_BYTE0, KASEIKYO_HEADER_BYTE1, dev, sub, obc, dev ^ sub ^ obc]
            .map(formatByteHex)
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
    setMessage(t('common.success'));
    onClose();
  };

  return (
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
            onChange={(e) => setLabel(e.target.value)}
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
              onChange={(e) => {
                if (isKeySection(e.target.value)) setSection(e.target.value);
              }}
            >
              {KEY_SECTION_ORDER.length === 0 ? (
                <option value="CUSTOM">CUSTOM</option>
              ) : (
                KEY_SECTION_ORDER.map((s) => (
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
              onChange={(e) => {
                if (isKeyProtocol(e.target.value)) setProtocol(e.target.value);
              }}
            >
              <option value="KASEIKYO">{t('irCommander.editorExtra.kaseikyoOption')}</option>
              <option value="PRONTO_RAW">{t('irCommander.editorExtra.prontoOption')}</option>
            </Select>
          </div>
        </div>

        {protocol === 'KASEIKYO' ? (
          <div className="grid grid-cols-3 gap-2">
            <div>
              <label htmlFor="ir-edit-dev" className="text-slate-400 block mb-1">
                {t('irCommander.editorExtra.devLabel')}
              </label>
              <Input id="ir-edit-dev" value={devHex} onChange={(e) => setDevHex(e.target.value)} />
            </div>
            <div>
              <label htmlFor="ir-edit-sub" className="text-slate-400 block mb-1">
                {t('irCommander.editorExtra.subLabel')}
              </label>
              <Input id="ir-edit-sub" value={subHex} onChange={(e) => setSubHex(e.target.value)} />
            </div>
            <div>
              <label htmlFor="ir-edit-obc" className="text-slate-400 block mb-1">
                {t('irCommander.editorExtra.obcLabel')}
              </label>
              <Input id="ir-edit-obc" value={obcHex} onChange={(e) => setObcHex(e.target.value)} />
            </div>
          </div>
        ) : (
          <div>
            <label htmlFor="ir-edit-pronto" className="text-slate-400 block mb-1">
              {t('irCommander.editorExtra.prontoLabel')}
            </label>
            <Input
              id="ir-edit-pronto"
              value={prontoHex}
              onChange={(e) => setProntoHex(e.target.value)}
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
              onChange={(e) => {
                if (isKeyTone(e.target.value)) setTone(e.target.value);
              }}
            >
              {KEY_TONES.map((k) => (
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
      {message && (
        <output className="block">
          <Alert variant="success">
            <AlertDescription>{message}</AlertDescription>
          </Alert>
        </output>
      )}
    </DialogContent>
  );
};

export const IrKeyEditorDialog: React.FC<IrKeyEditorDialogProps> = ({
  keyItem,
  onClose,
  onSave,
  onDelete,
}) => {
  if (!keyItem) return null;

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <IrKeyEditorForm
        key={keyItem.id}
        keyItem={keyItem}
        onClose={onClose}
        onSave={onSave}
        onDelete={onDelete}
      />
    </Dialog>
  );
};
