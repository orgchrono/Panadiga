import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from 'react';
import type { LiveNvramCaptureEvent, PicoDeviceStatus } from '@/models';
import { listenEvent } from '@/services/ipc';

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

interface WorkbenchModeContextValue {
  readonly mode: WorkbenchExperienceMode;
  readonly isAdvanced: boolean;
  readonly setMode: (mode: WorkbenchExperienceMode) => void;
  readonly toggleMode: () => void;
  readonly hudOpen: boolean;
  readonly setHudOpen: (open: boolean) => void;
  readonly toggleHud: () => void;
  readonly accessibility: AccessibilitySettings;
  readonly setTheme: (theme: WorkbenchTheme) => void;
  readonly setColorblindSafe: (enabled: boolean) => void;
  readonly setReducedMotion: (enabled: boolean) => void;
  readonly setFontScale: (scale: FontScaleMode) => void;
  readonly resetWorkbenchPreferences: () => void;
  readonly resetPipeline: () => void;
  readonly pipeline: PipelineProgressState;
  readonly markPipelineFlag: (key: keyof PipelineProgressState, value?: boolean) => void;
}

const STORAGE_MODE_KEY = 'panadiga.workbench.mode';
const STORAGE_A11Y_KEY = 'panadiga.workbench.accessibility';
const SPLIT_STORAGE_KEYS = [
  'panadiga.split.hex',
  'panadiga.split.manuals',
  'panadiga.split.picobench',
];

const DEFAULT_ACCESSIBILITY: AccessibilitySettings = Object.freeze({
  theme: 'dark-lab',
  colorblindSafe: false,
  reducedMotion: false,
  fontScale: 'normal',
});

const DEFAULT_PIPELINE: PipelineProgressState = Object.freeze({
  picoReady: false,
  nvramCleaned: false,
  hddInspected: false,
  unbrickReady: false,
});

const WorkbenchModeContext = createContext<WorkbenchModeContextValue | undefined>(undefined);

function readStorageString(key: string): string | null {
  if (typeof window === 'undefined') return null;
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}

function writeStorageString(key: string, value: string): void {
  if (typeof window === 'undefined') return;
  try {
    window.localStorage.setItem(key, value);
  } catch {
    // ignore storage restrictions
  }
}

function readInitialMode(): WorkbenchExperienceMode {
  const stored = readStorageString(STORAGE_MODE_KEY);
  return stored === 'basic' || stored === 'advanced' ? stored : 'advanced';
}

function readInitialAccessibility(): AccessibilitySettings {
  const raw = readStorageString(STORAGE_A11Y_KEY);
  if (raw) {
    try {
      const parsed = JSON.parse(raw) as Partial<AccessibilitySettings>;
      const validTheme: WorkbenchTheme =
        parsed.theme === 'dark-lab' ||
        parsed.theme === 'high-contrast' ||
        parsed.theme === 'daylight-bench' ||
        parsed.theme === 'amber-crt'
          ? parsed.theme
          : 'dark-lab';
      const validScale: FontScaleMode = parsed.fontScale === 'large' ? 'large' : 'normal';
      return {
        theme: validTheme,
        colorblindSafe: Boolean(parsed.colorblindSafe),
        reducedMotion: Boolean(parsed.reducedMotion),
        fontScale: validScale,
      };
    } catch {
      // fall through to defaults
    }
  }
  const prefersReduced =
    typeof window !== 'undefined' &&
    typeof window.matchMedia === 'function' &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  return { ...DEFAULT_ACCESSIBILITY, reducedMotion: prefersReduced };
}

export const WorkbenchModeProvider: React.FC<{ children: ReactNode }> = ({ children }) => {
  const [mode, setModeState] = useState<WorkbenchExperienceMode>(readInitialMode);
  const [hudOpen, setHudOpen] = useState<boolean>(false);
  const [pipeline, setPipeline] = useState<PipelineProgressState>(DEFAULT_PIPELINE);
  const [accessibility, setAccessibility] = useState<AccessibilitySettings>(readInitialAccessibility);

  const setMode = useCallback((next: WorkbenchExperienceMode) => {
    setModeState(next);
    writeStorageString(STORAGE_MODE_KEY, next);
  }, []);

  const toggleMode = useCallback(() => {
    setMode(mode === 'advanced' ? 'basic' : 'advanced');
  }, [mode, setMode]);

  const toggleHud = useCallback(() => {
    setHudOpen(prev => !prev);
  }, []);

  const updateAccessibility = useCallback((patch: Partial<AccessibilitySettings>) => {
    setAccessibility(prev => {
      const next = { ...prev, ...patch };
      writeStorageString(STORAGE_A11Y_KEY, JSON.stringify(next));
      return next;
    });
  }, []);

  const setTheme = useCallback((theme: WorkbenchTheme) => updateAccessibility({ theme }), [updateAccessibility]);
  const setColorblindSafe = useCallback(
    (colorblindSafe: boolean) => updateAccessibility({ colorblindSafe }),
    [updateAccessibility]
  );
  const setReducedMotion = useCallback(
    (reducedMotion: boolean) => updateAccessibility({ reducedMotion }),
    [updateAccessibility]
  );
  const setFontScale = useCallback(
    (fontScale: FontScaleMode) => updateAccessibility({ fontScale }),
    [updateAccessibility]
  );

  const resetWorkbenchPreferences = useCallback(() => {
    setMode('advanced');
    setHudOpen(false);
    setAccessibility(DEFAULT_ACCESSIBILITY);
    if (typeof window !== 'undefined') {
      try {
        window.localStorage.removeItem(STORAGE_A11Y_KEY);
        window.localStorage.removeItem(STORAGE_MODE_KEY);
        SPLIT_STORAGE_KEYS.forEach(k => window.localStorage.removeItem(k));
      } catch {
        // ignore storage errors
      }
    }
  }, [setMode]);

  const resetPipeline = useCallback(() => {
    setPipeline(DEFAULT_PIPELINE);
  }, []);

  useEffect(() => {
    if (typeof document === 'undefined') return;
    const root = document.documentElement;
    root.setAttribute('data-theme', accessibility.theme);
    root.setAttribute('data-colorblind', accessibility.colorblindSafe ? 'true' : 'false');
    root.setAttribute('data-reduced-motion', accessibility.reducedMotion ? 'true' : 'false');
    root.setAttribute('data-font-scale', accessibility.fontScale);
    root.style.colorScheme = accessibility.theme === 'daylight-bench' ? 'light' : 'dark';
  }, [accessibility]);

  const markPipelineFlag = useCallback(
    (key: keyof PipelineProgressState, value = true) => {
      setPipeline(prev => (prev[key] === value ? prev : { ...prev, [key]: value }));
    },
    []
  );

  useEffect(() => {
    let active = true;
    const unlisteners: Array<() => void> = [];

    const bind = async <T,>(eventName: string, handler: (payload: T) => void) => {
      const fn = await listenEvent<T>(eventName, payload => {
        if (active) handler(payload);
      });
      if (active) {
        unlisteners.push(fn);
      } else {
        fn();
      }
    };

    void bind<PicoDeviceStatus>('pico://telemetry', status => {
      if (status?.connected) {
        markPipelineFlag('picoReady', true);
      }
    });
    void bind<LiveNvramCaptureEvent>('pico://nvram-live-capture', evt => {
      if (evt?.report) {
        markPipelineFlag('picoReady', true);
        if (evt.report.nvram_non_ff_bytes === 0) {
          markPipelineFlag('nvramCleaned', true);
        }
      }
    });
    void bind<boolean>('nvram://cleaned', cleaned => {
      markPipelineFlag('nvramCleaned', Boolean(cleaned));
    });
    void bind<boolean>('hdd://inspected', inspected => {
      markPipelineFlag('hddInspected', Boolean(inspected));
    });
    void bind<boolean>('unbrick://completed', completed => {
      markPipelineFlag('unbrickReady', Boolean(completed));
    });

    return () => {
      active = false;
      unlisteners.forEach(fn => fn());
    };
  }, [markPipelineFlag]);

  return (
    <WorkbenchModeContext.Provider
      value={{
        mode,
        isAdvanced: mode === 'advanced',
        setMode,
        toggleMode,
        hudOpen,
        setHudOpen,
        toggleHud,
        accessibility,
        setTheme,
        setColorblindSafe,
        setReducedMotion,
        setFontScale,
        resetWorkbenchPreferences,
        resetPipeline,
        pipeline,
        markPipelineFlag,
      }}
    >
      {children}
    </WorkbenchModeContext.Provider>
  );
};

const FALLBACK_VALUE: WorkbenchModeContextValue = Object.freeze({
  mode: 'advanced',
  isAdvanced: true,
  setMode: () => {},
  toggleMode: () => {},
  hudOpen: false,
  setHudOpen: () => {},
  toggleHud: () => {},
  accessibility: DEFAULT_ACCESSIBILITY,
  setTheme: () => {},
  setColorblindSafe: () => {},
  setReducedMotion: () => {},
  setFontScale: () => {},
  resetWorkbenchPreferences: () => {},
  resetPipeline: () => {},
  pipeline: DEFAULT_PIPELINE,
  markPipelineFlag: () => {},
});

export function useWorkbenchMode(): WorkbenchModeContextValue {
  const ctx = useContext(WorkbenchModeContext);
  return ctx ?? FALLBACK_VALUE;
}
