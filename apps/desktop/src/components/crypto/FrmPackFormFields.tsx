/**
 * @file components/crypto/FrmPackFormFields.tsx
 * @brief Mode selector, output path, DIGADIAG options, and module table inputs for FrmPackCard.
 */

import React, { type Dispatch, type SetStateAction } from 'react';
import { Plus, Trash2 } from 'lucide-react';
import type { FrmPackModuleInput } from '@/models';
import { useTranslation } from '@/i18n/useTranslation';

export type FrmPackMode = 'sd' | 'dvd' | 'digadiag';

interface FrmPackFormFieldsProps {
  mode: FrmPackMode;
  onModeChange: (mode: FrmPackMode) => void;
  outPath: string;
  onOutPathChange: (value: string) => void;
  formError: string | null;
  diagInputPath: string;
  onDiagInputPathChange: (value: string) => void;
  dstAddr: string;
  onDstAddrChange: (value: string) => void;
  tag: string;
  onTagChange: (value: string) => void;
  buildName: string;
  onBuildNameChange: (value: string) => void;
  hwFamily: string;
  onHwFamilyChange: (value: string) => void;
  modelGroup: string;
  onModelGroupChange: (value: string) => void;
  versionU16: string;
  onVersionU16Change: (value: string) => void;
  modules: FrmPackModuleInput[];
  setModules: Dispatch<SetStateAction<FrmPackModuleInput[]>>;
}

export const FrmPackFormFields: React.FC<FrmPackFormFieldsProps> = ({
  mode,
  onModeChange,
  outPath,
  onOutPathChange,
  formError,
  diagInputPath,
  onDiagInputPathChange,
  dstAddr,
  onDstAddrChange,
  tag,
  onTagChange,
  buildName,
  onBuildNameChange,
  hwFamily,
  onHwFamilyChange,
  modelGroup,
  onModelGroupChange,
  versionU16,
  onVersionU16Change,
  modules,
  setModules,
}) => {
  const { t } = useTranslation();

  return (
    <>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
        <label className="space-y-1 block">
          <span className="text-slate-400 block">{t('crypto.packager.packMode')}</span>
          <select
            value={mode}
            onChange={e => onModeChange(e.target.value as FrmPackMode)}
            className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-2 text-slate-200"
          >
            <option value="sd">sd — PANA__SD.FRM (TOC SD 64B plaintext)</option>
            <option value="dvd">dvd — PANA_DVD.FRM (TOC Feistel 48B)</option>
            <option value="digadiag">digadiag — DIGADIAG.FRM (128 KiB FVU)</option>
          </select>
        </label>

        <label className="space-y-1 block">
          <span className="text-slate-400 block">{t('crypto.packager.outPath')}</span>
          <input
            type="text"
            value={outPath}
            aria-invalid={Boolean(formError && !outPath.trim())}
            aria-describedby={formError ? 'frm-pack-validation-err' : undefined}
            onChange={e => onOutPathChange(e.target.value)}
            className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-2 text-slate-200"
          />
        </label>
      </div>

      {mode === 'digadiag' ? (
        <div className="grid grid-cols-1 md:grid-cols-4 gap-3 pt-1">
          <input
            type="text"
            placeholder="Binário entrada (ex: diag.bin)"
            value={diagInputPath}
            onChange={e => onDiagInputPathChange(e.target.value)}
            className="md:col-span-2 bg-slate-950 border border-slate-800 rounded-lg px-3 py-2 text-slate-200"
          />
          <input
            type="text"
            placeholder="dst_addr (0x50400000)"
            value={dstAddr}
            onChange={e => onDstAddrChange(e.target.value)}
            className="bg-slate-950 border border-slate-800 rounded-lg px-3 py-2 text-slate-200"
          />
          <div className="flex gap-2">
            <input
              type="text"
              placeholder="Tag"
              value={tag}
              onChange={e => onTagChange(e.target.value)}
              className="w-20 bg-slate-950 border border-slate-800 rounded-lg px-2 py-2 text-slate-200"
            />
            <input
              type="text"
              placeholder="Build"
              value={buildName}
              onChange={e => onBuildNameChange(e.target.value)}
              className="flex-1 bg-slate-950 border border-slate-800 rounded-lg px-2 py-2 text-slate-200"
            />
          </div>
        </div>
      ) : (
        <div className="space-y-2 pt-1">
          <div className="grid grid-cols-3 gap-2">
            <input
              type="text"
              placeholder="HW (M6930)"
              value={hwFamily}
              onChange={e => onHwFamilyChange(e.target.value)}
              className="bg-slate-950 border border-slate-800 rounded-lg px-2.5 py-1.5 text-slate-200"
            />
            <input
              type="text"
              placeholder="Modelo (NP-309)"
              value={modelGroup}
              onChange={e => onModelGroupChange(e.target.value)}
              className="bg-slate-950 border border-slate-800 rounded-lg px-2.5 py-1.5 text-slate-200"
            />
            <input
              type="text"
              placeholder="Versão (0x0001)"
              value={versionU16}
              onChange={e => onVersionU16Change(e.target.value)}
              className="bg-slate-950 border border-slate-800 rounded-lg px-2.5 py-1.5 text-slate-200"
            />
          </div>

          {modules.length === 0 ? (
            <p className="text-[11px] text-slate-500 font-mono py-1">
              Nenhum módulo adicionado.
            </p>
          ) : (
            modules.map((mod, idx) => (
              <div key={idx} className="flex items-center gap-2">
                <input
                  type="text"
                  placeholder="Nome slot (ex: PROG061 )"
                  value={mod.name}
                  onChange={e =>
                    setModules(prev =>
                      prev.map((m, i) => (i === idx ? { ...m, name: e.target.value } : m))
                    )
                  }
                  className="w-40 bg-slate-950 border border-slate-800 rounded-lg px-2.5 py-1.5 text-slate-200"
                />
                <input
                  type="text"
                  placeholder="Caminho payload (ex: firmware_and_images/out/prog061.bin)"
                  value={mod.path}
                  onChange={e =>
                    setModules(prev =>
                      prev.map((m, i) => (i === idx ? { ...m, path: e.target.value } : m))
                    )
                  }
                  className="flex-1 bg-slate-950 border border-slate-800 rounded-lg px-2.5 py-1.5 text-slate-200"
                />
                <button
                  type="button"
                  onClick={() => setModules(prev => prev.filter((_, i) => i !== idx))}
                  className="p-1.5 text-slate-500 hover:text-rose-400"
                  aria-label="Remover módulo"
                >
                  <Trash2 className="w-3.5 h-3.5" />
                </button>
              </div>
            ))
          )}

          <button
            type="button"
            onClick={() => setModules(prev => [...prev, { name: '', path: '' }])}
            className="inline-flex items-center gap-1 text-[11px] text-cyan-400 hover:text-cyan-300"
          >
            <Plus className="w-3 h-3" />
            <span>{t('crypto.packager.addModule')}</span>
          </button>
        </div>
      )}

      {formError && (
        <p id="frm-pack-validation-err" role="alert" className="text-[11px] text-rose-400">
          {formError}
        </p>
      )}
    </>
  );
};
