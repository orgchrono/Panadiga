import { useCallback, useEffect, useRef, type Dispatch } from 'react';
import { irCommanderActions } from '@/store/slices/irCommanderSlice';
import { dispatchIrSignal } from '@/services/irTransportService';
import type {
  IrTimingOverride,
  IrTransportKind,
  RemoteUnitSet,
  VerifiedIrMacro,
} from '@/models/ir';
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
  t: (key: string) => string;
  showToast: (toast: ToastParams) => void;
}

function sleepAbortable(ms: number, signal: AbortSignal): Promise<boolean> {
  if (ms <= 0) return Promise.resolve(!signal.aborted);
  return new Promise(resolve => {
    if (signal.aborted) {
      resolve(false);
      return;
    }
    const timer = setTimeout(() => {
      signal.removeEventListener('abort', onAbort);
      resolve(true);
    }, ms);
    const onAbort = () => {
      clearTimeout(timer);
      resolve(false);
    };
    signal.addEventListener('abort', onAbort, { once: true });
  });
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
  const macroAbortRef = useRef<AbortController | null>(null);
  const beaconAbortRef = useRef<AbortController | null>(null);

  useEffect(() => {
    return () => {
      macroAbortRef.current?.abort();
      beaconAbortRef.current?.abort();
    };
  }, []);

  const executeMacro = useCallback(
    async (macro: VerifiedIrMacro) => {
      macroAbortRef.current?.abort();
      const ctrl = new AbortController();
      macroAbortRef.current = ctrl;
      const total = macro.steps.length;
      const title = locale === 'pt-BR' ? macro.title_pt : macro.title_en;

      dispatch(irCommanderActions.setTransmitting(true));
      try {
        for (let i = 0; i < total; i += 1) {
          if (ctrl.signal.aborted) break;
          const step = macro.steps[i];
          dispatch(irCommanderActions.setMacroProgress({ id: macro.id, step: i + 1, total }));
          const outcome = await dispatchIrSignal({
            protocol: 'KASEIKYO',
            device: step.device,
            subdevice: step.subdevice,
            obc: step.obc,
            respectRemoteMode: step.device === 0xb0,
            remoteUnit,
            transport,
            picoGpioPin,
            unitOverride: timingUnitOverride,
            repeatCount: Math.max(repeatCount, step.hold_ms > 1000 ? 2 : 1),
          });
          dispatch(
            irCommanderActions.setTransmissionTelemetry(
              buildMacroStepTelemetryPayload(title, i, total, step, outcome),
            ),
          );
          if (!outcome.ok) {
            throw new Error(outcome.error ?? `Macro step ${i + 1} (${step.label}) failed`);
          }
          const continued = await sleepAbortable(step.hold_ms + step.post_delay_ms, ctrl.signal);
          if (!continued) break;
        }
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
    macroAbortRef.current?.abort();
  }, []);

  const toggleCameraBeacon = useCallback(async () => {
    if (isBeaconRunning) {
      beaconAbortRef.current?.abort();
      return;
    }
    const ctrl = new AbortController();
    beaconAbortRef.current = ctrl;
    dispatch(irCommanderActions.setBeaconRunning(true));
    try {
      for (let pulse = 1; pulse <= 30; pulse += 1) {
        if (ctrl.signal.aborted) break;
        await dispatchIrSignal({
          protocol: 'KASEIKYO',
          device: 0xb0,
          subdevice: 0x00,
          obc: 0xfe,
          respectRemoteMode: true,
          remoteUnit,
          transport,
          picoGpioPin,
          unitOverride: timingUnitOverride,
          repeatCount: 1,
        });
        const ok = await sleepAbortable(250, ctrl.signal);
        if (!ok) break;
      }
      showToast({
        type: 'info',
        title: t('irCommander.diag.beaconTitle'),
        message: t('irCommander.diag.beaconDone'),
      });
    } finally {
      dispatch(irCommanderActions.setBeaconRunning(false));
    }
  }, [
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
