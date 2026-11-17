import React, { useState } from 'react';
import type {
  DynamicKeymapProfile,
  DynamicRemoteKey,
  PanasonicCatalogSummary,
  PanasonicCategory,
} from '@/models/ir';
import { IrCatalogSearchCard } from './IrCatalogSearchCard';
import { IrProfileKeymapCard } from './IrProfileKeymapCard';
import { IrExportXmlDialog, IrImportProfileDialog } from './IrCatalogDialogs';

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
  const [importOpen, setImportOpen] = useState(false);

  return (
    <div className="space-y-4">
      <IrCatalogSearchCard
        totalCodesets={totalCodesets}
        totalAliases={totalAliases}
        searchQuery={searchQuery}
        selectedCategory={selectedCategory}
        searchResults={searchResults}
        isSearching={isSearching}
        activeProfileId={activeProfile?.id ?? null}
        savedProfiles={savedProfiles}
        onSearchQueryChange={onSearchQueryChange}
        onCategoryChange={onCategoryChange}
        onSelectCodeset={onSelectCodeset}
        onSelectSavedProfile={onSelectSavedProfile}
        onOpenImport={() => setImportOpen(true)}
      />

      {activeProfile && (
        <IrProfileKeymapCard
          activeProfile={activeProfile}
          isTransmitting={isTransmitting}
          editMode={editMode}
          onTriggerKey={onTriggerKey}
          onEditKey={onEditKey}
          onAddBlankKey={onAddBlankKey}
          onSaveProfile={onSaveProfile}
          onExportIrplus={onExportIrplus}
        />
      )}

      <IrImportProfileDialog
        open={importOpen}
        onOpenChange={setImportOpen}
        onConfirmImport={onImportText}
      />

      <IrExportXmlDialog
        profileName={activeProfile?.name}
        exportedIrplusXml={exportedIrplusXml}
        onClose={onClearExportXml}
      />
    </div>
  );
};
