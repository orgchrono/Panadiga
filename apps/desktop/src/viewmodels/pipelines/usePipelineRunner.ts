import { useCallback, useEffect, useReducer } from 'react';
import { logger } from '@/utils/logger';
import type { PipelineProgressState } from '@/models';

const MAX_RETAINED_PIPELINE_LOGS = 50;
const RADIX_BASE36 = 36;
const ENTROPY_SLICE_START = 2;

export interface PipelineRunnerState<TData> {
  readonly runToken: string | null;
  readonly isBusy: boolean;
  readonly data: TData | null;
  readonly error: string | null;
  readonly successMessage: string | null;
  readonly progress: PipelineProgressState | null;
  readonly logs: readonly string[];
}

type PipelineAction<TData> =
  | { readonly type: 'start'; readonly runToken: string }
  | { readonly type: 'progress'; readonly progress: PipelineProgressState; readonly logLine?: string }
  | { readonly type: 'success'; readonly runToken: string; readonly data: TData; readonly successMessage: string }
  | { readonly type: 'error'; readonly runToken: string; readonly error: string }
  | { readonly type: 'reset' };

function createInitialState<TData>(): PipelineRunnerState<TData> {
  return {
    runToken: null,
    isBusy: false,
    data: null,
    error: null,
    successMessage: null,
    progress: null,
    logs: [],
  };
}

function appendCappedLog(logs: readonly string[], line?: string): readonly string[] {
  if (!line) return logs;
  const next = [...logs, line];
  const overflow = Math.max(0, next.length - MAX_RETAINED_PIPELINE_LOGS);
  return overflow > 0 ? next.slice(overflow) : next;
}

function pipelineReducer<TData>(
  state: PipelineRunnerState<TData>,
  action: PipelineAction<TData>,
): PipelineRunnerState<TData> {
  switch (action.type) {
    case 'start':
      return {
        ...state,
        runToken: action.runToken,
        isBusy: true,
        error: null,
        successMessage: null,
        progress: null,
        logs: [],
      };
    case 'progress':
      return {
        ...state,
        progress: action.progress,
        logs: appendCappedLog(state.logs, action.logLine),
      };
    case 'success':
      if (state.runToken !== action.runToken) return state;
      return {
        ...state,
        isBusy: false,
        data: action.data,
        error: null,
        successMessage: action.successMessage,
      };
    case 'error':
      if (state.runToken !== action.runToken) return state;
      return {
        ...state,
        isBusy: false,
        error: action.error,
      };
    case 'reset':
      return createInitialState<TData>();
  }
}

export interface UsePipelineRunnerConfig<TArgs, TData> {
  readonly executeFn: (args: TArgs) => Promise<TData>;
  readonly subscribeProgress?: (
    onProgress: (progress: PipelineProgressState, logLine?: string) => void,
  ) => Promise<() => void>;
  readonly validateFn?: (args: TArgs) => string | null;
  readonly formatSuccess?: (data: TData) => string;
}

function createRunToken(): string {
  return `${Date.now().toString(RADIX_BASE36)}-${Math.random().toString(RADIX_BASE36).slice(ENTROPY_SLICE_START)}`;
}

export function usePipelineRunner<TArgs, TData>({
  executeFn,
  subscribeProgress,
  validateFn,
  formatSuccess,
}: UsePipelineRunnerConfig<TArgs, TData>) {
  const [state, dispatch] = useReducer(
    pipelineReducer<TData>,
    undefined,
    createInitialState<TData>,
  );

  const reportProgress = useCallback((progress: PipelineProgressState, logLine?: string) => {
    dispatch({ type: 'progress', progress, logLine });
  }, []);

  useEffect(() => {
    if (!subscribeProgress) return undefined;
    const controller = new AbortController();
    const unsubPromise = subscribeProgress((progress, logLine) => {
      if (!controller.signal.aborted) {
        reportProgress(progress, logLine);
      }
    });
    return () => {
      controller.abort();
      void unsubPromise.then(unlisten => unlisten());
    };
  }, [reportProgress, subscribeProgress]);

  const execute = useCallback(
    async (args: TArgs): Promise<TData | null> => {
      const runToken = createRunToken();
      if (validateFn) {
        const validationError = validateFn(args);
        if (validationError) {
          dispatch({ type: 'start', runToken });
          dispatch({ type: 'error', runToken, error: validationError });
          return null;
        }
      }
      dispatch({ type: 'start', runToken });
      try {
        const result = await executeFn(args);
        dispatch({
          type: 'success',
          runToken,
          data: result,
          successMessage: formatSuccess ? formatSuccess(result) : '',
        });
        return result;
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        logger.error('PipelineRunner', 'Pipeline execution failed', err);
        dispatch({ type: 'error', runToken, error: msg });
        return null;
      }
    },
    [executeFn, formatSuccess, validateFn],
  );

  const reset = useCallback(() => {
    dispatch({ type: 'reset' });
  }, []);

  return {
    ...state,
    isLoading: state.isBusy,
    execute,
    reset,
    reportProgress,
  };
}
