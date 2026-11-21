import { describe, expect, it } from 'vitest';
import { isBleReplyComplete, splitGattChunks } from './bleService';

const SAMPLE_LONG_COMMAND = 'PANARSP NORUNLOCK:CONFIRM_UPPER_NOR JEDEC_ERASE:84200000\n';
const BLE_ATT_MTU23_CHUNK = 20;
const EXPECTED_CHUNK_COUNT = 3;

describe('bleService framing and GATT MTU chunking', () => {
  it('splits commands longer than 20 bytes into <=20B BLE 4.2 ATT_MTU chunks', () => {
    const encoded = new TextEncoder().encode(SAMPLE_LONG_COMMAND);
    const chunks = splitGattChunks(encoded, BLE_ATT_MTU23_CHUNK);
    expect(chunks).toHaveLength(EXPECTED_CHUNK_COUNT);
    chunks.forEach(chunk => {
      expect(chunk.byteLength).toBeLessThanOrEqual(BLE_ATT_MTU23_CHUNK);
    });
    const reassembled = new TextDecoder().decode(
      new Uint8Array(chunks.flatMap(chunk => Array.from(chunk))),
    );
    expect(reassembled).toBe(SAMPLE_LONG_COMMAND);
  });

  it('requires SSOT terminal framing (@P7501_RES newline, prompt, or legacy line) before completing', () => {
    expect(isBleReplyComplete('')).toBe(false);
    expect(isBleReplyComplete('VDD: 3.28V (OK)\n')).toBe(false);
    // Bootloader "DDR CHECK: OK" must NEVER terminate BOOTCAP/DIAGWATCH prematurely!
    expect(isBleReplyComplete('DDR CHECK: OK\r\n***** Loader ver0.61 *****\r\n')).toBe(false);
    // Partial line without trailing newline must not terminate across 20B BLE chunks
    expect(isBleReplyComplete('JOB: OK')).toBe(false);
    // Structured @P7501_IDENT frame must wait for @P7501_RES or prompt
    expect(
      isBleReplyComplete('@P7501_IDENT model=DMR-EH55\nIDENTIFY: OK (model=DMR-EH55)\n'),
    ).toBe(false);
    expect(isBleReplyComplete('@P7501_RES cmd=STATUS status=OK')).toBe(false);
    expect(isBleReplyComplete('@P7501_RES cmd=STATUS status=OK code=0 elapsed_us=100\n')).toBe(true);
    expect(isBleReplyComplete('VDD: 3.28V (OK)\nPANADIGA> ')).toBe(true);
    expect(isBleReplyComplete('PONG\n')).toBe(true);
    expect(isBleReplyComplete('JOB: ERROR code=E05\n')).toBe(true);
  });
});
