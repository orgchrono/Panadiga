/**
 * @file viewmodels/picoDevice/usePicoPortDiscovery.ts
 * @brief Hook handling serial/OTG/BLE enumeration, DIGA model profile selection, and raw command dispatch.
 */

import {
  useCallback,
  type Dispatch,
  type MutableRefObject,
  type SetStateAction,
} from 'react';
import type { DigaModelProfile } from '@/models';
import { invokeCommand } from '@/services/ipc';
import { bleService } from '@/services/bleService';
import {
  isAndroidNativeBridgeAvailable,
  listAndroidUsbOtgDevices,
  sendAndroidUsbOtgCommand,
} from '@/services/androidBridge';
import { logger } from '@/utils/logger';
import {
  PICO_PORT_PATTERN,
  pickPreferredSerialPort,
  type UsePicoDeviceOptions,
} from './stateHelpers';

export interface UsePicoPortDiscoveryParams {
  selectedPort: string;
  setSelectedPort: Dispatch<SetStateAction<string>>;
  setAvailablePorts: Dispatch<SetStateAction<string[]>>;
  modelCatalog: DigaModelProfile[];
  setModelCatalog: Dispatch<SetStateAction<DigaModelProfile[]>>;
  setSelectedModel: Dispatch<SetStateAction<string>>;
  setIsLoading: Dispatch<SetStateAction<boolean>>;
  isConnectedRef: MutableRefObject<boolean>;
  isConnectingRef: MutableRefObject<boolean>;
  manualDisconnectRef: MutableRefObject<boolean>;
  toggleConnectionRef: MutableRefObject<((overridePort?: unknown) => Promise<void>) | null>;
  onLog?: UsePicoDeviceOptions['onLog'];
}

export function usePicoPortDiscovery({
  selectedPort,
  setSelectedPort,
  setAvailablePorts,
  modelCatalog,
  setModelCatalog,
  setSelectedModel,
  setIsLoading,
  isConnectedRef,
  isConnectingRef,
  manualDisconnectRef,
  toggleConnectionRef,
  onLog,
}: UsePicoPortDiscoveryParams) {
  const sendDirectRawCommand = useCallback(
    async (command: string): Promise<string> => {
      if (selectedPort.startsWith('ANDROID-USB-OTG')) {
        const res = sendAndroidUsbOtgCommand(command, 3000);
        if (!res.ok) throw new Error(res.output);
        return res.output;
      }
      if (selectedPort.startsWith('BLE_') && bleService.isConnected()) {
        return await bleService.sendAndAwaitReply(command, 3500);
      }
      return await invokeCommand<string>('pico_send_serial_command', {
        portName: selectedPort,
        command,
      });
    },
    [selectedPort]
  );

  const refreshModelCatalog = useCallback(async () => {
    try {
      const catalog = await invokeCommand<DigaModelProfile[]>('get_diga_model_catalog');
      if (catalog && catalog.length > 0) {
        setModelCatalog(catalog);
      }
    } catch (err) {
      logger.warn('PicoDevice', 'Failed loading DIGA model catalog', err);
    }
  }, [setModelCatalog]);

  const selectTargetModel = useCallback(
    async (modelId: string) => {
      setSelectedModel(modelId);
      const profile = modelCatalog.find(m => m.model_id === modelId);
      if (modelId === 'AUTO') {
        try {
          const resp = await sendDirectRawCommand('AUTODETECT');
          onLog?.('info', 'PICO', `Modo AUTO ativado (Address Aliasing Probe): ${resp}`);
        } catch (err) {
          logger.warn('PicoDevice', 'AUTODETECT command failed', err);
        }
      } else if (profile) {
        try {
          const resp = await sendDirectRawCommand(`FLASH_SIZE ${profile.flash_size_mb}`);
          onLog?.(
            'info',
            'HOST',
            `Perfil ${profile.model_id} selecionado (${profile.flash_size_mb}MB | EEMUL ${profile.nvram_base_hex}): ${resp}`
          );
        } catch (err) {
          logger.warn('PicoDevice', 'FLASH_SIZE command failed', err);
        }
      }
    },
    [modelCatalog, sendDirectRawCommand, setSelectedModel, onLog]
  );

  const refreshPorts = useCallback(
    async (injectedPortsOrSilent?: unknown) => {
      const pushedPorts = Array.isArray(injectedPortsOrSilent)
        ? injectedPortsOrSilent.filter((p): p is string => typeof p === 'string')
        : null;
      const isSilent = pushedPorts !== null || injectedPortsOrSilent === true;
      if (!isSilent) setIsLoading(true);
      try {
        const ports = pushedPorts ?? (await invokeCommand<string[]>('list_pico_serial_ports'));
        const allPorts = [...(ports || [])];

        if (isAndroidNativeBridgeAvailable()) {
          listAndroidUsbOtgDevices().forEach(dev => {
            const label = `ANDROID-USB-OTG [${dev.deviceName} - ${dev.productName}${dev.isBootsel ? ' BOOTSEL' : ''}]`;
            if (!allPorts.includes(label)) {
              allPorts.unshift(label);
            }
          });
        }

        if (bleService.isSupported() && !allPorts.some(p => p.includes('BLE_PICO_W'))) {
          allPorts.push('BLE_PICO_W [Bluetooth LE - Nordic UART Service (NUS)]');
        }
        setAvailablePorts(allPorts);

        const targetDefault = pickPreferredSerialPort(allPorts);
        if (targetDefault) {
          if (!selectedPort || !allPorts.includes(selectedPort)) {
            setSelectedPort(targetDefault);
          }
          const canAutoConnect =
            PICO_PORT_PATTERN.test(targetDefault) &&
            !isConnectedRef.current &&
            !isConnectingRef.current &&
            !manualDisconnectRef.current;
          if (canAutoConnect) {
            onLog?.('info', 'HOST', `Pico detectado automaticamente em ${targetDefault}. Iniciando conexão...`);
            void toggleConnectionRef.current?.(targetDefault);
          }
        } else if (allPorts.length > 0 && !selectedPort) {
          setSelectedPort(allPorts[0]);
        } else if (allPorts.length === 0) {
          setSelectedPort('');
        }
      } catch (err) {
        logger.warn('PicoDevice', 'Failed loading system serial ports', err);
      } finally {
        if (!isSilent) setIsLoading(false);
      }
    },
    [
      isConnectedRef,
      isConnectingRef,
      manualDisconnectRef,
      onLog,
      selectedPort,
      setAvailablePorts,
      setIsLoading,
      setSelectedPort,
      toggleConnectionRef,
    ]
  );

  return {
    sendDirectRawCommand,
    refreshModelCatalog,
    selectTargetModel,
    refreshPorts,
  };
}
