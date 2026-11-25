/**
 * @file viewmodels/useSiliconExplorerViewModel.ts
 * @brief MVVM closure hook for Hot Bare-Metal Silicon Explorer (ndb> evidence replay, barriers, hot diff & rollback).
 */

import { useCallback, useState } from 'react';
import { useAsyncMountEffect } from '@/hooks/useAsyncMountEffect';
import { listPicoSerialPorts } from '@/services/picoService';
import {
  applySiliconVolatilePatch,
  captureSiliconLiveSnapshot,
  replaySiliconEvidenceLog,
  rollbackSiliconPatches,
  validateSiliconAddress,
  type NdbDdResult,
  type NdbLogReplaySummary,
  type RollbackReport,
  type SiliconPatchWidth,
  type VolatileDiffReport,
} from '@/services/siliconExplorerService';

export type { SiliconPatchWidth };

export interface SiliconExplorerViewModel {
  readonly ports: readonly string[];
  readonly selectedPort: string;
  readonly setSelectedPort: (port: string) => void;
  readonly replaySummary: NdbLogReplaySummary | null;
  readonly liveSnapshot: NdbDdResult | null;
  readonly targetAddrHex: string;
  readonly setTargetAddrHex: (addr: string) => void;
  readonly patchHex: string;
  readonly setPatchHex: (hex: string) => void;
  readonly width: SiliconPatchWidth;
  readonly setWidth: (width: SiliconPatchWidth) => void;
  readonly confirmVolatile: boolean;
  readonly setConfirmVolatile: (checked: boolean) => void;
  readonly barrierError: string | null;
  readonly patchReport: VolatileDiffReport | null;
  readonly rollbackReport: RollbackReport | null;
  readonly statusMessage: string | null;
  readonly isLoading: boolean;
  readonly busy: boolean;
  readonly replayVolatilePatchLog: () => Promise<void>;
  readonly replayVideoOsdDiagLog: () => Promise<void>;
  readonly readLiveSnapshot: () => Promise<void>;
  readonly applyVolatilePatch: () => Promise<void>;
  readonly rollbackLastPatches: () => Promise<void>;
}

const DEFAULT_PORT = 'COM10';
const DEFAULT_SDRAM_ADDR = '0x570DF34A';
const DEFAULT_PATCH_HEX = '40F0';

function estimatePatchByteLen(patchHex: string): number {
  const clean = patchHex.replace(/^0x/i, '').replace(/[\s_]/g, '');
  return Math.max(1, Math.floor(clean.length / 2));
}

const LIVE_SNAPSHOT_DEFAULT_BYTES = 16;

export function useSiliconExplorerViewModel(): SiliconExplorerViewModel {
  const [ports, setPorts] = useState<readonly string[]>([DEFAULT_PORT]);
  const [selectedPort, setSelectedPort] = useState<string>(DEFAULT_PORT);
  const [replaySummary, setReplaySummary] = useState<NdbLogReplaySummary | null>(null);
  const [liveSnapshot, setLiveSnapshot] = useState<NdbDdResult | null>(null);
  const [targetAddrHex, setTargetAddrHex] = useState<string>(DEFAULT_SDRAM_ADDR);
  const [patchHex, setPatchHex] = useState<string>(DEFAULT_PATCH_HEX);
  const [width, setWidth] = useState<SiliconPatchWidth>('u16');
  const [confirmVolatile, setConfirmVolatile] = useState<boolean>(false);
  const [barrierError, setBarrierError] = useState<string | null>(null);
  const [patchReport, setPatchReport] = useState<VolatileDiffReport | null>(null);
  const [rollbackReport, setRollbackReport] = useState<RollbackReport | null>(null);
  const [statusMessage, setStatusMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState<boolean>(false);

  const applyPorts = useCallback((found: string[]) => {
    if (!Array.isArray(found) || found.length === 0) {
      return;
    }
    setPorts(found);
    setSelectedPort(found[0]);
  }, []);

  const isLoading = useAsyncMountEffect(listPicoSerialPorts, applyPorts);

  const runReplay = useCallback(async (kind: 'volatile_patch' | 'video_osd_diag') => {
    setBusy(true);
    setBarrierError(null);
    try {
      const summary = await replaySiliconEvidenceLog(kind);
      setReplaySummary(summary);
      setStatusMessage(`Replayed ${summary.source_name} deterministically.`);
    } catch (err) {
      setBarrierError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }, []);

  const replayVolatilePatchLog = useCallback(() => runReplay('volatile_patch'), [runReplay]);
  const replayVideoOsdDiagLog = useCallback(() => runReplay('video_osd_diag'), [runReplay]);

  const readLiveSnapshot = useCallback(async () => {
    setBusy(true);
    setBarrierError(null);
    try {
      const barrier = await validateSiliconAddress(targetAddrHex, LIVE_SNAPSHOT_DEFAULT_BYTES, 'read');
      if (!barrier.allowed) {
        setBarrierError(barrier.reason);
        return;
      }
      const snap = await captureSiliconLiveSnapshot(selectedPort, targetAddrHex, LIVE_SNAPSHOT_DEFAULT_BYTES);
      setLiveSnapshot(snap);
      setStatusMessage(
        `Live ndb> dd capture at ${barrier.normalizedAddrHex} (${snap.bytes.length} B).`,
      );
    } catch (err) {
      setBarrierError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }, [selectedPort, targetAddrHex]);

  const applyVolatilePatch = useCallback(async () => {
    setBusy(true);
    setBarrierError(null);
    setRollbackReport(null);
    try {
      const byteLen = estimatePatchByteLen(patchHex);
      const barrier = await validateSiliconAddress(targetAddrHex, byteLen, 'write');
      if (!barrier.allowed) {
        setBarrierError(barrier.reason);
        return;
      }
      if (!confirmVolatile) {
        setBarrierError('Confirm volatile write checkbox before dispatching patch.');
        return;
      }
      const report = await applySiliconVolatilePatch(
        selectedPort,
        targetAddrHex,
        patchHex,
        width,
        true,
      );
      setPatchReport(report);
      setStatusMessage(
        `Applied diff (${report.changes.length} bytes changed) at ${barrier.normalizedAddrHex}.`,
      );
    } catch (err) {
      setBarrierError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }, [confirmVolatile, patchHex, selectedPort, targetAddrHex, width]);

  const rollbackLastPatches = useCallback(async () => {
    setBusy(true);
    setBarrierError(null);
    try {
      const rb = await rollbackSiliconPatches(selectedPort);
      setRollbackReport(rb);
      setStatusMessage(
        `Rollback verified clean: reverted ${rb.restored_ops} op(s) / ${rb.restored_bytes} B in reverse order.`,
      );
    } catch (err) {
      setBarrierError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }, [selectedPort]);

  return {
    ports,
    selectedPort,
    setSelectedPort,
    replaySummary,
    liveSnapshot,
    targetAddrHex,
    setTargetAddrHex,
    patchHex,
    setPatchHex,
    width,
    setWidth,
    confirmVolatile,
    setConfirmVolatile,
    barrierError,
    patchReport,
    rollbackReport,
    statusMessage,
    isLoading,
    busy,
    replayVolatilePatchLog,
    replayVideoOsdDiagLog,
    readLiveSnapshot,
    applyVolatilePatch,
    rollbackLastPatches,
  };
}
