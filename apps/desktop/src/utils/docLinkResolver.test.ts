import { describe, expect, it } from 'vitest';
import type { EngineeringDocEntry } from '@/models';
import {
  extractHeadings,
  findTranslationCounterpart,
  parseLineRangeAnchor,
  resolveDocReference,
} from './docLinkResolver';

describe('docLinkResolver', () => {
  it('resolves relative markdown, raw txt, debug log, root audit, and source references', () => {
    expect(resolveDocReference('../pseudocode/GHIDRA_UNDEF_AUDIT_REPORT.md', 'pt-BR/DOC.md')).toEqual({
      targetPath: 'pseudocode/GHIDRA_UNDEF_AUDIT_REPORT.md',
      anchor: undefined,
      isExternal: false,
    });
    expect(resolveDocReference('./raw/doc_boot_core.txt#L10-L25', 'pseudocode/INDICE.md')).toEqual({
      targetPath: 'pseudocode/raw/doc_boot_core.txt',
      anchor: 'L10-L25',
      isExternal: false,
    });
    expect(resolveDocReference('../../AUDITORIA_COMPLETA.md', 'pt-BR/DOC.md')).toEqual({
      targetPath: 'repo:AUDITORIA_COMPLETA.md',
      anchor: undefined,
      isExternal: false,
    });
    expect(
      resolveDocReference('../../crates/panadiga-core/src/pico/log_decoder.rs#L36-L47', 'en-US/DOC.md'),
    ).toEqual({
      targetPath: 'src:crates/panadiga-core/src/pico/log_decoder.rs',
      anchor: 'L36-L47',
      isExternal: false,
    });
    expect(resolveDocReference('en-US/GUIDE.md#section-2', 'INDEX.md')).toEqual({
      targetPath: 'en-US/GUIDE.md',
      anchor: 'section-2',
      isExternal: false,
    });
  });

  it('finds direct translation counterparts from header banner and root pairs', () => {
    const docs: EngineeringDocEntry[] = [
      {
        locale: 'en-US',
        file_name: 'MATSHITA_R6MV4_EMULATOR_AND_ATAPI_INJECTION.md',
        relative_path: 'en-US/MATSHITA_R6MV4_EMULATOR_AND_ATAPI_INJECTION.md',
        title: 'EN',
        size_bytes: 100,
      },
      {
        locale: 'pt-BR',
        file_name: 'EMULADOR_MATSHITA_R6MV4_E_INJECAO_ATAPI.md',
        relative_path: 'pt-BR/EMULADOR_MATSHITA_R6MV4_E_INJECAO_ATAPI.md',
        title: 'PT',
        size_bytes: 100,
      },
    ];
    const md = '> 🌐 [🇧🇷 Português](../pt-BR/EMULADOR_MATSHITA_R6MV4_E_INJECAO_ATAPI.md)\n# Title';
    expect(findTranslationCounterpart(md, docs, docs[0])?.relative_path).toBe(
      'pt-BR/EMULADOR_MATSHITA_R6MV4_E_INJECAO_ATAPI.md',
    );
  });

  it('extracts markdown headings and line range anchors cleanly', () => {
    expect(extractHeadings('# Top\n```\n# Inside Code\n```\n## Section Two')).toEqual([
      { level: 1, text: 'Top', slug: 'top' },
      { level: 2, text: 'Section Two', slug: 'section-two' },
    ]);
    expect(parseLineRangeAnchor('L12-L34')).toEqual({ start: 12, end: 34 });
  });
});
