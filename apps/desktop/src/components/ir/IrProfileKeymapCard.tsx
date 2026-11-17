import React from 'react';
import { Download, Plus, Save } from 'lucide-react';
import { useTranslation } from '@/i18n/useTranslation';
import { Button, Card, CardContent, CardHeader, CardTitle, EmptyState } from '@/components/ui';
import type { DynamicKeymapProfile, DynamicRemoteKey } from '@/models/ir';
import { IrKeyButton } from './IrKeyButton';

interface IrProfileKeymapCardProps {
  activeProfile: DynamicKeymapProfile;
  isTransmitting: boolean;
  editMode: boolean;
  onTriggerKey: (key: DynamicRemoteKey) => void;
  onEditKey: (key: DynamicRemoteKey) => void;
  onAddBlankKey: () => void;
  onSaveProfile: () => void;
  onExportIrplus: () => void;
}

export const IrProfileKeymapCard: React.FC<IrProfileKeymapCardProps> = ({
  activeProfile,
  isTransmitting,
  editMode,
  onTriggerKey,
  onEditKey,
  onAddBlankKey,
  onSaveProfile,
  onExportIrplus,
}) => {
  const { t } = useTranslation();

  return (
    <Card>
      <CardHeader className="pb-2 flex flex-row flex-wrap items-center justify-between gap-2">
        <CardTitle className="text-sm font-mono truncate">{activeProfile.name}</CardTitle>
        <div className="flex flex-wrap gap-1.5">
          <Button type="button" size="sm" variant="outline" onClick={onAddBlankKey} className="gap-1">
            <Plus className="w-3.5 h-3.5" />
            {t('irCommander.catalog.addKeyBtn')}
          </Button>
          <Button
            type="button"
            size="sm"
            variant="emerald"
            onClick={onSaveProfile}
            data-testid="ir-save-profile-btn"
            className="gap-1"
          >
            <Save className="w-3.5 h-3.5" />
            {t('irCommander.catalog.saveProfileBtn')}
          </Button>
          <Button
            type="button"
            size="sm"
            variant="outline"
            onClick={onExportIrplus}
            data-testid="ir-export-irplus-btn"
            className="gap-1"
          >
            <Download className="w-3.5 h-3.5" />
            {t('irCommander.catalog.exportIrplusBtn')}
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        {activeProfile.keys.length === 0 ? (
          <EmptyState
            compact
            title={t('irCommander.remote.emptyTitle')}
            description={t('irCommander.remote.emptyDesc')}
          />
        ) : (
          <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-2">
            {activeProfile.keys.map(k => (
              <IrKeyButton
                key={k.id}
                keyItem={k}
                disabled={isTransmitting}
                editMode={editMode}
                onTrigger={onTriggerKey}
                onEdit={onEditKey}
              />
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
};
