/**
 * @file viewmodels/picoDevice/usePicoBootFlash.ts
 * @brief Hook managing RP2040/RP2350 1200-baud BOOTSEL reset and UF2 flashing workflows.
 */

import { useCallback, type Dispatch, type SetStateAction } from 'react';
import {
  PICO_BOOTSEL_AUTOCONNECT_DELAY_MS,
  PICO_BOOTSEL_REBOOT_WAIT_MS,
  PICO_BOOTSEL_RECONNECT_MAX_ATTEMPTS,
  PICO_BOOTSEL_RECONNECT_POLL_MS,
} from '@/constants';
import { useToast } from '@/context/ToastContext';
import { useManagedTimeouts } from '@/hooks/useManagedTimeouts';
import { translate } from '@/i18n/useTranslation';
import type { PicoUf2Variant } from '@/models';
import {
  detectPicoBootselDrives,
  flashPicoFirmwareUf2,
  listPicoSerialPorts,
  triggerPicoBootselReboot,
} from '@/services/picoService';
import { PICO_PORT_PATTERN, type UsePicoDeviceOptions } from './stateHelpers';

function waitMs(delayMs: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, delayMs);
  });
}

async function pollForDetectedPicoPort(attemptsLeft: number): Promise<string> {
  if (attemptsLeft <= 0) return '';
  await waitMs(PICO_BOOTSEL_RECONNECT_POLL_MS);
  const ports = await listPicoSerialPorts().catch(() => []);
  const found = ports.find((port) => PICO_PORT_PATTERN.test(port));
  if (found) return found;
  return pollForDetectedPicoPort(attemptsLeft - 1);
}

interface UsePicoBootFlashParams {
  selectedPort: string;
  setSelectedPort: Dispatch<SetStateAction<string>>;
  setIsFlashing: Dispatch<SetStateAction<boolean>>;
  refreshPorts: () => Promise<void>;
  toggleConnection: (overridePort?: unknown) => Promise<void>;
  onLog?: UsePicoDeviceOptions['onLog'];
}

export function usePicoBootFlash({
  selectedPort,
  setSelectedPort,
  setIsFlashing,
  refreshPorts,
  toggleConnection,
  onLog,
}: UsePicoBootFlashParams) {
  const { showToast } = useToast();
  const { scheduleTimeout } = useManagedTimeouts();

  const triggerBootselReboot = useCallback(async () => {
    if (!selectedPort) {
      showToast({
        type: 'warning',
        title: translate('errors.pico.bootselSelectPortTitle'),
        message: translate('errors.pico.bootselSelectPort'),
      });
      return;
    }
    try {
      const msg = await triggerPicoBootselReboot(selectedPort);
      showToast({
        type: 'success',
        title: translate('errors.pico.bootselResetTitle'),
        message: translate('errors.pico.bootselResetMsg', { msg }),
      });
      onLog?.(
        'info',
        'HOST',
        translate('errors.pico.bootselResetLog', { port: selectedPort, msg }),
      );
    } catch (err) {
      showToast({
        type: 'error',
        title: translate('errors.pico.bootselResetFailedTitle'),
        message: err instanceof Error ? err.message : String(err),
      });
    }
  }, [selectedPort, showToast, onLog]);

  const flashPicoFirmware = useCallback(
    async (uf2Variant: PicoUf2Variant) => {
      setIsFlashing(true);
      try {
        const initialDrives = await detectPicoBootselDrives().catch(() => []);
        const needsReboot =
          initialDrives.length === 0 && Boolean(selectedPort) && PICO_PORT_PATTERN.test(selectedPort);
        const drives = needsReboot
          ? await (async () => {
              showToast({
                type: 'info',
                title: translate('errors.pico.bootselRebootingTitle'),
                message: translate('errors.pico.bootselRebootingMsg'),
              });
              await triggerPicoBootselReboot(selectedPort).catch(() => {});
              await waitMs(PICO_BOOTSEL_REBOOT_WAIT_MS);
              return detectPicoBootselDrives().catch(() => []);
            })()
          : initialDrives;

        if (drives.length === 0) {
          showToast({
            type: 'warning',
            title: translate('errors.pico.bootselNoDriveTitle'),
            message: translate('errors.pico.bootselNoDriveMsg'),
          });
          return;
        }

        const report = await flashPicoFirmwareUf2(uf2Variant);
        const sizeStr = report.uf2_size_bytes ? report.uf2_size_bytes.toLocaleString() : 'UF2';
        showToast({
          type: 'success',
          title: translate('errors.pico.flashSuccessTitle'),
          message: translate('errors.pico.flashSuccessMsg', {
            msg: report.message,
            family: report.board_family || uf2Variant,
            size: sizeStr,
          }),
        });

        const detectedPort = await pollForDetectedPicoPort(PICO_BOOTSEL_RECONNECT_MAX_ATTEMPTS);
        await refreshPorts();
        if (detectedPort) {
          setSelectedPort(detectedPort);
          showToast({
            type: 'success',
            title: translate('errors.pico.probeDetectedTitle'),
            message: translate('errors.pico.probeDetectedMsg', { port: detectedPort }),
          });
          scheduleTimeout(() => {
            void toggleConnection(detectedPort);
          }, PICO_BOOTSEL_AUTOCONNECT_DELAY_MS);
        }
      } catch (err) {
        showToast({
          type: 'error',
          title: translate('errors.pico.flashFailedTitle'),
          message: err instanceof Error ? err.message : String(err),
        });
      } finally {
        setIsFlashing(false);
      }
    },
    [
      selectedPort,
      setIsFlashing,
      setSelectedPort,
      refreshPorts,
      showToast,
      toggleConnection,
      scheduleTimeout,
    ],
  );

  return {
    triggerBootselReboot,
    flashPicoFirmware,
  };
}
