import type { FlashSectorInfo, LiveNvramCaptureEvent, NvramCleanReport } from '@/models';
import { invokeCommand } from '@/services/ipc';
import { bleService } from '@/services/bleService';
import {
  connectAndroidUsbOtg,
  getAndroidCapabilities,
  isAndroidNativeBridgeAvailable,
  listAndroidUsbOtgDevices,
  sendAndroidUsbOtgCommand,
} from '@/services/androidBridge';
import { ingestP7501DumpOutput, recordLiveNvramCapture } from '@/services/liveNvramCaptureStore';

/**
 * Dynamically synthesizes the Flash Sector Map (SSOT) directly from the live
 * NvramCleanReport returned by the Rust kernel, supporting any DIGA geometry
 * (2 MB, 4 MB, 8 MB, 16 MB) without hardcoded mock arrays.
 */
export function buildSectorMapFromReport(
  report: NvramCleanReport | null | undefined
): FlashSectorInfo[] {
  if (!report) return [];
  const sectors: FlashSectorInfo[] = [];
  const nvramStart = report.nvram_offset_start ?? 0x007f0000;
  const standardSectorSize = 65536; // 64 KB
  const standardSectorCount = Math.floor(nvramStart / standardSectorSize);

  for (let i = 0; i < standardSectorCount; i++) {
    const startOffset = i * standardSectorSize;
    const endOffset = startOffset + standardSectorSize - 1;
    const isBootloader = i < 16;

    sectors.push({
      index: i,
      name: `SA${i}`,
      start_offset: startOffset,
      end_offset: endOffset,
      size_bytes: standardSectorSize,
      sector_type: isBootloader ? 'bootloader' : 'os_firmware',
      is_corrupted_area: false,
      status: isBootloader ? 'immutable' : 'clean',
      active_bytes: standardSectorSize,
      notes: isBootloader
        ? `Primary AM34 Bootloader Bank (0x${startOffset.toString(16).toUpperCase().padStart(8, '0')})`
        : `Main OS & Codec Executable Segment (0x${startOffset.toString(16).toUpperCase().padStart(8, '0')})`,
    });
  }

  (report.microsectors || []).forEach((ms, idx) => {
    const sectorIndex = standardSectorCount + idx;
    const isCorrupted = ms.active_bytes > 0;
    sectors.push({
      index: sectorIndex,
      name: ms.sector_name,
      start_offset: ms.offset,
      end_offset: ms.offset + ms.total_bytes - 1,
      size_bytes: ms.total_bytes,
      sector_type: 'nvram_microsector',
      is_corrupted_area: isCorrupted,
      status: isCorrupted ? 'corrupted' : 'clean',
      active_bytes: ms.active_bytes,
      notes: `Top Parameter Bank: ${ms.sector_name} (EEMUL NVRAM / 0x${ms.offset.toString(16).toUpperCase()})`,
    });
  });

  return sectors;
}

/**
 * Captures live NVRAM hex words from Android USB-OTG, BLE NUS, or Desktop USB CDC.
 */
export async function captureLivePicoNvramDump(
  explicitPort?: string,
  baseAddr = 0x807f0000,
  wordCount = 16384
): Promise<LiveNvramCaptureEvent> {
  const androidCaps = isAndroidNativeBridgeAvailable() ? getAndroidCapabilities() : null;
  if (
    androidCaps?.usbHostSupported &&
    !explicitPort?.startsWith('COM') &&
    !explicitPort?.startsWith('/dev/')
  ) {
    const dumpCmd = `DUMP 0x${baseAddr.toString(16).toUpperCase()} ${Math.min(wordCount, 2048)}`;
    let reply = sendAndroidUsbOtgCommand(dumpCmd, 6000);
    if (!reply.ok && listAndroidUsbOtgDevices().length > 0) {
      const conn = connectAndroidUsbOtg();
      if (conn.connected) {
        reply = sendAndroidUsbOtgCommand(dumpCmd, 6000);
      }
    }
    if (!reply.ok) {
      throw new Error(reply.output || 'Falha no comando DUMP via Android USB-OTG.');
    }
    const captured = await ingestP7501DumpOutput(
      reply.output,
      'ANDROID_USB_OTG',
      'ANDROID-USB-OTG',
      baseAddr
    );
    if (!captured) {
      throw new Error('Nenhuma palavra hexadecimal capturada na resposta DUMP USB-OTG.');
    }
    return captured;
  }

  if (bleService.isConnected() && (!explicitPort || explicitPort.startsWith('BLE_'))) {
    const reply = await bleService.sendAndAwaitReply(
      `DUMP 0x${baseAddr.toString(16).toUpperCase()} ${Math.min(wordCount, 256)}`,
      6000
    );
    const captured = await ingestP7501DumpOutput(reply, 'BLE_NUS', 'BLE_PICO_W', baseAddr);
    if (!captured) {
      throw new Error('Nenhuma palavra hexadecimal capturada na resposta DUMP BLE NUS.');
    }
    return captured;
  }

  let targetPort = explicitPort?.trim() || '';
  if (!targetPort) {
    const ports = await invokeCommand<string[]>('list_pico_serial_ports');
    const preferred = ports.find(p => !p.includes('BOOTSEL') && !p.includes('PID:0003'));
    targetPort = (preferred || ports[0] || '').trim();
  }
  if (!targetPort) {
    throw new Error(
      'Nenhuma sonda Pico conectada (P7501 Async UART). Conecte via USB CDC/OTG ou BLE e entre em CDDIAG.'
    );
  }

  const evt = await invokeCommand<LiveNvramCaptureEvent>('pico_capture_nvram_live', {
    portName: targetPort,
    baseAddr,
    wordCount,
  });
  recordLiveNvramCapture(evt);
  return evt;
}
