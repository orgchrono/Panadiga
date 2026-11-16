/**
 * @file viewmodels/picoDevice/usePicoBootFlash.ts
 * @brief Hook managing RP2040/RP2350 1200-baud BOOTSEL reset and UF2 flashing workflows.
 */

import { useCallback, type Dispatch, type SetStateAction } from 'react';
import type { FlashReport, PicoUf2Variant } from '@/models';
import { invokeCommand } from '@/services/ipc';
import { useManagedTimeouts } from '@/hooks/useManagedTimeouts';
import { useToast } from '@/context/ToastContext';
import { PICO_PORT_PATTERN, type UsePicoDeviceOptions } from './stateHelpers';

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
        title: 'Porta Não Selecionada',
        message: 'Selecione a porta serial do Pico antes de solicitar o reset para BOOTSEL.',
      });
      return;
    }
    try {
      const msg = await invokeCommand<string>('pico_trigger_bootsel_reboot', {
        portName: selectedPort,
        port_name: selectedPort,
      });
      showToast({
        type: 'success',
        title: 'BOOTSEL Reset Iniciado',
        message: `${msg}. Aguarde 2 segundos para o drive RPI-RP2 / RP2350 montar e clique em "Flash Pico (BOOTSEL)".`,
      });
      onLog?.('info', 'HOST', `BOOTSEL reset iniciado em ${selectedPort}: ${msg}`);
    } catch (err) {
      showToast({
        type: 'error',
        title: 'Falha no Reset BOOTSEL',
        message: err instanceof Error ? err.message : String(err),
      });
    }
  }, [selectedPort, showToast, onLog]);

  const flashPicoFirmware = useCallback(
    async (uf2Variant: PicoUf2Variant) => {
      setIsFlashing(true);
      try {
        let drives = await invokeCommand<string[]>('pico_detect_bootsel_drives').catch(() => []);
        if ((!drives || drives.length === 0) && selectedPort && PICO_PORT_PATTERN.test(selectedPort)) {
          showToast({
            type: 'info',
            title: 'Reiniciando Pico em BOOTSEL',
            message: 'Enviando pulso 1200-baud para reiniciar a placa em modo de gravação...',
          });
          await invokeCommand('pico_trigger_bootsel_reboot', {
            portName: selectedPort,
            port_name: selectedPort,
          }).catch(() => {});
          await new Promise(resolve => setTimeout(resolve, 1600));
          drives = await invokeCommand<string[]>('pico_detect_bootsel_drives').catch(() => []);
        }

        if (!drives || drives.length === 0) {
          showToast({
            type: 'warning',
            title: 'Nenhum Drive BOOTSEL Detectado',
            message:
              'Nenhum drive RPI-RP2 / RP2350 encontrado. Clique em "BOOTSEL Reset" ou segure BOOTSEL ao conectar.',
          });
          return;
        }

        const report = await invokeCommand<FlashReport>('pico_flash_firmware', {
          targetFamily: uf2Variant,
          target_family: uf2Variant,
        });
        const sizeStr = report.uf2_size_bytes ? report.uf2_size_bytes.toLocaleString() : 'UF2';
        showToast({
          type: 'success',
          title: 'Firmware Pico Gravado',
          message: `${report.message} (${report.board_family || uf2Variant}, ${sizeStr} bytes). Reconectando USB...`,
        });

        let detectedPort = '';
        for (let i = 0; i < 12; i++) {
          await new Promise(resolve => setTimeout(resolve, 500));
          const ports = await invokeCommand<string[]>('list_pico_serial_ports').catch(() => []);
          const found = ports?.find(p => PICO_PORT_PATTERN.test(p));
          if (found) {
            detectedPort = found;
            break;
          }
        }

        await refreshPorts();
        if (detectedPort) {
          setSelectedPort(detectedPort);
          showToast({
            type: 'success',
            title: 'Pico Detectado',
            message: `Reconectando automaticamente em ${detectedPort}...`,
          });
          scheduleTimeout(() => {
            void toggleConnection(detectedPort);
          }, 400);
        }
      } catch (err) {
        showToast({
          type: 'error',
          title: 'Falha ao Gravar Firmware',
          message: err instanceof Error ? err.message : String(err),
        });
      } finally {
        setIsFlashing(false);
      }
    },
    [selectedPort, setIsFlashing, setSelectedPort, refreshPorts, showToast, toggleConnection, scheduleTimeout]
  );

  return {
    triggerBootselReboot,
    flashPicoFirmware,
  };
}
