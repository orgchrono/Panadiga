import React from 'react';
import { RotateCcw, Save, ShieldAlert, Sliders } from 'lucide-react';
import {
  Alert,
  AlertDescription,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import type { RuntimeSettings } from '@/models';
import {
  RuntimeIrLayoutSection,
  RuntimeSerialSection,
  RuntimeStorageSection,
  type FieldUpdateHandlers,
} from './RuntimeDefaultsSections';

export interface RuntimeDefaultsCardProps extends FieldUpdateHandlers {
  readonly draft: RuntimeSettings;
  readonly keywordsText: string;
  readonly onChangeKeywordsText: (val: string) => void;
  readonly isLoading: boolean;
  readonly saving: boolean;
  readonly error: string | null;
  readonly successMessage: string | null;
  readonly isDirty: boolean;
  readonly onPersistDefaults: () => void;
  readonly onResetDefaults: () => void;
}

export const RuntimeDefaultsCard: React.FC<RuntimeDefaultsCardProps> = ({
  draft,
  keywordsText,
  onChangeKeywordsText,
  isLoading,
  saving,
  error,
  successMessage,
  isDirty,
  onUpdateSerial,
  onUpdateStorage,
  onUpdateIr,
  onUpdateWorkbench,
  onPersistDefaults,
  onResetDefaults,
}) => {
  const { t } = useTranslation();
  const handlers: FieldUpdateHandlers = {
    onUpdateSerial,
    onUpdateStorage,
    onUpdateIr,
    onUpdateWorkbench,
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Sliders className="w-4 h-4 text-cyan-400" />
          <span>{t('settings.runtime.cardTitle')}</span>
        </CardTitle>
        <CardDescription>{t('settings.runtime.cardDesc')}</CardDescription>
      </CardHeader>

      <CardContent className="space-y-5 text-xs">
        <Alert variant="warning">
          <ShieldAlert className="w-4 h-4" />
          <AlertDescription>{t('settings.runtime.lockedShieldBanner')}</AlertDescription>
        </Alert>

        {error && (
          <Alert variant="destructive">
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}

        {isLoading ? (
          <div className="space-y-2" aria-busy="true">
            <div className="text-slate-400">{t('settings.runtime.loadingLabel')}</div>
            <div className="h-24 w-full rounded-lg bg-slate-800/60 animate-pulse" />
          </div>
        ) : (
          <div className="space-y-5">
            <RuntimeSerialSection draft={draft} handlers={handlers} />
            <RuntimeStorageSection
              draft={draft}
              keywordsText={keywordsText}
              onChangeKeywordsText={onChangeKeywordsText}
              handlers={handlers}
            />
            <RuntimeIrLayoutSection draft={draft} handlers={handlers} />

            <div className="flex flex-wrap items-center gap-2 pt-3 border-t border-slate-800">
              <Button
                type="button"
                size="sm"
                disabled={saving || !isDirty}
                onClick={onPersistDefaults}
                data-testid="runtime-settings-save-btn"
                className="gap-1.5"
              >
                <Save className="w-3.5 h-3.5" />
                <span>{t('settings.runtime.saveBtn')}</span>
              </Button>
              <Button
                type="button"
                size="sm"
                variant="outline"
                disabled={saving}
                onClick={onResetDefaults}
                data-testid="runtime-settings-reset-btn"
                className="gap-1.5"
              >
                <RotateCcw className="w-3.5 h-3.5" />
                <span>{t('settings.runtime.resetBtn')}</span>
              </Button>
            </div>
            {successMessage && (
              <output className="block">
                <Alert variant="success">
                  <AlertDescription>{successMessage}</AlertDescription>
                </Alert>
              </output>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  );
};
