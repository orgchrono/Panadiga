import React from 'react';
import {
  Activity,
  AlertTriangle,
  CheckCircle2,
  Cpu,
  History,
  Loader2,
  RotateCcw,
  ShieldAlert,
  Terminal,
} from 'lucide-react';
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
} from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import {
  useSiliconExplorerViewModel,
  type SiliconPatchWidth,
} from '@/viewmodels/useSiliconExplorerViewModel';

const HEX_RADIX = 16;
const HEX_U32_PAD = 8;
const HEX_U16_PAD = 4;
const HEX_BYTE_PAD = 2;
const BYTE_MASK = 0xff;
const AIOF_FS0_VA = 0x8800002c;

function formatHexU32(val: number): string {
  return `0x${(val >>> 0).toString(HEX_RADIX).toUpperCase().padStart(HEX_U32_PAD, '0')}`;
}

function formatHexU16(val: number): string {
  return val.toString(HEX_RADIX).toUpperCase().padStart(HEX_U16_PAD, '0');
}

function formatHexByte(val: number): string {
  return (val & BYTE_MASK).toString(HEX_RADIX).toUpperCase().padStart(HEX_BYTE_PAD, '0');
}

function formatHexBytes(bytes: readonly number[]): string {
  return bytes.map((b) => formatHexByte(b)).join(' ');
}

export const SiliconExplorerView: React.FC = () => {
  const { t } = useTranslation();
  const vm = useSiliconExplorerViewModel();

  return (
    <div className="space-y-6 max-w-6xl mx-auto pb-12 font-mono">
      <div className="rounded-2xl bg-bench-900 border border-slate-800 p-5 shadow-xl flex flex-col lg:flex-row lg:items-center justify-between gap-4">
        <div className="flex items-center gap-3.5">
          <div className="p-3 rounded-xl bg-amber-500/10 text-amber-400 border border-amber-500/30">
            <Cpu className="w-6 h-6" />
          </div>
          <div>
            <h2 className="text-base font-bold text-slate-100">{t('silicon.title')}</h2>
            <p className="text-xs text-slate-400 font-sans mt-0.5">{t('silicon.subtitle')}</p>
          </div>
        </div>
        <Badge variant="warning">{t('silicon.badgeHeader')}</Badge>
      </div>

      {vm.isLoading && (
        <div className="flex items-center gap-2 text-xs text-cyan-400">
          <Loader2 className="w-3.5 h-3.5 animate-spin" />
          <span>{t('silicon.loadingPortsLabel')}</span>
        </div>
      )}

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <History className="w-4 h-4 text-cyan-400" />
            <span>{t('silicon.replayTitle')}</span>
          </CardTitle>
          <CardDescription>{t('silicon.barriersNote')}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex flex-wrap gap-2">
            <Button
              type="button"
              size="sm"
              variant="outline"
              data-testid="silicon-replay-volatile-btn"
              disabled={vm.busy}
              onClick={() => void vm.replayVolatilePatchLog()}
            >
              {t('silicon.replayVolatileBtn')}
            </Button>
            <Button
              type="button"
              size="sm"
              variant="outline"
              data-testid="silicon-replay-diag-btn"
              disabled={vm.busy}
              onClick={() => void vm.replayVideoOsdDiagLog()}
            >
              {t('silicon.replayDiagBtn')}
            </Button>
          </div>

          {vm.replaySummary && (
            <div className="space-y-3 text-xs">
              {vm.replaySummary.main3060np_osd_caveat && (
                <Alert variant="warning">
                  <AlertTriangle className="w-4 h-4" />
                  <AlertTitle>{t('silicon.caveatTitle')}</AlertTitle>
                  <AlertDescription>{vm.replaySummary.main3060np_osd_caveat}</AlertDescription>
                </Alert>
              )}

              {vm.replaySummary.bcu_fault && (
                <Alert variant="destructive">
                  <ShieldAlert className="w-4 h-4" />
                  <AlertTitle>{t('silicon.bcuFaultTitle')}</AlertTitle>
                  <AlertDescription>
                    {t('silicon.bcuFaultDetail', {
                      bcberr: vm.replaySummary.bcu_fault.bcberr_hex,
                      addr: formatHexU32(vm.replaySummary.bcu_fault.fault_address).slice(HEX_BYTE_PAD),
                      tasks: vm.replaySummary.bcu_fault.task_dump_count,
                    })}
                  </AlertDescription>
                </Alert>
              )}

              {vm.replaySummary.standby_gate_tripped && (
                <Alert variant="destructive">
                  <ShieldAlert className="w-4 h-4" />
                  <AlertTitle>{t('silicon.standbyTitle')}</AlertTitle>
                  <AlertDescription>
                    {`STANDBY_GATE_STOP: ${vm.replaySummary.standby_gate_reason ?? ''}`}
                  </AlertDescription>
                </Alert>
              )}

              <div className="p-3 rounded-lg bg-slate-950 border border-slate-800 space-y-1.5">
                <div className="text-[11px] font-bold text-cyan-300">{t('silicon.ddReadsTitle')}</div>
                {vm.replaySummary.dd_reads.length === 0 ? (
                  <div className="text-[11px] text-slate-500">{t('silicon.emptyDdReads')}</div>
                ) : (
                  vm.replaySummary.dd_reads.map((dd) => (
                    <div
                      key={`${dd.base_va}-${dd.words.join(',')}`}
                      className="text-[11px] text-slate-300 break-all"
                    >
                      <span className="text-emerald-400 font-bold">{formatHexU32(dd.base_va)}</span>:{' '}
                      {formatHexBytes(dd.bytes)}
                    </div>
                  ))
                )}
              </div>

              {vm.replaySummary.write_echoes.length > 0 && (
                <div className="p-3 rounded-lg bg-slate-950 border border-slate-800 space-y-1">
                  <div className="text-[11px] font-bold text-amber-300">{t('silicon.echoesTitle')}</div>
                  {vm.replaySummary.write_echoes.map((echo) => (
                    <div
                      key={`${echo.addr}-${echo.old_value}-${echo.new_value}`}
                      className="text-[11px] text-slate-300"
                    >
                      {`${formatHexU32(echo.addr)}: ${formatHexU16(echo.old_value)} -> ${formatHexU16(echo.new_value)}${echo.addr === AIOF_FS0_VA ? ' (AIOF_FS0)' : ''}`}
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Terminal className="w-4 h-4 text-amber-400" />
            <span>{t('silicon.patchCardTitle')}</span>
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4 text-xs">
          <div className="grid grid-cols-1 md:grid-cols-4 gap-3">
            <div className="space-y-1">
              <label htmlFor="silicon-port-select" className="text-slate-400 block">
                {t('silicon.portLabel')}
              </label>
              <select
                id="silicon-port-select"
                value={vm.selectedPort}
                onChange={(e) => vm.setSelectedPort(e.target.value)}
                className="w-full px-3 py-2 rounded-lg bg-slate-950 border border-slate-700 text-slate-100"
              >
                {vm.ports.map((p) => (
                  <option key={p} value={p}>{p}</option>
                ))}
              </select>
            </div>

            <div className="space-y-1">
              <label htmlFor="silicon-target-addr-input" className="text-slate-400 block">
                {t('silicon.addrLabel')}
              </label>
              <input
                id="silicon-target-addr-input"
                data-testid="silicon-target-addr-input"
                type="text"
                value={vm.targetAddrHex}
                onChange={(e) => vm.setTargetAddrHex(e.target.value)}
                className="w-full px-3 py-2 rounded-lg bg-slate-950 border border-slate-700 text-slate-100"
              />
            </div>

            <div className="space-y-1">
              <label htmlFor="silicon-patch-hex-input" className="text-slate-400 block">
                {t('silicon.patchLabel')}
              </label>
              <input
                id="silicon-patch-hex-input"
                data-testid="silicon-patch-hex-input"
                type="text"
                value={vm.patchHex}
                onChange={(e) => vm.setPatchHex(e.target.value)}
                className="w-full px-3 py-2 rounded-lg bg-slate-950 border border-slate-700 text-slate-100"
              />
            </div>

            <div className="space-y-1">
              <label htmlFor="silicon-width-select" className="text-slate-400 block">
                {t('silicon.widthLabel')}
              </label>
              <select
                id="silicon-width-select"
                value={vm.width}
                onChange={(e) => {
                  const v = e.target.value;
                  const next: SiliconPatchWidth = v === 'bytes' ? 'bytes' : 'u16';
                  vm.setWidth(next);
                }}
                className="w-full px-3 py-2 rounded-lg bg-slate-950 border border-slate-700 text-slate-100"
              >
                <option value="u16">{t('silicon.widthU16Label')}</option>
                <option value="bytes">{t('silicon.widthBytesLabel')}</option>
              </select>
            </div>
          </div>

          <label className="flex items-center gap-2 text-slate-300 cursor-pointer">
            <input
              type="checkbox"
              data-testid="silicon-volatile-confirm-checkbox"
              checked={vm.confirmVolatile}
              onChange={(e) => vm.setConfirmVolatile(e.target.checked)}
              className="rounded border-slate-600 bg-slate-950"
            />
            <span>{t('silicon.confirmCheckboxLabel')}</span>
          </label>

          <div className="flex flex-wrap gap-2">
            <Button
              type="button"
              size="sm"
              variant="secondary"
              data-testid="silicon-live-dd-btn"
              disabled={vm.busy}
              onClick={() => void vm.readLiveSnapshot()}
            >
              <Terminal className="w-3.5 h-3.5 mr-1.5" />
              <span>{t('silicon.liveDdBtn')}</span>
            </Button>
            <Button
              type="button"
              size="sm"
              data-testid="silicon-apply-patch-btn"
              disabled={vm.busy}
              onClick={() => void vm.applyVolatilePatch()}
            >
              <Activity className="w-3.5 h-3.5 mr-1.5" />
              <span>{t('silicon.applyPatchBtn')}</span>
            </Button>
            <Button
              type="button"
              size="sm"
              variant="outline"
              data-testid="silicon-rollback-btn"
              disabled={vm.busy}
              onClick={() => void vm.rollbackLastPatches()}
            >
              <RotateCcw className="w-3.5 h-3.5 mr-1.5" />
              <span>{t('silicon.rollbackBtn')}</span>
            </Button>
          </div>

          {vm.liveSnapshot && (
            <div className="p-3 rounded-lg bg-slate-950 border border-cyan-500/40 text-[11px] text-cyan-200 break-all">
              {`${formatHexU32(vm.liveSnapshot.base_va)}: ${formatHexBytes(vm.liveSnapshot.bytes)}`}
            </div>
          )}

          {vm.barrierError && (
            <Alert variant="destructive" data-testid="silicon-barrier-error">
              <ShieldAlert className="w-4 h-4" />
              <AlertTitle>{t('silicon.barrierBlockedTitle')}</AlertTitle>
              <AlertDescription>{vm.barrierError}</AlertDescription>
            </Alert>
          )}

          {vm.statusMessage && (
            <Alert data-testid="silicon-status-banner">
              <CheckCircle2 className="w-4 h-4 text-emerald-400" />
              <AlertTitle>{t('silicon.telemetryTitle')}</AlertTitle>
              <AlertDescription>{vm.statusMessage}</AlertDescription>
            </Alert>
          )}

          {vm.patchReport && vm.patchReport.changes.length > 0 && (
            <div className="p-3 rounded-lg bg-slate-950 border border-slate-800 space-y-1">
              <div className="text-[11px] font-bold text-cyan-300">{t('silicon.diffTitle')}</div>
              {vm.patchReport.before_snapshots.map((snap) => (
                <div
                  key={`before-${snap.base_va}-${snap.words.join(',')}`}
                  className="text-[11px] text-slate-400 break-all"
                >
                  {`${t('silicon.beforeSnapshotLabel')} ${formatHexU32(snap.base_va)}: ${formatHexBytes(snap.bytes)}`}
                </div>
              ))}
              {vm.patchReport.after_snapshots.map((snap) => (
                <div
                  key={`after-${snap.base_va}-${snap.words.join(',')}`}
                  className="text-[11px] text-cyan-200 break-all"
                >
                  {`${t('silicon.afterSnapshotLabel')} ${formatHexU32(snap.base_va)}: ${formatHexBytes(snap.bytes)}`}
                </div>
              ))}
              {vm.patchReport.changes.map((entry) => (
                <div key={entry.addr} className="text-[11px] text-slate-300">
                  {`diff ${formatHexU32(entry.addr)}: 0x${formatHexByte(entry.before)} -> 0x${formatHexByte(entry.after)}`}
                </div>
              ))}
            </div>
          )}

          {vm.rollbackReport && (
            <div className="p-3 rounded-lg bg-slate-950 border border-emerald-500/40 text-emerald-300">
              {t('silicon.rollbackSummary', {
                clean: String(vm.rollbackReport.verified_clean),
                ops: vm.rollbackReport.restored_ops,
                bytes: vm.rollbackReport.restored_bytes,
              })}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
};
