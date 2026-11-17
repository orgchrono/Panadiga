import { useCallback, type Dispatch } from 'react';
import {
  IR_CAMERA_BEACON_INTERVAL_MS,
  IR_CAMERA_BEACON_PULSE_COUNT,
  IR_LONG_HOLD_THRESHOLD_MS,
  PANASONIC_BEACON_OBC,
  PANASONIC_DEFAULT_SUBDEVICE,
  PANASONIC_DVD_DEVICE_CODE,
} from '@/constants';
import { useAbortSlot } from '@/hooks/useAbortSlot';
import type { TranslationKey } from '@/i18n/useTranslation';
import type {
  IrMacroStep,
  IrTimingOverride,
  IrTransportKind,
  RemoteUnitSet,
  VerifiedIrMacro,
} from '@/models/ir';
import { dispatchIrSignal } from '@/services/irTransportService';
import { irCommanderActions } from '@/store/slices/irCommanderSlice';
import { buildMacroStepTelemetryPayload, getErrorMessage } from './irStorage';

type IrCommanderAction = ReturnType<
  (typeof irCommanderActions)[keyof typeof irCommanderActions]
>;

interface ToastParams {
  type: 'info' | 'success' | 'warning' | 'error';
  title: string;
  message: string;
}

export interface UseIrMacroBeaconRunnerParams {
  dispatch: Dispatch<IrCommanderAction>;
  remoteUnit: RemoteUnitSet;
  transport: IrTransportKind;
  picoGpioPin: number;
  timingUnitOverride: IrTimingOverride;
  repeatCount: number;
  isBeaconRunning: boolean;
  locale: string;
  t: (key: TranslationKey, opts?: Record<string, unknown>) => string;
  showToast: (toast: ToastParams) => void;
}

function sleepAbortable(ms: number, signal: AbortSignal): Promise<boolean> {
  if (ms <= 0) return Promise.resolve(!signal.aborted);
  return new Promise((resolve) => {
    if (signal.aborted) {
      resolve(false);
      return;
    }
    const onAbort = () => {
      clearTimeout(timer);
      resolve(false);
    };
    const timer = setTimeout(() => {
      signal.removeEventListener('abort', onAbort);
      resolve(true);
    }, ms);
    signal.addEventListener('abort', onAbort, { once: true });
  });
}

interface MacroStepContext {
  readonly steps: readonly IrMacroStep[];
  readonly title: string;
  readonly signal: AbortSignal;
  readonly dispatch: Dispatch<IrCommanderAction>;
  readonly remoteUnit: RemoteUnitSet;
  readonly transport: IrTransportKind;
  readonly picoGpioPin: number;
  readonly timingUnitOverride: IrTimingOverride;
  readonly repeatCount: number;
  readonly t: (key: TranslationKey, opts?: Record<string, unknown>) => string;
}

async function runMacroStepsSequentially(
  index: number,
  ctx: MacroStepContext,
): Promise<void> {
  if (index >= ctx.steps.length || ctx.signal.aborted) return;
  const step = ctx.steps[index];
  ctx.dispatch(
    irCommanderActions.setMacroProgress({
      id: ctx.title,
      step: index + 1,
      total: ctx.steps.length,
    }),
  );
  const outcome = await dispatchIrSignal({
    protocol: 'KASEIKYO',
    device: step.device,
    subdevice: step.subdevice,
    obc: step.obc,
    respectRemoteMode: step.device === PANASONIC_DVD_DEVICE_CODE,
    remoteUnit: ctx.remoteUnit,
    transport: ctx.transport,
    picoGpioPin: ctx.picoGpioPin,
    unitOverride: ctx.timingUnitOverride,
    repeatCount: Math.max(ctx.repeatCount, step.hold_ms > IR_LONG_HOLD_THRESHOLD_MS ? 2 : 1),
  });
  ctx.dispatch(
    irCommanderActions.setTransmissionTelemetry(
      buildMacroStepTelemetryPayload(ctx.title, index, ctx.steps.length, step, outcome),
    ),
  );
  if (!outcome.ok) {
    throw new Error(
      outcome.error ??
        ctx.t('errors.ir.macroStepFailed', { step: index + 1, label: step.label }),
    );
  }
  const continued = await sleepAbortable(step.hold_ms + step.post_delay_ms, ctx.signal);
  if (!continued) return;
  await runMacroStepsSequentially(index + 1, ctx);
}

interface BeaconPulseContext {
  readonly signal: AbortSignal;
  readonly remoteUnit: RemoteUnitSet;
  readonly transport: IrTransportKind;
  readonly picoGpioPin: number;
  readonly timingUnitOverride: IrTimingOverride;
}

async function runBeaconPulses(pulse: number, ctx: BeaconPulseContext): Promise<void> {
  if (pulse > IR_CAMERA_BEACON_PULSE_COUNT || ctx.signal.aborted) return;
  await dispatchIrSignal({
    protocol: 'KASEIKYO',
    device: PANASONIC_DVD_DEVICE_CODE,
    subdevice: PANASONIC_DEFAULT_SUBDEVICE,
    obc: PANASONIC_BEACON_OBC,
    respectRemoteMode: true,
    remoteUnit: ctx.remoteUnit,
    transport: ctx.transport,
    picoGpioPin: ctx.picoGpioPin,
    unitOverride: ctx.timingUnitOverride,
    repeatCount: 1,
  });
  const ok = await sleepAbortable(IR_CAMERA_BEACON_INTERVAL_MS, ctx.signal);
  if (!ok) return;
  await runBeaconPulses(pulse + 1, ctx);
}

export function useIrMacroBeaconRunner({
  dispatch,
  remoteUnit,
  transport,
  picoGpioPin,
  timingUnitOverride,
  repeatCount,
  isBeaconRunning,
  locale,
  t,
  showToast,
}: UseIrMacroBeaconRunnerParams) {
  const macroSlot = useAbortSlot();
  const beaconSlot = useAbortSlot();

  const executeMacro = useCallback(
    async (macro: VerifiedIrMacro) => {
      const ctrl = macroSlot.acquire();
      const title = locale === 'pt-BR' ? macro.title_pt : macro.title_en;

      dispatch(irCommanderActions.setTransmitting(true));
      try {
        await runMacroStepsSequentially(0, {
          steps: macro.steps,
          title,
          signal: ctrl.signal,
          dispatch,
          remoteUnit,
          transport,
          picoGpioPin,
          timingUnitOverride,
          repeatCount,
          t,
        });
        if (ctrl.signal.aborted) {
          showToast({ type: 'warning', title, message: t('irCommander.macro.abortedToast') });
        } else {
          showToast({ type: 'success', title, message: t('irCommander.macro.completedToast') });
        }
      } catch (err) {
        const msg = getErrorMessage(err);
        dispatch(irCommanderActions.setError(msg));
        showToast({ type: 'error', title, message: msg });
      } finally {
        dispatch(irCommanderActions.setMacroProgress({ id: null, step: 0, total: 0 }));
        dispatch(irCommanderActions.setTransmitting(false));
      }
    },
    [
      dispatch,
      locale,
      macroSlot,
      picoGpioPin,
      remoteUnit,
      repeatCount,
      showToast,
      t,
      timingUnitOverride,
      transport,
    ],
  );

  const abortMacro = useCallback(() => {
    macroSlot.abortAll();
  }, [macroSlot]);

  const toggleCameraBeacon = useCallback(async () => {
    if (isBeaconRunning) {
      beaconSlot.abortAll();
      return;
    }
    const ctrl = beaconSlot.acquire();
    dispatch(irCommanderActions.setBeaconRunning(true));
    try {
      await runBeaconPulses(1, {
        signal: ctrl.signal,
        remoteUnit,
        transport,
        picoGpioPin,
        timingUnitOverride,
      });
      showToast({
        type: 'info',
        title: t('irCommander.diag.beaconTitle'),
        message: t('irCommander.diag.beaconDone'),
      });
    } finally {
      dispatch(irCommanderActions.setBeaconRunning(false));
    }
  }, [
    beaconSlot,
    dispatch,
    isBeaconRunning,
    picoGpioPin,
    remoteUnit,
    showToast,
    t,
    timingUnitOverride,
    transport,
  ]);

  return { executeMacro, abortMacro, toggleCameraBeacon };
}
