/**
 * @file components/pico/PicoBenchBanners.tsx
 * @brief SSOT loading/error states, operation success banners, and section filter toolbar for PicoBenchView.
 */

import React from 'react';
import { StatusBadge } from '@/components/StatusBadge';
import { Button } from '@/components/ui';
import { AlertCircle, CheckCircle2, RefreshCw, ShieldCheck } from 'lucide-react';
import type {
  FirstContactBackupResult,
  InCircuitNvramEraseResult,
  PicoBenchSectionFilter,
} from '@/models';
import { useTranslation } from '@/i18n/useTranslation';

export type { PicoBenchSectionFilter };

const SECTION_FILTER_IDS: ReadonlyArray<{
  id: PicoBenchSectionFilter;
  labelKey:
    | 'pico.sections.all'
    | 'pico.sections.diag'
    | 'pico.sections.bridge'
    | 'pico.sections.gdb'
    | 'pico.sections.terminal';
}> = [
  { id: 'all', labelKey: 'pico.sections.all' },
  { id: 'diag', labelKey: 'pico.sections.diag' },
  { id: 'bridge', labelKey: 'pico.sections.bridge' },
  { id: 'gdb', labelKey: 'pico.sections.gdb' },
  { id: 'terminal', labelKey: 'pico.sections.terminal' },
];

export const PicoBenchSsotLoading: React.FC = () => (
  <div className="flex flex-col items-center justify-center h-96 gap-3 text-slate-400 font-mono text-xs">
    <RefreshCw className="w-6 h-6 animate-spin text-cyan-400" />
    <span>Carregando especificação SSOT de hardware do backend...</span>
  </div>
);

interface PicoBenchSsotErrorProps {
  ssotError: string | null;
  onReload: () => void;
}

export const PicoBenchSsotError: React.FC<PicoBenchSsotErrorProps> = ({ ssotError, onReload }) => (
  <div className="max-w-2xl mx-auto my-16 p-6 rounded-2xl bg-bench-900 border border-rose-500/30 text-center space-y-4 font-mono">
    <div className="w-12 h-12 rounded-full bg-rose-500/10 text-rose-400 border border-rose-500/30 flex items-center justify-center mx-auto">
      <AlertCircle className="w-6 h-6" />
    </div>
    <div>
      <h3 className="text-sm font-bold text-slate-100">
        Falha ao Carregar Especificação Hardware SSOT
      </h3>
      <p className="text-xs text-slate-400 mt-1">
        {ssotError || 'Não foi possível obter a tabela de pinagem P7501 e comandos PICO via IPC nativo.'}
      </p>
    </div>
    <Button type="button" onClick={onReload} className="gap-2 mx-auto font-bold text-xs">
      <RefreshCw className="w-4 h-4" />
      <span>Recarregar SSOT</span>
    </Button>
  </div>
);

interface PicoBenchActionBannersProps {
  backupResult: FirstContactBackupResult | null;
  eraseResult: InCircuitNvramEraseResult | null;
}

export const PicoBenchActionBanners: React.FC<PicoBenchActionBannersProps> = ({
  backupResult,
  eraseResult,
}) => (
  <>
    {backupResult?.success && (
      <div className="rounded-xl bg-cyan-500/10 border border-cyan-500/40 p-4 flex items-center justify-between shadow-xl gap-3">
        <div className="flex items-center gap-3">
          <ShieldCheck className="w-5 h-5 text-cyan-400 shrink-0" />
          <div>
            <h4 className="text-xs font-bold text-cyan-300">
              BACKUP DE 1º CONTATO &amp; DOUBLE-READ CONCLUÍDOS COM SUCESSO!
            </h4>
            <p className="text-[11px] text-slate-300 mt-0.5 font-sans">
              Modelo: {backupResult.detected_model} | NVRAM: {backupResult.nvram_bytes.toLocaleString()} bytes | SRAM:{' '}
              {backupResult.sram_bytes.toLocaleString()} bytes. Integridade verificada via SHA-256.
            </p>
          </div>
        </div>
        <StatusBadge variant="info" label="BACKUP SEGURO" />
      </div>
    )}

    {eraseResult?.success && (
      <div className="rounded-xl bg-emerald-500/10 border border-emerald-500/40 p-4 flex items-center justify-between shadow-xl gap-3">
        <div className="flex items-center gap-3">
          <CheckCircle2 className="w-5 h-5 text-emerald-400 shrink-0" />
          <div>
            <h4 className="text-xs font-bold text-emerald-300">
              INJEÇÃO DE RAM LOADER &amp; APAGAMENTO IN-CIRCUIT CONCLUÍDOS!
            </h4>
            <p className="text-[11px] text-slate-300 mt-0.5 font-sans">
              Executado em {eraseResult.elapsed_ms}ms via P7501 UART (SW_UART / XRESET). NVRAM limpa para 0xFF (65.536 bytes). Alvo pronto para boot.
            </p>
          </div>
        </div>
        <StatusBadge variant="success" label="HARDWARE RESTAURADO" />
      </div>
    )}
  </>
);

interface PicoBenchSectionToolbarProps {
  sectionFilter: PicoBenchSectionFilter;
  onSelectFilter: (filter: PicoBenchSectionFilter) => void;
}

export const PicoBenchSectionToolbar: React.FC<PicoBenchSectionToolbarProps> = ({
  sectionFilter,
  onSelectFilter,
}) => {
  const { t } = useTranslation();
  return (
    <div
      role="toolbar"
      aria-label={t('pico.sections.toolbarAria')}
      className="flex items-center gap-2 flex-wrap rounded-xl bg-bench-900/70 border border-slate-800 px-3 py-2"
    >
      <span className="text-[11px] uppercase tracking-wider text-slate-400 mr-1">
        {t('pico.sections.label')}
      </span>
      {SECTION_FILTER_IDS.length === 0 ? (
        <span className="text-xs text-slate-500">{t('pico.sections.empty')}</span>
      ) : (
        SECTION_FILTER_IDS.map(({ id, labelKey }) => (
          <Button
            key={id}
            type="button"
            size="sm"
            variant={sectionFilter === id ? 'default' : 'outline'}
            aria-pressed={sectionFilter === id}
            onClick={() => onSelectFilter(id)}
            className="h-7 px-2.5 text-xs font-mono"
          >
            {t(labelKey)}
          </Button>
        ))
      )}
    </div>
  );
};
