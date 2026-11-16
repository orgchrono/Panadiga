import React, { useState } from 'react';
import { PackagePlus, RefreshCw } from 'lucide-react';
import type { FrmPackModuleInput, FrmPackReport, PackFrmRequest } from '@/models';
import { StatusBadge } from '@/components/StatusBadge';
import { Button } from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import { FrmPackFormFields, type FrmPackMode } from './FrmPackFormFields';
import { FrmPackResultBanner } from './FrmPackResultBanner';

interface FrmPackCardProps {
  packLoading: boolean;
  packError: string | null;
  packResult: FrmPackReport | null;
  onPack: (request: PackFrmRequest) => Promise<FrmPackReport | null>;
  onInspectOutput: (outPath: string) => void;
}

const DEFAULT_OUT_BY_MODE: Record<FrmPackMode, string> = {
  sd: 'firmware_and_images/out/PANA__SD.FRM',
  dvd: 'firmware_and_images/out/PANA_DVD.FRM',
  digadiag: 'firmware_and_images/out/DIGADIAG.FRM',
};

export const FrmPackCard: React.FC<FrmPackCardProps> = ({
  packLoading,
  packError,
  packResult,
  onPack,
  onInspectOutput,
}) => {
  const { t } = useTranslation();
  const [mode, setMode] = useState<FrmPackMode>('sd');
  const [outPath, setOutPath] = useState(DEFAULT_OUT_BY_MODE.sd);
  const [modules, setModules] = useState<FrmPackModuleInput[]>([
    { name: 'DRV S211', path: '' },
    { name: 'PROG061 ', path: '' },
    { name: 'MAIN3060NP', path: '' },
  ]);
  const [hwFamily, setHwFamily] = useState('M6930');
  const [modelGroup, setModelGroup] = useState('NP-309');
  const [versionU16, setVersionU16] = useState('0x0001');

  const [diagInputPath, setDiagInputPath] = useState('');
  const [dstAddr, setDstAddr] = useState('0x50400000');
  const [tag, setTag] = useState('FVU ');
  const [buildName, setBuildName] = useState('DIAG01');
  const [formError, setFormError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  const handleModeChange = (nextMode: FrmPackMode) => {
    setMode(nextMode);
    setFormError(null);
    if (Object.values(DEFAULT_OUT_BY_MODE).includes(outPath)) {
      setOutPath(DEFAULT_OUT_BY_MODE[nextMode]);
    }
  };

  const validModules = modules.filter(m => m.name.trim() && m.path.trim());

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (packLoading) return;
    if (!outPath.trim()) {
      setFormError('Informe o caminho de saída para gravar o contêiner .FRM.');
      setSuccess(null);
      return;
    }
    if (mode === 'digadiag' && !diagInputPath.trim()) {
      setFormError('Informe o binário de entrada para encapsular no DIGADIAG.FRM.');
      setSuccess(null);
      return;
    }
    if (mode !== 'digadiag' && validModules.length === 0) {
      setFormError('Preencha nome e caminho de ao menos um módulo de firmware.');
      setSuccess(null);
      return;
    }

    setFormError(null);
    setSuccess(null);
    const report = await onPack(
      mode === 'digadiag'
        ? {
            mode,
            out_path: outPath.trim(),
            input_path: diagInputPath.trim(),
            dst_addr: dstAddr.trim() || '0x50400000',
            tag: tag || 'FVU ',
            build_name: buildName.trim() || 'DIAG01',
          }
        : {
            mode,
            out_path: outPath.trim(),
            modules: validModules,
            hw_family: hwFamily.trim() || 'M6930',
            model_group: modelGroup.trim() || 'NP-309',
            version_u16: versionU16.trim() || '0x0001',
          }
    );
    if (report) {
      setSuccess(
        `Contêiner ${mode.toUpperCase()} gerado (${report.total_bytes.toLocaleString()} bytes) em ${outPath.trim()}.`
      );
    }
  };

  return (
    <div className="rounded-xl bg-bench-900 border border-slate-800 p-5 shadow-xl space-y-4">
      <div className="flex items-start justify-between gap-3">
        <div>
          <h3 className="text-sm font-bold text-slate-100 flex items-center gap-2">
            <PackagePlus className="w-4 h-4 text-emerald-400" />
            <span>{t('crypto.packager.packTitle')}</span>
          </h3>
          <p className="text-xs text-slate-400 mt-0.5">{t('crypto.packager.packDesc')}</p>
        </div>
        <StatusBadge variant="info" label={mode.toUpperCase()} />
      </div>

      <form onSubmit={handleSubmit} className="space-y-3 text-xs" noValidate>
        <FrmPackFormFields
          mode={mode}
          onModeChange={handleModeChange}
          outPath={outPath}
          onOutPathChange={val => {
            setOutPath(val);
            if (formError) setFormError(null);
          }}
          formError={formError}
          diagInputPath={diagInputPath}
          onDiagInputPathChange={setDiagInputPath}
          dstAddr={dstAddr}
          onDstAddrChange={setDstAddr}
          tag={tag}
          onTagChange={setTag}
          buildName={buildName}
          onBuildNameChange={setBuildName}
          hwFamily={hwFamily}
          onHwFamilyChange={setHwFamily}
          modelGroup={modelGroup}
          onModelGroupChange={setModelGroup}
          versionU16={versionU16}
          onVersionU16Change={setVersionU16}
          modules={modules}
          setModules={setModules}
        />

        <div className="flex items-center gap-3 pt-1">
          <Button type="submit" size="sm" disabled={packLoading} className="gap-1.5 font-bold">
            {packLoading && <RefreshCw className="w-3.5 h-3.5 animate-spin" />}
            <span>{t('crypto.packager.packSubmit')}</span>
          </Button>
          {packResult && (
            <Button
              type="button"
              size="sm"
              variant="outline"
              onClick={() => onInspectOutput(outPath)}
              className="text-xs"
            >
              {t('crypto.packager.inspectPacked')}
            </Button>
          )}
        </div>
      </form>

      <FrmPackResultBanner
        packLoading={packLoading}
        packError={packError}
        packResult={packResult}
        success={success}
      />
    </div>
  );
};
