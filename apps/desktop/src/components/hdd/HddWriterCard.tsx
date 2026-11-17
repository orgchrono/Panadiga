import React from 'react';
import {
  AlertTriangle,
  CheckCircle2,
  Flame,
  HardDrive,
  RefreshCw,
  ShieldCheck,
} from 'lucide-react';
import { StatusBadge } from '@/components/StatusBadge';
import {
  DECIMAL_GB_BYTES,
  HDD_DEFAULT_CAPACITY_GB,
  HDD_MAX_CAPACITY_GB,
  HDD_MIN_CAPACITY_GB,
  HDD_WRITER_QUICK_GBS,
  LBA_SECTOR_SIZE_BYTES,
  ONE_TERABYTE_GB,
} from '@/constants';
import { useTranslation } from '@/i18n/useTranslation';
import type { DriveWriteReport, PhysicalDriveInfo } from '@/models';

const RAW_IMAGE_PRESETS: ReadonlyArray<{ readonly label: string; readonly path: string }> = [
  {
    label: '250GB + TBC (.img)',
    path: 'firmware_and_images/panasonic_dmr_eh55_hdd_250gb_patched.img',
  },
  {
    label: '250GB ODD Stock (.img)',
    path: 'firmware_and_images/panasonic_dmr_eh55_hdd_250gb_odd_stock.img',
  },
  {
    label: 'Original hdd_100.bin',
    path: 'firmware_and_images/panasonic DMR EH-55 Firmware hdd_100.bin',
  },
];

export interface HddWriterCardProps {
  readonly drives: readonly PhysicalDriveInfo[];
  readonly selectedDrive: PhysicalDriveInfo | null;
  readonly setSelectedDrive: (drive: PhysicalDriveInfo | null) => void;
  readonly loadingDrives: boolean;
  readonly confirmInput: string;
  readonly setConfirmInput: (val: string) => void;
  readonly customSizeGb: number | null;
  readonly setCustomSizeGb: (gb: number | null) => void;
  readonly rawImagePath: string;
  readonly setRawImagePath: (path: string) => void;
  readonly resolvedTargetSectors: number;
  readonly executing: boolean;
  readonly writeReport: DriveWriteReport | null;
  readonly expectedToken: string;
  readonly isTokenValid: boolean;
  readonly fetchDrives: () => Promise<void>;
  readonly handleProceduralFormat: () => Promise<void>;
  readonly handleWriteRawImage: () => Promise<void>;
  readonly handleRestoreMirrors: () => Promise<void>;
}

export const HddWriterCard: React.FC<HddWriterCardProps> = ({
  drives,
  selectedDrive,
  setSelectedDrive,
  loadingDrives,
  confirmInput,
  setConfirmInput,
  customSizeGb,
  setCustomSizeGb,
  rawImagePath,
  setRawImagePath,
  resolvedTargetSectors,
  executing,
  writeReport,
  expectedToken,
  isTokenValid,
  fetchDrives,
  handleProceduralFormat,
  handleWriteRawImage,
  handleRestoreMirrors,
}) => {
  const { t } = useTranslation();

  return (
    <div className="rounded-xl bg-bench-900 border border-slate-800 p-6 shadow-xl space-y-6 font-mono">
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 border-b border-slate-800 pb-4">
        <div>
          <h3 className="text-sm font-bold text-slate-100 flex items-center gap-2">
            <HardDrive className="w-5 h-5 text-cyan-400" />
            <span>{t('hdd.writer.title')}</span>
          </h3>
          <p className="text-xs text-slate-400 mt-1">{t('hdd.writer.subtitle')}</p>
        </div>
        <button
          type="button"
          onClick={() => void fetchDrives()}
          disabled={loadingDrives}
          className="px-3 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-750 text-slate-300 text-xs font-semibold flex items-center gap-1.5 transition-colors"
        >
          <RefreshCw className={`w-3.5 h-3.5 ${loadingDrives ? 'animate-spin' : ''}`} />
          <span>{t('hdd.writer.refreshBtn')}</span>
        </button>
      </div>

      <div className="p-4 rounded-xl bg-amber-950/30 border border-amber-800/60 text-xs space-y-2">
        <div className="flex items-center gap-2 text-amber-400 font-bold">
          <AlertTriangle className="w-4 h-4 shrink-0" />
          <span>{t('hdd.writer.safetyTitle')}</span>
        </div>
        <p className="text-slate-300 leading-relaxed text-[11px]">
          {t('hdd.writer.safetyBodyWindowsPrefix')}{' '}
          <code className="text-rose-400">\\.\PhysicalDrive0</code> {t('hdd.writer.safetyBodyMiddle')}{' '}
          <code className="text-rose-400">/dev/sda</code> {t('hdd.writer.safetyBodySuffix')}
        </p>
      </div>

      <div className="space-y-3">
        <span className="text-xs text-slate-400 font-semibold uppercase block">
          {t('hdd.writer.selectTargetLabel')}
        </span>
        {drives.length === 0 ? (
          <div className="p-4 rounded-xl bg-slate-950 border border-slate-800 text-xs text-slate-500 text-center">
            {t('hdd.writer.emptyDrives')}
          </div>
        ) : (
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            {drives.map((drive) => {
              const isSelected = selectedDrive?.device_path === drive.device_path;
              return (
                <button
                  key={drive.device_path}
                  type="button"
                  disabled={!drive.safe_to_write}
                  aria-pressed={isSelected}
                  onClick={() => drive.safe_to_write && setSelectedDrive(drive)}
                  className={`p-4 rounded-xl border text-left transition-all ${
                    drive.is_system_drive
                      ? 'bg-rose-950/20 border-rose-900/60 opacity-60 cursor-not-allowed'
                      : isSelected
                        ? 'bg-cyan-500/10 border-cyan-500/50 shadow-lg cursor-pointer'
                        : 'bg-slate-950 border-slate-800 hover:border-slate-700 cursor-pointer'
                  }`}
                >
                  <div className="flex items-center justify-between mb-2">
                    <span className="font-bold text-xs text-slate-200">{drive.display_name}</span>
                    {drive.is_system_drive ? (
                      <StatusBadge variant="error" label={t('hdd.writer.badgeLocked')} />
                    ) : (
                      <StatusBadge variant="success" label={t('hdd.writer.badgeSafe')} />
                    )}
                  </div>
                  <span className="text-[11px] font-mono text-cyan-400 block">
                    {drive.device_path}
                  </span>
                  {drive.lockout_reason && (
                    <span className="text-[10px] text-rose-400 block mt-1">
                      {drive.lockout_reason}
                    </span>
                  )}
                </button>
              );
            })}
          </div>
        )}
      </div>

      {selectedDrive && selectedDrive.safe_to_write && (
        <div className="p-5 rounded-xl bg-slate-950 border border-slate-800 space-y-4">
          <div className="flex items-center gap-2 text-cyan-300 font-semibold text-xs">
            <ShieldCheck className="w-4 h-4 text-cyan-400" />
            <span>
              {t('hdd.writer.confirmSectionTitle', { path: selectedDrive.device_path })}
            </span>
          </div>

          <div className="p-3.5 rounded-xl bg-bench-900 border border-slate-800 space-y-2.5 text-xs">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <span className="text-[11px] text-slate-300 font-semibold uppercase">
                {t('hdd.writer.superblockTitle')}
              </span>
              <span className="text-[11px] text-emerald-300 font-bold">
                {t('hdd.writer.sectorsSummary', {
                  sectors: resolvedTargetSectors.toLocaleString(),
                  gb: ((resolvedTargetSectors * LBA_SECTOR_SIZE_BYTES) / DECIMAL_GB_BYTES).toFixed(
                    1,
                  ),
                })}
              </span>
            </div>
            <div className="flex flex-wrap items-center gap-1.5">
              <button
                type="button"
                onClick={() => setCustomSizeGb(null)}
                className={`px-2.5 py-1 rounded-lg text-[11px] font-bold border transition-colors ${
                  customSizeGb === null
                    ? 'bg-emerald-500 text-slate-950 border-emerald-400'
                    : 'bg-slate-950 text-slate-300 border-slate-800 hover:border-slate-700'
                }`}
              >
                {t('hdd.writer.autoPhysicalPreset')}
              </button>
              {HDD_WRITER_QUICK_GBS.map((gb) => {
                const label =
                  gb >= ONE_TERABYTE_GB
                    ? `${gb / ONE_TERABYTE_GB} ${t('hdd.writer.tbUnit')}`
                    : `${gb} GB`;
                return (
                  <button
                    key={gb}
                    type="button"
                    onClick={() => setCustomSizeGb(gb)}
                    className={`px-2.5 py-1 rounded-lg text-[11px] font-bold border transition-colors ${
                      customSizeGb === gb
                        ? 'bg-emerald-500 text-slate-950 border-emerald-400'
                        : 'bg-slate-950 text-slate-300 border-slate-800 hover:border-slate-700'
                    }`}
                  >
                    {label}
                  </button>
                );
              })}
              <input
                type="number"
                aria-label={t('hdd.writer.gbCustomPlaceholder')}
                min={HDD_MIN_CAPACITY_GB}
                max={HDD_MAX_CAPACITY_GB}
                placeholder={t('hdd.writer.gbCustomPlaceholder')}
                value={customSizeGb ?? ''}
                onChange={(e) => {
                  const val = e.target.value.trim();
                  setCustomSizeGb(
                    val === ''
                      ? null
                      : Math.max(
                          HDD_MIN_CAPACITY_GB,
                          Math.min(HDD_MAX_CAPACITY_GB, Number(val) || HDD_DEFAULT_CAPACITY_GB),
                        ),
                  );
                }}
                className="w-24 px-2 py-1 rounded-lg bg-slate-950 border border-slate-700 text-[11px] text-cyan-300 focus:outline-none focus:border-cyan-400 ml-auto"
              />
            </div>
          </div>

          <div className="space-y-2">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <label htmlFor="hdd-writer-raw-path" className="text-[11px] text-slate-400">
                {t('hdd.writer.rawImageLabel')}
              </label>
              <div className="flex flex-wrap gap-1.5">
                {RAW_IMAGE_PRESETS.map((preset) => (
                  <button
                    key={preset.path}
                    type="button"
                    onClick={() => setRawImagePath(preset.path)}
                    className={`px-2 py-0.5 rounded text-[10px] border transition-colors ${
                      rawImagePath === preset.path
                        ? 'bg-cyan-500/20 text-cyan-200 border-cyan-500/50 font-bold'
                        : 'bg-bench-900 text-slate-400 border-slate-800 hover:border-slate-700'
                    }`}
                  >
                    {preset.label}
                  </button>
                ))}
              </div>
            </div>
            <input
              id="hdd-writer-raw-path"
              type="text"
              value={rawImagePath}
              onChange={(e) => setRawImagePath(e.target.value)}
              placeholder={t('hdd.writer.rawPlaceholder')}
              className="w-full px-3 py-2 rounded-xl bg-slate-900 border border-slate-700 text-xs text-cyan-300 focus:outline-none focus:border-cyan-500 font-mono"
            />
          </div>

          <div className="space-y-1.5">
            <label htmlFor="hdd-writer-confirm-token" className="text-[11px] text-slate-400 block">
              {t('hdd.writer.tokenLabelPrefix')}{' '}
              <code className="text-amber-400 font-bold">{expectedToken}</code>{' '}
              {t('hdd.writer.tokenLabelSuffix')}
            </label>
            <input
              id="hdd-writer-confirm-token"
              type="text"
              value={confirmInput}
              onChange={(e) => setConfirmInput(e.target.value)}
              placeholder={expectedToken}
              className="w-full px-3 py-2 rounded-xl bg-slate-900 border border-slate-700 text-xs text-amber-300 focus:outline-none focus:border-amber-500 font-mono"
            />
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 pt-2">
            <button
              type="button"
              onClick={() => void handleProceduralFormat()}
              disabled={executing || !isTokenValid}
              className="py-2.5 px-3 rounded-xl bg-emerald-500 hover:bg-emerald-400 text-slate-950 font-bold text-xs flex items-center justify-center gap-1.5 transition-all disabled:opacity-40"
            >
              <HardDrive className="w-4 h-4" />
              <span>{t('hdd.writer.formatBtn')}</span>
            </button>

            <button
              type="button"
              onClick={() => void handleWriteRawImage()}
              disabled={executing || !isTokenValid}
              className="py-2.5 px-3 rounded-xl bg-cyan-500 hover:bg-cyan-400 text-slate-950 font-bold text-xs flex items-center justify-center gap-1.5 transition-all disabled:opacity-40"
            >
              <Flame className="w-4 h-4" />
              <span>{t('hdd.writer.writeRawBtn')}</span>
            </button>

            <button
              type="button"
              onClick={() => void handleRestoreMirrors()}
              disabled={executing || !isTokenValid}
              className="py-2.5 px-3 rounded-xl bg-purple-500 hover:bg-purple-400 text-slate-950 font-bold text-xs flex items-center justify-center gap-1.5 transition-all disabled:opacity-40"
            >
              <ShieldCheck className="w-4 h-4" />
              <span>{t('hdd.writer.restoreMirrorBtn')}</span>
            </button>
          </div>
        </div>
      )}

      {writeReport && (
        <div className="p-4 rounded-xl bg-emerald-500/10 border border-emerald-500/40 text-xs space-y-1">
          <div className="flex items-center gap-2 text-emerald-300 font-bold">
            <CheckCircle2 className="w-4 h-4" />
            <span>{t('hdd.writer.reportSuccessTitle', { op: writeReport.operation })}</span>
          </div>
          <p className="text-slate-300 text-[11px]">
            {t('hdd.writer.reportSuccessMeta', {
              msg: writeReport.message,
              bytes: writeReport.bytes_written.toLocaleString(),
              sectors: writeReport.sectors_written.toLocaleString(),
              device: writeReport.device_path,
            })}
          </p>
        </div>
      )}
    </div>
  );
};
