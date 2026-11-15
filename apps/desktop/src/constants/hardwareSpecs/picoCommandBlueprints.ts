/**
 * @file constants/hardwareSpecs/picoCommandBlueprints.ts
 * @brief Frozen CLI command blueprints for the RP2040/RP2350 P7501 probe.
 */

import type { PicoCommandSpec } from '@/models';
import type { SsotLocaleDict } from './ssotBlueprints';

export type CommandLocaleKey = keyof SsotLocaleDict['commands'];

export interface PicoCommandLocaleEntry {
  readonly label: string;
  readonly description: string;
  readonly arg_label?: string;
  readonly arg_placeholder?: string;
  readonly options?: Readonly<Record<string, string>>;
}

export interface CommandBlueprint {
  readonly id: CommandLocaleKey;
  readonly category: PicoCommandSpec['category'];
  readonly syntax: string;
  readonly default_arg: string | null;
  readonly optionPairs?: ReadonlyArray<readonly [string, string]>;
  readonly dangerous?: boolean;
  readonly timeout_ms?: number;
}

export const PICO_COMMAND_BLUEPRINTS: readonly CommandBlueprint[] = Object.freeze([
  { id: 'PINCHECK', category: 'electrical', syntax: 'PINCHECK', default_arg: null, timeout_ms: 4000 },
  { id: 'BUSDIAG', category: 'electrical', syntax: 'BUSDIAG', default_arg: null, timeout_ms: 12000 },
  {
    id: 'SNIFF',
    category: 'electrical',
    syntax: 'SNIFF [ms]',
    default_arg: '250',
    optionPairs: [['w250', '250'], ['w1000', '1000'], ['w3000', '3000']],
    timeout_ms: 5000,
  },
  { id: 'ISOLATE', category: 'electrical', syntax: 'ISOLATE', default_arg: null, timeout_ms: 1500 },
  { id: 'ENGAGE', category: 'electrical', syntax: 'ENGAGE', default_arg: null, timeout_ms: 1500 },
  { id: 'RESET', category: 'electrical', syntax: 'RESET', default_arg: null, timeout_ms: 2000 },
  { id: 'HANDSHAKE', category: 'ocd_flash', syntax: 'HANDSHAKE', default_arg: null, timeout_ms: 4000 },
  {
    id: 'BREAK_IN',
    category: 'ocd_flash',
    syntax: 'BREAK_IN [0..3]',
    default_arg: '2',
    optionPairs: [['m0', '0'], ['m1', '1'], ['m2', '2'], ['m3', '3']],
    timeout_ms: 3000,
  },
  { id: 'SCAN', category: 'ocd_flash', syntax: 'SCAN', default_arg: null, timeout_ms: 18000 },
  { id: 'AUTODETECT', category: 'ocd_flash', syntax: 'AUTODETECT', default_arg: null, timeout_ms: 3000 },
  { id: 'CFI', category: 'ocd_flash', syntax: 'CFI', default_arg: null, timeout_ms: 3000 },
  {
    id: 'FLASH_SIZE',
    category: 'ocd_flash',
    syntax: 'FLASH_SIZE [2|4|8|16]',
    default_arg: '8',
    optionPairs: [['mb2', '2'], ['mb4', '4'], ['mb8', '8'], ['mb16', '16']],
    timeout_ms: 2000,
  },
  {
    id: 'DUMP',
    category: 'ocd_flash',
    syntax: 'DUMP <hex_addr> <words>',
    default_arg: '0x807F0000 16',
    optionPairs: [
      ['nvram', '0x807F0000 16'],
      ['bootloader', '0x80000000 16'],
      ['sram', '0x00004000 16'],
    ],
    timeout_ms: 8000,
  },
  {
    id: 'BOOTCAP',
    category: 'uart_fmu',
    syntax: 'BOOTCAP [ms]',
    default_arg: '7500',
    optionPairs: [['s3', '3000'], ['s7_5', '7500'], ['s15', '15000']],
    timeout_ms: 32000,
  },
  {
    id: 'UARTMON',
    category: 'uart_fmu',
    syntax: 'UARTMON [ms]',
    default_arg: '5000',
    optionPairs: [['s2', '2000'], ['s5', '5000'], ['s10', '10000']],
    timeout_ms: 32000,
  },
  {
    id: 'JIGBOOT',
    category: 'uart_fmu',
    syntax: 'JIGBOOT [ms]',
    default_arg: '7500',
    optionPairs: [['s5', '5000'], ['s7_5', '7500']],
    timeout_ms: 32000,
  },
  {
    id: 'BAUD',
    category: 'uart_fmu',
    syntax: 'BAUD [ms]',
    default_arg: '1500',
    optionPairs: [['s1_5', '1500'], ['s3_0', '3000']],
    timeout_ms: 4500,
  },
  {
    id: 'FRAMING',
    category: 'uart_fmu',
    syntax: 'FRAMING [8N1|8E1|8O1]',
    default_arg: '8N1',
    optionPairs: [['8n1', '8N1'], ['8e1', '8E1'], ['8o1', '8O1']],
    timeout_ms: 1500,
  },
  {
    id: 'BAUDSCAN',
    category: 'uart_fmu',
    syntax: 'BAUDSCAN [ms] [NORMAL|JIG] [APPLY]',
    default_arg: '3500 NORMAL',
    optionPairs: [['normal3_5', '3500 NORMAL'], ['jig4_0', '4000 JIG']],
    timeout_ms: 8000,
  },
  {
    id: 'UARTSTREAM',
    category: 'uart_fmu',
    syntax: 'UARTSTREAM [ms] [NORMAL|JIG[:ms]|NOPULL]',
    default_arg: '8000 JIG:350',
    optionPairs: [['jig8', '8000 JIG:350'], ['normal8', '8000 NORMAL'], ['passive5', '5000']],
    timeout_ms: 32000,
  },
  {
    id: 'HEXCAP',
    category: 'uart_fmu',
    syntax: 'HEXCAP [ms] [NORMAL|JIG[:ms]|NOPULL]',
    default_arg: '5000 JIG:350',
    optionPairs: [['jig5', '5000 JIG:350'], ['normal4', '4000 NORMAL']],
    timeout_ms: 32000,
  },
  {
    id: 'UARTTXHEX',
    category: 'uart_fmu',
    syntax: 'UARTTXHEX <hex> [BRK] [rx_ms]',
    default_arg: '0D0A 1500',
    optionPairs: [['crlf', '0D0A 1500'], ['help', '68656C700D 2000']],
    timeout_ms: 5000,
  },
  {
    id: 'WAITTX',
    category: 'uart_fmu',
    syntax: 'WAITTX <ms> <NORMAL|JIG[:ms]|NOPULL> <trigger> <hex|BRK> [post_ms]',
    default_arg: '8000 JIG:350 "wait 1" 0D 4000',
    optionPairs: [
      ['wait1', '8000 JIG:350 "wait 1" 0D 4000'],
      ['hitkey', '20000 JIG:350 "Hit Any Key" 1B0D 6000'],
    ],
    timeout_ms: 32000,
  },
  {
    id: 'PWRTRACE',
    category: 'electrical',
    syntax: 'PWRTRACE [ms] [NORMAL|JIG[:ms]|NOPULL]',
    default_arg: '4000 JIG:350',
    optionPairs: [['jig4', '4000 JIG:350'], ['normal3', '3000 NORMAL']],
    timeout_ms: 15000,
  },
  {
    id: 'BOOTTRACE',
    category: 'electrical',
    syntax: 'BOOTTRACE [ms] [NORMAL|JIG[:ms]|NOPULL]',
    default_arg: '4500 JIG:2500',
    optionPairs: [['jig2500', '4500 JIG:2500'], ['normal4500', '4500 NORMAL']],
    timeout_ms: 32000,
  },
  {
    id: 'PIOLOGIC',
    category: 'electrical',
    syntax: 'PIOLOGIC [ms] [NORMAL|JIG[:ms]|NOPULL] [khz] [CTRL]',
    default_arg: '4000 JIG:350',
    optionPairs: [
      ['jig4', '4000 JIG:350'],
      ['jig2500ctrl', '4000 JIG:2500 1000 CTRL'],
      ['normal4', '4000 NORMAL'],
      ['passive2', '2000'],
    ],
    timeout_ms: 18000,
  },
  {
    id: 'JIGSWEEP',
    category: 'electrical',
    syntax: 'JIGSWEEP [HOLD|DELAY|PREBOOT] [ms..]',
    default_arg: 'HOLD',
    optionPairs: [['hold', 'HOLD'], ['delay', 'DELAY'], ['preboot', 'PREBOOT']],
    timeout_ms: 12000,
  },
  {
    id: 'PANARSP',
    category: 'uart_fmu',
    syntax: 'PANARSP [ms] [boot] [MAGIC] [WAIT:<trig>] <PROBESEQ|REGS|G|Z0|Z1|MEM:addr,len|XMEM:addr,len|DUMPNOR[:addr[,len]]|PASSTHROUGH|CALL:addr|HEX:..>',
    default_arg: '6000 JIG:350 PROBESEQ',
    optionPairs: [
      ['probeJig', '6000 JIG:350 PROBESEQ'],
      ['regsDecode', '5000 JIG:350 REGS'],
      ['magicG', '6000 JIG:350 MAGIC G'],
      ['wait1Probe', '8000 JIG:350 "WAIT:wait 1" PROBESEQ'],
      ['mem515', '4000 MEM:515B2E38,10'],
      ['xmem840', '4000 XMEM:84000000,40'],
      ['dumpnor', '180000 DUMPNOR 84000000 800000'],
    ],
    timeout_ms: 185000,
  },
  {
    id: 'DIAGWATCH',
    category: 'uart_fmu',
    syntax: 'DIAGWATCH [ms] [NORMAL|JIG[:ms]|NOPULL] [AUTO|BYPASS|READ[:addr[,len]]|WRITE:<addr,val>|FORMAT|DUMP|JUMP:<addr>|ERASE:<nvram_addr>|QUIT|NONE] [RELEASE|HOLD]',
    default_arg: '25000 JIG:2500 AUTO RELEASE',
    optionPairs: [
      ['autoBypass', '25000 JIG:2500 AUTO RELEASE'],
      ['readNvram', '25000 JIG:2500 READ:847F0000,20 RELEASE'],
      ['readMem', '25000 JIG:2500 READ RELEASE'],
      ['writeMem', '25000 JIG:2500 WRITE RELEASE'],
      ['formatBgo', '35000 JIG:2500 FORMAT RELEASE'],
      ['dumpNor', '180000 JIG:2500 DUMP RELEASE'],
      ['passive', '15000 NORMAL NONE RELEASE'],
    ],
    timeout_ms: 185000,
  },
  {
    id: 'DRVPANA',
    category: 'uart_fmu',
    syntax: 'DRVPANA [ms] [NORMAL|JIG[:ms]|NOPULL] [NOPASS] <cmd...>',
    default_arg: 'DSCREG',
    optionPairs: [
      ['dscreg', 'DSCREG'],
      ['passUnlock', 'PASS 0x4d415453 0x48495441'],
      ['fep', 'FEP'],
      ['start', 'START'],
      ['pon', 'PON'],
      ['stop', 'STOP'],
    ],
    timeout_ms: 15000,
  },
  {
    id: 'BRIDGE',
    category: 'uart_fmu',
    syntax: 'BRIDGE [baud] [NORMAL|JIG[:ms]|NOPULL]',
    default_arg: '115200 JIG:350',
    optionPairs: [['jig115200', '115200 JIG:350'], ['passive115200', '115200']],
    dangerous: true,
    timeout_ms: 15000,
  },
  {
    id: 'FMUSCAN',
    category: 'uart_fmu',
    syntax: 'FMUSCAN [modo]',
    default_arg: 'all',
    optionPairs: [['all', 'all'], ['m0', '0'], ['m1', '1'], ['m2', '2'], ['m3', '3']],
    timeout_ms: 20000,
  },
  {
    id: 'FMUTEST',
    category: 'uart_fmu',
    syntax: 'FMUTEST <hex>',
    default_arg: '0D',
    optionPairs: [['cr', '0D'], ['crlf', '0D0A'], ['help', '3F0D']],
    timeout_ms: 10000,
  },
  {
    id: 'FMUBATCH',
    category: 'uart_fmu',
    syntax: 'FMUBATCH [step]',
    default_arg: '',
    timeout_ms: 22000,
  },
  {
    id: 'UARTSEND',
    category: 'uart_fmu',
    syntax: 'UARTSEND <texto>',
    default_arg: 'help',
    optionPairs: [['help', 'help'], ['ver', 'ver'], ['info', 'info']],
    timeout_ms: 5000,
  },
  {
    id: 'UARTPROBE',
    category: 'uart_fmu',
    syntax: 'UARTPROBE [0..8]',
    default_arg: '5',
    optionPairs: [['m5', '5'], ['m6', '6'], ['m7', '7']],
    timeout_ms: 8000,
  },
  { id: 'PING', category: 'probe', syntax: 'PING', default_arg: null, timeout_ms: 1500 },
  { id: 'STATUS', category: 'probe', syntax: 'STATUS', default_arg: null, timeout_ms: 1500 },
  { id: 'BANNER', category: 'probe', syntax: 'BANNER', default_arg: null, timeout_ms: 1500 },
  {
    id: 'LED',
    category: 'probe',
    syntax: 'LED <modo>',
    default_arg: 'IDLE',
    optionPairs: [
      ['idle', 'IDLE'],
      ['active', 'ACTIVE'],
      ['gdb', 'GDB'],
      ['fault', 'FAULT'],
      ['off', 'OFF'],
      ['auto', 'AUTO'],
    ],
    timeout_ms: 1500,
  },
  { id: 'BOOTSEL', category: 'probe', syntax: 'BOOTSEL', default_arg: null, dangerous: true, timeout_ms: 1500 },
]);
