import React from 'react';
import { Wrench } from 'lucide-react';
import { PipelineStepRunner } from '@/components/pipelines';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import type { PipelineStepDescriptor } from '@/models';
import { SdHardenSection, type SdHardenSectionProps } from './SdHardenSection';
import { TbcBypassLabSection, type TbcBypassLabSectionProps } from './TbcBypassLabSection';

export interface TbcSdPipelineVm extends TbcBypassLabSectionProps, SdHardenSectionProps {
  readonly steps: readonly PipelineStepDescriptor[];
  readonly hardenSuccess: string | null;
  readonly resetAll: () => void;
}

export const TbcLabAndSdHardenPipeline: React.FC<{
  readonly vm: TbcSdPipelineVm;
  readonly hideSdHardenSection?: boolean;
}> = ({ vm, hideSdHardenSection = false }) => {
  const { t } = useTranslation();
  return (
    <Card className="bg-bench-900 border-slate-800 space-y-4">
      <CardHeader className="pb-2">
        <CardTitle className="text-base text-slate-100 flex items-center gap-2 font-mono">
          <Wrench className="w-4 h-4 text-emerald-400" />
          <span>{t('pipelines.tbcSd.title')}</span>
        </CardTitle>
        <CardDescription className="text-xs text-slate-400">{t('pipelines.tbcSd.subtitle')}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <PipelineStepRunner
          title={t('pipelines.tbcSd.title')}
          steps={vm.steps}
          isBusy={vm.tbcBusy || vm.hardenBusy}
          error={vm.tbcError ?? vm.hardenError}
          successMessage={vm.hardenSuccess}
          onReset={vm.resetAll}
        />
        <TbcBypassLabSection {...vm} />
        {!hideSdHardenSection && <SdHardenSection {...vm} />}
      </CardContent>
    </Card>
  );
};
