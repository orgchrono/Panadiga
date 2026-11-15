import React from 'react';
import { Disc3 } from 'lucide-react';
import type {
  BootSequenceReport,
  FrmUnlockVerificationSummary,
  MatshitaBootScenario,
  MatshitaInjectWriteResult,
  PackUnlockFrmResult,
  SerialCliResponse,
} from '@/models';
import {
  Badge,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import { MatshitaBootSection } from './MatshitaBootSection';
import { MatshitaInjectSection } from './MatshitaInjectSection';
import { MatshitaSerialFrmSection } from './MatshitaSerialFrmSection';
import type { MatshitaAsyncSlice } from './matshitaTypes';

interface MatshitaEmulatorCardProps {
  bootState: MatshitaAsyncSlice<BootSequenceReport>;
  injectState: MatshitaAsyncSlice<MatshitaInjectWriteResult>;
  serialState: MatshitaAsyncSlice<SerialCliResponse>;
  frmVerifyState: MatshitaAsyncSlice<FrmUnlockVerificationSummary>;
  frmPackState: MatshitaAsyncSlice<PackUnlockFrmResult>;
  onRunBoot: (scenario: MatshitaBootScenario) => Promise<BootSequenceReport | null>;
  onInjectWrite: (params: {
    targetVa?: string;
    offset?: number;
    mode?: number;
    useVendorEd?: boolean;
    payloadHex: string;
  }) => Promise<MatshitaInjectWriteResult | null>;
  onRunSerial: (command: string, unlockPass?: boolean) => Promise<SerialCliResponse | null>;
  onVerifyFrm: (containerKind: string, frmPath: string) => Promise<FrmUnlockVerificationSummary | null>;
  onPackFrm: (params: {
    containerKind: string;
    outPath?: string;
    payloadHex?: string;
  }) => Promise<PackUnlockFrmResult | null>;
}

export const MatshitaEmulatorCard: React.FC<MatshitaEmulatorCardProps> = ({
  bootState,
  injectState,
  serialState,
  frmVerifyState,
  frmPackState,
  onRunBoot,
  onInjectWrite,
  onRunSerial,
  onVerifyFrm,
  onPackFrm,
}) => {
  const { t } = useTranslation();

  return (
    <Card data-testid="matshita-emulator-card">
      <CardHeader>
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <Disc3 className="w-4 h-4 text-cyan-400" />
            <CardTitle>{t('unbrick.matshita.title')}</CardTitle>
          </div>
          <Badge variant="outline" className="font-mono text-[10px] border-cyan-500/40 text-cyan-300">
            R6-MV4 + E05 IPL RO
          </Badge>
        </div>
        <CardDescription>{t('unbrick.matshita.desc')}</CardDescription>
      </CardHeader>

      <CardContent className="grid grid-cols-1 xl:grid-cols-2 gap-5">
        <MatshitaBootSection bootState={bootState} onRunBoot={onRunBoot} />
        <MatshitaInjectSection injectState={injectState} onInjectWrite={onInjectWrite} />
        <MatshitaSerialFrmSection
          serialState={serialState}
          frmVerifyState={frmVerifyState}
          frmPackState={frmPackState}
          onRunSerial={onRunSerial}
          onVerifyFrm={onVerifyFrm}
          onPackFrm={onPackFrm}
        />
      </CardContent>
    </Card>
  );
};
