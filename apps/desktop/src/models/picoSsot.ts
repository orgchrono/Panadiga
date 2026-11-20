/**
 * Hardware SSOT, Transport & P7501 Pico Command Spec Models
 */

import type { ColdBreakinMode } from './pico';

export type PicoTransportMode = 'usb' | 'usb-otg' | 'ble';

export type PicoUf2Variant = 'unified' | 'rp2040' | 'rp2350';

export type PicoCommandCategory = 'electrical' | 'ocd_flash' | 'uart_fmu' | 'probe';

export interface HardwareThresholds {
  bus_freq_hz: number;
  nominal_vdd_volts: number;
  min_vdd_threshold_volts: number;
  safe_operating_min_volts: number;
  max_vdd_ceiling_volts: number;
  default_baud_rate: number;
  default_gdb_socket: string;
  ram_loader_base_hex: string;
  default_nvram_base_hex: string;
}

export interface PicoUsbIdentity {
  raspberry_pi_vid: number;
  rp2040_cdc_pid: number;
  rp2040_w_cdc_pid: number;
  rp2350_cdc_pid: number;
  rp2350_w_cdc_pid: number;
  rp2350_riscv_cdc_pid: number;
  bootsel_pid: number;
}

export interface P7501PinMapping {
  manual_pin: number;
  physical_pin: number;
  pin_label: string;
  pico_gpio: string;
  pico_pin: string;
  signal_name: string;
  direction: string;
  voltage: string;
  critical_note?: string | null;
}

export interface BreakInModeSpec {
  mode: ColdBreakinMode;
  label: string;
  badge: string;
  badge_variant: string;
  description: string;
  signal_diagram: string;
}

export interface FlashGeometrySpec {
  size_mb: number;
  size_bytes: number;
  chip_name: string;
  nvram_base_hex: string;
  nvram_size_bytes: number;
  microsector_range: string;
}

export interface MemoryRegionSpec {
  name: string;
  start: string;
  end: string;
  size_kb: number;
  perms: string;
}

export interface PicoCommandArgOption {
  label: string;
  value: string;
}

export interface PicoCommandSpec {
  id: string;
  category: PicoCommandCategory;
  label: string;
  description: string;
  syntax: string;
  arg_label?: string | null;
  arg_placeholder?: string | null;
  default_arg?: string | null;
  arg_options: PicoCommandArgOption[];
  dangerous: boolean;
  timeout_ms: number;
}

export interface HardwareSsotSpec {
  thresholds: HardwareThresholds;
  usb_ids: PicoUsbIdentity;
  indexed_manuals_count: number;
  pinout: P7501PinMapping[];
  breakin_modes: BreakInModeSpec[];
  flash_geometries: FlashGeometrySpec[];
  memory_regions: MemoryRegionSpec[];
  pico_commands: PicoCommandSpec[];
}
