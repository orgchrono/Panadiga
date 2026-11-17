/**
 * @file viewmodels/picoDevice/usePicoPortDiscovery.ts
 * @brief Hook handling serial/OTG/BLE enumeration and DIGA model profile selection.
 */

import { useCallback, type Dispatch, type SetStateAction } from 'react';
import { translate } from '@/i18n/useTranslation';
import type { DigaModelProfile } from '@/models';
import {
  isAndroidNativeBridgeAvailable,
  listAndroidUsbOtgDevices,
} from '@/services/androidBridge';
import { bleService } from '@/services/bleService';
import { getDigaModelCatalog, listPicoSerialPorts } from '@/services/picoService';
import { logger } from '@/utils/logger';
import { sendDirectRawCommandForPort } from './picoConnectionActions';
import { pickPreferredSerialPort, type UsePicoDeviceOptions } from './stateHelpers';

export interface UsePicoPortDiscoveryParams {
  selectedPort: string;
  setSelectedPort: Dispatch<SetStateAction<string>>;
  setAvailablePorts: Dispatch<SetStateAction<string[]>>;
  modelCatalog: DigaModelProfile[];
  setModelCatalog: Dispatch<SetStateAction<DigaModelProfile[]>>;
  setSelectedModel: Dispatch<SetStateAction<string>>;
  setIsLoading: Dispatch<SetStateAction<boolean>>;
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
  onLog,
}: UsePicoPortDiscoveryParams) {
  const refreshModelCatalog = useCallback(async () => {
    try {
      const catalog = await getDigaModelCatalog();
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
      const profile = modelCatalog.find((m) => m.model_id === modelId);
      if (modelId === 'AUTO') {
        try {
          const resp = await sendDirectRawCommandForPort(selectedPort, 'AUTODETECT');
          onLog?.('info', 'PICO', translate('errors.pico.autoModeActivated', { resp }));
        } catch (err) {
          logger.warn('PicoDevice', 'AUTODETECT command failed', err);
        }
      } else if (profile) {
        try {
          const resp = await sendDirectRawCommandForPort(
            selectedPort,
            `FLASH_SIZE ${profile.flash_size_mb}`,
          );
          onLog?.(
            'info',
            'HOST',
            translate('errors.pico.profileSelected', {
              model: profile.model_id,
              flashMb: profile.flash_size_mb,
              nvramHex: profile.nvram_base_hex,
              resp,
            }),
          );
        } catch (err) {
          logger.warn('PicoDevice', 'FLASH_SIZE command failed', err);
        }
      }
    },
    [modelCatalog, selectedPort, setSelectedModel, onLog],
  );

  const refreshPorts = useCallback(
    async (injectedPortsOrSilent?: unknown) => {
      const pushedPorts = Array.isArray(injectedPortsOrSilent)
        ? injectedPortsOrSilent.filter((p): p is string => typeof p === 'string')
        : null;
      const isSilent = pushedPorts !== null || injectedPortsOrSilent === true;
      if (!isSilent) setIsLoading(true);
      try {
        const ports = pushedPorts ?? (await listPicoSerialPorts());
        const otgLabels = isAndroidNativeBridgeAvailable()
          ? listAndroidUsbOtgDevices()
              .map(
                (dev) =>
                  `ANDROID-USB-OTG [${dev.deviceName} - ${dev.productName}${dev.isBootsel ? ' BOOTSEL' : ''}]`,
              )
              .filter((label) => !(ports ?? []).includes(label))
          : [];
        const bleEntries =
          bleService.isSupported() && !(ports ?? []).some((p) => p.includes('BLE_PICO_W'))
            ? ['BLE_PICO_W [Bluetooth LE - Nordic UART Service (NUS)]']
            : [];
        const allPorts = [...otgLabels, ...(ports ?? []), ...bleEntries];

        setAvailablePorts(allPorts);
        const targetDefault = pickPreferredSerialPort(allPorts);
        setSelectedPort((prev) => {
          if (targetDefault) {
            return !prev || !allPorts.includes(prev) ? targetDefault : prev;
          }
          if (allPorts.length > 0 && !prev) return allPorts[0];
          if (allPorts.length === 0) return '';
          return prev;
        });
      } catch (err) {
        logger.warn('PicoDevice', 'Failed loading system serial ports', err);
      } finally {
        if (!isSilent) setIsLoading(false);
      }
    },
    [setAvailablePorts, setIsLoading, setSelectedPort],
  );

  return {
    refreshModelCatalog,
    selectTargetModel,
    refreshPorts,
  };
}
