> 🌐 **Language / Idioma:** **English** | [Português (BR)](../pt-BR/EMULADOR_MATSHITA_R6MV4_E_INJECAO_ATAPI.md)
> 🧭 **Portal:** [Docs Index](../INDEX.md) | [Complete Audit](../../COMPLETE_AUDIT.md) | [Audits](../audits/README.md) | [Ghidra Pseudocode](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Debug Logs](../debug_logs/README.md)

# Matshita R6-MV4 ODD Emulator, Boot Verification & ATAPI Write Injection

This engineering dossier documents the **Matshita `R6-MV4` Optical Disc Drive (ODD) Emulator**, boot sequence verifier, ATAPI/DRV guarded write injection engine, and SD/CD `.FRM` unlock container tools integrated across `panadiga-core`, `panadiga-cli`, `panadiga-mcp`, Tauri IPC, and the desktop UI (`MatshitaEmulatorCard`).

---

## 1. Architecture & Hardware Basis

During `DMR-EH55` initialization (**`Loader ver0.61 [06/03/06]`** on real hardware; cross-referenced with `/BOOT.BIN` `ver0.63`), the host MN103S/AM34 controller interrogates the IDE/ATAPI optical drive (`MATSHITA R6-MV4          `, revision `S211` / `1.00`). Hardware bench captures on `COM10` proved the failure modes simulated deterministically in [`crates/panadiga-core/src/re/matshita_emu/mod.rs`](../../crates/panadiga-core/src/re/matshita_emu/mod.rs):

| Scenario Identifier | Physical / Optical Condition | Observed Log & Sense Code | Emulator Outcome |
| :--- | :--- | :--- | :--- |
| `healthy_with_disc` | Tray closed, standard disc TOC mounted | `INQUIRY` + `TEST UNIT READY` + `START STOP UNIT` pass | `startup_succeeded = true`, `0x00000000` |
| `unlock_disc_frm` | Disc contains valid `PANA_DVD.FRM` at virtual LBA 16 | Verified optical unlock container | `unlock_media_verified = true`, `jump_target_va_hex = 0x50400000` |
| `tray_cannot_close_80045300` | Disconnected 4-pin tray/switch ribbon cable | `** can not close tray **`, Sense `04/53/00`, `80045300` | `startup_succeeded = false`, `0x80045300` |
| `laser_apc_failure_80040904` | Aged diode (`< 0.5 mW` APC fault / `0x83A7` countdown) | `drive start error(80040904)`, Sense `04/09/04` | `startup_succeeded = false`, `0x80040904` |

---

## 2. ATAPI CDB Dispatch & Guarded Write Injection

Disassembly of `EH55_DRV_S211` and `DRV_decomp` established both standard MMC/ATAPI commands and Matsushita vendor commands:

- **`0x00` `TEST UNIT READY`** (`0x40014C00`) — tray closure (`can not close tray`) and media presence (`Medium Not Present`).
- **`0x12` `INQUIRY`** (`0x40014B00`) — returns `"MATSHITA R6-MV4          "`.
- **`0x1B` `START STOP UNIT`** (`0x44009CEA`) — motor spin-up & APC laser calibration (`80040904` / `80045300`).
- **`0x1E` `PREVENT ALLOW MEDIUM REMOVAL`** (`0x40015000`), **`0x28` `READ(10)`**, **`0x2A` `WRITE(10)`**, **`0x43` `READ TOC/PMA/ATIP`**.
- **`0x3B` `WRITE BUFFER`** (`0x4002168D`) & **`0x3C` `READ BUFFER`** (`0x40021476`) — buffer injection (mode `2` data, mode `5` microcode) and SHA-256 readback verification.
- **`0xED` Vendor Write/Diag** (`FUN_400209f3` @ `0x400209F3`) — two-step Matsushita staging (`0x91` stages target VA into SRAM `0x0242` mode, `0x93` writes payload).
- **Vendor `0xFD`** (`0x40020147`) and **Vendor `0xFE`** (`0x4002025E` — `MMODE`/`SLOG`/`PASS`).

### Guarded Read/Write & Hardware IPL Lockout (`E05`)

Every ATAPI `0xED` target VA and every DRV serial write (`WB`, `WW`, `WRITE`, `WTLONG`) validates the target address before committing bytes:

- **Permitted writable apertures**: MN103S SDRAM (`0x51800000` workspace / RAM windows), internal SRAM (`sram_0cd8_auth`, `sram_0d78_s211`, `sram_0242_ed_mode`), and writable top-boot NVRAM (`0x807F0000..=0x807FFFFF`).
- **Strictly read-only (rejected with `E05`)**: Spansion S29JL064H Bootloader IPL NOR Flash (`0x80000000..=0x807EFFFF` cached and `0x84000000..=0x847EFFFF` uncached mirror).

---

## 3. Emulated `DRV` Factory Serial CLI

The emulated `DRV` monitor models the 20-byte `<8sIII>` dispatch table (`0x400DF5C0` in `DRV_decomp`) and SRAM state flags:

- `PASS 0x4d415453 0x48495441` (`"MATSHITA"`, handler `0x4013420A` in `DRV_decomp` / `0x40130EFA` in `EH55_DRV_S211`): sets `sram_0cd8_auth = 1`, unlocking protected commands.
- `MMODE` (`0x40134382`), `SLOGON` (`0x40133ED0`), `SLOGOFF` (`0x40133EDE`), `RD`/`RB`/`RW`, and guarded `WB`/`WW` (`WW 0x51800000 0xDEADBEEF`).

---

## 4. SD & CD Unlock `.FRM` Containers

- **SD Card Stage-1 (`DIGADIAG.FRM`)**: Must reside at root of a **FAT16** card (`> 0xFFF5` clusters rejected by `FUN_4400c696`), formatted as **`262,144`-byte (`0x40000`) raw uncompressed** (`0x90400000 -> JMP 0x50400000`) for real DMR-EH55 **`Loader ver0.61`**, or **`131,072`-byte (`0x20000`) `EXTRHEAD`** for **`Loader ver0.63`** (`/BOOT.BIN`).
- **SD (`PANA__SD.FRM`)** and **CD/DVD Optical (`PANA_DVD.FRM`)**: `sd48` (`ver0.61`) / `sd64` (`ver0.63`) / `MATSHITA` optical containers verified via `verify_unlock_frm_container` and synthesized via `build_sample_unlock_frm`.

---

## 5. Multi-Surface Entry Points

| Layer | Command / Symbol |
| :--- | :--- |
| **CLI (`panadiga-cli`)** | `panadiga unbrick matshita-emu boot --scenario unlock_disc_frm` |
| **CLI Inject / Serial** | `panadiga unbrick matshita-emu inject-write --vendor-ed --va 0x51800000 --payload-hex DEADBEEF` · `serial-cmd --unlock-pass "WW 0x51800000 0xDEADBEEF"` |
| **CLI `.FRM` Unlock** | `panadiga unbrick matshita-emu verify-frm --kind digadiag DIGADIAG.FRM` · `pack-unlock-frm --kind sd -o out/DIGADIAG.FRM` |
| **MCP (`panadiga-mcp`)** | `panadiga_sd_unbrick_kit` with `"action": "matshita_emu"` and `"matshita_op": "boot" \| "inject_write" \| "serial_cmd" \| "verify_frm" \| "pack_unlock_frm"` |
| **Tauri IPC & Desktop UI** | `run_matshita_boot_emu`, `inject_matshita_atapi_write`, `run_matshita_serial_cmd`, `verify_frm_unlock_command`, `pack_unlock_frm_command` -> **Unbrick Center (`MatshitaEmulatorCard`)** |
