import React from 'react';
import type { Sc0BridgeInputMode } from '@/models';

export const RSP_QUICK_PROBES: ReadonlyArray<{
  readonly label: string;
  readonly mode: Sc0BridgeInputMode;
  readonly bridgePayload: string;
  readonly panarspArg: string;
}> = [
  { label: '0x15 NAK', mode: 'hex', bridgePayload: '15', panarspArg: '4000 JIG:350 NAK' },
  { label: 'Magic 0E0402', mode: 'hex', bridgePayload: '0E0402', panarspArg: '4000 JIG:350 MAGIC NONAK' },
  { label: '$g REGS', mode: 'rsp', bridgePayload: 'g', panarspArg: '5000 JIG:350 REGS' },
  { label: '$g#67 Regs', mode: 'rsp', bridgePayload: 'g', panarspArg: '6000 JIG:350 G' },
  { label: '$x#.. Bin', mode: 'rsp', bridgePayload: 'x84000000,40', panarspArg: '4000 XMEM:84000000,40' },
  { label: '$M51800000', mode: 'rsp', bridgePayload: 'M51800000,4:deadbeef', panarspArg: '4000 WMEM:51800000:deadbeef' },
  { label: '$Z0#8A Exit', mode: 'rsp', bridgePayload: 'Z0', panarspArg: '4000 Z0' },
  { label: '$Z1#8B', mode: 'rsp', bridgePayload: 'Z1', panarspArg: '4000 Z1' },
];

export const DRIVE_QUICK_COMMANDS: ReadonlyArray<{
  readonly label: string;
  readonly cmd: string;
}> = [
  { label: 'F/E Unlock', cmd: 'PASS 0x4d415453 0x48495441' },
  { label: 'PASS', cmd: 'PASS' },
  { label: 'START', cmd: 'START' },
  { label: 'LDD', cmd: 'LDD' },
  { label: 'DSCREG', cmd: 'DSCREG' },
  { label: 'FEP', cmd: 'FEP' },
  { label: 'PON', cmd: 'PON' },
  { label: 'STOP', cmd: 'STOP' },
];

export interface Sc0BridgeProbeHandlers {
  readonly onTriggerWaittx: (spec: string) => void;
  readonly onSendRspPreset: (
    mode: Sc0BridgeInputMode,
    bridgePayload: string,
    panarspArg: string
  ) => void;
  readonly onSendDriveCommand: (cmd: string) => void;
  readonly onRunDiagnostic?: (command: string, arg: string) => void;
}

export interface BridgeNoticeBannerProps {
  readonly role: 'note' | 'alert';
  readonly toneClass: string;
  readonly titleClass: string;
  readonly bodyClass: string;
  readonly title: string;
  readonly body: string;
}

export const BridgeNoticeBanner: React.FC<BridgeNoticeBannerProps> = ({
  role,
  toneClass,
  titleClass,
  bodyClass,
  title,
  body,
}) => (
  <div
    role={role}
    aria-label={title}
    className={`rounded-lg border px-3 py-2 space-y-0.5 ${toneClass}`}
  >
    <div className={`font-semibold ${titleClass}`}>{title}</div>
    <p className={`leading-relaxed ${bodyClass}`}>{body}</p>
  </div>
);
