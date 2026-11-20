import React from 'react';
import { StatusBadge } from '@/components/StatusBadge';
import { Sha256ArtifactCard } from '@/components/pipelines';
import { useTranslation } from '@/i18n/useTranslation';
import type { ApplyRegionCalibrationResponse, TvCalibrationBundleReport } from '@/models';

export const CalibrationAuditCards: React.FC<{
  readonly report: TvCalibrationBundleReport | null;
}> = ({ report }) => {
  const { t } = useTranslation();
  if (!report) return null;
  const tail = report.ifodev2_tail;
  const mainReg = report.main_region;
  const frmToc = report.frm_toc;

  return (
    <div className="grid grid-cols-1 md:grid-cols-3 gap-3 text-xs font-mono">
      {tail && (
        <div className="p-3 rounded-lg bg-slate-950 border border-slate-800 space-y-1">
          <div className="font-bold text-cyan-300">{t('pipelines.calibration.results.ifodev2TailTitle')}</div>
          <div className="text-slate-300">{tail.offset_range_hex}</div>
          <div className="text-slate-400">
            {t('pipelines.calibration.results.nonFfBytes', { count: tail.non_ff_bytes })}
          </div>
        </div>
      )}
      {mainReg && (
        <div className="p-3 rounded-lg bg-slate-950 border border-slate-800 space-y-1">
          <div className="font-bold text-emerald-300">{t('pipelines.calibration.results.mainRegionTitle')}</div>
          <div className="text-slate-300">{mainReg.va_hex} ({mainReg.file_offset_hex})</div>
          <StatusBadge variant="info" label={mainReg.state} />
        </div>
      )}
      {frmToc && (
        <div className="p-3 rounded-lg bg-slate-950 border border-slate-800 space-y-1">
          <div className="font-bold text-amber-300">{t('pipelines.calibration.results.frmTocTitle')}</div>
          <div className="text-slate-300">
            {t('pipelines.calibration.results.slotsCount', { count: frmToc.entry_count })}
          </div>
          <StatusBadge
            variant={frmToc.all_list_sums_valid ? 'success' : 'error'}
            label={
              frmToc.all_list_sums_valid
                ? t('pipelines.calibration.results.listSumsOk')
                : t('pipelines.calibration.results.listSumsFail')
            }
          />
        </div>
      )}
    </div>
  );
};

interface OutputArtifactItem {
  readonly key: string;
  readonly path: string;
  readonly sha256: string;
  readonly source?: string;
  readonly bytes?: number;
}

function collectCalibrationArtifacts(
  result: ApplyRegionCalibrationResponse | null,
  devBinPath: string,
  mainPath: string,
  frmPath: string,
): readonly OutputArtifactItem[] {
  if (!result) return [];
  const list: OutputArtifactItem[] = [];
  const entries: readonly (OutputArtifactItem | null)[] = [
    result.devBinOutputPath && result.devBinReport
      ? {
          key: 'devbin',
          path: result.devBinOutputPath,
          sha256: result.devBinReport.sha256,
          source: devBinPath || undefined,
          bytes: result.devBinReport.size_bytes,
        }
      : null,
    result.mainOutputPath && result.mainReport
      ? {
          key: 'main',
          path: result.mainOutputPath,
          sha256: result.mainReport.sha256,
          source: mainPath || undefined,
          bytes: result.mainReport.size_bytes,
        }
      : null,
    result.frmOutputPath && result.frmReport
      ? {
          key: 'frm',
          path: result.frmOutputPath,
          sha256: result.frmReport.sha256,
          source: frmPath || undefined,
          bytes: result.frmReport.total_bytes,
        }
      : null,
  ];
  return entries.reduce<readonly OutputArtifactItem[]>(
    (acc, item) => (item ? [...acc, item] : acc),
    list,
  );
}

export const CalibrationOutputArtifacts: React.FC<{
  readonly applyResult: ApplyRegionCalibrationResponse | null;
  readonly devBinPath: string;
  readonly mainPath: string;
  readonly frmPath: string;
  readonly copiedHash: boolean;
  readonly copyDigest: (digest: string) => void;
}> = ({ applyResult, devBinPath, mainPath, frmPath, copiedHash, copyDigest }) => {
  const artifacts = collectCalibrationArtifacts(applyResult, devBinPath, mainPath, frmPath);
  if (artifacts.length === 0) return null;
  return (
    <div className="space-y-3">
      {artifacts.map(art => (
        <Sha256ArtifactCard
          key={art.key}
          sourcePath={art.source}
          outputPath={art.path}
          outputSha256={art.sha256}
          bytesProcessed={art.bytes}
          verified
          copied={copiedHash}
          onCopyHash={copyDigest}
        />
      ))}
    </div>
  );
};
