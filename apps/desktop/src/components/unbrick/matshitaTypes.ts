import type {
  FrmUnlockVerificationSummary,
  PackUnlockFrmResult,
  SerialCliResponse,
} from '@/models';

export interface MatshitaAsyncSlice<T> {
  loading: boolean;
  data: T | null;
  error: string | null;
}

export interface MatshitaSerialFrmProps {
  serialState: MatshitaAsyncSlice<SerialCliResponse>;
  frmVerifyState: MatshitaAsyncSlice<FrmUnlockVerificationSummary>;
  frmPackState: MatshitaAsyncSlice<PackUnlockFrmResult>;
  onRunSerial: (command: string, unlockPass?: boolean) => Promise<SerialCliResponse | null>;
  onVerifyFrm: (containerKind: string, frmPath: string) => Promise<FrmUnlockVerificationSummary | null>;
  onPackFrm: (params: {
    containerKind: string;
    outPath?: string;
    payloadHex?: string;
  }) => Promise<PackUnlockFrmResult | null>;
}

export const MATSHITA_SELECT_CLS =
  'flex h-9 w-full rounded-xl border border-slate-800 bg-slate-950 px-3 py-2 text-xs font-mono text-slate-100 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-cyan-500';
