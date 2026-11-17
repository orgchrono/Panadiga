import { useCallback, useEffect, useMemo, useState } from 'react';
import { CLIPBOARD_FEEDBACK_MS } from '@/constants';
import { useToast } from '@/context/ToastContext';
import { useManagedTimeouts } from '@/hooks/useManagedTimeouts';
import { translate } from '@/i18n/useTranslation';
import type {
  HddAnalysisReport,
  HddTab,
  MeihdfsAllocationReport,
  PartitionExtractionManifest,
} from '@/models';
import { copyToClipboard } from '@/services/browserActions';
import { extractHddPartitions, inspectHddImage } from '@/services/storageCryptoService';
import { emitHddInspectedEvent } from '@/services/systemService';
import { logger } from '@/utils/logger';

export type { HddTab };

export function useHddStorageViewModel() {
  const { showToast } = useToast();
  const { scheduleTimeout, clearAllTimeouts } = useManagedTimeouts();
  const [hddReport, setHddReport] = useState<HddAnalysisReport | null>(null);
  const [meihdfsReport, setMeihdfsReport] = useState<MeihdfsAllocationReport | null>(null);
  const [activeTab, setActiveTab] = useState<HddTab>('overview');
  const [loading, setLoading] = useState<boolean>(false);
  const [extracting, setExtracting] = useState<boolean>(false);
  const [copiedSha, setCopiedSha] = useState<boolean>(false);
  const [manifest, setManifest] = useState<PartitionExtractionManifest | null>(null);
  const [tagFilter, setTagFilter] = useState<string>('all');
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [imagePath, setImagePath] = useState<string>('');
  const [error, setError] = useState<string | null>(null);

  const loadData = useCallback(
    async (customPath?: string) => {
      setLoading(true);
      setError(null);
      const targetPath = customPath !== undefined ? customPath : imagePath;
      try {
        const { report, allocReport } = await inspectHddImage(targetPath);
        setHddReport(report);
        setMeihdfsReport(allocReport);
        emitHddInspectedEvent(true);
        if (report?.image_path) {
          setImagePath(report.image_path);
        }
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        setError(msg);
        logger.warn('HddStorageViewModel', 'Failed loading HDD forensics', err);
      } finally {
        setLoading(false);
      }
    },
    [imagePath],
  );

  useEffect(() => {
    const controller = new AbortController();
    void Promise.resolve().then(() => {
      if (!controller.signal.aborted) {
        void loadData();
      }
    });
    return () => controller.abort();
  }, [loadData]);

  const extractPartitions = useCallback(async () => {
    setExtracting(true);
    try {
      const res = await extractHddPartitions(imagePath);
      setManifest(res);
      setActiveTab('extract');
      emitHddInspectedEvent(true);
      showToast({
        type: 'success',
        title: translate('errors.hdd.extractSuccess'),
        message: translate('errors.hdd.extractSavedMsg', {
          count: res.extractedPartitions.length,
          dir: res.outputDirectory,
        }),
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: translate('errors.hdd.extractError'),
        message: err instanceof Error ? err.message : String(err),
      });
      logger.error('HddStorageViewModel', 'Failed extracting partitions', err);
    } finally {
      setExtracting(false);
    }
  }, [imagePath, showToast]);

  const filteredRecords = useMemo(() => {
    if (!meihdfsReport) return [];
    const upperQuery = searchQuery.toUpperCase();
    const lowerQuery = searchQuery.toLowerCase();
    return meihdfsReport.records.filter((rec) => {
      const matchesTag = tagFilter === 'all' || rec.tag_name.includes(tagFilter);
      const matchesSearch =
        searchQuery === '' ||
        rec.raw_hex.includes(upperQuery) ||
        rec.inverted_hex.includes(upperQuery) ||
        rec.description.toLowerCase().includes(lowerQuery) ||
        rec.index.toString() === searchQuery;
      return matchesTag && matchesSearch;
    });
  }, [meihdfsReport, tagFilter, searchQuery]);

  const copyManifestSha = useCallback(
    (text: string) => {
      void copyToClipboard(text);
      clearAllTimeouts();
      setCopiedSha(true);
      showToast({
        type: 'info',
        title: translate('hdd.shaCopiedToastTitle'),
        message: translate('hdd.shaCopiedToastMsg'),
      });
      scheduleTimeout(() => setCopiedSha(false), CLIPBOARD_FEEDBACK_MS);
    },
    [clearAllTimeouts, scheduleTimeout, showToast],
  );

  return {
    hddReport,
    meihdfsReport,
    filteredRecords,
    activeTab,
    setActiveTab,
    loading,
    extracting,
    copiedSha,
    copyManifestSha,
    manifest,
    tagFilter,
    setTagFilter,
    searchQuery,
    setSearchQuery,
    imagePath,
    setImagePath,
    error,
    extractPartitions,
    refresh: loadData,
  };
}
