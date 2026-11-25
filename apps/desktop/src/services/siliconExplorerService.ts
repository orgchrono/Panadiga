/**
 * @file services/siliconExplorerService.ts
 * @brief Typed Tauri 2 IPC client for Hot Bare-Metal Silicon Explorer (`ndb>` over PicoFW BRIDGE).
 */

import { mintOperationNonceId } from './benchStreamService';
import { invokeCommand } from './ipc';

export type SiliconEvidenceKind = 'volatile_patch' | 'video_osd_diag';
export type SiliconAccessKind = 'read' | 'write';
export type SiliconPatchWidth = 'u16' | 'bytes';

export interface NdbDdResult {
  readonly base_va: number;
  readonly words: readonly number[];
  readonly bytes: readonly number[];
}

export interface NdbWriteEcho {
  readonly addr: number;
  readonly old_value: number;
  readonly new_value: number;
}

export interface NdbBcuFault {
  readonly bcberr_hex: string;
  readonly fault_address: number;
  readonly task_dump_count: number;
}

export interface VolatileDiffEntry {
  readonly addr: number;
  readonly before: number;
  readonly after: number;
}

export interface NdbLogReplaySummary {
  readonly source_name: string;
  readonly dd_reads: readonly NdbDdResult[];
  readonly write_echoes: readonly NdbWriteEcho[];
  readonly bcu_fault: NdbBcuFault | null;
  readonly standby_gate_tripped: boolean;
  readonly standby_gate_reason: string | null;
  readonly main3060np_osd_caveat: string | null;
  readonly cleaned_preview: string;
}

export interface SiliconBarrierCheck {
  readonly allowed: boolean;
  readonly normalizedAddrHex: string;
  readonly reason: string;
}

export interface VolatileDiffReport {
  readonly changes: readonly VolatileDiffEntry[];
  readonly before_snapshots: readonly NdbDdResult[];
  readonly after_snapshots: readonly NdbDdResult[];
}

export interface RollbackReport {
  readonly restored_ops: number;
  readonly restored_bytes: number;
  readonly verified_clean: boolean;
}

export const replaySiliconEvidenceLog = (
  logKind: SiliconEvidenceKind,
): Promise<NdbLogReplaySummary> =>
  invokeCommand<NdbLogReplaySummary>('silicon_explorer_replay_log', { logKind });

export const validateSiliconAddress = (
  addrHex: string,
  byteLen: number,
  accessKind: SiliconAccessKind,
): Promise<SiliconBarrierCheck> =>
  invokeCommand<SiliconBarrierCheck>('silicon_explorer_validate_address', {
    addrHex,
    byteLen,
    accessKind,
  });

export const diffSiliconSnapshots = (
  beforeRaw: string,
  afterRaw: string,
): Promise<VolatileDiffEntry[]> =>
  invokeCommand<VolatileDiffEntry[]>('silicon_explorer_diff_snapshots', {
    beforeRaw,
    afterRaw,
  });

export const captureSiliconLiveSnapshot = (
  portName: string,
  addrHex: string,
  byteLen: number,
): Promise<NdbDdResult> =>
  invokeCommand<NdbDdResult>('silicon_explorer_live_snapshot', {
    portName,
    addrHex,
    byteLen,
  });

export async function applySiliconVolatilePatch(
  portName: string,
  addrHex: string,
  patchHex: string,
  width: SiliconPatchWidth,
  confirmVolatile: boolean,
  nonceId?: string,
): Promise<VolatileDiffReport> {
  const resolvedNonce = await mintOperationNonceId(
    'ram_probe',
    addrHex.trim(),
    patchHex.trim() || '00',
    nonceId,
  );
  return invokeCommand<VolatileDiffReport>('silicon_explorer_apply_volatile_patch', {
    portName,
    addrHex,
    patchHex,
    width,
    confirmVolatile,
    ...(resolvedNonce ? { nonceId: resolvedNonce } : {}),
  });
}

export async function rollbackSiliconPatches(
  portName: string,
  nonceId?: string,
): Promise<RollbackReport> {
  const resolvedNonce = await mintOperationNonceId(
    'ram_probe',
    'ndb_rollback',
    'reverse_undo',
    nonceId,
  );
  return invokeCommand<RollbackReport>('silicon_explorer_rollback', {
    portName,
    ...(resolvedNonce ? { nonceId: resolvedNonce } : {}),
  });
}
