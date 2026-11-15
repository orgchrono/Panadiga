import { useCallback, useState } from 'react';
import type { Dispatch, SetStateAction } from 'react';
import type {
  BootSequenceReport,
  FrmUnlockVerificationSummary,
  MatshitaBootScenario,
  MatshitaInjectWriteResult,
  PackUnlockFrmResult,
  SerialCliResponse,
} from '@/models';
import { invokeCommand } from '@/services/ipc';

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
  toastTitle?: string
) => Promise<T | null>;

/**
 * Modular action hook managing the Matsushita R6-MV4 optical drive emulator,
 * ATAPI write injection (`0x3B` / `0xED`), factory serial CLI, and `.FRM` packer.
 */
export function useMatshitaEmuActions(runChannel: RunChannelFn) {
  const [matshitaBoot, setMatshitaBoot] = useState<AsyncChannelState<BootSequenceReport>>(() =>
    emptyChannel()
  );
  const [matshitaInject, setMatshitaInject] = useState<
    AsyncChannelState<MatshitaInjectWriteResult>
  >(() => emptyChannel());
  const [matshitaSerial, setMatshitaSerial] = useState<AsyncChannelState<SerialCliResponse>>(() =>
    emptyChannel()
  );
  const [frmVerify, setFrmVerify] = useState<AsyncChannelState<FrmUnlockVerificationSummary>>(() =>
    emptyChannel()
  );
  const [frmPack, setFrmPack] = useState<AsyncChannelState<PackUnlockFrmResult>>(() =>
    emptyChannel()
  );

  const runMatshitaBoot = useCallback(
    async (scenario: MatshitaBootScenario): Promise<BootSequenceReport | null> =>
      runChannel(
        setMatshitaBoot,
        'Matshita R6-MV4 Boot',
        () => invokeCommand<BootSequenceReport>('run_matshita_boot_emu', { scenario }),
        `Sequência de boot (${scenario}) emulada`
      ),
    [runChannel]
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
        'ATAPI Write Injection',
        () =>
          invokeCommand<MatshitaInjectWriteResult>('inject_matshita_atapi_write', {
            targetVa: params.targetVa?.trim() || undefined,
            offset: params.offset ?? 0,
            mode: params.mode ?? 2,
            useVendorEd: Boolean(params.useVendorEd),
            payloadHex: params.payloadHex.trim(),
          }),
        'Injeção ATAPI concluída'
      ),
    [runChannel]
  );

  const runMatshitaSerial = useCallback(
    async (command: string, unlockPass = true): Promise<SerialCliResponse | null> =>
      runChannel(
        setMatshitaSerial,
        'Matshita DRV CLI',
        () =>
          invokeCommand<SerialCliResponse>('run_matshita_serial_cmd', {
            command: command.trim(),
            unlockPass,
          }),
        'Comando DRV executado'
      ),
    [runChannel]
  );

  const verifyUnlockFrm = useCallback(
    async (
      containerKind: string,
      frmPath: string
    ): Promise<FrmUnlockVerificationSummary | null> =>
      runChannel(
        setFrmVerify,
        'FRM Unlock Verifier',
        () =>
          invokeCommand<FrmUnlockVerificationSummary>('verify_frm_unlock_command', {
            containerKind: containerKind.trim(),
            frmPath: frmPath.trim(),
          }),
        'Container .FRM verificado'
      ),
    [runChannel]
  );

  const packUnlockFrm = useCallback(
    async (params: {
      containerKind: string;
      outPath?: string;
      payloadHex?: string;
    }): Promise<PackUnlockFrmResult | null> =>
      runChannel(
        setFrmPack,
        'FRM Unlock Builder',
        () =>
          invokeCommand<PackUnlockFrmResult>('pack_unlock_frm_command', {
            containerKind: params.containerKind.trim(),
            outPath: params.outPath?.trim() || undefined,
            payloadHex: params.payloadHex?.trim() || undefined,
          }),
        'Container unlock .FRM gerado'
      ),
    [runChannel]
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
