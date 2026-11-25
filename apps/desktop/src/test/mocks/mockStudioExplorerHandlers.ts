/**
 * Isolated mock IPC handlers for DigaOS SSOT Studio and Hot Bare-Metal Silicon Explorer
 * used exclusively in Playwright E2E and Vitest harnesses (`src/test/mocks/`).
 */

import { DIGAOS_CONFIG, DIGAOS_SSOT_CONTRACT_HASH_STR, type DigaOsConfigSpec } from '@/models/generated/digaos';
import type {
  NdbDdResult,
  NdbLogReplaySummary,
  RollbackReport,
  SiliconBarrierCheck,
  VolatileDiffEntry,
  VolatileDiffReport,
} from '@/services/siliconExplorerService';
import type {
  SsotStudioBundle,
  SsotStudioSaveResult,
  SsotStudioValidationResult,
} from '@/services/ssotStudioService';
import { readBooleanProp, readNumberProp, readStringProp } from '@/utils/typeGuards';
import { readMockRequest } from './mockEventBus';

interface UndoMockEntry {
  readonly addr: number;
  readonly before: readonly number[];
  readonly after: readonly number[];
}

const mockStudioState: { config: DigaOsConfigSpec; hashHex: string } = {
  config: DIGAOS_CONFIG,
  hashHex: `0x${DIGAOS_SSOT_CONTRACT_HASH_STR}`,
};

const mockSiliconUndoStack: UndoMockEntry[] = [];

function checkSafeAddress(addr: number, byteLen: number, accessKind: string): SiliconBarrierCheck {
  const end = addr + Math.max(1, byteLen);
  const normalizedAddrHex = `0x${(addr >>> 0).toString(16).toUpperCase().padStart(8, '0')}`;
  if (
    (addr < 0xc000_1010 && end > 0xc000_100c) ||
    (addr < 0xdc0f_1000 && end > 0xdc0f_0000)
  ) {
    return {
      allowed: false,
      normalizedAddrHex,
      reason: `BLOCKED [LETHAL_MMIO]: ${accessKind} at ${normalizedAddrHex} touches fatal AM34 register (0xC000100C / 0xDC0F0000..0xDC0F0FFF).`,
    };
  }
  if (addr < 0xd400_4010 && end > 0xd400_4000) {
    return {
      allowed: false,
      normalizedAddrHex,
      reason: `BLOCKED [BCU_FAULT_GUARD]: ${accessKind} at ${normalizedAddrHex} overlaps BCU fault site 0xD4004000.`,
    };
  }
  if (accessKind.toLowerCase() === 'write' && addr >= 0x8000_0000 && addr < 0x8800_0000) {
    return {
      allowed: false,
      normalizedAddrHex,
      reason: `BLOCKED [E05_NOR_READONLY]: CS0 NOR Flash write at ${normalizedAddrHex} is permanently blocked.`,
    };
  }
  return {
    allowed: true,
    normalizedAddrHex,
    reason: `SAFE: ${accessKind} access permitted at ${normalizedAddrHex}.`,
  };
}

function validateConfigJson(raw: string): {
  readonly valid: boolean;
  readonly errors: readonly string[];
  readonly parsed?: DigaOsConfigSpec;
} {
  try {
    const parsed = JSON.parse(raw) as DigaOsConfigSpec;
    for (const prof of parsed.power_profiles ?? []) {
      for (const rng of prof.allowed_mmio_writes ?? []) {
        const addr = Number.parseInt(String(rng.start).replace(/^0x/i, ''), 16);
        const check = checkSafeAddress(addr, Number(rng.size ?? 4), 'write');
        if (!check.allowed) {
          return { valid: false, errors: [check.reason] };
        }
      }
    }
    return { valid: true, errors: [], parsed };
  } catch (err) {
    return { valid: false, errors: [String(err)] };
  }
}

export function handleMockStudioExplorerCommand(
  cmd: string,
  args?: Record<string, unknown>,
): unknown | undefined {
  const req = readMockRequest(args);
  switch (cmd) {
    case 'ssot_studio_get_bundle': {
      const bundle: SsotStudioBundle = {
        contractHash: Number.parseInt(mockStudioState.hashHex.replace(/^0x/i, ''), 16),
        contractHashHex: mockStudioState.hashHex,
        config: mockStudioState.config,
        rawJson: JSON.stringify(mockStudioState.config, null, 2),
        configPath: 'profiles/digaos/default_digaos_config.json',
      };
      return bundle;
    }

    case 'ssot_studio_validate_config': {
      const raw = readStringProp(req, 'configJson') ?? '{}';
      const checked = validateConfigJson(raw);
      const res: SsotStudioValidationResult = {
        valid: checked.valid,
        errors: checked.errors,
        contractHash: Number.parseInt(mockStudioState.hashHex.replace(/^0x/i, ''), 16),
        contractHashHex: mockStudioState.hashHex,
      };
      return res;
    }

    case 'ssot_studio_save_custom_config': {
      const raw = readStringProp(req, 'configJson') ?? '{}';
      const checked = validateConfigJson(raw);
      if (!checked.valid || !checked.parsed) {
        throw new Error(checked.errors.join('; '));
      }
      mockStudioState.config = checked.parsed;
      const saved: SsotStudioSaveResult = {
        saved: true,
        contractHash: Number.parseInt(mockStudioState.hashHex.replace(/^0x/i, ''), 16),
        contractHashHex: mockStudioState.hashHex,
        updatedPaths: [
          'profiles/digaos/default_digaos_config.json',
          'apps/desktop/src/models/generated/digaos.ts',
        ],
        message: `Saved DigaOS SSOT configuration cleanly (DIGAOS_SSOT_CONTRACT_HASH ${mockStudioState.hashHex}).`,
      };
      return saved;
    }

    case 'silicon_explorer_replay_log': {
      const kind = readStringProp(req, 'logKind') ?? 'volatile_patch';
      if (kind === 'video_osd_diag') {
        const diag: NdbLogReplaySummary = {
          source_name: 'com10_live_video_osd_diag_20261124.log',
          dd_reads: [],
          write_echoes: [],
          bcu_fault: {
            bcberr_hex: '00002102',
            fault_address: 0xd4004000,
            task_dump_count: 57,
          },
          standby_gate_tripped: true,
          standby_gate_reason: 'STANDBY_GATE_STOP: VDD=0.04V < 2.50V (NSW5V=0)',
          main3060np_osd_caveat: null,
          cleaned_preview: 'BCBERR: 00002102 Address: D4004000',
        };
        return diag;
      }
      const vol: NdbLogReplaySummary = {
        source_name: 'com10_live_volatile_patch_20261124.log',
        dd_reads: [
          {
            base_va: 0x570df348,
            words: [0xf0800080, 0x00cbfcf0],
            bytes: [0x80, 0x00, 0xf0, 0xfc, 0xcb],
          },
        ],
        write_echoes: [
          { addr: 0x8800002c, old_value: 0xc0f0, new_value: 0x40f0 },
        ],
        bcu_fault: null,
        standby_gate_tripped: false,
        standby_gate_reason: null,
        main3060np_osd_caveat:
          'MAIN3060NP build limitation: OSD menus still lack Western/Portuguese glyphs because /HDDEX/UDA_M6930_H_JPN_V006 is absent on Brazilian HDDs.',
        cleaned_preview: '570DF348 -> 570DF34A | 8800002C (AIOF_FS0): C0F0 -> 40F0',
      };
      return vol;
    }

    case 'silicon_explorer_validate_address': {
      const addrStr = readStringProp(req, 'addrHex') ?? '0x570DF34A';
      const byteLen = readNumberProp(req, 'byteLen') ?? 2;
      const accessKind = readStringProp(req, 'accessKind') ?? 'write';
      const addr = Number.parseInt(addrStr.replace(/^0x/i, ''), 16);
      return checkSafeAddress(addr, byteLen, accessKind);
    }

    case 'silicon_explorer_apply_volatile_patch': {
      const confirm = readBooleanProp(req, 'confirmVolatile') ?? false;
      if (!confirm) {
        throw new Error('Explicit volatile confirmation required.');
      }
      const addrStr = readStringProp(req, 'addrHex') ?? '0x570DF34A';
      const patchHex = (readStringProp(req, 'patchHex') ?? '40F0').replace(/^0x/i, '').replace(/[\s_]/g, '');
      const addr = Number.parseInt(addrStr.replace(/^0x/i, ''), 16);
      const afterBytes: number[] = [];
      for (let i = 0; i < patchHex.length; i += 2) {
        afterBytes.push(Number.parseInt(patchHex.slice(i, i + 2), 16));
      }
      const barrier = checkSafeAddress(addr, afterBytes.length, 'write');
      if (!barrier.allowed) {
        throw new Error(barrier.reason);
      }
      const beforeBytes = afterBytes.map((_, idx) => (idx === 0 ? 0xf0 : 0xfc));
      mockSiliconUndoStack.push({ addr, before: beforeBytes, after: afterBytes });
      const changes: VolatileDiffEntry[] = afterBytes.map((afterVal, idx) => ({
        addr: addr + idx,
        before: beforeBytes[idx] ?? 0,
        after: afterVal,
      }));
      const beforeSnap: NdbDdResult = { base_va: addr & ~3, words: [0xfcf00080], bytes: beforeBytes };
      const afterSnap: NdbDdResult = { base_va: addr & ~3, words: [0x40f00080], bytes: afterBytes };
      const report: VolatileDiffReport = {
        changes,
        before_snapshots: [beforeSnap],
        after_snapshots: [afterSnap],
      };
      return report;
    }

    case 'silicon_explorer_rollback': {
      const popped = mockSiliconUndoStack.splice(0, mockSiliconUndoStack.length);
      const restoredBytes = popped.reduce((sum, entry) => sum + entry.before.length, 0) || 2;
      const report: RollbackReport = {
        restored_ops: popped.length || 1,
        restored_bytes: restoredBytes,
        verified_clean: true,
      };
      return report;
    }

    default:
      return undefined;
  }
}
