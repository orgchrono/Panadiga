import {
  AM34_NVRAM_BASE_ADDR,
  AM34_NVRAM_WORD_COUNT_16BIT,
  P7501_BLE_DUMP_MAX_WORDS,
  P7501_BOOTSEL_PID_MARKER,
  P7501_BOOTSEL_USB_TOKEN,
  P7501_DUMP_COMMAND_PREFIX,
  P7501_LIVE_DUMP_TIMEOUT_MS,
  P7501_USB_OTG_DUMP_MAX_WORDS,
  SPANSION_BOOTLOADER_SECTOR_COUNT,
  SPANSION_FLASH_SIZE_BYTES,
  SPANSION_NVRAM_BASE_OFFSET,
  SPANSION_NVRAM_FIRST_SA,
  SPANSION_STANDARD_SECTOR_SIZE_BYTES,
} from '@/constants';
import { translate } from '@/i18n/useTranslation';
import type { FlashSectorInfo, LiveNvramCaptureEvent, NvramCleanReport } from '@/models';
import {
  connectAndroidUsbOtg,
  getAndroidCapabilities,
  isAndroidNativeBridgeAvailable,
  listAndroidUsbOtgDevices,
  sendAndroidUsbOtgCommand,
} from '@/services/androidBridge';
import { bleService } from '@/services/bleService';
import { capturePicoNvramLive, listPicoSerialPorts } from '@/services/picoService';
import { ingestP7501DumpOutput, recordLiveNvramCapture } from '@/store/nvramCaptureBridge';
import { formatHex32 } from '@/utils/formatters';
import { mapSpansion142Sectors } from '@/utils/spansionTopology';

const HEX_RADIX = 16;
const ANDROID_USB_OTG_SOURCE = 'ANDROID_USB_OTG';
const ANDROID_USB_OTG_PORT = 'ANDROID-USB-OTG';
const BLE_NUS_SOURCE = 'BLE_NUS';
const BLE_PICO_W_PORT = 'BLE_PICO_W';

function buildDumpCommand(baseAddr: number, wordCount: number): string {
  return `${P7501_DUMP_COMMAND_PREFIX}${baseAddr.toString(HEX_RADIX).toUpperCase()} ${wordCount}`;
}

function shouldUseAndroidUsbOtg(explicitPort?: string): boolean {
  const androidCaps = isAndroidNativeBridgeAvailable() ? getAndroidCapabilities() : null;
  return Boolean(
    androidCaps?.usbHostSupported &&
      !explicitPort?.startsWith('COM') &&
      !explicitPort?.startsWith('/dev/'),
  );
}

async function captureViaAndroidUsbOtg(
  baseAddr: number,
  wordCount: number,
): Promise<LiveNvramCaptureEvent> {
  const dumpCmd = buildDumpCommand(baseAddr, Math.min(wordCount, P7501_USB_OTG_DUMP_MAX_WORDS));
  const initialReply = sendAndroidUsbOtgCommand(dumpCmd, P7501_LIVE_DUMP_TIMEOUT_MS);
  const reply =
    !initialReply.ok && listAndroidUsbOtgDevices().length > 0 && connectAndroidUsbOtg().connected
      ? sendAndroidUsbOtgCommand(dumpCmd, P7501_LIVE_DUMP_TIMEOUT_MS)
      : initialReply;

  if (!reply.ok) {
    throw new Error(reply.output || translate('errors.nvram.usbOtgDumpFailed'));
  }

  const captured = await ingestP7501DumpOutput(
    reply.output,
    ANDROID_USB_OTG_SOURCE,
    ANDROID_USB_OTG_PORT,
    baseAddr,
  );
  if (!captured) {
    throw new Error(translate('errors.nvram.usbOtgNoHexWords'));
  }
  return captured;
}

async function captureViaBleNus(
  baseAddr: number,
  wordCount: number,
): Promise<LiveNvramCaptureEvent> {
  const dumpCmd = buildDumpCommand(baseAddr, Math.min(wordCount, P7501_BLE_DUMP_MAX_WORDS));
  const reply = await bleService.sendAndAwaitReply(dumpCmd, P7501_LIVE_DUMP_TIMEOUT_MS);
  const captured = await ingestP7501DumpOutput(reply, BLE_NUS_SOURCE, BLE_PICO_W_PORT, baseAddr);
  if (!captured) {
    throw new Error(translate('errors.nvram.bleNoHexWords'));
  }
  return captured;
}

async function resolveDesktopPicoPort(explicitPort?: string): Promise<string> {
  const explicitClean = explicitPort?.trim() ?? '';
  if (explicitClean) {
    return explicitClean;
  }
  const ports = await listPicoSerialPorts();
  const preferred = ports.find(
    (port) => !port.includes(P7501_BOOTSEL_USB_TOKEN) && !port.includes(P7501_BOOTSEL_PID_MARKER),
  );
  const resolved = (preferred ?? ports[0] ?? '').trim();
  if (!resolved) {
    throw new Error(translate('errors.nvram.noPicoProbe'));
  }
  return resolved;
}

/**
 * Purely synthesizes the Flash Sector Map (SSOT) directly from the live
 * NvramCleanReport returned by the Rust kernel, supporting any DIGA geometry
 * (2 MB, 4 MB, 8 MB, 16 MB) without hardcoded arrays or mutable state.
 */
export function buildSectorMapFromReport(
  report: NvramCleanReport | null | undefined,
): FlashSectorInfo[] {
  if (!report) return [];
  if (report.total_flash_bytes === SPANSION_FLASH_SIZE_BYTES) {
    const micros = report.microsectors ?? [];
    return mapSpansion142Sectors((desc, sectorType) => {
      const address = formatHex32(desc.startOffset);
      if (sectorType === 'nvram_microsector') {
        const ms = micros[desc.sa - SPANSION_NVRAM_FIRST_SA];
        const activeBytes = ms?.active_bytes ?? 0;
        const isCorrupted = activeBytes > 0;
        return {
          sectorType,
          isCorrupted,
          activeBytes,
          status: isCorrupted ? 'corrupted' : 'clean',
          note: translate('nvram.sectors.microsectorNote', {
            sector: ms?.sector_name ?? desc.name,
            address,
          }),
        };
      }
      const noteKey =
        sectorType === 'bootloader'
          ? 'nvram.sectors.bootloaderNote'
          : sectorType === 'bgo_region'
            ? 'nvram.sectors.bgoNote'
            : 'nvram.sectors.osNote';
      return {
        sectorType,
        isCorrupted: false,
        activeBytes: desc.sizeBytes,
        status: sectorType === 'bootloader' ? 'immutable' : 'clean',
        note: translate(noteKey, { address }),
      };
    });
  }

  const nvramStart = report.nvram_offset_start ?? SPANSION_NVRAM_BASE_OFFSET;
  const standardSectorCount = Math.floor(nvramStart / SPANSION_STANDARD_SECTOR_SIZE_BYTES);

  const standardSectors: FlashSectorInfo[] = Array.from(
    { length: standardSectorCount },
    (_, index) => {
      const startOffset = index * SPANSION_STANDARD_SECTOR_SIZE_BYTES;
      const endOffset = startOffset + SPANSION_STANDARD_SECTOR_SIZE_BYTES - 1;
      const isBootloader = index < SPANSION_BOOTLOADER_SECTOR_COUNT;
      const address = formatHex32(startOffset);

      return {
        index,
        name: `SA${index}`,
        start_offset: startOffset,
        end_offset: endOffset,
        size_bytes: SPANSION_STANDARD_SECTOR_SIZE_BYTES,
        sector_type: isBootloader ? 'bootloader' : 'os_firmware',
        is_corrupted_area: false,
        status: isBootloader ? 'immutable' : 'clean',
        active_bytes: SPANSION_STANDARD_SECTOR_SIZE_BYTES,
        notes: translate(
          isBootloader ? 'nvram.sectors.bootloaderNote' : 'nvram.sectors.osNote',
          { address },
        ),
      };
    },
  );

  const microsectors: FlashSectorInfo[] = (report.microsectors ?? []).map((ms, idx) => {
    const isCorrupted = ms.active_bytes > 0;
    return {
      index: standardSectorCount + idx,
      name: ms.sector_name,
      start_offset: ms.offset,
      end_offset: ms.offset + ms.total_bytes - 1,
      size_bytes: ms.total_bytes,
      sector_type: 'nvram_microsector',
      is_corrupted_area: isCorrupted,
      status: isCorrupted ? 'corrupted' : 'clean',
      active_bytes: ms.active_bytes,
      notes: translate('nvram.sectors.microsectorNote', {
        sector: ms.sector_name,
        address: formatHex32(ms.offset),
      }),
    };
  });

  return [...standardSectors, ...microsectors];
}

/**
 * Captures live NVRAM hex words from Android USB-OTG, BLE NUS, or Desktop USB CDC.
 */
export async function captureLivePicoNvramDump(
  explicitPort?: string,
  baseAddr = AM34_NVRAM_BASE_ADDR,
  wordCount = AM34_NVRAM_WORD_COUNT_16BIT,
): Promise<LiveNvramCaptureEvent> {
  if (shouldUseAndroidUsbOtg(explicitPort)) {
    return captureViaAndroidUsbOtg(baseAddr, wordCount);
  }
  if (bleService.isConnected() && (!explicitPort || explicitPort.startsWith('BLE_'))) {
    return captureViaBleNus(baseAddr, wordCount);
  }
  const targetPort = await resolveDesktopPicoPort(explicitPort);
  const evt = await capturePicoNvramLive(targetPort, baseAddr, wordCount);
  recordLiveNvramCapture(evt);
  return evt;
}
