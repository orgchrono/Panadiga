import React, { useState } from 'react';
import {
  BarChart3,
  BookOpen,
  FileCheck,
  FolderArchive,
  HardDrive,
  Layers,
  Table,
  Unlock,
} from 'lucide-react';
import type { HddTab } from '@/viewmodels/useHddStorageViewModel';
import { useTranslation } from '@/i18n/useTranslation';

interface HddNavigationBarProps {
  readonly showPathInput: boolean;
  readonly inputPath: string;
  readonly onInputPathChange: (val: string) => void;
  readonly onOpenPath: () => void;
  readonly onSelectImagePath: (path: string) => void;
  readonly activeTab: HddTab;
  readonly onSelectTab: (tab: HddTab) => void;
  readonly isAdvanced: boolean;
  readonly onEnableAdvanced: () => void;
  readonly hasManifest: boolean;
}

export const HddNavigationBar: React.FC<HddNavigationBarProps> = ({
  showPathInput,
  inputPath,
  onInputPathChange,
  onOpenPath,
  onSelectImagePath,
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
    const droppedFile = event.dataTransfer.files?.[0] as (File & { path?: string }) | undefined;
    const textPath = event.dataTransfer.getData('text/plain')?.trim();
    const resolved = droppedFile?.path || droppedFile?.name || textPath;
    if (resolved) {
      onSelectImagePath(resolved);
    }
  };

  return (
    <>
      {showPathInput && (
        <div
          onDragOver={e => {
            e.preventDefault();
            setDragOverPath(true);
          }}
          onDragLeave={() => setDragOverPath(false)}
          onDrop={handleDropImagePath}
          className={`p-4 rounded-xl bg-slate-950 border shadow-xl space-y-2 transition-colors ${
            dragOverPath ? 'border-emerald-400 bg-emerald-950/20' : 'border-cyan-500/40'
          }`}
        >
          <label className="text-[11px] text-cyan-300 font-bold uppercase block">
            Caminho do Arquivo de Imagem (.bin / .raw / .img) — Arraste o arquivo ou digite
          </label>
          <div className="flex gap-2">
            <input
              type="text"
              placeholder="Ex: firmware_and_images/panasonic DMR EH-55 Firmware hdd_100.bin"
              value={inputPath}
              onChange={e => onInputPathChange(e.target.value)}
              onKeyDown={e => e.key === 'Enter' && onOpenPath()}
              className="flex-1 px-3 py-2 rounded-lg bg-bench-900 border border-slate-700 text-xs text-slate-100 focus:outline-none focus:border-cyan-400"
            />
            <button
              type="button"
              onClick={onOpenPath}
              className="px-4 py-2 rounded-lg bg-cyan-500 hover:bg-cyan-400 text-slate-950 font-bold text-xs transition-colors"
            >
              Carregar
            </button>
          </div>
        </div>
      )}

      <div
        className="flex flex-wrap items-center gap-2 border-b border-slate-800 pb-2 text-xs"
        role="tablist"
        aria-label="Navegação de Forense do Disco"
      >
        <button
          type="button"
          onClick={() => onSelectTab('overview')}
          role="tab"
          aria-selected={activeTab === 'overview'}
          className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg font-semibold transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-cyan-400 ${
            activeTab === 'overview'
              ? 'bg-cyan-500/10 text-cyan-400 border border-cyan-500/30'
              : 'text-slate-400 hover:text-slate-200'
          }`}
        >
          <Layers className="w-3.5 h-3.5" />
          <span>{t('hdd.tabs.overview', 'Visão Geral do Disco')}</span>
        </button>

        <button
          type="button"
          onClick={() => onSelectTab('mbr')}
          role="tab"
          aria-selected={activeTab === 'mbr'}
          className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg font-semibold transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-cyan-400 ${
            activeTab === 'mbr'
              ? 'bg-cyan-500/10 text-cyan-400 border border-cyan-500/30'
              : 'text-slate-400 hover:text-slate-200'
          }`}
        >
          <FileCheck className="w-3.5 h-3.5" />
          <span>{t('hdd.tabs.mbr', 'MBR vs Espelho LBA 57.344')}</span>
        </button>

        {isAdvanced && (
          <>
            <button
              type="button"
              onClick={() => onSelectTab('meihdfs')}
              role="tab"
              aria-selected={activeTab === 'meihdfs'}
              className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg font-semibold transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-cyan-400 ${
                activeTab === 'meihdfs'
                  ? 'bg-cyan-500/10 text-cyan-400 border border-cyan-500/30'
                  : 'text-slate-400 hover:text-slate-200'
              }`}
            >
              <Table className="w-3.5 h-3.5" />
              <span>{t('hdd.tabs.meihdfs', '256 Registros MEIHDFS (XOR 0xFF)')}</span>
            </button>

            <button
              type="button"
              onClick={() => onSelectTab('journal')}
              role="tab"
              aria-selected={activeTab === 'journal'}
              className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg font-semibold transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-cyan-400 ${
                activeTab === 'journal'
                  ? 'bg-cyan-500/10 text-cyan-400 border border-cyan-500/30'
                  : 'text-slate-400 hover:text-slate-200'
              }`}
            >
              <BookOpen className="w-3.5 h-3.5" />
              <span>{t('hdd.tabs.journal', 'Journal WAL (10 Blocos)')}</span>
            </button>

            <button
              type="button"
              onClick={() => onSelectTab('density')}
              role="tab"
              aria-selected={activeTab === 'density'}
              className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg font-semibold transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-cyan-400 ${
                activeTab === 'density'
                  ? 'bg-cyan-500/10 text-cyan-400 border border-cyan-500/30'
                  : 'text-slate-400 hover:text-slate-200'
              }`}
            >
              <BarChart3 className="w-3.5 h-3.5" />
              <span>{t('hdd.tabs.density', 'Mapa de Densidade (100 MB)')}</span>
            </button>
          </>
        )}

        {!isAdvanced && (
          <button
            type="button"
            onClick={onEnableAdvanced}
            className="ml-auto px-2.5 py-1 rounded-lg border border-slate-800 text-[11px] text-cyan-300 hover:bg-slate-800 transition-colors"
          >
            + Abas Forenses (MEIHDFS / WAL / Densidade)
          </button>
        )}

        {hasManifest && (
          <button
            type="button"
            onClick={() => onSelectTab('extract')}
            role="tab"
            aria-selected={activeTab === 'extract'}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg font-semibold transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-emerald-400 ${
              activeTab === 'extract'
                ? 'bg-emerald-500/10 text-emerald-400 border border-emerald-500/30'
                : 'text-slate-400 hover:text-slate-200'
            }`}
          >
            <FolderArchive className="w-3.5 h-3.5" />
            <span>Manifesto de Partições</span>
          </button>
        )}

        <button
          type="button"
          onClick={() => onSelectTab('capacity')}
          role="tab"
          aria-selected={activeTab === 'capacity'}
          className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg font-semibold transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-emerald-400 ${
            activeTab === 'capacity'
              ? 'bg-emerald-500/10 text-emerald-400 border border-emerald-500/30'
              : 'text-slate-400 hover:text-slate-200'
          }`}
        >
          <Unlock className="w-3.5 h-3.5" />
          <span>Desbloqueio de Capacidade (250GB / 2TB)</span>
        </button>

        <button
          type="button"
          onClick={() => onSelectTab('writer')}
          role="tab"
          aria-selected={activeTab === 'writer'}
          className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg font-semibold transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-purple-400 ${
            activeTab === 'writer'
              ? 'bg-purple-500/10 text-purple-400 border border-purple-500/30'
              : 'text-slate-400 hover:text-slate-200'
          }`}
        >
          <HardDrive className="w-3.5 h-3.5" />
          <span>Gravação Física & mkfs</span>
        </button>
      </div>
    </>
  );
};
