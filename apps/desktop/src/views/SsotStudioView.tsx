import React from 'react';
import {
  AlertTriangle,
  CheckCircle2,
  Cpu,
  Layers,
  Loader2,
  Palette,
  Save,
  ShieldCheck,
  Zap,
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
import { rgb565ToCssHex, useSsotStudioViewModel } from '@/viewmodels/useSsotStudioViewModel';

const MIN_CONTRAST_INPUT = 1;
const MAX_CONTRAST_INPUT = 100;
const MILLIVOLTS_PER_VOLT = 1000;
const SWATCH_PX = 20;

export const SsotStudioView: React.FC = () => {
  const { t } = useTranslation();
  const vm = useSsotStudioViewModel();
  const activeMenu = vm.config.menus.find((m) => m.id === vm.selectedMenuId) ?? vm.config.menus[0];

  return (
    <div className="space-y-6 max-w-6xl mx-auto pb-12 font-mono">
      <div className="rounded-2xl bg-bench-900 border border-slate-800 p-5 shadow-xl flex flex-col lg:flex-row lg:items-center justify-between gap-4">
        <div className="flex items-center gap-3.5">
          <div className="p-3 rounded-xl bg-cyan-500/10 text-cyan-400 border border-cyan-500/30">
            <Layers className="w-6 h-6" />
          </div>
          <div>
            <h2 className="text-base font-bold text-slate-100">{t('studio.title')}</h2>
            <p className="text-xs text-slate-400 font-sans mt-0.5">{t('studio.subtitle')}</p>
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-2 text-xs">
          <Badge variant="secondary">
            {t('studio.promptLabel')}: {vm.config.prompt}
          </Badge>
          <Badge variant="default" data-testid="ssotstudio-contract-hash">
            {t('studio.contractHashLabel')}: {vm.contractHashHex}
          </Badge>
        </div>
      </div>

      {vm.isLoading && (
        <div className="flex items-center gap-2 text-xs text-cyan-400">
          <Loader2 className="w-3.5 h-3.5 animate-spin" />
          <span>{t('studio.loadingLabel')}</span>
        </div>
      )}

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <ShieldCheck className="w-4 h-4 text-emerald-400" />
            <span>{t('studio.mmioFirewallTitle')}</span>
          </CardTitle>
          <CardDescription>{t('silicon.barriersNote')}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="space-y-1.5">
              <label htmlFor="ssotstudio-mmio-input" className="text-xs text-slate-300 block">
                {t('studio.mmioInputLabel')}
              </label>
              <div className="flex items-center gap-2">
                <input
                  id="ssotstudio-mmio-input"
                  data-testid="ssotstudio-mmio-input"
                  type="text"
                  value={vm.mmioInput}
                  onChange={(e) => vm.setMmioInput(e.target.value)}
                  className="flex-1 px-3 py-2 rounded-lg bg-slate-950 border border-slate-700 text-xs text-slate-100 focus:outline-none focus:border-cyan-500"
                />
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  data-testid="ssotstudio-validate-btn"
                  disabled={vm.busy}
                  onClick={() => void vm.validateMmioProbe()}
                >
                  {vm.busy ? t('studio.validating') : t('studio.validateBtn')}
                </Button>
              </div>
            </div>

            <div className="space-y-1.5">
              <label htmlFor="ssotstudio-theme-contrast-input" className="text-xs text-slate-300 block">
                {t('studio.contrastInputLabel')}
              </label>
              <div className="flex items-center gap-2">
                <input
                  id="ssotstudio-theme-contrast-input"
                  data-testid="ssotstudio-theme-contrast-input"
                  type="number"
                  min={MIN_CONTRAST_INPUT}
                  max={MAX_CONTRAST_INPUT}
                  value={vm.contrastValue}
                  onChange={(e) => vm.updateThemeContrast(e.target.value)}
                  className="w-28 px-3 py-2 rounded-lg bg-slate-950 border border-slate-700 text-xs text-slate-100 focus:outline-none focus:border-cyan-500"
                />
                <Button
                  type="button"
                  size="sm"
                  data-testid="ssotstudio-save-btn"
                  disabled={vm.busy}
                  onClick={() => void vm.saveConfiguration()}
                >
                  <Save className="w-3.5 h-3.5 mr-1.5" />
                  <span>{vm.busy ? t('studio.saving') : t('studio.saveBtn')}</span>
                </Button>
              </div>
            </div>
          </div>

          {vm.validationError && (
            <Alert variant="destructive" data-testid="ssotstudio-validation-error">
              <AlertTriangle className="w-4 h-4" />
              <AlertTitle>{t('studio.firewallBlockedTitle')}</AlertTitle>
              <AlertDescription>{vm.validationError}</AlertDescription>
            </Alert>
          )}
          {vm.validationStatus && (
            <Alert data-testid="ssotstudio-validation-ok">
              <CheckCircle2 className="w-4 h-4 text-emerald-400" />
              <AlertTitle>{t('studio.verifiedLabel')}</AlertTitle>
              <AlertDescription>{vm.validationStatus}</AlertDescription>
            </Alert>
          )}
          {vm.saveMessage && (
            <Alert data-testid="ssotstudio-save-banner">
              <CheckCircle2 className="w-4 h-4 text-cyan-400" />
              <AlertTitle>{t('studio.contractHashLabel')}</AlertTitle>
              <AlertDescription>{vm.saveMessage}</AlertDescription>
            </Alert>
          )}
        </CardContent>
      </Card>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Layers className="w-4 h-4 text-cyan-400" />
              <span>{t('studio.menusSection')}</span>
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            {vm.config.menus.length === 0 ? (
              <div className="text-xs text-slate-500">{t('studio.emptyMenus')}</div>
            ) : (
              <div className="flex flex-wrap gap-1.5">
                {vm.config.menus.map((menu) => (
                  <Button
                    key={menu.id}
                    type="button"
                    size="sm"
                    variant={menu.id === activeMenu?.id ? 'default' : 'outline'}
                    onClick={() => vm.setSelectedMenuId(menu.id)}
                  >
                    {menu.id} ({t('studio.itemsLabel', { count: menu.items.length })})
                  </Button>
                ))}
              </div>
            )}
            <div className="space-y-1.5 text-xs">
              {!activeMenu || activeMenu.items.length === 0 ? (
                <div className="text-xs text-slate-500">{t('studio.emptyMenus')}</div>
              ) : (
                activeMenu.items.map((item) => (
                <div
                  key={item.id}
                  className="flex items-center justify-between px-3 py-2 rounded-lg bg-slate-950/90 border border-slate-800"
                >
                  <div>
                    <div className="font-semibold text-slate-200">{item.command}</div>
                    <div className="text-[11px] text-slate-500">{item.label_key}</div>
                  </div>
                  <Badge variant={item.requires_unlock ? 'destructive' : 'secondary'}>
                    {`T${item.min_safety_tier}${item.requires_unlock ? ' UNLOCK' : ''}`}
                  </Badge>
                </div>
                ))
              )}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Palette className="w-4 h-4 text-amber-400" />
              <span>{t('studio.themesSection')}</span>
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-3 text-xs">
            {vm.config.themes.map((theme) => (
              <div
                key={theme.id}
                className="p-3 rounded-lg bg-slate-950/90 border border-slate-800 flex items-center justify-between gap-3"
              >
                <div className="space-y-1">
                  <div className="font-semibold text-slate-100">{theme.id}</div>
                  <div className="text-[11px] text-slate-400">
                    {t('studio.overscanLabel', { px: theme.overscan_px, font: theme.osd_font_mode })}
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  {[
                    ['bg', theme.bg_rgb565],
                    ['fg', theme.fg_rgb565],
                    ['acc', theme.accent_rgb565],
                    ['dng', theme.danger_rgb565],
                  ].map(([role, hex]) => (
                    <svg
                      key={`${theme.id}-${role}`}
                      aria-hidden="true"
                      className="w-5 h-5 rounded border border-slate-700 shrink-0"
                      viewBox={`0 0 ${SWATCH_PX} ${SWATCH_PX}`}
                    >
                      <rect width={SWATCH_PX} height={SWATCH_PX} fill={rgb565ToCssHex(hex)} />
                    </svg>
                  ))}
                  <Badge variant="secondary">{`C=${theme.contrast_level}%`}</Badge>
                </div>
              </div>
            ))}
          </CardContent>
        </Card>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Zap className="w-4 h-4 text-emerald-400" />
              <span>{t('studio.powerSection')}</span>
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-3 text-xs">
            {vm.config.power_profiles.map((profile) => (
              <div
                key={profile.id}
                className="p-3 rounded-lg bg-slate-950/90 border border-slate-800 space-y-1.5"
              >
                <div className="flex items-center justify-between">
                  <span className="font-bold text-slate-100">{profile.id}</span>
                  <Badge variant={profile.unlock_required ? 'warning' : 'success'}>
                    {`${profile.uart_baud} bps UDMA${profile.sata_udma_mode}`}
                  </Badge>
                </div>
                <div className="text-[11px] text-slate-400">
                  {t('studio.powerRowSummary', {
                    soc: profile.soc_core_id,
                    tbc: profile.tbc_mode,
                    fs0: profile.aiof_fs0_mode,
                    count: profile.allowed_mmio_writes.length,
                  })}
                </div>
              </div>
            ))}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Cpu className="w-4 h-4 text-cyan-400" />
              <span>{t('studio.socSection')}</span>
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-2.5 text-xs">
            {vm.config.soc_targets.map((soc) => (
              <div
                key={soc.soc_core_id}
                className="p-3 rounded-lg bg-slate-950/90 border border-slate-800 flex items-center justify-between gap-2"
              >
                <div>
                  <div className="font-bold text-slate-100">
                    {`${soc.isa} (${soc.soc_id.toUpperCase()})`}
                  </div>
                  <div className="text-[11px] text-slate-400">
                    {t('studio.socRowSub', {
                      coreId: soc.soc_core_id,
                      volts: (soc.logic_voltage_mv / MILLIVOLTS_PER_VOLT).toFixed(1),
                      endian: soc.is_big_endian ? 'BE' : 'LE',
                    })}
                  </div>
                </div>
                <Badge variant={soc.mmio_verified ? 'success' : 'secondary'}>
                  {soc.mmio_verified ? t('studio.verifiedLabel') : t('studio.unverifiedLabel')}
                </Badge>
              </div>
            ))}
          </CardContent>
        </Card>
      </div>
    </div>
  );
};
