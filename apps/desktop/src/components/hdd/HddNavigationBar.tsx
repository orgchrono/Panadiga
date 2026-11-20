import React, { useState } from 'react';
import {
  BarChart3,
  BookOpen,
  FileCheck,
  FolderArchive,
  HardDrive,
  Layers,
  ShieldCheck,
  Table,
  Unlock,
} from 'lucide-react';
import type { TranslationKey } from '@/i18n/translations';
import { useTranslation } from '@/i18n/useTranslation';
import type { DriveRoleFilter, HddTab, PhysicalDriveInfo } from '@/models';
import { isRecord, readStringProp } from '@/utils/typeGuards';
import { StorageDeviceSelector } from './StorageDeviceSelector';

interface HddNavigationBarProps {
  readonly showPathInput: boolean;
  readonly inputPath: string;
  readonly onInputPathChange: (val: string) => void;
  readonly onOpenPath: () => void;
  readonly onSelectImagePath: (path: string) => void;
  readonly drives: readonly PhysicalDriveInfo[];
  readonly roleFilter: DriveRoleFilter;
  readonly onChangeRoleFilter: (filter: DriveRoleFilter) => void;
  readonly loadingDrives: boolean;
  readonly onRefreshDrives: () => void;
  readonly activeTab: HddTab;
  readonly onSelectTab: (tab: HddTab) => void;
  readonly isAdvanced: boolean;
  readonly onEnableAdvanced: () => void;
  readonly hasManifest: boolean;
}

const CORE_HDD_TABS: ReadonlyArray<{
  readonly id: HddTab;
  readonly labelKey: TranslationKey;
  readonly Icon: React.ComponentType<{ className?: string }>;
}> = [
  { id: 'overview', labelKey: 'hdd.tabs.overview', Icon: Layers },
  { id: 'mbr', labelKey: 'hdd.tabs.mbr', Icon: FileCheck },
];

const ADVANCED_HDD_TABS: ReadonlyArray<{
  readonly id: HddTab;
  readonly labelKey: TranslationKey;
  readonly Icon: React.ComponentType<{ className?: string }>;
}> = [
  { id: 'meihdfs', labelKey: 'hdd.tabs.meihdfs', Icon: Table },
  { id: 'journal', labelKey: 'hdd.tabs.journal', Icon: BookOpen },
  { id: 'density', labelKey: 'hdd.tabs.density', Icon: BarChart3 },
];

export const HddNavigationBar: React.FC<HddNavigationBarProps> = ({
  showPathInput,
  inputPath,
  onInputPathChange,
  onOpenPath,
  onSelectImagePath,
  drives,
  roleFilter,
  onChangeRoleFilter,
  loadingDrives,
  onRefreshDrives,
  activeTab,
  onSelectTab,
  isAdvanced,
  onEnableAdvanced,
  hasManifest,
}) => {
  const { t } = useTranslation();
  const [dragOverPath, setDragOverPath] = useState(false);

  const handleDropImagePath = (event: React.DragEvent<HTMLDivElement>) => {
    event.preventDefault();
    setDragOverPath(false);
    const droppedFile: unknown = event.dataTransfer.files?.[0];
    const droppedPath = isRecord(droppedFile)
      ? readStringProp(droppedFile, 'path') ?? readStringProp(droppedFile, 'name')
      : undefined;
    const textPath = event.dataTransfer.getData('text/plain')?.trim();
    const resolved = droppedPath || textPath;
    if (resolved) {
      onSelectImagePath(resolved);
    }
  };

  const renderTabButton = (
    tab: HddTab,
    label: string,
    Icon: React.ComponentType<{ className?: string }>,
    activeClass = 'bg-cyan-500/10 text-cyan-400 border border-cyan-500/30',
  ) => {
    const active = activeTab === tab;
    return (
      <button
        key={tab}
        type="button"
        data-testid={`hdd-tab-${tab}`}
        onClick={() => onSelectTab(tab)}
        role="tab"
        aria-selected={active}
        className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg font-semibold transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-cyan-400 ${
          active ? activeClass : 'text-slate-400 hover:text-slate-200'
        }`}
      >
        <Icon className="w-3.5 h-3.5" />
        <span>{label}</span>
      </button>
    );
  };

  return (
    <>
      {showPathInput && (
        <div
          onDragOver={(e) => {
            e.preventDefault();
            setDragOverPath(true);
          }}
          onDragLeave={() => setDragOverPath(false)}
          onDrop={handleDropImagePath}
          className={`p-4 rounded-xl bg-slate-950 border shadow-xl space-y-2 transition-colors ${
            dragOverPath ? 'border-emerald-400 bg-emerald-950/20' : 'border-cyan-500/40'
          }`}
        >
          <label
            htmlFor="hdd-image-path-input"
            className="text-[11px] text-cyan-300 font-bold uppercase block"
          >
            {t('hdd.navBar.pathLabel')}
          </label>
          <div className="flex gap-2">
            <input
              id="hdd-image-path-input"
              type="text"
              placeholder={t('hdd.navBar.pathPlaceholder')}
              value={inputPath}
              onChange={(e) => onInputPathChange(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && onOpenPath()}
              className="flex-1 px-3 py-2 rounded-lg bg-bench-900 border border-slate-700 text-xs text-slate-100 focus:outline-none focus:border-cyan-400"
            />
            <button
              type="button"
              onClick={onOpenPath}
              className="px-4 py-2 rounded-lg bg-cyan-500 hover:bg-cyan-400 text-slate-950 font-bold text-xs transition-colors"
            >
              {t('hdd.navBar.loadBtn')}
            </button>
          </div>
          <div className="pt-2 border-t border-slate-800 space-y-2">
            <span className="text-[11px] text-slate-400 block">
              {t('hdd.devices.inspectDeviceLabel')}
            </span>
            <StorageDeviceSelector
              compact
              allowSelectSystemDrive={false}
              drives={drives}
              selectedDevicePath={inputPath}
              roleFilter={roleFilter}
              onChangeRoleFilter={onChangeRoleFilter}
              loading={loadingDrives}
              onRefresh={onRefreshDrives}
              onSelectDrive={(drive) => onSelectImagePath(drive.device_path)}
            />
          </div>
        </div>
      )}

      <div
        className="flex flex-wrap items-center gap-2 border-b border-slate-800 pb-2 text-xs"
        role="tablist"
        aria-label={t('hdd.navBar.tablistAria')}
      >
        {CORE_HDD_TABS.length === 0
          ? null
          : CORE_HDD_TABS.map(({ id, labelKey, Icon }) => renderTabButton(id, t(labelKey), Icon))}

        {isAdvanced ? (
          ADVANCED_HDD_TABS.map(({ id, labelKey, Icon }) => renderTabButton(id, t(labelKey), Icon))
        ) : (
          <button
            type="button"
            onClick={onEnableAdvanced}
            className="ml-auto px-2.5 py-1 rounded-lg border border-slate-800 text-[11px] text-cyan-300 hover:bg-slate-800 transition-colors"
          >
            {t('hdd.navBar.enableAdvancedBtn')}
          </button>
        )}

        {hasManifest &&
          renderTabButton(
            'extract',
            t('hdd.navBar.tabExtract'),
            FolderArchive,
            'bg-emerald-500/10 text-emerald-400 border border-emerald-500/30',
          )}

        {renderTabButton(
          'capacity',
          t('hdd.navBar.tabCapacity'),
          Unlock,
          'bg-emerald-500/10 text-emerald-400 border border-emerald-500/30',
        )}

        {renderTabButton(
          'safe-flash',
          t('hdd.navBar.tabSafeFlash'),
          ShieldCheck,
          'bg-cyan-500/10 text-cyan-400 border border-cyan-500/30',
        )}

        {renderTabButton(
          'writer',
          t('hdd.navBar.tabWriter'),
          HardDrive,
          'bg-purple-500/10 text-purple-400 border border-purple-500/30',
        )}
      </div>
    </>
  );
};
