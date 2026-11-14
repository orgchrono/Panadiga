import React, { useEffect, useState } from 'react';
import { ArchiveRestore, Cpu, Layers, Loader2 } from 'lucide-react';
import type {
  FrmModuleExtractionReport,
  UnbrickWorkspaceReport,
} from '@/models';
import {
  Alert,
  AlertDescription,
  Badge,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  EmptyState,
  Input,
} from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';

interface UnbrickArchitectureCardProps {
  workbench: UnbrickWorkspaceReport | null;
  extractLoading: boolean;
  extractResult: FrmModuleExtractionReport | null;
  onExtractFrm: (frmPath: string, outDir: string) => Promise<FrmModuleExtractionReport | null>;
}

export const UnbrickArchitectureCard: React.FC<UnbrickArchitectureCardProps> = ({
  workbench,
  extractLoading,
  extractResult,
  onExtractFrm,
}) => {
  const { t } = useTranslation();
  const [frmPath, setFrmPath] = useState(
    workbench?.discovered_pana_dvd ?? 'firmware_and_images/PANA_DVD.FRM'
  );
  const [outDir, setOutDir] = useState('unbrick_extracted_frm');
  const [frmErr, setFrmErr] = useState<string | null>(null);
  const [extractSuccessMessage, setExtractSuccessMessage] = useState<string | null>(null);

  useEffect(() => {
    if (workbench?.discovered_pana_dvd) setFrmPath(workbench.discovered_pana_dvd);
  }, [workbench?.discovered_pana_dvd]);

  const handleExtractSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!frmPath.trim() || !outDir.trim()) {
      setFrmErr(t('unbrick.errors.extractPathsRequired'));
      return;
    }
    setFrmErr(null);
    setExtractSuccessMessage(null);
    const extracted = await onExtractFrm(frmPath.trim(), outDir.trim());
    if (extracted) {
      setExtractSuccessMessage(
        `Extraídos ${extracted.modules.length} módulos para ${extracted.output_dir}`
      );
    }
  };

  const facts = workbench?.hardware_facts ?? [];
  const extractedModules = extractResult?.modules ?? [];

  return (
    <Card>
      <CardHeader>
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div>
            <CardTitle className="flex items-center gap-2">
              <Layers className="w-4 h-4 text-amber-400" />
              <span>{t('unbrick.arch.title')}</span>
            </CardTitle>
            <CardDescription className="mt-1">{t('unbrick.arch.desc')}</CardDescription>
          </div>
          <div className="flex flex-wrap gap-1.5">
            <Badge variant={workbench?.prog061_bgo ? 'default' : 'outline'}>
              PROG061: {workbench?.prog061_bgo?.detected_mode ?? 'NONE'}
            </Badge>
            <Badge variant={workbench?.main_capacity?.is_dynamic_unclamped ? 'success' : 'warning'}>
              HDD Limit:{' '}
              {workbench?.main_capacity
                ? workbench.main_capacity.is_dynamic_unclamped
                  ? 'DYNAMIC'
                  : `${workbench.main_capacity.effective_default_limit_gb} GB`
                : 'N/A'}
            </Badge>
          </div>
        </div>
      </CardHeader>

      <CardContent className="space-y-4">
        {facts.length === 0 ? (
          <EmptyState compact title={t('unbrick.arch.factsEmpty')} />
        ) : (
          <div className="grid grid-cols-1 lg:grid-cols-3 gap-3">
            {facts.map(fact => (
              <div
                key={fact.topic}
                className="rounded-xl border border-slate-800 bg-slate-950/70 p-3 space-y-1.5"
              >
                <div className="flex items-start justify-between gap-2">
                  <span className="text-xs font-bold text-slate-100 font-mono leading-snug">
                    {fact.topic}
                  </span>
                  <Cpu className="w-3.5 h-3.5 text-cyan-400 shrink-0 mt-0.5" />
                </div>
                <Badge variant="outline">{fact.verdict}</Badge>
                <p className="text-[11px] text-slate-400 leading-relaxed">{fact.evidence}</p>
              </div>
            ))}
          </div>
        )}

        <form
          onSubmit={handleExtractSubmit}
          className="border-t border-slate-800/80 pt-3 flex flex-col lg:flex-row lg:items-end gap-2.5 text-xs"
          noValidate
        >
          <div className="flex-1 space-y-1">
            <label htmlFor="arch-extract-frm" className="text-slate-400 block font-mono">
              {t('unbrick.arch.frmInput')}
            </label>
            <Input
              id="arch-extract-frm"
              value={frmPath}
              aria-invalid={Boolean(frmErr)}
              onChange={e => {
                setFrmPath(e.target.value);
                if (frmErr) setFrmErr(null);
              }}
            />
          </div>
          <div className="flex-1 space-y-1">
            <label htmlFor="arch-extract-out" className="text-slate-400 block font-mono">
              {t('unbrick.arch.extractDir')}
            </label>
            <Input
              id="arch-extract-out"
              value={outDir}
              aria-invalid={Boolean(frmErr)}
              onChange={e => {
                setOutDir(e.target.value);
                if (frmErr) setFrmErr(null);
              }}
            />
          </div>
          <Button type="submit" size="sm" variant="outline" disabled={extractLoading}>
            {extractLoading ? (
              <Loader2 className="w-3.5 h-3.5 animate-spin mr-1.5" />
            ) : (
              <ArchiveRestore className="w-3.5 h-3.5 mr-1.5 text-amber-400" />
            )}
            <span>{t('unbrick.arch.extractBtn')}</span>
          </Button>
        </form>

        {frmErr && <p className="text-[11px] text-rose-400 font-mono">{frmErr}</p>}

        {extractSuccessMessage && (
          <Alert variant="success">
            <AlertDescription className="font-mono text-[11px]">
              {extractSuccessMessage}
            </AlertDescription>
          </Alert>
        )}

        {extractedModules.length > 0 && (
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-2 text-[11px] font-mono">
            {extractedModules.map(mod => (
              <div
                key={mod.name}
                className="rounded-lg border border-slate-800 bg-slate-950/80 p-2 space-y-0.5"
              >
                <div className="text-amber-300 font-bold">{mod.name}</div>
                <div className="text-slate-400">
                  enc={mod.encrypted_size.toLocaleString()}B
                  {mod.unpacked_size ? ` -> raw=${mod.unpacked_size.toLocaleString()}B` : ''}
                </div>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
};
