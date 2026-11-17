import { useCallback, useEffect, useState } from 'react';
import type {
  ExtrheadUnpackReport,
  FrmPackReport,
  PackFrmRequest,
  UnpackExtrheadParams,
} from '@/models';
import {
  listFrmContainers,
  packFrmContainer,
  unpackExtrheadPayload,
} from '@/services/storageCryptoService';
import { logger } from '@/utils/logger';

export type { UnpackExtrheadParams };

/**
 * ViewModel for listing .FRM containers, packing PANA__SD.FRM / PANA_DVD.FRM / DIGADIAG.FRM,
 * and unpacking/verifying EXTRHEAD/EXTRFOOT payloads via native Rust backend.
 */
export function useFrmPackager() {
  const [containers, setContainers] = useState<string[]>([]);
  const [loadingContainers, setLoadingContainers] = useState<boolean>(true);

  const [packLoading, setPackLoading] = useState<boolean>(false);
  const [packError, setPackError] = useState<string | null>(null);
  const [packResult, setPackResult] = useState<FrmPackReport | null>(null);

  const [unpackLoading, setUnpackLoading] = useState<boolean>(false);
  const [unpackError, setUnpackError] = useState<string | null>(null);
  const [unpackResult, setUnpackResult] = useState<ExtrheadUnpackReport | null>(null);

  const refreshContainers = useCallback(async () => {
    setLoadingContainers(true);
    try {
      const list = await listFrmContainers();
      setContainers(Array.isArray(list) ? list : []);
    } catch (err) {
      logger.warn('FrmPackager', 'Failed listing .FRM containers', err);
    } finally {
      setLoadingContainers(false);
    }
  }, []);

  useEffect(() => {
    void Promise.resolve().then(() => refreshContainers());
  }, [refreshContainers]);

  const packFrm = useCallback(
    async (request: PackFrmRequest): Promise<FrmPackReport | null> => {
      setPackLoading(true);
      setPackError(null);
      try {
        const report = await packFrmContainer(request);
        setPackResult(report);
        await refreshContainers();
        return report;
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        setPackError(msg);
        return null;
      } finally {
        setPackLoading(false);
      }
    },
    [refreshContainers],
  );

  const unpackExtrhead = useCallback(
    async ({ inputPath, mode, outPath }: UnpackExtrheadParams): Promise<ExtrheadUnpackReport | null> => {
      setUnpackLoading(true);
      setUnpackError(null);
      try {
        const trimmedOut = outPath?.trim();
        const report = await unpackExtrheadPayload({
          inputPath: inputPath.trim(),
          mode,
          outPath: trimmedOut || undefined,
        });
        setUnpackResult(report);
        return report;
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        setUnpackError(msg);
        return null;
      } finally {
        setUnpackLoading(false);
      }
    },
    [],
  );

  const isLoading = loadingContainers || packLoading || unpackLoading;

  return {
    containers,
    isLoading,
    loadingContainers,
    refreshContainers,
    packLoading,
    packError,
    packResult,
    packFrm,
    unpackLoading,
    unpackError,
    unpackResult,
    unpackExtrhead,
  };
}
