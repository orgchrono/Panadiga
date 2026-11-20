/**
 * Mock IPC handlers for the Matshita R6-MV4 Drive Emulator & Unlock FRM containers.
 */

export function handleMockMatshitaCommand(
  cmd: string,
  args?: Record<string, unknown>,
): unknown | undefined {
  switch (cmd) {
    case 'run_matshita_boot_emu': {
      const scenario = String(args?.scenario ?? 'healthy_with_disc');
      const trayError = scenario === 'tray_cannot_close_80045300';
      const laserError = scenario === 'laser_apc_failure_80040904';
      const unlockFrm = scenario === 'unlock_disc_frm';
      return {
        scenario,
        inquiry_id: 'MATSHITA R6-MV4          ',
        steps: [
          {
            step_index: 1,
            boot_fn_va: '0x440062B8',
            boot_fn_name: 'atapi_test_unit_ready',
            cdb_hex: '00 00 00 00 00 00 00 00 00 00 00 00',
            sense_hex: trayError
              ? '70 00 04 00 00 00 00 0A 00 00 02 01 53 00 00 00 00 00'
              : '00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00',
            boot_log_line: trayError ? '** can not close tray **' : '[RTSD] Drive Ready',
            passed: !trayError,
          },
          {
            step_index: 2,
            boot_fn_va: '0x440067D0',
            boot_fn_name: unlockFrm ? 'disc_pana_dvd_frm_verify' : 'drive_startup_check',
            cdb_hex: '1B 00 00 00 03 00 01 00 00 00 00 00',
            sense_hex: laserError
              ? '70 00 04 00 00 00 00 0A 00 00 08 03 A7 00 00 00 00 00'
              : '00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00',
            boot_log_line: laserError
              ? 'drive start error(80040904)'
              : unlockFrm
                ? 'PANA_DVD.FRM verified -> JMP 0x50400000'
                : 'Normal disc TOC verified',
            passed: !trayError && !laserError,
          },
        ],
        uart_boot_transcript: [
          '***** Loader ver0.61 [06/03/06] *****',
          trayError
            ? '[RTSD] === DRIVE ACCESS ERROR === ( 80045300 )'
            : laserError
              ? 'drive start error(80040904)'
              : '[MATSHITA R6-MV4] Boot check completed',
        ],
        startup_succeeded: !trayError && !laserError,
        unlock_media_verified: unlockFrm,
        jump_target_va_hex: unlockFrm ? '0x50400000' : null,
        final_boot_code_hex: trayError ? '0x80045300' : laserError ? '0x80040904' : '0x00000000',
      };
    }

    case 'inject_matshita_atapi_write': {
      const targetVaRaw = String(args?.targetVa ?? args?.target_va ?? '0x51800000').trim();
      const parsedVa = Number.parseInt(targetVaRaw, 16);
      const inIplLow = Number.isFinite(parsedVa) && parsedVa >= 0x80000000 && parsedVa <= 0x807effff;
      const inIplHigh = Number.isFinite(parsedVa) && parsedVa >= 0x84000000 && parsedVa <= 0x847effff;
      if (inIplLow || inIplHigh) {
        throw new Error(
          `E05 IPL NOR Write Protected: gravação bloqueada em ${targetVaRaw} (região 0x80000000..0x807EFFFF somente leitura).`,
        );
      }
      const useEd = Boolean(args?.useVendorEd ?? args?.use_vendor_ed);
      const payloadHex = String(args?.payloadHex ?? args?.payload_hex ?? 'DEADBEEF').trim();
      const lenBytes = Math.max(1, Math.floor(payloadHex.length / 2));
      const record = {
        seq: 1,
        source: useEd ? 'ATAPI_VENDOR_ED' : 'ATAPI_WRITE_BUFFER_3B',
        cdb0_or_cmd: useEd ? '0xED' : '0x3B',
        mode: Number(args?.mode ?? 2),
        target_va_or_lba: Number.isFinite(parsedVa) ? parsedVa : 0x51800000,
        len_bytes: lenBytes,
        payload_sha256: 'e3b0c44298fc1c149afbf4c8996fb924',
        status: 'OK',
        note: `Injeção SDRAM ${targetVaRaw} concluída (IPL protegido)`,
      };
      return {
        record,
        snapshot: {
          inquiry_vendor: 'MATSHITA',
          inquiry_product: 'R6-MV4          ',
          inquiry_revision: '1.00',
          tray_closed: true,
          media_present: true,
          sram_0cd8_auth: 1,
          sram_0d78_s211: 0,
          sram_0242_ed_mode: useEd ? 1 : 0,
          mmode: Number(args?.mode ?? 2),
          slog_enabled: false,
          slog_entries: [],
          buffer_len: lenBytes,
          buffer_sha256: record.payload_sha256,
          disc_lba_sectors: 2295104,
          history_count: 1,
          injections: [record],
        },
      };
    }

    case 'run_matshita_serial_cmd': {
      const command = String(args?.command ?? 'WW 0x51800000 0xDEADBEEF').trim();
      const upper = command.toUpperCase();
      if (upper.includes('0X80') || upper.includes('0X84')) {
        throw new Error('E05 IPL NOR Write Protected: escrita via DRV serial bloqueada na IPL.');
      }
      return {
        command,
        authorized_flag_cd8: 1,
        s211_flag_d78: 0,
        slog_enabled: false,
        output_lines: [`> ${command}`, 'OK (SDRAM/SRAM guarded R/W)'],
        status: 'OK',
      };
    }

    case 'verify_frm_unlock_command': {
      const kind = String(args?.containerKind ?? args?.container_kind ?? 'sd').trim();
      const isSd = kind.toLowerCase().includes('sd') || kind.toLowerCase().includes('extr');
      return {
        container_kind: kind,
        container_bytes: isSd ? 131_072 : 262_144,
        sha256: '9f86d081884c7d659a2feaa0c55ad015',
        entry_count: 1,
        bootloader_jump_va_hex: '0x50400000',
        extrhead_valid: isSd ? true : null,
        verified: true,
        notes: [
          isSd ? 'EXTRHEAD + JMP 0x50400000 válido' : 'Header MATSHITA PANA_DVD.FRM válido',
        ],
      };
    }

    case 'pack_unlock_frm_command': {
      const kind = String(args?.containerKind ?? args?.container_kind ?? 'sd').trim();
      const isSd = kind.toLowerCase().includes('sd') || kind.toLowerCase().includes('extr');
      const containerBytes = isSd ? 131_072 : 262_144;
      return {
        verification: {
          container_kind: kind,
          container_bytes: containerBytes,
          sha256: '9f86d081884c7d659a2feaa0c55ad015',
          entry_count: 1,
          bootloader_jump_va_hex: '0x50400000',
          extrhead_valid: isSd ? true : null,
          verified: true,
          notes: ['Container gerado e verificado contra regras Loader ver0.61'],
        },
        output_path: (args?.outPath as string) || (args?.out_path as string) || null,
        container_bytes: containerBytes,
      };
    }

    default:
      return undefined;
  }
}
