import React from 'react';
import { Search } from 'lucide-react';
import { NvramSectorMap } from '@/components/nvram/NvramSectorMap';
import { StatusBadge } from '@/components/StatusBadge';
import { Alert, AlertDescription, Badge, Button, EmptyState, Input, Select } from '@/components/ui';
import { HEX32_PAD_WIDTH } from '@/constants';
import { useTranslation } from '@/i18n/useTranslation';
import type { BgoAuditSourceMode, Eh55BgoAuditReport, FlashSectorInfo } from '@/models';
import { isBgoAuditSourceMode } from '@/utils/typeGuards';

export interface BgoSectorMapStateProps {
  readonly sectors: readonly FlashSectorInfo[];
  readonly sectorFilter: 'all' | 'bgo' | 'bootloader' | 'nvram';
  readonly setSectorFilter: (f: 'all' | 'bgo' | 'bootloader' | 'nvram') => void;
  readonly selectedSector: FlashSectorInfo | null;
  readonly onSelectSector: (sec: FlashSectorInfo) => void;
  readonly f2Integrity: { readonly count: number; readonly sum: number; readonly valid: boolean };
}

export interface BgoL2pAuditSectionProps extends BgoSectorMapStateProps {
  readonly auditSourceMode: BgoAuditSourceMode;
  readonly setAuditSourceMode: (m: BgoAuditSourceMode) => void;
  readonly norPath: string;
  readonly setNorPath: (v: string) => void;
  readonly portName: string;
  readonly auditBusy: boolean;
  readonly auditError: string | null;
  readonly auditReport: Eh55BgoAuditReport | null;
  readonly runBgoAudit: () => Promise<Eh55BgoAuditReport | null>;
}

function filterSectors(
  sectors: readonly FlashSectorInfo[],
  filter: BgoL2pAuditSectionProps['sectorFilter'],
): readonly FlashSectorInfo[] {
  if (filter === 'nvram') return sectors.filter(s => s.sector_type === 'nvram_microsector');
  if (filter === 'bgo') return sectors.filter(s => s.sector_type === 'bgo_region');
  if (filter === 'bootloader') return sectors.filter(s => s.sector_type === 'bootloader');
  return sectors;
}

export const BgoL2pAuditSection: React.FC<BgoL2pAuditSectionProps> = (props) => {
  const { t } = useTranslation();
  const visibleSectors = filterSectors(props.sectors, props.sectorFilter);
  const needsDumpPath = props.auditSourceMode === 'DUMP_FILE' && !props.norPath.trim();
  const needsLivePort = props.auditSourceMode === 'LIVE_PICO' && !props.portName.trim();
  const auditInvalid = needsDumpPath || needsLivePort;

  return (
    <div className="rounded-xl bg-slate-950 border border-slate-800 p-4 space-y-4 font-mono">
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-slate-800 pb-2">
        <h4 className="text-xs font-bold text-slate-100">{t('pipelines.bgoBench.auditTitle')}</h4>
        {props.auditReport && (
          <StatusBadge
            variant={props.auditReport.healthy ? 'success' : 'error'}
            label={props.auditReport.healthy ? t('pipelines.bgoBench.metrics.statusHealthy') : t('pipelines.bgoBench.metrics.statusCorrupt')}
          />
        )}
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-3 text-xs items-end">
        <div className="space-y-1">
          <label htmlFor="bgo-audit-source" className="text-slate-400 block">{t('pipelines.bgoBench.auditSourceLabel')}</label>
          <Select id="bgo-audit-source" data-testid="bgo-audit-source" value={props.auditSourceMode} onChange={e => { if (isBgoAuditSourceMode(e.target.value)) props.setAuditSourceMode(e.target.value); }}>
            <option value="DUMP_FILE">{t('pipelines.bgoBench.modeDump')}</option>
            <option value="LIVE_PICO">{t('pipelines.bgoBench.modeLive')}</option>
          </Select>
        </div>
        <div className="space-y-1">
          <label htmlFor="bgo-nor-path" className="text-slate-400 block">{t('pipelines.bgoBench.norPathLabel')}</label>
          <Input id="bgo-nor-path" data-testid="bgo-nor-path" type="text" value={props.norPath} aria-invalid={needsDumpPath} onChange={e => props.setNorPath(e.target.value)} />
          {needsDumpPath && <span className="text-[11px] text-rose-400">{t('pipelines.bgoBench.norPathRequired')}</span>}
        </div>
        <Button type="button" size="sm" data-testid="bgo-audit-btn" disabled={props.auditBusy || auditInvalid} onClick={() => void props.runBgoAudit()} className="gap-1.5">
          <Search className="w-3.5 h-3.5" />
          <span>{t('pipelines.bgoBench.auditBtn')}</span>
        </Button>
      </div>

      {props.auditError && (
        <Alert variant="destructive"><AlertDescription>{props.auditError}</AlertDescription></Alert>
      )}

      {props.auditReport && (
        <div className="flex flex-wrap gap-2 text-[11px]">
          <Badge variant={props.f2Integrity.valid ? 'success' : 'destructive'}>
            {t('pipelines.bgoBench.metrics.f2CountLabel', { count: props.f2Integrity.count, sum: props.f2Integrity.sum })} • {props.f2Integrity.valid ? t('pipelines.bgoBench.metrics.f2SumOk') : t('pipelines.bgoBench.metrics.f2SumMismatch')}
          </Badge>
          <Badge variant="secondary">{t('pipelines.bgoBench.metrics.l2pPagesLabel', { pages: props.auditReport.active_l2p_pages })}</Badge>
          <Badge variant={props.auditReport.l2p_tag_mismatches > 0 ? 'destructive' : 'success'}>
            {t('pipelines.bgoBench.metrics.l2pMismatchLabel', { count: props.auditReport.l2p_tag_mismatches })}
          </Badge>
          <Badge variant={props.auditReport.fat_mirror_mismatches > 0 ? 'destructive' : 'success'}>
            {t('pipelines.bgoBench.metrics.fatMirrorLabel', { count: props.auditReport.fat_mirror_mismatches })}
          </Badge>
          <Badge variant="secondary">{t('pipelines.bgoBench.metrics.nfromLabel', { count: props.auditReport.nfrom_file_count })}</Badge>
          {props.auditReport.ifodev2_model && (
            <Badge variant="outline">{t('pipelines.bgoBench.metrics.ifodev2ModelLabel', { model: props.auditReport.ifodev2_model })}</Badge>
          )}
        </div>
      )}

      <NvramSectorMap
        sectorMap={visibleSectors}
        selectedSector={props.selectedSector}
        filterType={props.sectorFilter}
        cleanSuccess={Boolean(props.auditReport?.healthy)}
        onSelectSector={props.onSelectSector}
        onSelectFilter={f => props.setSectorFilter(f === 'bgo' || f === 'bootloader' || f === 'nvram' ? f : 'all')}
      />

      <div className="space-y-2 text-xs">
        <div className="flex items-center justify-between gap-2">
          <span className="font-bold text-slate-200">{t('pipelines.bgoBench.sectorTableTitle')}</span>
          <span className="text-[11px] text-slate-400 font-sans">{t('pipelines.bgoBench.l2pNote')}</span>
        </div>
        {!props.auditReport || props.auditReport.sectors.length === 0 ? (
          <EmptyState compact title={t('pipelines.bgoBench.sectorTableEmpty')} />
        ) : (
          <div className="max-h-52 overflow-y-auto border border-slate-800 rounded-lg">
            <table className="w-full text-left text-[11px]">
              <tbody className="divide-y divide-slate-800/70">
                {props.auditReport.sectors.map(sec => (
                  <tr key={sec.sa} className="hover:bg-slate-900/60">
                    <td className="px-2 py-1 font-bold text-cyan-300">SA{sec.sa}</td>
                    <td className="px-2 py-1 text-slate-400">0x{sec.phys_va.toString(16).toUpperCase().padStart(HEX32_PAD_WIDTH, '0')}</td>
                    <td className="px-2 py-1 text-slate-200">{sec.kind}</td>
                    <td className="px-2 py-1 text-slate-300">id={sec.sector_id}</td>
                    <td className="px-2 py-1"><StatusBadge variant={sec.zero_marker_ok ? 'success' : 'error'} label={sec.zero_marker_ok ? '0x00' : 'ERR'} /></td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
};
