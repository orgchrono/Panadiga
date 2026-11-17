/**
 * Mock IPC handlers for system overview, manuals lookup, hex viewer, external tools, and RAG queries.
 */

import { CANONICAL_HARDWARE_SSOT_SPEC, DEFAULT_RUNTIME_SETTINGS } from '@/constants';
import { parseRuntimeSettings } from '@/utils/runtimeSettingsSchema';
import {
  generateMockHexDump,
  mockManualDocuments,
  mockManualSearchResults,
  mockSystemOverview,
} from './mockData';

export function handleMockSystemCommand(
  cmd: string,
  args?: Record<string, unknown>
): unknown | undefined {
  switch (cmd) {
    case 'get_hardware_ssot_spec':
      return CANONICAL_HARDWARE_SSOT_SPEC;

    case 'get_system_overview':
      return mockSystemOverview;

    case 'list_service_manuals':
      return mockManualDocuments.map(doc => ({
        file_name: doc.file_name,
        relative_path: doc.file_path,
        size_bytes: doc.size_bytes,
        formatted_size: `${(doc.size_bytes / (1024 * 1024)).toFixed(2)} MB`,
      }));

    case 'search_manuals': {
      const query = (args?.query as string)?.toUpperCase();
      if (!query) {
        return mockManualSearchResults;
      }
      return mockManualSearchResults.filter(item => (
        item.title.toUpperCase().includes(query) ||
        item.file_name.toUpperCase().includes(query) ||
        Object.keys(item.matches).some(k => k.toUpperCase().includes(query)) ||
        item.snippets.some(s => s.context.toUpperCase().includes(query))
      ));
    }

    case 'read_hex_dump': {
      const offset = (args?.offset as number) ?? 0;
      const count = (args?.count as number) ?? 32;
      return generateMockHexDump(offset, count);
    }

    case 'launch_external_tool_command': {
      const tool = (args?.tool as string) || 'desconhecido';
      return `[Modo Demonstração] Ferramenta externa '${tool}' executada com sucesso.`;
    }

    case 'query_rag_engine': {
      const q = (args?.query as string) || '';
      return {
        query: q,
        answer:
          'Error U99 indicates NVRAM corruption on the Spansion S29JL064H NOR Flash (micro-sectors SA134..SA141). Erase sectors SA134..SA141 (0x007F0000..0x007FFFFF) to restore virgin 0xFF state while preserving bootloader IPL at SA0..SA133.',
        confidence: 0.98,
        groundedness_score: 1.0,
        is_abstained: false,
        abstention_reason: null,
        citations: [
          {
            doc: 'manuals_and_schematics/dmr-eh55/service/panasonic_dmr_eh55p9_eh55pc9_service_manual_north_america.pdf',
            section: 'Self-Diagnostic and Error Codes',
            lines: 'P.21-22',
            excerpt: 'U99: System microprocessor communication fault or NVRAM checksum mismatch.',
          },
        ],
        safety: {
          is_safe: true,
          violations: [],
          alerts: [],
        },
        critic: {
          is_faithful: true,
          ungrounded_entities: [],
        },
        retrieved_chunks_count: 3,
        execution_time_ms: 12,
      };
    }

    case 'list_engineering_docs':
      return [
        {
          locale: 'root',
          file_name: 'INDEX.md',
          relative_path: 'INDEX.md',
          title: 'Panadiga Technical Documentation Index',
          size_bytes: 4096,
        },
        {
          locale: 'pt-BR',
          file_name: 'EMULADOR_MATSHITA_R6MV4_E_INJECAO_ATAPI.md',
          relative_path: 'pt-BR/EMULADOR_MATSHITA_R6MV4_E_INJECAO_ATAPI.md',
          title: 'Emulador Matshita R6-MV4 e Injeção ATAPI (0x3B / 0xED)',
          size_bytes: 8192,
        },
        {
          locale: 'en-US',
          file_name: 'MATSHITA_R6MV4_EMULATOR_AND_ATAPI_INJECTION.md',
          relative_path: 'en-US/MATSHITA_R6MV4_EMULATOR_AND_ATAPI_INJECTION.md',
          title: 'Matshita R6-MV4 Emulator and ATAPI Injection (0x3B / 0xED)',
          size_bytes: 8192,
        },
      ];

    case 'read_engineering_doc': {
      const rel = String(
        (args as Record<string, unknown> | undefined)?.relativePath ??
          (args as Record<string, unknown> | undefined)?.relative_path ??
          '',
      ).trim();
      if (!rel || rel.includes('..')) {
        throw new Error('Acesso bloqueado fora do diretório docs/.');
      }
      return `# ${rel}\n\n> 🌐 [🇺🇸 English](../en-US/MATSHITA_R6MV4_EMULATOR_AND_ATAPI_INJECTION.md) · [🇧🇷 Português](../pt-BR/EMULADOR_MATSHITA_R6MV4_E_INJECAO_ATAPI.md)\n\nMock engineering documentation content verified for \`${rel}\`.\n`;
    }

    case 'decode_engineering_log': {
      return {
        loader_version: 'Loader ver0.61 [06/03/06]',
        boot_mode: 'cd_diag',
        bgo_failed_code: 1,
        startup_reason: '5',
        sd_capacity_lba: 0x3c4000,
        sdcd_errors: [0],
        drive_error_codes: ['80045300'],
        sense_packets: [],
        vdd_volts: 3.25,
        dump_line_count: 0,
        panarsp_summaries: [],
        notes: ['Mock decoded COM10 log telemetry'],
        diagnosis: 'BGO driver init failed[1] -> U99 hang (ODD tray 80045300 also observed)',
        boot_stages: ['SD_HW_OK', 'BGO_FAIL', 'ODD_NO_TRAY_80045300'],
      };
    }

    case 'get_runtime_settings':
      return DEFAULT_RUNTIME_SETTINGS;

    case 'save_runtime_settings':
      return parseRuntimeSettings(args?.settings);

    case 'reset_runtime_settings':
      return DEFAULT_RUNTIME_SETTINGS;

    case 'log_frontend_event':
      return null;

    default:
      return undefined;
  }
}
