import React from 'react';
import { Loader2, Search, Upload } from 'lucide-react';
import { useTranslation } from '@/i18n/useTranslation';
import {
  Badge,
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  EmptyState,
  Input,
} from '@/components/ui';
import type {
  DynamicKeymapProfile,
  PanasonicCatalogSummary,
  PanasonicCategory,
} from '@/models/ir';

interface IrCatalogSearchCardProps {
  totalCodesets: number;
  totalAliases: number;
  searchQuery: string;
  selectedCategory: PanasonicCategory;
  searchResults: PanasonicCatalogSummary[];
  isSearching: boolean;
  activeProfileId: string | null;
  savedProfiles: DynamicKeymapProfile[];
  onSearchQueryChange: (q: string) => void;
  onCategoryChange: (cat: PanasonicCategory) => void;
  onSelectCodeset: (codesetId: string) => void;
  onSelectSavedProfile: (profileId: string) => void;
  onOpenImport: () => void;
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

export const IrCatalogSearchCard: React.FC<IrCatalogSearchCardProps> = ({
  totalCodesets,
  totalAliases,
  searchQuery,
  selectedCategory,
  searchResults,
  isSearching,
  activeProfileId,
  savedProfiles,
  onSearchQueryChange,
  onCategoryChange,
  onSelectCodeset,
  onSelectSavedProfile,
  onOpenImport,
}) => {
  const { t } = useTranslation();

  return (
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
            onClick={onOpenImport}
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
                variant={activeProfileId === sp.id ? 'emerald' : 'outline'}
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
  );
};
