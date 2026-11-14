import React, { useEffect, useState } from 'react';
import { CheckCircle2, Disc3, HardDriveDownload, Loader2, ShieldCheck } from 'lucide-react';
import type {
  SdMediaVerificationReport,
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
  EmptyState,
  Input,
  Select,
} from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';

interface SdKitBuilderCardProps {
  discoveredPanaDvd: string | null;
  discoveredSdDir: string | null;
  kitLoading: boolean;
  kitResult: SdUnbrickKitReport | null;
  kitError: string | null;
  verifyLoading: boolean;
  verifyResult: SdMediaVerificationReport | null;
  verifyError: string | null;
  onBuildKit: (request: SdUnbrickKitRequestDto) => Promise<SdUnbrickKitReport | null>;
  onVerifyMedia: (sdRootDir: string) => Promise<SdMediaVerificationReport | null>;
}

const SEVERITY_BADGE: Record<string, 'success' | 'warning' | 'destructive' | 'secondary'> = {
  pass: 'success',
  warn: 'warning',
  fail: 'destructive',
};

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
}) => {
  const { t } = useTranslation();
  const [outputDir, setOutputDir] = useState('sd_card_eh55_kit');
  const [panaDvdPath, setPanaDvdPath] = useState(
    discoveredPanaDvd ?? 'firmware_and_images/PANA_DVD.FRM'
  );
  const [verifyDir, setVerifyDir] = useState(discoveredSdDir ?? 'sd_card_eh55');
  const [bgoMode, setBgoMode] = useState<'auto_format' | 'bypass' | 'uart_monitor' | 'factory'>(
    'auto_format'
  );
  const [mainMode, setMainMode] = useState<'av_passthrough' | 'factory_original'>('av_passthrough');
  const [includePanaSd, setIncludePanaSd] = useState(true);
  const [unlockHddDynamic, setUnlockHddDynamic] = useState(true);

  const [outDirError, setOutDirError] = useState<string | null>(null);
  const [verifyDirError, setVerifyDirError] = useState<string | null>(null);
  const [buildSuccessMessage, setBuildSuccessMessage] = useState<string | null>(null);

  useEffect(() => {
    if (discoveredPanaDvd) setPanaDvdPath(discoveredPanaDvd);
  }, [discoveredPanaDvd]);

  useEffect(() => {
    if (discoveredSdDir) setVerifyDir(discoveredSdDir);
  }, [discoveredSdDir]);

  const handleBuildSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const cleanOut = outputDir.trim();
    if (!cleanOut) {
      setOutDirError(t('unbrick.errors.outputDirRequired'));
      return;
    }
    setOutDirError(null);
    setBuildSuccessMessage(null);
    const built = await onBuildKit({
      output_dir: cleanOut,
      pana_dvd_path: panaDvdPath.trim() || undefined,
      bgo_mode: bgoMode,
      main_passthrough_mode: mainMode,
      include_pana_sd: includePanaSd,
      unlock_hdd_dynamic: unlockHddDynamic,
    });
    if (built) {
      setVerifyDir(built.output_dir);
      setBuildSuccessMessage(`${t('unbrick.sdKit.successTitle')}: ${built.digadiag_path}`);
    }
  };

  const handleVerifyClick = async () => {
    const cleanDir = verifyDir.trim();
    if (!cleanDir) {
      setVerifyDirError(t('unbrick.errors.verifyDirRequired'));
      return;
    }
    setVerifyDirError(null);
    await onVerifyMedia(cleanDir);
  };

  const checks = verifyResult?.checks ?? [];

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
          <Badge variant={verifyResult?.overall_status === 'FAIL' ? 'destructive' : 'success'}>
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
                placeholder="sd_card_eh55_kit"
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
                onChange={e => setPanaDvdPath(e.target.value)}
                placeholder="firmware_and_images/PANA_DVD.FRM"
              />
            </div>

            <div className="space-y-1">
              <label htmlFor="sd-kit-bgo-mode" className="text-slate-400 block font-mono">
                {t('unbrick.sdKit.bgoMode')}
              </label>
              <Select
                id="sd-kit-bgo-mode"
                value={bgoMode}
                onChange={e => setBgoMode(e.target.value as typeof bgoMode)}
              >
                <option value="auto_format">auto_format (PROG061 -&gt; BGO Auto-Format U99)</option>
                <option value="bypass">bypass (Skip U99 Boot Checks Only)</option>
                <option value="uart_monitor">uart_monitor (Hook _dev_SERIAL PANAMON1)</option>
                <option value="factory">factory (Stock PROG061 Unmodified)</option>
              </Select>
            </div>

            <div className="space-y-1">
              <label htmlFor="sd-kit-main-mode" className="text-slate-400 block font-mono">
                {t('unbrick.sdKit.mainMode')}
              </label>
              <Select
                id="sd-kit-main-mode"
                value={mainMode}
                onChange={e => setMainMode(e.target.value as typeof mainMode)}
              >
                <option value="av_passthrough">av_passthrough (TBC E-E No-ODD/No-FPC)</option>
                <option value="factory_original">factory_original (Stock MAIN3410PQ)</option>
              </Select>
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-4 pt-1 text-[11px] text-slate-300">
            <label className="inline-flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={includePanaSd}
                onChange={e => setIncludePanaSd(e.target.checked)}
                className="rounded border-slate-700 bg-slate-950 text-emerald-500 focus:ring-emerald-500"
              />
              <span>{t('unbrick.sdKit.includePanaSd')}</span>
            </label>
            <label className="inline-flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={unlockHddDynamic}
                onChange={e => setUnlockHddDynamic(e.target.checked)}
                className="rounded border-slate-700 bg-slate-950 text-cyan-500 focus:ring-cyan-500"
              />
              <span>{t('unbrick.sdKit.unlockHddDynamic')}</span>
            </label>
          </div>

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
              <AlertTitle>{buildSuccessMessage ?? t('unbrick.sdKit.successTitle')}</AlertTitle>
              <AlertDescription className="font-mono text-[11px] break-all">
                DIGADIAG: {kitResult.digadiag_path} ({kitResult.bgo_mode_applied})
                {kitResult.pana_sd_path && ` | PANA__SD: ${kitResult.pana_sd_path}`}
              </AlertDescription>
            </div>
          </Alert>
        )}

        <div className="border-t border-slate-800/80 pt-3 space-y-2.5">
          <div className="flex flex-col sm:flex-row gap-2">
            <div className="flex-1 space-y-1">
              <Input
                value={verifyDir}
                aria-label={t('unbrick.sdKit.verifyInputLabel')}
                aria-invalid={Boolean(verifyDirError)}
                aria-describedby={verifyDirError ? 'sd-verify-dir-err' : undefined}
                onChange={e => {
                  setVerifyDir(e.target.value);
                  if (verifyDirError) setVerifyDirError(null);
                }}
                placeholder="sd_card_eh55"
              />
              {verifyDirError && (
                <p id="sd-verify-dir-err" className="text-[11px] text-rose-400 font-mono">
                  {verifyDirError}
                </p>
              )}
            </div>
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={verifyLoading}
              onClick={handleVerifyClick}
            >
              {verifyLoading ? (
                <Loader2 className="w-3.5 h-3.5 animate-spin mr-1.5" />
              ) : (
                <ShieldCheck className="w-3.5 h-3.5 mr-1.5 text-cyan-400" />
              )}
              <span>{t('unbrick.sdKit.verifyBtn')}</span>
            </Button>
          </div>

          {verifyError && (
            <Alert variant="destructive">
              <AlertDescription>{verifyError}</AlertDescription>
            </Alert>
          )}

          {checks.length === 0 ? (
            <EmptyState compact title={t('unbrick.sdKit.checksEmpty')} />
          ) : (
            <div className="space-y-1.5 max-h-52 overflow-y-auto pr-1">
              {checks.map(item => (
                <div
                  key={item.id}
                  className="rounded-xl border border-slate-800 bg-slate-950/70 p-2.5 text-xs space-y-1"
                >
                  <div className="flex items-center justify-between gap-2">
                    <span className="font-mono font-semibold text-slate-200">{item.title}</span>
                    <Badge variant={SEVERITY_BADGE[item.severity] ?? 'secondary'}>
                      {item.severity.toUpperCase()}
                    </Badge>
                  </div>
                  <p className="text-[11px] text-slate-400 leading-relaxed">{item.detail}</p>
                </div>
              ))}
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
};
