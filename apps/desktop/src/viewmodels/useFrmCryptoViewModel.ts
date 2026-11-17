import { useState, useEffect, useCallback, useMemo } from 'react';
import type {
  CryptanalysisReport,
  FrmTocDumpReport,
  CodebookReport,
} from '@/models';
import { analyzeFrmBundle } from '@/services/storageCryptoService';
import { logger } from '@/utils/logger';

export type FrmTab = 'entropy' | 'toc' | 'packager' | 'codebook' | 'correlation';

export const DEFAULT_FRM_CONTAINER_PATH = 'firmware_and_images/PANA_DVD.FRM';

export function useFrmCryptoViewModel(initialFrmPath = DEFAULT_FRM_CONTAINER_PATH) {
  const [frmPath, setFrmPath] = useState<string>(initialFrmPath);
  const [cryptReport, setCryptReport] = useState<CryptanalysisReport | null>(null);
  const [tocReport, setTocReport] = useState<FrmTocDumpReport | null>(null);
  const [codebookReport, setCodebookReport] = useState<CodebookReport | null>(null);
  const [activeTab, setActiveTab] = useState<FrmTab>('entropy');
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [codebookFilter, setCodebookFilter] = useState<string>('');

  const loadData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [cryptOutcome, tocOutcome, codebookOutcome] = await analyzeFrmBundle(frmPath);

      if (cryptOutcome.status === 'fulfilled') {
        setCryptReport(cryptOutcome.value);
      } else {
        setCryptReport(null);
        logger.error('FrmCryptoViewModel', 'Failed analyzing FRM entropy', cryptOutcome.reason);
        setError(
          cryptOutcome.reason instanceof Error
            ? cryptOutcome.reason.message
            : String(cryptOutcome.reason),
        );
      }

      if (tocOutcome.status === 'fulfilled') {
        setTocReport(tocOutcome.value);
      } else {
        setTocReport(null);
        logger.error('FrmCryptoViewModel', 'Failed dumping FRM TOC', tocOutcome.reason);
      }

      if (codebookOutcome.status === 'fulfilled') {
        setCodebookReport(codebookOutcome.value);
      } else {
        setCodebookReport(null);
        logger.error('FrmCryptoViewModel', 'Failed getting codebook', codebookOutcome.reason);
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
      logger.error('FrmCryptoViewModel', 'Failed loading crypto forensic reports', err);
    } finally {
      setLoading(false);
    }
  }, [frmPath]);

  useEffect(() => {
    void Promise.resolve().then(() => loadData());
  }, [loadData]);

  const filteredCodebook = useMemo(() => {
    if (!codebookReport) return [];
    if (!codebookFilter.trim()) return codebookReport.dictionary;

    const query = codebookFilter.trim().toUpperCase();
    return codebookReport.dictionary.filter(
      entry =>
        entry.block_hex.toUpperCase().includes(query) ||
        (entry.known_plaintext_hex && entry.known_plaintext_hex.toUpperCase().includes(query)) ||
        entry.interpretation.toUpperCase().includes(query),
    );
  }, [codebookReport, codebookFilter]);

  return {
    frmPath,
    setFrmPath,
    cryptReport,
    tocReport,
    codebookReport,
    filteredCodebook,
    activeTab,
    setActiveTab,
    loading,
    error,
    codebookFilter,
    setCodebookFilter,
    refresh: loadData,
  };
}
