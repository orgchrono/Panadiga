import React, { useState } from 'react';
import { CheckCircle2, Disc3, HardDriveDownload, Loader2 } from 'lucide-react';
import type {
  SdUnbrickKitReport,
  SdUnbrickKitRequestDto,
} from '@/models';
import {
  Alert,
  AlertDescription,
  AlertTitle,
  Badge,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  Input,
} from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import {
  type BgoMode,
  buildSdKitRequestPayload,
  type MainPassthroughMode,
  resolveSdOverallBadgeVariant,
  type SdDiskVerificationProps,
  type SdLoaderProfile,
} from './unbrickDomain';
import { SdKitProfileControls } from './SdKitProfileControls';
import { SdVerificationPanel } from './SdVerificationPanel';

interface SdKitBuilderCardProps extends SdDiskVerificationProps {
  discoveredPanaDvd: string | null;
  discoveredSdDir: string | null;
  kitLoading: boolean;
  kitResult: SdUnbrickKitReport | null;
  kitError: string | null;
  onBuildKit: (request: SdUnbrickKitRequestDto) => Promise<SdUnbrickKitReport | null>;
}

export const SdKitBuilderCard: React.FC<SdKitBuilderCardProps> = ({
  discoveredPanaDvd,
  discoveredSdDir,
  kitLoading,
  kitResult,
  kitError,
  verifyLoading,
  verifyResult,
  verifyError,
  onBuildKit,
  onVerifyMedia,
  diskLoading = false,
  diskResult = null,
  diskError = null,
  onInspectSdDisk,
  sdDrives,
  sdDriveFilter,
  onChangeSdDriveFilter,
  sdDrivesLoading,
  onRefreshSdDrives,
  autoSdDiskTarget,
  autoSdMountDir,
}) => {
  const { t } = useTranslation();
  const [outputDir, setOutputDir] = useState('firmware_and_images/sd_unbrick_kit');
  const [editedPanaDvd, setEditedPanaDvd] = useState<string | null>(null);
  const [editedVerifyDir, setEditedVerifyDir] = useState<string | null>(null);
  const panaDvdPath = editedPanaDvd ?? discoveredPanaDvd ?? 'firmware_and_images/PANA_DVD.FRM';
  const verifyDir =
    editedVerifyDir ?? discoveredSdDir ?? 'firmware_and_images/sd_unbrick_kit';
  const [loaderProfile, setLoaderProfile] = useState<SdLoaderProfile>('ver061');
  const [bgoMode, setBgoMode] = useState<BgoMode>('bypass');
  const [mainMode, setMainMode] = useState<MainPassthroughMode>('av_passthrough');
  const [includePanaSd, setIncludePanaSd] = useState(true);
  const [unlockHddDynamic, setUnlockHddDynamic] = useState(true);

  const [outDirError, setOutDirError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  const handleBuildSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const { errorKey, request } = buildSdKitRequestPayload({
      outputDir,
      panaDvdPath,
      bgoMode,
      mainMode,
      loaderProfile,
      includePanaSd,
      unlockHddDynamic,
    });
    if (errorKey || !request) {
      setOutDirError(errorKey ? t(errorKey) : null);
      return;
    }
    setOutDirError(null);
    setSuccess(null);
    const built = await onBuildKit(request);
    if (built) {
      setEditedVerifyDir(built.output_dir);
      setSuccess(`${t('unbrick.sdKit.successTitle')}: ${built.digadiag_path}`);
    }
  };

  return (
    <Card className="h-full flex flex-col">
      <CardHeader>
        <div className="flex items-start justify-between gap-2">
          <div>
            <CardTitle className="flex items-center gap-2">
              <HardDriveDownload className="w-4 h-4 text-emerald-400" />
              <span>{t('unbrick.sdKit.title')}</span>
            </CardTitle>
            <CardDescription className="mt-1">{t('unbrick.sdKit.desc')}</CardDescription>
          </div>
          <Badge variant={resolveSdOverallBadgeVariant(verifyResult?.overall_status)}>
            {verifyResult?.overall_status ?? 'READY'}
          </Badge>
        </div>
      </CardHeader>

      <CardContent className="space-y-4 flex-1">
        <form onSubmit={handleBuildSubmit} className="space-y-3 text-xs" noValidate>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <div className="space-y-1">
              <label htmlFor="sd-kit-outdir" className="text-slate-400 block font-mono">
                {t('unbrick.sdKit.outputDir')}
              </label>
              <Input
                id="sd-kit-outdir"
                value={outputDir}
                aria-invalid={Boolean(outDirError)}
                aria-describedby={outDirError ? 'sd-kit-outdir-err' : undefined}
                onChange={e => {
                  setOutputDir(e.target.value);
                  if (outDirError) setOutDirError(null);
                }}
                placeholder="firmware_and_images/sd_unbrick_kit"
              />
              {outDirError && (
                <p id="sd-kit-outdir-err" className="text-[11px] text-rose-400 font-mono">
                  {outDirError}
                </p>
              )}
            </div>

            <div className="space-y-1">
              <label htmlFor="sd-kit-pana-dvd" className="text-slate-400 block font-mono">
                {t('unbrick.sdKit.sourceFrm')}
              </label>
              <Input
                id="sd-kit-pana-dvd"
                value={panaDvdPath}
                onChange={e => setEditedPanaDvd(e.target.value)}
                placeholder="firmware_and_images/PANA_DVD.FRM"
              />
            </div>
          </div>

          <SdKitProfileControls
            bgoMode={bgoMode}
            onChangeBgoMode={setBgoMode}
            mainMode={mainMode}
            onChangeMainMode={setMainMode}
            loaderProfile={loaderProfile}
            onChangeLoaderProfile={setLoaderProfile}
            includePanaSd={includePanaSd}
            onChangeIncludePanaSd={setIncludePanaSd}
            unlockHddDynamic={unlockHddDynamic}
            onChangeUnlockHddDynamic={setUnlockHddDynamic}
          />

          <Button
            type="submit"
            data-testid="sd-kit-submit-btn"
            variant="emerald"
            size="sm"
            disabled={kitLoading}
            className="w-full sm:w-auto"
          >
            {kitLoading ? (
              <Loader2 className="w-3.5 h-3.5 animate-spin mr-1.5" />
            ) : (
              <Disc3 className="w-3.5 h-3.5 mr-1.5" />
            )}
            <span>{kitLoading ? t('unbrick.sdKit.building') : t('unbrick.sdKit.submit')}</span>
          </Button>
        </form>

        {kitError && (
          <Alert variant="destructive">
            <AlertTitle>{t('unbrick.sdKit.errorTitle')}</AlertTitle>
            <AlertDescription>{kitError}</AlertDescription>
          </Alert>
        )}

        {kitResult && (
          <Alert variant="success">
            <CheckCircle2 className="w-4 h-4 shrink-0 text-emerald-400" />
            <div className="space-y-1 min-w-0">
              <AlertTitle>{success ?? t('unbrick.sdKit.successTitle')}</AlertTitle>
              <AlertDescription className="font-mono text-[11px] break-all">
                DIGADIAG: {kitResult.digadiag_path} ({kitResult.bgo_mode_applied})
                {kitResult.pana_sd_path && ` | PANA__SD: ${kitResult.pana_sd_path}`}
              </AlertDescription>
            </div>
          </Alert>
        )}

        <SdVerificationPanel
          verifyDir={verifyDir}
          onChangeVerifyDir={setEditedVerifyDir}
          verifyLoading={verifyLoading}
          verifyResult={verifyResult}
          verifyError={verifyError}
          onVerifyMedia={onVerifyMedia}
          diskLoading={diskLoading}
          diskResult={diskResult}
          diskError={diskError}
          onInspectSdDisk={onInspectSdDisk}
          sdDrives={sdDrives}
          sdDriveFilter={sdDriveFilter}
          onChangeSdDriveFilter={onChangeSdDriveFilter}
          sdDrivesLoading={sdDrivesLoading}
          onRefreshSdDrives={onRefreshSdDrives}
          autoSdDiskTarget={autoSdDiskTarget}
          autoSdMountDir={autoSdMountDir}
        />
      </CardContent>
    </Card>
  );
};
