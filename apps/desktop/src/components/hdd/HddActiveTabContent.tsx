import React from 'react';
import type {
  HddAnalysisReport,
  HddTab,
  MeihdfsAllocationRecord,
  MeihdfsAllocationReport,
  PartitionExtractionManifest,
} from '@/models';
import {
  HddCapacityAndFlashPipeline,
  type HddSafeFlashVm,
} from './HddCapacityAndFlashPipeline';
import {
  HddCapacityPatcherCard,
  type HddCapacityPatcherCardProps,
} from './HddCapacityPatcherCard';
import { HddDensityMap } from './HddDensityMap';
import { HddExtractManifestCard } from './HddExtractManifestCard';
import { HddJournalTable } from './HddJournalTable';
import { HddMbrComparison } from './HddMbrComparison';
import { HddMeihdfsTable } from './HddMeihdfsTable';
import { HddPartitionTable } from './HddPartitionTable';
import { HddWriterCard, type HddWriterCardProps } from './HddWriterCard';

export interface HddActiveTabContentProps {
  readonly activeTab: HddTab;
  readonly hddReport: HddAnalysisReport | null;
  readonly meihdfsReport: MeihdfsAllocationReport | null;
  readonly filteredRecords: MeihdfsAllocationRecord[];
  readonly searchQuery: string;
  readonly setSearchQuery: (q: string) => void;
  readonly tagFilter: string;
  readonly setTagFilter: (tag: string) => void;
  readonly manifest: PartitionExtractionManifest | null;
  readonly copiedSha: boolean;
  readonly onCopySha: (text: string) => void;
  readonly capacityProps: HddCapacityPatcherCardProps;
  readonly writerProps: HddWriterCardProps;
  readonly safeFlashVm: HddSafeFlashVm;
}

export const HddActiveTabContent: React.FC<HddActiveTabContentProps> = ({
  activeTab,
  hddReport,
  meihdfsReport,
  filteredRecords,
  searchQuery,
  setSearchQuery,
  tagFilter,
  setTagFilter,
  manifest,
  copiedSha,
  onCopySha,
  capacityProps,
  writerProps,
  safeFlashVm,
}) => {
  switch (activeTab) {
    case 'overview':
      return hddReport ? <HddPartitionTable hddReport={hddReport} /> : null;
    case 'mbr':
      return hddReport ? <HddMbrComparison hddReport={hddReport} /> : null;
    case 'meihdfs':
      return (
        <HddMeihdfsTable
          filteredRecords={filteredRecords}
          searchQuery={searchQuery}
          tagFilter={tagFilter}
          onSearchChange={setSearchQuery}
          onTagFilterChange={setTagFilter}
        />
      );
    case 'journal':
      return meihdfsReport ? <HddJournalTable meihdfsReport={meihdfsReport} /> : null;
    case 'density':
      return hddReport ? <HddDensityMap hddReport={hddReport} /> : null;
    case 'extract':
      return manifest ? (
        <HddExtractManifestCard manifest={manifest} copiedSha={copiedSha} onCopySha={onCopySha} />
      ) : null;
    case 'capacity':
      return <HddCapacityPatcherCard {...capacityProps} />;
    case 'writer':
      return <HddWriterCard {...writerProps} />;
    case 'safe-flash':
      return <HddCapacityAndFlashPipeline vm={safeFlashVm} />;
    default:
      return null;
  }
};
