/**
 * @file types/workbench.ts
 * @brief Shared workbench, accessibility, toast, and shell telemetry domain types for Redux and UI.
 */

export type WorkbenchExperienceMode = 'basic' | 'advanced';
export type WorkbenchTheme = 'dark-lab' | 'high-contrast' | 'daylight-bench' | 'amber-crt';
export type FontScaleMode = 'normal' | 'large';

export interface AccessibilitySettings {
  readonly theme: WorkbenchTheme;
  readonly colorblindSafe: boolean;
  readonly reducedMotion: boolean;
  readonly fontScale: FontScaleMode;
}

export interface PipelineProgressState {
  readonly picoReady: boolean;
  readonly nvramCleaned: boolean;
  readonly hddInspected: boolean;
  readonly unbrickReady: boolean;
}

export type ToastType = 'success' | 'error' | 'warning' | 'info';

export interface ToastItem {
  readonly id: string;
  readonly type: ToastType;
  readonly title: string;
  readonly message?: string;
  readonly durationMs?: number;
}

export interface ShellTelemetryStatus {
  readonly connected: boolean;
  readonly vdd_volts: number;
}
