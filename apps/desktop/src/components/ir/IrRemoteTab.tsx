import React from 'react';
import { Layers, Loader2, Pencil } from 'lucide-react';
import { useTranslation } from '@/i18n/useTranslation';
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
  EmptyState,
  Switch,
} from '@/components/ui';
import type {
  DynamicKeymapProfile,
  DynamicRemoteKey,
  RemoteMode,
  RemoteUnitSet,
} from '@/models/ir';
import { IrKeyButton } from './IrKeyButton';
import { KEY_SECTION_ORDER } from './irToneStyles';

interface IrRemoteTabProps {
  mode: RemoteMode;
  remoteUnit: RemoteUnitSet;
  editMode: boolean;
  isLoading: boolean;
  isTransmitting: boolean;
  profile: DynamicKeymapProfile | null;
  onSelectMode: (mode: RemoteMode) => void;
  onSelectComboDeck: () => void;
  onSelectRemoteUnit: (unit: RemoteUnitSet) => void;
  onToggleEditMode: (enabled: boolean) => void;
  onTriggerKey: (key: DynamicRemoteKey) => void;
  onEditKey: (key: DynamicRemoteKey) => void;
}

const DECK_MODES: ReadonlyArray<Exclude<RemoteMode, 'CATALOG'>> = ['DVD', 'VCR', 'TV'];
const UNIT_SETS: readonly RemoteUnitSet[] = [1, 2, 3];

export const IrRemoteTab: React.FC<IrRemoteTabProps> = ({
  mode,
  remoteUnit,
  editMode,
  isLoading,
  isTransmitting,
  profile,
  onSelectMode,
  onSelectComboDeck,
  onSelectRemoteUnit,
  onToggleEditMode,
  onTriggerKey,
  onEditKey,
}) => {
  const { t } = useTranslation();

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader className="pb-3">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <CardTitle className="flex items-center gap-2 text-sm">
              <Layers className="w-4 h-4 text-cyan-400" />
              <span>{t('irCommander.remote.deckTitle')}</span>
              {profile && (
                <Badge variant="outline" className="text-[10px] font-mono">
                  {profile.source_codeset_id} · {profile.keys.length}
                </Badge>
              )}
            </CardTitle>

            <label className="flex items-center gap-2 text-xs text-slate-300 cursor-pointer">
              <Pencil className="w-3.5 h-3.5 text-amber-400" />
              <span>{t('irCommander.keymap.editMode')}</span>
              <Switch
                checked={editMode}
                onCheckedChange={onToggleEditMode}
                data-testid="ir-remote-edit-switch"
              />
            </label>
          </div>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <div className="flex flex-wrap gap-1.5">
              {DECK_MODES.map(deck => (
                <Button
                  key={deck}
                  type="button"
                  size="sm"
                  variant={mode === deck ? 'default' : 'outline'}
                  onClick={() => onSelectMode(deck)}
                  data-testid={`ir-deck-${deck.toLowerCase()}`}
                >
                  {deck}
                </Button>
              ))}
              <Button
                type="button"
                size="sm"
                variant={profile?.source_codeset_id === 'curated_diga_combo_vhs_dvd' ? 'purple' : 'outline'}
                onClick={onSelectComboDeck}
                data-testid="ir-deck-combo"
              >
                COMBO (VHS+DVD)
              </Button>
            </div>

            <div className="flex items-center gap-1.5">
              <span className="text-[11px] font-mono text-slate-400">SET:</span>
              {UNIT_SETS.map(u => (
                <Button
                  key={u}
                  type="button"
                  size="sm"
                  variant={remoteUnit === u ? 'emerald' : 'outline'}
                  onClick={() => onSelectRemoteUnit(u)}
                  data-testid={`ir-unit-set-${u}`}
                  className="px-2.5"
                >
                  {u}
                </Button>
              ))}
            </div>
          </div>

          <Alert variant="info">
            <AlertTitle>{t('irCommander.remote.teletextNoteTitle')}</AlertTitle>
            <AlertDescription className="text-xs">
              {t('irCommander.remote.teletextNoteBody')}
            </AlertDescription>
          </Alert>
        </CardContent>
      </Card>

      {isLoading ? (
        <div className="flex items-center justify-center gap-2 py-10 text-xs text-slate-400 font-mono">
          <Loader2 className="w-4 h-4 animate-spin text-cyan-400" />
          <span>{t('common.loading')}</span>
        </div>
      ) : !profile || profile.keys.length === 0 ? (
        <EmptyState
          compact
          title={t('irCommander.remote.emptyTitle')}
          description={t('irCommander.remote.emptyDesc')}
        />
      ) : (
        <div className="space-y-4">
          {KEY_SECTION_ORDER.map(section => {
            const sectionKeys = profile.keys.filter(k => k.section === section);
            if (sectionKeys.length === 0) return null;
            return (
              <Card key={section}>
                <CardHeader className="py-2.5 px-4 border-b border-slate-800/80">
                  <CardTitle className="text-xs font-mono uppercase tracking-wider text-slate-300">
                    {t(`irCommander.sections.${section}`)}
                  </CardTitle>
                </CardHeader>
                <CardContent className="p-3 grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-2">
                  {sectionKeys.map(keyItem => (
                    <IrKeyButton
                      key={keyItem.id}
                      keyItem={keyItem}
                      disabled={isTransmitting}
                      editMode={editMode}
                      onTrigger={onTriggerKey}
                      onEdit={onEditKey}
                    />
                  ))}
                </CardContent>
              </Card>
            );
          })}
        </div>
      )}
    </div>
  );
};
