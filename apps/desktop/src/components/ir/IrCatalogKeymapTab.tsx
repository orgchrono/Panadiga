import React, { useState } from 'react';
import { Download, Loader2, Plus, Save, Search, Upload } from 'lucide-react';
import { useTranslation } from '@/i18n/useTranslation';
import {
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
  Input,
} from '@/components/ui';
import type {
  DynamicKeymapProfile,
  DynamicRemoteKey,
  PanasonicCatalogSummary,
  PanasonicCategory,
} from '@/models/ir';
import { IrKeyButton } from './IrKeyButton';

interface IrCatalogKeymapTabProps {
  totalCodesets: number;
  totalAliases: number;
  searchQuery: string;
  selectedCategory: PanasonicCategory;
  searchResults: PanasonicCatalogSummary[];
  isSearching: boolean;
  isTransmitting: boolean;
  editMode: boolean;
  activeProfile: DynamicKeymapProfile | null;
  savedProfiles: DynamicKeymapProfile[];
  exportedIrplusXml: string | null;
  onSearchQueryChange: (q: string) => void;
  onCategoryChange: (cat: PanasonicCategory) => void;
  onSelectCodeset: (codesetId: string) => void;
  onSelectSavedProfile: (profileId: string) => void;
  onTriggerKey: (key: DynamicRemoteKey) => void;
  onEditKey: (key: DynamicRemoteKey) => void;
  onAddBlankKey: () => void;
  onSaveProfile: () => void;
  onImportText: (format: 'irplus' | 'flipper', content: string, name: string) => void;
  onExportIrplus: () => void;
  onClearExportXml: () => void;
}

const CATEGORIES: readonly PanasonicCategory[] = [
  'ALL',
  'DIGA',
  'COMBO',
  'VCR',
  'DVD',
  'TV',
  'AUDIO',
  'STB',
  'PROJ',
  'AC',
  'OTHER',
];

export const IrCatalogKeymapTab: React.FC<IrCatalogKeymapTabProps> = ({
  totalCodesets,
  totalAliases,
  searchQuery,
  selectedCategory,
  searchResults,
  isSearching,
  isTransmitting,
  editMode,
  activeProfile,
  savedProfiles,
  exportedIrplusXml,
  onSearchQueryChange,
  onCategoryChange,
  onSelectCodeset,
  onSelectSavedProfile,
  onTriggerKey,
  onEditKey,
  onAddBlankKey,
  onSaveProfile,
  onImportText,
  onExportIrplus,
  onClearExportXml,
}) => {
  const { t } = useTranslation();
  const [importOpen, setImportOpen] = useState(false);
  const [importFormat, setImportFormat] = useState<'flipper' | 'irplus'>('flipper');
  const [importName, setImportName] = useState('Imported Panasonic Profile');
  const [importBody, setImportBody] = useState('');

  const handleConfirmImport = () => {
    if (!importBody.trim()) return;
    onImportText(importFormat, importBody, importName.trim() || 'Custom Profile');
    setImportOpen(false);
    setImportBody('');
  };

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader className="pb-3">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <CardTitle className="flex items-center gap-2 text-sm">
              <Search className="w-4 h-4 text-cyan-400" />
              <span>{t('irCommander.catalog.title')}</span>
            </CardTitle>
            <Badge variant="outline" className="font-mono text-[10px]">
              {totalCodesets} codesets · {totalAliases} models
            </Badge>
          </div>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="flex flex-wrap gap-2">
            <Input
              value={searchQuery}
              onChange={e => onSearchQueryChange(e.target.value)}
              placeholder={t('irCommander.catalog.searchPlaceholder')}
              data-testid="ir-catalog-search-input"
              className="flex-1 min-w-[200px]"
            />
            <Button
              type="button"
              size="sm"
              variant="outline"
              onClick={() => setImportOpen(true)}
              data-testid="ir-open-import-btn"
              className="gap-1.5"
            >
              <Upload className="w-3.5 h-3.5" />
              {t('irCommander.catalog.importBtn')}
            </Button>
          </div>

          <div className="flex flex-wrap gap-1.5">
            {CATEGORIES.map(cat => (
              <Button
                key={cat}
                type="button"
                size="sm"
                variant={selectedCategory === cat ? 'default' : 'outline'}
                onClick={() => onCategoryChange(cat)}
                data-testid={`ir-cat-${cat.toLowerCase()}`}
                className="h-7 px-2 text-[11px]"
              >
                {cat}
              </Button>
            ))}
          </div>

          {savedProfiles.length > 0 && (
            <div className="flex flex-wrap items-center gap-1.5 pt-1 border-t border-slate-800">
              <span className="text-[11px] font-mono text-slate-400">
                {t('irCommander.catalog.savedLabel')}:
              </span>
              {savedProfiles.map(sp => (
                <Button
                  key={sp.id}
                  type="button"
                  size="sm"
                  variant={activeProfile?.id === sp.id ? 'emerald' : 'outline'}
                  onClick={() => onSelectSavedProfile(sp.id)}
                  className="h-7 text-[11px]"
                >
                  {sp.name}
                </Button>
              ))}
            </div>
          )}

          {isSearching ? (
            <div className="flex items-center gap-2 py-4 text-xs text-slate-400 font-mono">
              <Loader2 className="w-4 h-4 animate-spin text-cyan-400" />
              <span>{t('common.loading')}</span>
            </div>
          ) : searchResults.length === 0 ? (
            <EmptyState compact title={t('irCommander.catalog.emptySearch')} />
          ) : (
            <div className="max-h-56 overflow-y-auto space-y-1.5 pr-1">
              {searchResults.map(hit => (
                <button
                  key={hit.id}
                  type="button"
                  onClick={() => onSelectCodeset(hit.id)}
                  data-testid={`ir-codeset-hit-${hit.id}`}
                  className="w-full text-left rounded-lg border border-slate-800 bg-bench-950/70 hover:border-cyan-500/50 px-3 py-2 flex items-center justify-between gap-2 transition-colors"
                >
                  <div className="min-w-0">
                    <div className="text-xs font-semibold text-slate-100 truncate">
                      {hit.primary_name}
                    </div>
                    <div className="text-[11px] text-slate-400 font-mono truncate">
                      {hit.models.slice(0, 6).join(' · ')}
                    </div>
                  </div>
                  <div className="flex items-center gap-1.5 shrink-0">
                    <Badge variant="outline" className="text-[10px]">
                      {hit.category}
                    </Badge>
                    <Badge variant="outline" className="text-[10px] text-cyan-300 border-cyan-500/40">
                      {hit.command_count} cmds
                    </Badge>
                  </div>
                </button>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {activeProfile && (
        <Card>
          <CardHeader className="pb-2 flex flex-row flex-wrap items-center justify-between gap-2">
            <CardTitle className="text-sm font-mono truncate">
              {activeProfile.name}
            </CardTitle>
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
                .irplus XML
              </Button>
            </div>
          </CardHeader>
          <CardContent className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-2">
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
          </CardContent>
        </Card>
      )}

      <Dialog open={importOpen} onOpenChange={setImportOpen}>
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
            <Button type="button" variant="outline" size="sm" onClick={() => setImportOpen(false)}>
              {t('common.cancel')}
            </Button>
            <Button
              type="button"
              size="sm"
              disabled={!importBody.trim()}
              onClick={handleConfirmImport}
              data-testid="ir-import-confirm-btn"
            >
              {t('irCommander.catalog.importConfirmBtn')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={exportedIrplusXml !== null} onOpenChange={open => !open && onClearExportXml()}>
        <DialogContent className="max-w-lg font-mono">
          <DialogHeader>
            <DialogTitle>Exported .irplus XML</DialogTitle>
            <DialogDescription>{activeProfile?.name ?? 'Panadiga IR Profile'}</DialogDescription>
          </DialogHeader>
          <textarea
            readOnly
            rows={10}
            value={exportedIrplusXml ?? ''}
            data-testid="ir-export-xml-output"
            className="w-full rounded-lg bg-bench-950 border border-slate-700 p-2.5 text-[11px] text-emerald-300 font-mono"
          />
          <DialogFooter>
            <Button type="button" size="sm" onClick={onClearExportXml}>
              {t('common.cancel')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
};
