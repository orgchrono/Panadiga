/**
 * Panasonic Service Manual & Schematics Search Models
 * Matches src/manuals.rs & src/constants.rs
 */

export interface SearchMatchSnippet {
  term: string;
  page: number;
  context: string;
}

export interface PdfSearchResult {
  file_name: string;
  file_path: string;
  total_pages: number;
  matches: Record<string, number[]>;
}

export interface ManualSearchResult {
  file_name: string;
  file_path: string;
  total_pages: number;
  title: string;
  category: 'service_manual' | 'schematic' | 'operating_instructions' | 'bulletin';
  total_match_count: number;
  matches: Record<string, number[]>;
  snippets: SearchMatchSnippet[];
}

export interface ManualDocumentMetadata {
  id: string;
  file_name: string;
  file_path: string;
  title: string;
  description: string;
  category: 'service_manual' | 'schematic' | 'operating_instructions' | 'bulletin';
  size_bytes: number;
  page_count: number;
  key_ics: string[];
}

export interface ServiceManualCatalogItem {
  file_name: string;
  relative_path: string;
  size_bytes: number;
  formatted_size: string;
}

export type DocLocaleFilter =
  | 'all'
  | 'pt-BR'
  | 'en-US'
  | 'audits'
  | 'pseudocode'
  | 'raw_txt'
  | 'debug_logs'
  | 'root';

export interface EngineeringDocEntry {
  locale: 'root' | 'en-US' | 'pt-BR' | 'audits' | 'pseudocode' | 'raw_txt' | 'debug_logs' | string;
  file_name: string;
  relative_path: string;
  title: string;
  size_bytes: number;
}

export interface SensePacketReport {
  raw_hex: string;
  sense_key: number;
  asc: number;
  ascq: number;
  mecha_error_code: string | null;
  interpretation: string;
}

export interface Am34BootLogReport {
  loader_version: string | null;
  boot_mode: string | null;
  bgo_failed_code: number | null;
  startup_reason: string | null;
  sd_capacity_lba: number | null;
  sdcd_errors: number[];
  drive_error_codes: string[];
  sense_packets: SensePacketReport[];
  vdd_volts: number | null;
  dump_line_count: number;
  panarsp_summaries: string[];
  notes: string[];
}
