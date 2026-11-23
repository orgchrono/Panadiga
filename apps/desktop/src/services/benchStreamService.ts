/**
 * @file services/benchStreamService.ts
 * @brief Typed unidirectional `"panadiga://bench-event"` stream + pull snapshot + ephemeral operation nonces (`Plano B`).
 */

import {
  SSOT_BENCH_STREAM_LIMITS,
  SSOT_IPC_EVENTS,
  type BenchEventEnvelope,
  type BenchOpKind,
  type BenchSnapshot,
  type OperationNonceChallenge,
} from '@/models/generated/ssot';
import { invokeCommand, listenEvent } from './ipc';

export interface RequestOperationNonceInput {
  readonly opKind: BenchOpKind;
  readonly target: string;
  readonly inputSha256: string;
}

export interface BenchStreamCallbacks {
  readonly onSnapshot: (snapshot: BenchSnapshot) => void;
  readonly onEvent: (envelope: BenchEventEnvelope) => void;
  readonly onResync?: (expectedSeq: number, receivedSeq: number) => void;
}

/**
 * Mints a single-use short-lived confirmation nonce bound to exact physical parameters.
 */
export const requestOperationNonce = (
  request: RequestOperationNonceInput,
): Promise<OperationNonceChallenge> =>
  invokeCommand<OperationNonceChallenge>('request_operation_nonce', { request });

/**
 * Shared DRY helper requesting an ephemeral `nor_write` authorization nonce for EH55 live BGO repair.
 */
export const requestLiveBgoNonce = (
  norPath?: string,
  planPath?: string,
): Promise<OperationNonceChallenge> => {
  const inputSha256 = norPath?.trim() || planPath?.trim() || 'live_plan';
  return requestOperationNonce({
    opKind: 'nor_write',
    target: 'SA26..SA70',
    inputSha256,
  });
};

/**
 * Pulls the latest atomic backend snapshot from `get_bench_snapshot`.
 */
export const getBenchSnapshot = (): Promise<BenchSnapshot> =>
  invokeCommand<BenchSnapshot>('get_bench_snapshot');

/**
 * Subscribes directly to raw throttled envelopes on `"panadiga://bench-event"`.
 */
export const subscribeBenchStream = (
  handler: (envelope: BenchEventEnvelope) => void,
): Promise<() => void> => listenEvent(SSOT_IPC_EVENTS.BENCH_STREAM, handler);

function insertSortedEnvelope(
  acc: readonly BenchEventEnvelope[],
  item: BenchEventEnvelope,
): readonly BenchEventEnvelope[] {
  const splitAt = acc.findIndex(existing => existing.seq > item.seq);
  return splitAt < 0
    ? [...acc, item]
    : [...acc.slice(0, splitAt), item, ...acc.slice(splitAt)];
}

/**
 * Subscribes to `"panadiga://bench-event"` BEFORE pulling `get_bench_snapshot()`,
 * dropping stale envelopes (`seq <= snap.seq`) and automatically resyncing on sequence gaps.
 */
export async function subscribeBenchStreamWithSnapshot(
  callbacks: BenchStreamCallbacks,
): Promise<() => void> {
  const controller = new AbortController();
  const buffered = new Map<number, BenchEventEnvelope>();
  const flags = new Set<'hydrated' | 'resyncing'>();
  const seqCursor = new Map<'last', number>();

  const bufferEnvelope = (env: BenchEventEnvelope): void => {
    if (buffered.size >= SSOT_BENCH_STREAM_LIMITS.logBufferCapLines) {
      const oldestKey = buffered.keys().next().value;
      if (oldestKey !== undefined) {
        buffered.delete(oldestKey);
      }
    }
    buffered.set(env.seq, env);
  };

  const triggerResync = async (): Promise<void> => {
    if (flags.has('resyncing') || controller.signal.aborted) {
      return;
    }
    flags.add('resyncing');
    try {
      await pullSnapshotAndDrain();
    } catch {
      flags.delete('resyncing');
    }
  };

  const processEnvelope = (env: BenchEventEnvelope): void => {
    if (controller.signal.aborted) {
      return;
    }
    if (!flags.has('hydrated') || flags.has('resyncing')) {
      bufferEnvelope(env);
      return;
    }
    const lastSeq = seqCursor.get('last') ?? 0;
    if (env.seq <= lastSeq) {
      return;
    }
    const expectedSeq = lastSeq + 1;
    if (env.seq !== expectedSeq) {
      callbacks.onResync?.(expectedSeq, env.seq);
      bufferEnvelope(env);
      void triggerResync();
      return;
    }
    seqCursor.set('last', env.seq);
    callbacks.onEvent(env);
  };

  const pullSnapshotAndDrain = async (): Promise<void> => {
    const snap = await getBenchSnapshot();
    if (controller.signal.aborted) {
      return;
    }
    flags.add('hydrated');
    flags.delete('resyncing');
    seqCursor.set('last', snap.seq);
    callbacks.onSnapshot(snap);
    const orderedPending = Array.from(buffered.values()).reduce<readonly BenchEventEnvelope[]>(
      insertSortedEnvelope,
      [],
    );
    buffered.clear();
    orderedPending.forEach(item => {
      if (!controller.signal.aborted) {
        processEnvelope(item);
      }
    });
  };

  const unlisten = await subscribeBenchStream(processEnvelope);
  if (controller.signal.aborted) {
    unlisten();
    return () => {};
  }

  try {
    await pullSnapshotAndDrain();
  } catch (err) {
    unlisten();
    throw err;
  }

  return () => {
    controller.abort();
    buffered.clear();
    unlisten();
  };
}
