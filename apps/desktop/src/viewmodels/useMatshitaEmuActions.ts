import { useCallback, useState, type Dispatch, type SetStateAction } from 'react';
import { translate } from '@/i18n/useTranslation';
import type {
  BootSequenceReport,
  FrmUnlockVerificationSummary,
  MatshitaBootScenario,
  MatshitaInjectWriteResult,
  PackUnlockFrmResult,
  SerialCliResponse,
} from '@/models';
import {
  injectMatshitaAtapiWrite,
  packFrmUnlockContainer,
  runMatshitaBootEmu,
  runMatshitaSerialCmd,
  verifyFrmUnlockContainer,
} from '@/services/unbrickService';

const DEFAULT_ATAPI_WRITE_MODE = 2;

export interface AsyncChannelState<T> {
  loading: boolean;
  data: T | null;
  error: string | null;
}

export function emptyChannel<T>(initial: T | null = null): AsyncChannelState<T> {
  return { loading: false, data: initial, error: null };
}

export type RunChannelFn = <T>(
  setter: Dispatch<SetStateAction<AsyncChannelState<T>>>,
  tag: string,
  fn: () => Promise<T>,
  toastTitle?: string,
) => Promise<T | null>;

/**
 * Modular action hook managing the Matsushita R6-MV4 optical drive emulator,
 * ATAPI write injection (`0x3B` / `0xED`), factory serial CLI, and `.FRM` packer.
 */
export function useMatshitaEmuActions(runChannel: RunChannelFn) {
  const [matshitaBoot, setMatshitaBoot] = useState<AsyncChannelState<BootSequenceReport>>(() =>
    emptyChannel(),
  );
  const [matshitaInject, setMatshitaInject] = useState<
    AsyncChannelState<MatshitaInjectWriteResult>
  >(() => emptyChannel());
  const [matshitaSerial, setMatshitaSerial] = useState<AsyncChannelState<SerialCliResponse>>(() =>
    emptyChannel(),
  );
  const [frmVerify, setFrmVerify] = useState<AsyncChannelState<FrmUnlockVerificationSummary>>(() =>
    emptyChannel(),
  );
  const [frmPack, setFrmPack] = useState<AsyncChannelState<PackUnlockFrmResult>>(() =>
    emptyChannel(),
  );

  const runMatshitaBoot = useCallback(
    async (scenario: MatshitaBootScenario): Promise<BootSequenceReport | null> =>
      runChannel(
        setMatshitaBoot,
        translate('errors.unbrick.matshitaBootTag'),
        () => runMatshitaBootEmu(scenario),
        translate('errors.unbrick.matshitaBootDone', { scenario }),
      ),
    [runChannel],
  );

  const injectMatshitaWrite = useCallback(
    async (params: {
      targetVa?: string;
      offset?: number;
      mode?: number;
      useVendorEd?: boolean;
      payloadHex: string;
    }): Promise<MatshitaInjectWriteResult | null> =>
      runChannel(
        setMatshitaInject,
        translate('errors.unbrick.matshitaInjectTag'),
        () =>
          injectMatshitaAtapiWrite({
            targetVa: params.targetVa?.trim() || undefined,
            offset: params.offset ?? 0,
            mode: params.mode ?? DEFAULT_ATAPI_WRITE_MODE,
            useVendorEd: Boolean(params.useVendorEd),
            payloadHex: params.payloadHex.trim(),
          }),
        translate('errors.unbrick.matshitaInjectDone'),
      ),
    [runChannel],
  );

  const runMatshitaSerial = useCallback(
    async (command: string, unlockPass = true): Promise<SerialCliResponse | null> =>
      runChannel(
        setMatshitaSerial,
        translate('errors.unbrick.matshitaSerialTag'),
        () => runMatshitaSerialCmd(command.trim(), unlockPass),
        translate('errors.unbrick.matshitaSerialDone'),
      ),
    [runChannel],
  );

  const verifyUnlockFrm = useCallback(
    async (
      containerKind: string,
      frmPath: string,
    ): Promise<FrmUnlockVerificationSummary | null> =>
      runChannel(
        setFrmVerify,
        translate('errors.unbrick.frmVerifyTag'),
        () => verifyFrmUnlockContainer(containerKind.trim(), frmPath.trim()),
        translate('errors.unbrick.frmVerifyDone'),
      ),
    [runChannel],
  );

  const packUnlockFrm = useCallback(
    async (params: {
      containerKind: string;
      outPath?: string;
      payloadHex?: string;
    }): Promise<PackUnlockFrmResult | null> =>
      runChannel(
        setFrmPack,
        translate('errors.unbrick.frmPackTag'),
        () =>
          packFrmUnlockContainer({
            containerKind: params.containerKind.trim(),
            outPath: params.outPath?.trim() || undefined,
            payloadHex: params.payloadHex?.trim() || undefined,
          }),
        translate('errors.unbrick.frmPackDone'),
      ),
    [runChannel],
  );

  return {
    matshitaBoot,
    matshitaInject,
    matshitaSerial,
    frmVerify,
    frmPack,
    runMatshitaBoot,
    injectMatshitaWrite,
    runMatshitaSerial,
    verifyUnlockFrm,
    packUnlockFrm,
  };
}
