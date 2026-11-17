/**
 * @file constants/uiLayout.ts
 * @brief SSOT numeric constants for resizable split panes, range scrubbers, and telemetry UI thresholds.
 */

import type { WorkbenchDocksState } from '@/types/workbench';

export const PIPELINE_TOTAL_STEPS = 4;

export const DEFAULT_DOCKS_STATE: WorkbenchDocksState = Object.freeze({
  leftOpen: false,
  rightOpen: false,
  bottomOpen: false,
  bottomExpanded: false,
});

export const DEFAULT_SPLIT_LEFT_PCT = 40;
export const MIN_SPLIT_LEFT_PCT = 24;
export const MAX_SPLIT_LEFT_PCT = 74;
export const SPLIT_KEYBOARD_STEP_PCT = 2;
export const PERCENT_TENTH_SCALE = 10;
export const PERCENT_SCALE = 100;
export const RANGE_MIN_VISUAL_WIDTH_PCT = 2;
export const RANGE_THUMB_HALF_PX = 9;
export const VIEW_TRANSITION_DURATION_SEC = 0.16;
export const HEADER_VIEW_Y_OFFSET_PX = 6;
export const MANUALS_SPLIT_INITIAL_PCT = 64;
export const MANUALS_SPLIT_MIN_PCT = 42;
export const MANUALS_SPLIT_MAX_PCT = 76;
export const MAX_BYTE_VALUE = 0xff;
export const KASEIKYO_HEADER_BYTE0 = 0x02;
export const KASEIKYO_HEADER_BYTE1 = 0x20;
export const DEFAULT_IR_PICO_GPIO = 6;
export const DENSITY_HIGH_THRESHOLD_PCT = 90;
export const DENSITY_MEDIUM_THRESHOLD_PCT = 50;
export const MAX_IR_MODEL_PREVIEW = 6;
export const MAX_MANUAL_PAGE_PREVIEW = 5;
export const MAX_DOC_DECODE_NOTES_PREVIEW = 3;
export const MD_FENCE_MARKER_LEN = 3;
export const MD_BOLD_WRAPPER_LEN = 4;
export const INLINE_KEY_PREVIEW_LEN = 24;
export const TOOL_LAUNCH_FEEDBACK_MS = 1200;
export const GDB_TELEMETRY_PREVIEW_LIMIT = 12;
export const DIAGNOSTIC_HISTORY_PREVIEW_LIMIT = 6;
export const SD_DISK_CHECKS_PREVIEW = 4;
export const FRM_HIGH_REP_THRESHOLD = 1000;
export const FRM_ENTROPY_PRECISION = 5;
export const HEX16_PAD_WIDTH = 4;
export const HEX_PAGE_BYTES = 384;
export const HEX_SPLIT_INITIAL_PCT = 34;
export const HEX_SPLIT_MIN_PCT = 22;
export const HEX_SPLIT_MAX_PCT = 55;
export const HDD_ENTROPY_PRECISION = 4;
export const HDD_METRIC_ENTROPY_PRECISION = 3;
export const SECTORS_PER_MEIHDFS_BLOCK = 4;
export const IR_UNIT_SET_NUMBERS = [1, 2, 3] as const;
