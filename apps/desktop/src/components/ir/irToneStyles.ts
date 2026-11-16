import type { KeySection, KeyTone } from '@/models/ir';

export const KEY_TONE_CLASSES: Record<KeyTone, string> = {
  NEUTRAL: 'bg-bench-850 border-slate-700 text-slate-100 hover:bg-bench-800',
  PRIMARY: 'bg-blue-600/20 border-blue-500/50 text-blue-200 hover:bg-blue-600/30',
  INFO: 'bg-cyan-500/15 border-cyan-500/40 text-cyan-200 hover:bg-cyan-500/25',
  SUCCESS: 'bg-emerald-500/15 border-emerald-500/45 text-emerald-200 hover:bg-emerald-500/25',
  WARNING: 'bg-amber-500/15 border-amber-500/45 text-amber-200 hover:bg-amber-500/25',
  DANGER: 'bg-rose-500/20 border-rose-500/50 text-rose-200 hover:bg-rose-500/30',
  DANGER_DARK: 'bg-red-700/25 border-red-500/60 text-red-200 hover:bg-red-700/40',
  ACCENT: 'bg-purple-500/20 border-purple-500/50 text-purple-200 hover:bg-purple-500/30',
};

export const KEY_SECTION_ORDER: readonly KeySection[] = [
  'POWER_INPUT',
  'NAVIGATION',
  'TRANSPORT',
  'RECORDING_EDIT',
  'TUNING_AUDIO',
  'NUMERIC',
  'CUSTOM',
];
