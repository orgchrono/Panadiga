import { useState, useEffect, useCallback } from 'react';
import {
  DIGA_HEX_BOOKMARKS,
  HEX_INSPECTOR_BYTES_PER_ROW,
  LBA_SECTOR_SIZE_BYTES,
  TOAST_DURATION_MS,
} from '@/constants';
import { useTransientFlag } from '@/hooks/useManagedTimeouts';
import { translate } from '@/i18n/useTranslation';
import type { HexDumpRow, HexBookmark } from '@/models';
import { readHexDump } from '@/services/systemService';
import { getRuntimeSettingsSnapshot } from '@/store/runtimeSettingsBridge';
import { logger } from '@/utils/logger';
import { useToast } from '../context/ToastContext';

const HEX_RADIX = 16;
const DECIMAL_RADIX = 10;
const HEX_PREFIX_LENGTH = 2;

export function useHexInspectorViewModel() {
  const { showToast } = useToast();
  const { active: hasFeedback, trigger: triggerFeedback } = useTransientFlag(TOAST_DURATION_MS);
  const [currentOffset, setCurrentOffset] = useState<number>(0);
  const [hexRows, setHexRows] = useState<HexDumpRow[]>([]);
  const [bookmarks] = useState<HexBookmark[]>(DIGA_HEX_BOOKMARKS);
  const [selectedBookmark, setSelectedBookmark] = useState<HexBookmark | null>(DIGA_HEX_BOOKMARKS[0]);
  const [jumpInput, setJumpInput] = useState<string>('0x00000000');
  const [loading, setLoading] = useState<boolean>(true);
  const [feedbackText, setFeedbackText] = useState<string | null>(null);
  const rowsPerPage = Math.max(
    1,
    Math.floor(getRuntimeSettingsSnapshot().storage.hex_page_bytes / HEX_INSPECTOR_BYTES_PER_ROW),
  );

  const fetchRows = useCallback(async (offset: number) => {
    setLoading(true);
    try {
      const data = await readHexDump(offset, rowsPerPage);
      setHexRows(data);
    } catch (err) {
      logger.error('HexInspectorViewModel', 'Failed reading hex dump', err);
    } finally {
      setLoading(false);
    }
  }, [rowsPerPage]);

  useEffect(() => {
    void Promise.resolve().then(() => fetchRows(currentOffset));
  }, [currentOffset, fetchRows]);

  const jumpToOffset = useCallback((offset: number) => {
    const cleanOffset = Math.max(0, offset - (offset % HEX_INSPECTOR_BYTES_PER_ROW));
    setCurrentOffset(cleanOffset);
    setJumpInput(`0x${cleanOffset.toString(HEX_RADIX).toUpperCase()}`);
  }, []);

  const jumpToLba = useCallback(
    (lba: number) => {
      jumpToOffset(lba * LBA_SECTOR_SIZE_BYTES);
    },
    [jumpToOffset],
  );

  const selectBookmark = useCallback(
    (bm: HexBookmark) => {
      setSelectedBookmark(bm);
      jumpToOffset(bm.offset);
    },
    [jumpToOffset],
  );

  const nextPage = useCallback(() => {
    jumpToOffset(currentOffset + rowsPerPage * HEX_INSPECTOR_BYTES_PER_ROW);
  }, [currentOffset, rowsPerPage, jumpToOffset]);

  const prevPage = useCallback(() => {
    jumpToOffset(Math.max(0, currentOffset - rowsPerPage * HEX_INSPECTOR_BYTES_PER_ROW));
  }, [currentOffset, rowsPerPage, jumpToOffset]);

  const jumpToInputOffset = useCallback(() => {
    const trimmed = jumpInput.trim();
    const parsed = trimmed.toLowerCase().startsWith('0x')
      ? Number.parseInt(trimmed.substring(HEX_PREFIX_LENGTH), HEX_RADIX)
      : Number.parseInt(trimmed, DECIMAL_RADIX);
    if (!isNaN(parsed)) {
      jumpToOffset(parsed);
      const msg = translate('errors.hex.jumpedToOffset', {
        hex: parsed.toString(HEX_RADIX).toUpperCase(),
      });
      setFeedbackText(msg);
      showToast({
        type: 'success',
        title: translate('errors.hex.offsetNavTitle'),
        message: msg,
      });
      triggerFeedback();
    }
  }, [jumpInput, jumpToOffset, showToast, triggerFeedback]);

  return {
    currentOffset,
    currentLba: Math.floor(currentOffset / LBA_SECTOR_SIZE_BYTES),
    hexRows,
    bookmarks,
    selectedBookmark,
    jumpInput,
    setJumpInput,
    loading,
    feedbackMessage: hasFeedback ? feedbackText : null,
    selectBookmark,
    jumpToOffset,
    jumpToLba,
    jumpToInputOffset,
    nextPage,
    prevPage,
    refresh: () => fetchRows(currentOffset),
  };
}
