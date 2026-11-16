/**
 * @file viewmodels/picoDevice/usePicoTransport.ts
 * @brief Transport hook managing USB CDC, Android USB-OTG, BLE NUS connection, port enumeration, and target profiles.
 */

import {
  useCallback,
  useEffect,
  useRef,
  type Dispatch,
  type MutableRefObject,
  type SetStateAction,
} from 'react';
import type { ColdBreakinMode, DigaModelProfile, PicoDeviceStatus } from '@/models';
import { invokeCommand } from '@/services/ipc';
import { bleService } from '@/services/bleService';
import {
  connectAndroidUsbOtg,
  disconnectAndroidUsbOtg,
  isAndroidNativeBridgeAvailable,
  listAndroidUsbOtgDevices,
  sendAndroidUsbOtgCommand,
} from '@/services/androidBridge';
import { logger } from '@/utils/logger';
import {
  buildMobileDeviceStatus,
  DEFAULT_DISCONNECTED_STATUS,
  isGdbBridgeRunning,
  parseVddFromStatusRaw,
  PICO_PORT_PATTERN,
  pickPreferredSerialPort,
  type UsePicoDeviceOptions,
} from './stateHelpers';

interface UsePicoTransportParams {
  deviceStatus: PicoDeviceStatus | null;
  setDeviceStatus: Dispatch<SetStateAction<PicoDeviceStatus | null>>;
  setIsLoading: Dispatch<SetStateAction<boolean>>;
  setAvailablePorts: Dispatch<SetStateAction<string[]>>;
  selectedPort: string;
  setSelectedPort: Dispatch<SetStateAction<string>>;
  modelCatalog: DigaModelProfile[];
  setModelCatalog: Dispatch<SetStateAction<DigaModelProfile[]>>;
  setSelectedModel: Dispatch<SetStateAction<string>>;
  isConnectingRef: MutableRefObject<boolean>;
  onLog?: UsePicoDeviceOptions['onLog'];
}

export function usePicoTransport({
  deviceStatus,
  setDeviceStatus,
  setIsLoading,
  setAvailablePorts,
  selectedPort,
  setSelectedPort,
  modelCatalog,
  setModelCatalog,
  setSelectedModel,
  isConnectingRef,
  onLog,
}: UsePicoTransportParams) {
  const manualDisconnectRef = useRef<boolean>(false);
  const isConnectedRef = useRef<boolean>(Boolean(deviceStatus?.connected));
  const toggleConnectionRef = useRef<((overridePort?: unknown) => Promise<void>) | null>(null);

  useEffect(() => {
    isConnectedRef.current = Boolean(deviceStatus?.connected);
  }, [deviceStatus?.connected]);

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

  const refreshPorts = useCallback(async (injectedPortsOrSilent?: unknown) => {
    const pushedPorts = Array.isArray(injectedPortsOrSilent)
      ? injectedPortsOrSilent.filter((p): p is string => typeof p === 'string')
      : null;
    const isSilent = pushedPorts !== null || injectedPortsOrSilent === true;
    if (!isSilent) setIsLoading(true);
    try {
      const ports = pushedPorts ?? (await invokeCommand<string[]>('list_pico_serial_ports'));
      const allPorts = [...(ports || [])];

      if (isAndroidNativeBridgeAvailable()) {
        const otgDevices = listAndroidUsbOtgDevices();
        otgDevices.forEach(dev => {
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
  }, [isConnectingRef, onLog, selectedPort, setAvailablePorts, setIsLoading, setSelectedPort]);

  const pollDeviceStatus = useCallback(async () => {
    if (!selectedPort || isConnectingRef.current) return;
    if (await isGdbBridgeRunning()) return;
    if (selectedPort.startsWith('ANDROID-USB-OTG') || selectedPort.startsWith('BLE_')) {
      try {
        const raw = await sendDirectRawCommand('STATUS');
        const vdd = parseVddFromStatusRaw(raw);
        setDeviceStatus(prev => buildMobileDeviceStatus(prev, selectedPort, vdd));
      } catch (err) {
        logger.warn('PicoDevice', 'Failed mobile STATUS telemetry poll', err);
      }
      return;
    }

    setIsLoading(true);
    try {
      const status = await invokeCommand<PicoDeviceStatus>('pico_get_status', {
        portName: selectedPort,
        port_name: selectedPort,
      });
      setDeviceStatus(status);
    } catch (err) {
      logger.warn('PicoDevice', 'Failed querying Pico device telemetry', err);
    } finally {
      setIsLoading(false);
    }
  }, [isConnectingRef, selectedPort, sendDirectRawCommand, setDeviceStatus, setIsLoading]);

  const changeBreakinMode = useCallback(
    async (mode: ColdBreakinMode) => {
      try {
        if (selectedPort.startsWith('ANDROID-USB-OTG') || selectedPort.startsWith('BLE_')) {
          await sendDirectRawCommand(`BREAK_IN ${mode}`);
          setDeviceStatus(prev => (prev ? { ...prev, breakin_mode: mode } : prev));
          onLog?.('info', 'HOST', `Modo de Cold Break-In alterado para: Modo ${mode}`);
          return;
        }
        const updated = await invokeCommand<PicoDeviceStatus>('pico_set_breakin_mode', {
          portName: selectedPort,
          port_name: selectedPort,
          mode,
        });
        setDeviceStatus(updated);
        onLog?.('info', 'HOST', `Modo de Cold Break-In alterado para: Modo ${mode}`);
      } catch (err) {
        onLog?.('error', 'HOST', `Erro ao alterar modo: ${err}`);
      }
    },
    [selectedPort, sendDirectRawCommand, setDeviceStatus, onLog]
  );

  const toggleConnection = useCallback(
    async (overridePort?: unknown) => {
      const targetPort =
        typeof overridePort === 'string' && overridePort.trim() ? overridePort : selectedPort;
      if (!targetPort) {
        onLog?.('warn', 'HOST', 'Nenhuma porta selecionada para conexão.');
        return;
      }

      if (isConnectingRef.current) {
        onLog?.('warn', 'HOST', 'Conexão já em andamento — aguarde antes de tentar novamente.');
        return;
      }

      if (deviceStatus?.connected) {
        manualDisconnectRef.current = true;
        try {
          await invokeCommand('set_pico_telemetry_stream', { active: false }).catch(() => {});
          if (targetPort.startsWith('ANDROID-USB-OTG')) {
            await sendDirectRawCommand('ISOLATE').catch(() => {});
            disconnectAndroidUsbOtg();
          } else if (targetPort.startsWith('BLE_') && bleService.isConnected()) {
            bleService.disconnect();
          } else {
            await invokeCommand<string>('pico_send_serial_command', {
              portName: targetPort,
              port_name: targetPort,
              command: 'ISOLATE',
            }).catch(() => {});
          }
        } finally {
          setDeviceStatus(prev => ({
            ...(prev || DEFAULT_DISCONNECTED_STATUS),
            connected: false,
            core_state: 'DISCONNECTED',
            vdd_volts: 0.0,
            target_powered: false,
            port: targetPort,
          }));
          onLog?.(
            'warn',
            'HOST',
            `Desconectado da interface P7501 UART / OCD (${targetPort}). Pinos colocados em Hi-Z.`
          );
        }
        return;
      }

      if (await isGdbBridgeRunning()) {
        onLog?.(
          'warn',
          'HOST',
          `${targetPort} está em uso pela Ponte GDB RSP (a sessão de depuração já usa o Pico). Pare a ponte para enviar comandos CDC diretos.`
        );
        return;
      }

      manualDisconnectRef.current = false;
      setIsLoading(true);
      isConnectingRef.current = true;
      onLog?.('info', 'HOST', `Estabelecendo conexão física com ${targetPort} (115200 bps)...`);

      try {
        if (targetPort.startsWith('ANDROID-USB-OTG')) {
          const devMatch = targetPort.match(/\[([^\s\]]+)/);
          const otgRes = connectAndroidUsbOtg(devMatch?.[1] || '');
          if (!otgRes.connected) {
            throw new Error(otgRes.message || 'Dispositivo USB-OTG não conectado.');
          }
          const vdd = parseVddFromStatusRaw(sendAndroidUsbOtgCommand('STATUS', 2000).output);
          setDeviceStatus(prev => buildMobileDeviceStatus(prev, targetPort, vdd, 'RP2040/RP2350 OTG'));
          onLog?.('success', 'HOST', `${otgRes.message} | VDD: ${vdd.toFixed(2)}V`);
          return;
        }

        if (targetPort.startsWith('BLE_')) {
          const bleDev = await bleService.connect();
          let measuredVdd = 3.3;
          try {
            const statusOut = await bleService.sendAndAwaitReply('STATUS', 2500);
            measuredVdd = parseVddFromStatusRaw(statusOut, 3.3);
          } catch {
            // Keep default BLE handshake if status reply times out on first packet
          }
          onLog?.('success', 'HOST', `Conectado via Bluetooth LE NUS: ${bleDev.name} (VDD: ${measuredVdd.toFixed(2)}V)`);
          setDeviceStatus(prev => buildMobileDeviceStatus(prev, targetPort, measuredVdd, 'Pico W (BLE)'));
          return;
        }

        const result = await invokeCommand<{
          connected: boolean;
          port_name: string;
          baud_rate: number;
          vdd_voltage: number;
          target_powered: boolean;
          firmware_banner: string;
          message: string;
        }>('pico_connect_and_test', {
          portName: targetPort,
          port_name: targetPort,
          baudRate: 115200,
          baud_rate: 115200,
        });

        if (result && result.connected) {
          const isRp2350 = result.port_name.includes('2350') || result.firmware_banner.includes('2350');
          setDeviceStatus(prev => ({
            ...(prev || DEFAULT_DISCONNECTED_STATUS),
            connected: true,
            port: targetPort,
            baudRate: result.baud_rate,
            vdd_volts: result.vdd_voltage,
            target_powered: result.target_powered,
            core_state: result.target_powered ? 'HALTED' : 'RUNNING',
            firmware_version: result.firmware_banner || 'P7501 UART / Panadiga Master',
            chip_id: isRp2350 ? 'RP2350-A2' : 'RP2040-B2',
            last_keepalive: 'OK',
          }));

          onLog?.('success', 'PICO', result.firmware_banner);
          onLog?.(
            'success',
            'HOST',
            result.message || `Conexão estabelecida com sucesso em ${targetPort}! VDD: ${result.vdd_voltage.toFixed(2)}V`
          );

          await invokeCommand('set_pico_telemetry_stream', {
            active: true,
            portName: targetPort,
            port_name: targetPort,
          }).catch(() => {});
        } else {
          throw new Error(result?.message || 'Dispositivo não respondeu ao handshake inicial.');
        }
      } catch (err) {
        const errMsg = err instanceof Error ? err.message : String(err);
        onLog?.('error', 'HOST', `Falha ao conectar em ${targetPort}: ${errMsg}`);
        setDeviceStatus(prev => ({
          ...(prev || DEFAULT_DISCONNECTED_STATUS),
          connected: false,
          port: targetPort,
          core_state: 'DISCONNECTED',
          vdd_volts: 0.0,
          target_powered: false,
        }));
      } finally {
        setIsLoading(false);
        isConnectingRef.current = false;
      }
    },
    [deviceStatus, isConnectingRef, onLog, selectedPort, sendDirectRawCommand, setDeviceStatus, setIsLoading]
  );

  useEffect(() => {
    toggleConnectionRef.current = toggleConnection;
  }, [toggleConnection]);

  return {
    refreshModelCatalog,
    selectTargetModel,
    refreshPorts,
    pollDeviceStatus,
    changeBreakinMode,
    toggleConnection,
  };
}
