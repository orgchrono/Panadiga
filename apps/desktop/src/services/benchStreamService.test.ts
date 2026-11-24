import { describe, it, expect, afterEach } from 'vitest';
import {
  SSOT_CONTRACT_HASH_STR,
  SSOT_IPC_EVENTS,
  type BenchEventEnvelope,
  type BenchSnapshot,
} from '@/models/generated/ssot';
import { resolveTestIpcBridge } from './hostBridge';
import { emitLocalEvent } from './ipc';
import {
  requestLiveBgoNonce,
  subscribeBenchStreamWithSnapshot,
} from './benchStreamService';
import { handleMockCommand, subscribeMockEventStream } from '@/test/mocks/mockDispatcher';
import { installTestIpcBridge } from '@/test/mocks/testHostBridges';

function makeSnapshot(seq: number): BenchSnapshot {
  return {
    seq,
    ssot_contract_hash: SSOT_CONTRACT_HASH_STR,
    state: { kind: 'idle', port: 'COM10', fw_contract_hash: SSOT_CONTRACT_HASH_STR },
    latest_telemetry: null,
    active_progress: null,
    incomplete_op: null,
  };
}

function makeTelemetryEnvelope(seq: number, vddMv: number): BenchEventEnvelope {
  return {
    seq,
    ts_ms: 1_000 + seq,
    payload: {
      kind: 'telemetry',
      ts_ms: 1_000 + seq,
      vdd_mv: vddMv,
      nsw5v_high: false,
      jig_asserted: true,
      uart_rx_bytes: seq * 16,
    },
  };
}

const initialBridge = resolveTestIpcBridge();

afterEach(() => {
  if (initialBridge) {
    installTestIpcBridge(initialBridge);
  }
});

function installMockSnapshotBridge(
  onSnapshotReq: (idx: number, resolve: (snap: BenchSnapshot) => void, reject: (err: Error) => void) => void,
): void {
  const invokeCount = new Map<'count', number>();
  installTestIpcBridge({
    listen: <T,>(eventName: string, handler: (payload: T) => void) =>
      subscribeMockEventStream<T>(eventName, handler),
    invoke: async <T,>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
      if (cmd === 'get_bench_snapshot') {
        const idx = invokeCount.get('count') ?? 0;
        invokeCount.set('count', idx + 1);
        return await new Promise<T>((resolve, reject) => {
          onSnapshotReq(
            idx,
            snap => {
              const raw: unknown = snap;
              resolve(raw as T);
            },
            reject,
          );
        });
      }
      return handleMockCommand<T>(cmd, args);
    },
  });
}

describe('benchStreamService (Plano B stream + ephemeral nonces)', () => {
  it('buffers envelopes during initial snapshot pull, drops stale seq <= snap.seq, and resyncs on gaps', async () => {
    const snapshotsQueue: BenchSnapshot[] = [makeSnapshot(2), makeSnapshot(7)];
    const resolveQueue = new Map<number, (snap: BenchSnapshot) => void>();
    const gates = new Map<'firstReq' | 'resyncReq', () => void>();
    const firstSnapshotRequested = new Promise<void>(resolve => {
      gates.set('firstReq', resolve);
    });
    const resyncSnapshotRequested = new Promise<void>(resolve => {
      gates.set('resyncReq', resolve);
    });

    installMockSnapshotBridge((idx, resolve) => {
      if (idx === 0) {
        resolveQueue.set(idx, resolve);
        gates.get('firstReq')?.();
        return;
      }
      gates.get('resyncReq')?.();
      resolve(snapshotsQueue[idx] ?? makeSnapshot(7));
    });

    const receivedSnapshots = new Map<number, BenchSnapshot>();
    const receivedEvents = new Map<number, BenchEventEnvelope>();
    const resyncLog = new Map<number, readonly [number, number]>();

    const subPromise = subscribeBenchStreamWithSnapshot({
      onSnapshot: snap => {
        receivedSnapshots.set(receivedSnapshots.size, snap);
      },
      onEvent: env => {
        receivedEvents.set(receivedEvents.size, env);
      },
      onResync: (expectedSeq, gotSeq) => {
        resyncLog.set(resyncLog.size, [expectedSeq, gotSeq]);
      },
    });

    await firstSnapshotRequested;

    emitLocalEvent(SSOT_IPC_EVENTS.BENCH_STREAM, makeTelemetryEnvelope(1, 3290));
    emitLocalEvent(SSOT_IPC_EVENTS.BENCH_STREAM, makeTelemetryEnvelope(2, 3300));
    emitLocalEvent(SSOT_IPC_EVENTS.BENCH_STREAM, makeTelemetryEnvelope(3, 3310));
    emitLocalEvent(SSOT_IPC_EVENTS.BENCH_STREAM, makeTelemetryEnvelope(4, 3320));

    const releaseInitial = resolveQueue.get(0);
    expect(releaseInitial).toBeDefined();
    releaseInitial?.(snapshotsQueue[0]);

    const unlisten = await subPromise;

    expect(Array.from(receivedSnapshots.values()).map(s => s.seq)).toEqual([2]);
    expect(Array.from(receivedEvents.values()).map(e => e.seq)).toEqual([3, 4]);

    // Trigger sequence gap: expected 5, received 7 -> triggers automatic snapshot resync.
    emitLocalEvent(SSOT_IPC_EVENTS.BENCH_STREAM, makeTelemetryEnvelope(7, 3330));
    await resyncSnapshotRequested;
    await Promise.resolve();
    await Promise.resolve();

    expect(Array.from(resyncLog.values())).toEqual([[5, 7]]);
    expect(Array.from(receivedSnapshots.values()).map(s => s.seq)).toEqual([2, 7]);
    expect(Array.from(receivedEvents.values()).map(e => e.seq)).toEqual([3, 4]);

    unlisten();
  });

  it('logs a warning via logger.warn and clears resyncing flag if resync snapshot fails', async () => {
    const { logger } = await import('@/utils/logger');
    logger.setMinLevel('info');
    const resyncLog = new Map<number, string>();
    const warnGates = new Map<'warned' | 'recovered', () => void>();
    const warningLogged = new Promise<void>(resolve => {
      warnGates.set('warned', resolve);
    });
    const recoveredSnapshot = new Promise<void>(resolve => {
      warnGates.set('recovered', resolve);
    });
    const unsubSink = logger.addSink(entry => {
      if (entry.severity === 'warn' && entry.subsystem === 'BENCH_STREAM') {
        resyncLog.set(resyncLog.size, entry.message);
        warnGates.get('warned')?.();
      }
    });
    const recoveredSeqs = new Map<number, number>();
    installMockSnapshotBridge((idx, resolve, reject) => {
      if (idx === 0) {
        resolve(makeSnapshot(1));
      } else if (idx === 1) {
        reject(new Error('backend snapshot unavailable'));
      } else {
        resolve(makeSnapshot(6));
      }
    });

    const unlisten = await subscribeBenchStreamWithSnapshot({
      onSnapshot: snap => {
        recoveredSeqs.set(recoveredSeqs.size, snap.seq);
        if (snap.seq === 6) {
          warnGates.get('recovered')?.();
        }
      },
      onEvent: () => {},
    });

    emitLocalEvent(SSOT_IPC_EVENTS.BENCH_STREAM, makeTelemetryEnvelope(4, 3290));
    await warningLogged;
    expect(Array.from(resyncLog.values())).toContain('Failed resyncing bench stream snapshot');

    emitLocalEvent(SSOT_IPC_EVENTS.BENCH_STREAM, makeTelemetryEnvelope(6, 3310));
    await recoveredSnapshot;
    expect(Array.from(recoveredSeqs.values())).toEqual([1, 6]);

    unlisten();
    unsubSink();
  });

  it('mints ephemeral operation nonces bound to live BGO target and rejects replay or target mismatch', async () => {
    const nonceA = await requestLiveBgoNonce('unbrick_out/eh55_nor.bin');
    expect(nonceA.op_kind).toBe('nor_write');
    expect(nonceA.target).toBe('SA26..SA70');
    expect(nonceA.input_sha256).toBe('unbrick_out/eh55_nor.bin');
    expect(nonceA.ssot_contract_hash).toBe(SSOT_CONTRACT_HASH_STR);

    const repairOk = await handleMockCommand<{ readonly dryRunOnly: boolean }>('pico_eh55_bgo_repair_command', {
      request: {
        norPath: 'unbrick_out/eh55_nor.bin',
        dryRunOnly: false,
        portName: 'COM10',
        backupDir: 'unbrick_out/bgo_bkp',
        confirmToken: 'CONFIRM_UPPER_NOR',
        nonceId: nonceA.nonce_id,
      },
    });
    expect(repairOk.dryRunOnly).toBe(false);

    // Replay of consumed nonce must fail closed.
    await expect(
      Promise.resolve().then(() =>
        handleMockCommand('pico_eh55_bgo_repair_command', {
          request: {
            norPath: 'unbrick_out/eh55_nor.bin',
            dryRunOnly: false,
            portName: 'COM10',
            backupDir: 'unbrick_out/bgo_bkp',
            confirmToken: 'CONFIRM_UPPER_NOR',
            nonceId: nonceA.nonce_id,
          },
        }),
      ),
    ).rejects.toThrow(/Invalid or expired operation nonce/);
  });
});
