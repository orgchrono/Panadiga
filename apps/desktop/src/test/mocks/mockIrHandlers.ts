/**
 * Mock IPC handlers for Android-only DIGA Commander IR endpoints (`ir_*`).
 * Strictly confined to Vitest / Playwright test fixtures (`src/test/mocks/`).
 */

import type {
  DynamicKeymapProfile,
  DynamicRemoteKey,
  IrCatalogStats,
  IrPreparedSignal,
  PanasonicCatalogCodeset,
  PanasonicCatalogSummary,
  PanasonicCategory,
  VerifiedIrMacro,
} from '@/models/ir';

const MOCK_KEYS: readonly DynamicRemoteKey[] = [
  {
    id: 'curated_diga_eh55_eur7659_0',
    label: 'Power Toggle',
    section: 'POWER_INPUT',
    protocol: 'KASEIKYO',
    device: 0xb0,
    subdevice: 0x00,
    obc: 0x3d,
    respect_remote_mode: true,
    pronto_hex: null,
    tone: 'DANGER',
    confirm_required: false,
    wire_preview: '02 20 B0 00 3D 8D',
  },
  {
    id: 'curated_diga_eh55_eur7659_1',
    label: 'Play',
    section: 'TRANSPORT',
    protocol: 'KASEIKYO',
    device: 0xb0,
    subdevice: 0x00,
    obc: 0x0a,
    respect_remote_mode: true,
    pronto_hex: null,
    tone: 'SUCCESS',
    confirm_required: false,
    wire_preview: '02 20 B0 00 0A BA',
  },
  {
    id: 'curated_diga_eh55_eur7659_2',
    label: 'Red',
    section: 'TUNING_AUDIO',
    protocol: 'KASEIKYO',
    device: 0xb0,
    subdevice: 0x01,
    obc: 0x41,
    respect_remote_mode: true,
    pronto_hex: null,
    tone: 'DANGER',
    confirm_required: false,
    wire_preview: '02 20 B0 01 41 F0',
  },
];

const MOCK_SUMMARIES: readonly PanasonicCatalogSummary[] = [
  {
    id: 'curated_diga_eh55_eur7659',
    category: 'DIGA',
    primary_name: 'DMR-EH55 (EUR7659Y60 / EUR7659Y70 Verified Master)',
    models: ['DMR-EH55', 'EUR7659Y60', 'EUR7659Y70', 'DMR-EH50', 'DMR-EH65'],
    command_count: 69,
  },
  {
    id: 'curated_diga_combo_vhs_dvd',
    category: 'COMBO',
    primary_name: 'DMR-EH75V / ES30V / ES35V / ES46V / EZ48V Combo Verified',
    models: ['DMR-EH75V', 'DMR-ES30V', 'DMR-ES35V'],
    command_count: 82,
  },
  {
    id: 'curated_viera_tv_128',
    category: 'TV',
    primary_name: 'Panasonic Viera / CRT TV (0x80 Master)',
    models: ['Viera', 'TV-128-0'],
    command_count: 68,
  },
  {
    id: 'curated_panasonic_vcr_144',
    category: 'VCR',
    primary_name: 'Panasonic VCR Standalone (0x90 Master)',
    models: ['NV-HV60', 'PV-9662'],
    command_count: 58,
  },
];

const MOCK_MACROS: readonly VerifiedIrMacro[] = [
  {
    id: 'test_service_enter',
    title_pt: 'Entrar no Modo de Serviço (Mock Verificado)',
    title_en: 'Enter Service Mode (Verified Mock)',
    description_pt: 'Sequência de teste declarativa carregada via mock IPC.',
    description_en: 'Declarative test sequence loaded from mock IPC.',
    risk: 'SAFE',
    verification_source: 'Test SSOT',
    steps: [
      {
        label: 'Status',
        device: 0xb0,
        subdevice: 0x00,
        obc: 0x5f,
        hold_ms: 0,
        post_delay_ms: 50,
      },
    ],
  },
];

function computeWireHex(dev: number, sub: number, obc: number): string {
  const bytes = [0x02, 0x20, dev & 0xff, sub & 0xff, obc & 0xff, (dev ^ sub ^ obc) & 0xff];
  return bytes.map(b => b.toString(16).toUpperCase().padStart(2, '0')).join(' ');
}

export function handleMockIrCommand(
  cmd: string,
  args?: Record<string, unknown>,
): unknown {
  switch (cmd) {
    case 'ir_catalog_stats':
      return { codeset_count: 1763, command_count: 14744 } satisfies IrCatalogStats;

    case 'ir_search_catalog': {
      const q = String(args?.query ?? '').toLowerCase();
      const cat = String(args?.category ?? 'ALL') as PanasonicCategory;
      return MOCK_SUMMARIES.filter(
        s =>
          (cat === 'ALL' || s.category === cat) &&
          (!q || s.primary_name.toLowerCase().includes(q) || s.models.some(m => m.toLowerCase().includes(q))),
      );
    }

    case 'ir_assemble_codeset': {
      const id = String(args?.codesetId ?? 'curated_diga_eh55_eur7659');
      const hit = MOCK_SUMMARIES.find(s => s.id === id) ?? MOCK_SUMMARIES[0];
      const profile: DynamicKeymapProfile = {
        id: `profile_${hit.id}`,
        name: hit.primary_name,
        source_codeset_id: hit.id,
        category: hit.category,
        keys: [...MOCK_KEYS],
        is_user_modified: false,
      };
      const codeset: PanasonicCatalogCodeset = {
        id: hit.id,
        category: hit.category,
        models: [...hit.models],
        commands: [],
      };
      return { codeset, profile };
    }

    case 'ir_import_profile': {
      const name = String(args?.fallbackName ?? 'Imported Mock Profile');
      return {
        id: 'imported_mock',
        name,
        source_codeset_id: 'imported',
        category: 'DIGA',
        keys: [...MOCK_KEYS],
        is_user_modified: true,
      } satisfies DynamicKeymapProfile;
    }

    case 'ir_export_irplus': {
      const profile = args?.profile as DynamicKeymapProfile | undefined;
      return `<irplus><device manufacturer="PANASONIC" model="${profile?.name ?? 'DIGA'}" format="WINLIRC_NEC1" /></irplus>`;
    }

    case 'ir_prepare_transmission': {
      const req = (args?.request ?? {}) as Record<string, unknown>;
      const dev = Number(req.device ?? 0xb0);
      const sub = Number(req.subdevice ?? 0x00);
      const obc = Number(req.obc ?? 0x3d);
      const unitOverride = String(req.unitOverride ?? 'AUTO');
      const effectiveUnit =
        unitOverride === 'AUTO' ? 'MICROSECONDS' : (unitOverride as IrPreparedSignal['effective_unit']);
      const pattern = Array.from({ length: 100 }, (_, i) => (i % 2 === 0 ? 432 : 864));
      return {
        carrier_hz: 37010,
        pattern_us: pattern,
        encoded_pattern: pattern,
        sysfs_payload: `37010,${pattern.join(',')}`,
        expects_cycles_in_sysfs: effectiveUnit === 'CARRIER_CYCLES',
        recommended_unit: effectiveUnit,
        effective_unit: effectiveUnit,
        wire_hex: computeWireHex(dev, sub, obc),
        pronto_hex: '0000 0070 0000 0032 0080 0040',
      } satisfies IrPreparedSignal;
    }

    case 'ir_get_verified_macros':
      return MOCK_MACROS;

    default:
      return undefined;
  }
}
