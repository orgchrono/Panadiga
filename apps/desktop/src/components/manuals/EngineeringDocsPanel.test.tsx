import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import React from 'react';
import type { Am34BootLogReport, EngineeringDocEntry } from '@/models';
import { EngineeringDocsPanel } from './EngineeringDocsPanel';

const SAMPLE_DOCS: EngineeringDocEntry[] = [
  {
    locale: 'en-US',
    file_name: 'RE_REPORT_FEISTEL_TOC_EXTRHEAD_AND_UART0_GATES.md',
    relative_path: 'en-US/RE_REPORT_FEISTEL_TOC_EXTRHEAD_AND_UART0_GATES.md',
    title: 'Feistel & UART0 Report',
    size_bytes: 4096,
  },
  {
    locale: 'pt-BR',
    file_name: 'RELATORIO_RE_FEISTEL_TOC_EXTRHEAD_E_GATES_UART0.md',
    relative_path: 'pt-BR/RELATORIO_RE_FEISTEL_TOC_EXTRHEAD_E_GATES_UART0.md',
    title: 'Relatório Feistel & UART0',
    size_bytes: 4200,
  },
  {
    locale: 'raw_txt',
    file_name: 'doc_boot_core.txt',
    relative_path: 'pseudocode/raw/doc_boot_core.txt',
    title: 'doc_boot_core.txt',
    size_bytes: 97709,
  },
  {
    locale: 'debug_logs',
    file_name: 'captura_sd_flat_fix_20261115_025903.log',
    relative_path: 'debug_logs/captura_sd_flat_fix_20261115_025903.log',
    title: 'captura_sd_flat_fix_20261115_025903.log',
    size_bytes: 3265,
  },
];

const MARKDOWN_SAMPLE = [
  '> 🌐 **Language / Idioma:** **English** | [Português (BR)](../pt-BR/RELATORIO_RE_FEISTEL_TOC_EXTRHEAD_E_GATES_UART0.md)',
  '> 🧭 **Portal:** [Docs Index](../INDEX.md) | [Ghidra Pseudocode](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md)',
  '',
  '# Feistel & UART0 Report',
  '',
  '## Verification Section',
  'Inspect [Raw Boot Core](../pseudocode/raw/doc_boot_core.txt#L10-L20) or `crates/panadiga-core/src/pico/log_decoder.rs`.',
].join('\n');

const DECODED_LOG_SAMPLE: Am34BootLogReport = {
  loader_version: '0.61',
  boot_mode: 'cd_diag',
  bgo_failed_code: null,
  startup_reason: '5',
  sd_capacity_lba: 0x3c4000,
  sdcd_errors: [-91006],
  drive_error_codes: ['80040904'],
  sense_packets: [],
  vdd_volts: 3.25,
  dump_line_count: 14,
  panarsp_summaries: [],
  notes: ['SD 4-bit mode initialized at 25000 Kbps'],
};

function renderDocsPanel(overrides: Partial<React.ComponentProps<typeof EngineeringDocsPanel>> = {}) {
  const props: React.ComponentProps<typeof EngineeringDocsPanel> = {
    docs: SAMPLE_DOCS,
    allDocs: SAMPLE_DOCS,
    localeFilter: 'all',
    onLocaleFilterChange: vi.fn(),
    searchQuery: '',
    onSearchQueryChange: vi.fn(),
    selectedDoc: SAMPLE_DOCS[0],
    content: MARKDOWN_SAMPLE,
    loading: false,
    activeAnchor: null,
    onSelectAnchor: vi.fn(),
    decodedLog: null,
    canGoBack: true,
    canGoForward: false,
    onGoBack: vi.fn(),
    onGoForward: vi.fn(),
    onSelectDoc: vi.fn(),
    onNavigateReference: vi.fn(),
    ...overrides,
  };
  return { props, ...render(<EngineeringDocsPanel {...props} />) };
}

describe('EngineeringDocsPanel', () => {
  beforeEach(() => {
    Element.prototype.scrollIntoView = vi.fn();
    if (typeof globalThis.CSS === 'undefined') {
      Object.defineProperty(globalThis, 'CSS', {
        value: { escape: (val: string) => val },
        configurable: true,
      });
    } else if (!globalThis.CSS.escape) {
      globalThis.CSS.escape = (val: string) => val;
    }
  });

  it('supports doc selection, search/locale filters, TOC jump, cross-reference navigation, and language switching', () => {
    const { props } = renderDocsPanel();

    fireEvent.click(screen.getByTestId('eng-docs-filter-debug_logs'));
    expect(props.onLocaleFilterChange).toHaveBeenCalledWith('debug_logs');

    fireEvent.change(screen.getByTestId('eng-docs-search-input'), { target: { value: 'Feistel' } });
    expect(props.onSearchQueryChange).toHaveBeenCalledWith('Feistel');

    fireEvent.click(screen.getByText('doc_boot_core.txt'));
    expect(props.onSelectDoc).toHaveBeenCalledWith(SAMPLE_DOCS[2]);

    fireEvent.click(screen.getByText('Raw Boot Core'));
    expect(props.onNavigateReference).toHaveBeenCalledWith('../pseudocode/raw/doc_boot_core.txt#L10-L20');

    fireEvent.click(screen.getByText('crates/panadiga-core/src/pico/log_decoder.rs'));
    expect(props.onNavigateReference).toHaveBeenCalledWith('crates/panadiga-core/src/pico/log_decoder.rs');

    fireEvent.change(screen.getByLabelText(/Jump to section|Ir para seção/i), {
      target: { value: 'verification-section' },
    });
    expect(props.onSelectAnchor).toHaveBeenCalledWith('verification-section');

    const switchBtn = screen.getByTestId('eng-docs-switch-lang');
    expect(switchBtn.textContent).toContain('pt-BR');
    fireEvent.click(switchBtn);
    expect(props.onSelectDoc).toHaveBeenCalledWith(SAMPLE_DOCS[1]);

    fireEvent.click(screen.getByRole('button', { name: /Back|Voltar/i }));
    expect(props.onGoBack).toHaveBeenCalled();
  });

  it('renders raw .log captures with decoded COM10 telemetry banner and line-range highlighting', () => {
    renderDocsPanel({
      selectedDoc: SAMPLE_DOCS[3],
      content: '***** Loader ver0.61 *****\nSD = 0x3C4000[LBA]\nSD diag load failed[0].',
      activeAnchor: 'L2-L3',
      decodedLog: DECODED_LOG_SAMPLE,
    });

    expect(screen.getByText(/Loader:\s*0\.61/)).toBeTruthy();
    expect(screen.getByText(/\*\*\*\*\* Loader ver0\.61 \*\*\*\*\*/)).toBeTruthy();
    expect(screen.getByText(/cd_diag/)).toBeTruthy();
    expect(screen.getByText(/SD LBA:\s*0x3C4000/)).toBeTruthy();
    expect(screen.getByText(/VDD:\s*3\.25V/)).toBeTruthy();
    expect(screen.getByText(/-91006, 80040904/)).toBeTruthy();
    expect(screen.getByText(/SD 4-bit mode initialized at 25000 Kbps/)).toBeTruthy();

    const lineTwoRow = document.getElementById('line-2');
    expect(lineTwoRow?.className).toContain('bg-amber-500/20');
  });
});
