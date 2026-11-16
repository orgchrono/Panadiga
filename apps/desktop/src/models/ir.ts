export type IrTimingUnit = 'MICROSECONDS' | 'CARRIER_CYCLES' | 'PERIOD_SCALED';

export type IrTimingOverride = 'AUTO' | IrTimingUnit;

export type PanasonicCategory =
  | 'ALL'
  | 'DIGA'
  | 'COMBO'
  | 'VCR'
  | 'DVD'
  | 'TV'
  | 'AUDIO'
  | 'STB'
  | 'PROJ'
  | 'AC'
  | 'OTHER';

export type KeySection =
  | 'POWER_INPUT'
  | 'NAVIGATION'
  | 'TRANSPORT'
  | 'TUNING_AUDIO'
  | 'NUMERIC'
  | 'RECORDING_EDIT'
  | 'CUSTOM';

export type KeyProtocol = 'KASEIKYO' | 'PRONTO_RAW';

export type KeyTone =
  | 'NEUTRAL'
  | 'PRIMARY'
  | 'INFO'
  | 'SUCCESS'
  | 'WARNING'
  | 'DANGER'
  | 'DANGER_DARK'
  | 'ACCENT';

export type RemoteMode = 'DVD' | 'VCR' | 'TV' | 'CATALOG';

export type RemoteUnitSet = 1 | 2 | 3;

export type IrTransportKind = 'AUTO' | 'ANDROID_HAL' | 'PICO_USB_OTG' | 'PICO_BLE' | 'PICO_TAURI';

export interface IrFrequencyRange {
  min_hz: number;
  max_hz: number;
}

export interface IrDeviceInfo {
  manufacturer: string;
  model: string;
  hardware: string;
  board: string;
  product: string;
  device: string;
  android_release: string;
  sdk_int: number;
}

export interface AndroidIrCapabilities {
  hasEmitter: boolean;
  serviceFound: boolean;
  hasSecIrNode: boolean;
  hasFpgaNode: boolean;
  suAvailable: boolean;
  ranges: IrFrequencyRange[];
  device: IrDeviceInfo;
}

export interface AndroidIrStoredSettings {
  timingUnitOverride: IrTimingOverride;
  repeatCount: number;
  customProfilesJson: string;
}

export interface AndroidIrTransmitResult {
  ok: boolean;
  backendUsed?: string;
  error?: string;
}

export interface IrPreparedSignal {
  carrier_hz: number;
  pattern_us: number[];
  encoded_pattern: number[];
  sysfs_payload: string;
  expects_cycles_in_sysfs: boolean;
  recommended_unit: IrTimingUnit;
  effective_unit: IrTimingUnit;
  wire_hex: string;
  pronto_hex: string;
}

export interface PanasonicCatalogSummary {
  id: string;
  category: PanasonicCategory;
  primary_name: string;
  models: string[];
  command_count: number;
}

export interface CatalogCommandSpec {
  name: string;
  protocol: KeyProtocol;
  device: number;
  subdevice: number;
  obc: number;
  pronto_hex?: string | null;
}

export interface PanasonicCatalogCodeset {
  id: string;
  category: PanasonicCategory;
  models: string[];
  commands: CatalogCommandSpec[];
}

export interface DynamicRemoteKey {
  id: string;
  label: string;
  section: KeySection;
  protocol: KeyProtocol;
  device: number;
  subdevice: number;
  obc: number;
  respect_remote_mode: boolean;
  pronto_hex?: string | null;
  tone: KeyTone;
  confirm_required: boolean;
  wire_preview: string;
}

export interface DynamicKeymapProfile {
  id: string;
  name: string;
  source_codeset_id: string;
  category: PanasonicCategory;
  keys: DynamicRemoteKey[];
  is_user_modified: boolean;
}

export interface IrCatalogStats {
  codeset_count: number;
  command_count: number;
}

export interface IrMacroStep {
  label: string;
  device: number;
  subdevice: number;
  obc: number;
  hold_ms: number;
  post_delay_ms: number;
}

export type IrMacroRiskLevel = 'SAFE' | 'MODERATE' | 'DESTRUCTIVE';

export interface VerifiedIrMacro {
  id: string;
  title_pt: string;
  title_en: string;
  description_pt: string;
  description_en: string;
  risk: IrMacroRiskLevel;
  verification_source: string;
  steps: IrMacroStep[];
}
