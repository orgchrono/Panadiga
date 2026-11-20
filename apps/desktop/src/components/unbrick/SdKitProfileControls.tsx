import React from 'react';
import { Select } from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import { isBgoMode, isMainPassthroughMode, isSdLoaderProfile } from '@/utils/typeGuards';
import type { BgoMode, MainPassthroughMode, SdLoaderProfile } from './unbrickDomain';

export interface SdKitProfileControlsProps {
  readonly bgoMode: BgoMode;
  readonly onChangeBgoMode: (mode: BgoMode) => void;
  readonly mainMode: MainPassthroughMode;
  readonly onChangeMainMode: (mode: MainPassthroughMode) => void;
  readonly loaderProfile: SdLoaderProfile;
  readonly onChangeLoaderProfile: (profile: SdLoaderProfile) => void;
  readonly includePanaSd: boolean;
  readonly onChangeIncludePanaSd: (checked: boolean) => void;
  readonly unlockHddDynamic: boolean;
  readonly onChangeUnlockHddDynamic: (checked: boolean) => void;
}

export const SdKitProfileControls: React.FC<SdKitProfileControlsProps> = ({
  bgoMode,
  onChangeBgoMode,
  mainMode,
  onChangeMainMode,
  loaderProfile,
  onChangeLoaderProfile,
  includePanaSd,
  onChangeIncludePanaSd,
  unlockHddDynamic,
  onChangeUnlockHddDynamic,
}) => {
  const { t } = useTranslation();

  return (
    <>
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
        <div className="space-y-1">
          <label htmlFor="sd-kit-bgo-mode" className="text-slate-400 block font-mono">
            {t('unbrick.sdKit.bgoMode')}
          </label>
          <Select
            id="sd-kit-bgo-mode"
            value={bgoMode}
            onChange={e => {
              if (isBgoMode(e.target.value)) {
                onChangeBgoMode(e.target.value);
              }
            }}
          >
            <option value="bypass">{t('unbrick.sdKit.bgoBypass')}</option>
            <option value="uart_monitor">{t('unbrick.sdKit.bgoUart')}</option>
            <option value="factory">{t('unbrick.sdKit.bgoFactory')}</option>
            <option value="auto_format" disabled>
              {t('unbrick.sdKit.bgoAutoFormat')}
            </option>
          </Select>
        </div>

        <div className="space-y-1">
          <label htmlFor="sd-kit-main-mode" className="text-slate-400 block font-mono">
            {t('unbrick.sdKit.mainMode')}
          </label>
          <Select
            id="sd-kit-main-mode"
            value={mainMode}
            onChange={e => {
              if (isMainPassthroughMode(e.target.value)) {
                onChangeMainMode(e.target.value);
              }
            }}
          >
            <option value="av_passthrough">{t('unbrick.sdKit.mainAv')}</option>
            <option value="factory_original">{t('unbrick.sdKit.mainFactory')}</option>
          </Select>
        </div>

        <div className="space-y-1 sm:col-span-2">
          <label htmlFor="sd-kit-loader-profile" className="text-slate-400 block font-mono">
            {t('unbrick.sdKit.loaderProfile')}
          </label>
          <Select
            id="sd-kit-loader-profile"
            value={loaderProfile}
            onChange={e => {
              if (isSdLoaderProfile(e.target.value)) {
                onChangeLoaderProfile(e.target.value);
              }
            }}
          >
            <option value="ver061">{t('unbrick.sdKit.loaderVer061')}</option>
            <option value="ver063">{t('unbrick.sdKit.loaderVer063')}</option>
          </Select>
        </div>
      </div>

      <div className="flex flex-wrap items-center gap-4 pt-1 text-[11px] text-slate-300">
        <label className="inline-flex items-center gap-2 cursor-pointer">
          <input
            type="checkbox"
            checked={includePanaSd}
            onChange={e => onChangeIncludePanaSd(e.target.checked)}
            className="rounded border-slate-700 bg-slate-950 text-emerald-500 focus:ring-emerald-500"
          />
          <span>{t('unbrick.sdKit.includePanaSd')}</span>
        </label>
        <label className="inline-flex items-center gap-2 cursor-pointer">
          <input
            type="checkbox"
            checked={unlockHddDynamic}
            onChange={e => onChangeUnlockHddDynamic(e.target.checked)}
            className="rounded border-slate-700 bg-slate-950 text-cyan-500 focus:ring-cyan-500"
          />
          <span>{t('unbrick.sdKit.unlockHddDynamic')}</span>
        </label>
      </div>
    </>
  );
};
