> 🌐 **Idioma / Language:** [English (US)](../en-US/MATSHITA_R6MV4_EMULATOR_AND_ATAPI_INJECTION.md) | **Português (BR)**
> 🧭 **Portal:** [Índice da Documentação](../INDEX.md) | [Auditoria Completa](../../AUDITORIA_COMPLETA.md) | [Auditorias](../audits/README.md) | [Pseudocódigo Ghidra](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Logs de Debug](../debug_logs/README.md)

# Emulador Matshita R6-MV4, Verificação de Boot e Injeção ATAPI (`0x3B` / `0xED`)

Este dossiê documenta o **Emulador do Drive Óptico Matshita `R6-MV4`**, o verificador da sequência de boot (`/BOOT.BIN`), o motor de injeção de gravação protegida via ATAPI e CLI serial `DRV`, e os empacotadores/verificadores de `.FRM` de unlock para **SD** e **CD/DVD** disponíveis em `panadiga-core`, `panadiga-cli`, `panadiga-mcp`, IPC Tauri e na interface desktop (`MatshitaEmulatorCard`).

---

## 1. Base Empírica em Bancada e Cenários de Boot

Durante o boot do `DMR-EH55` (**`Loader ver0.61 [06/03/06]`** no aparelho físico; comparado também com `/BOOT.BIN` `ver0.63`), a CPU MN103S/AM34 autentica o drive óptico (`MATSHITA R6-MV4          `, revisão `S211` / `1.00`). Os logs empíricos na `COM10` deram origem a quatro cenários determinísticos em [`crates/panadiga-core/src/re/matshita_emu/mod.rs`](../../crates/panadiga-core/src/re/matshita_emu/mod.rs):

| Cenário (`scenario`) | Condição Física Simulada | Log e Sense ATAPI | Resultado no Emulador |
| :--- | :--- | :--- | :--- |
| `healthy_with_disc` | Bandeja fechada, disco válido montado | `INQUIRY` + `TEST UNIT READY` + `START STOP UNIT` aprovados | `startup_succeeded = true`, `0x00000000` |
| `unlock_disc_frm` | Disco com `PANA_DVD.FRM` válido em LBA 16 virtual | Container óptico validado pelo loader | `unlock_media_verified = true`, `jump_target_va_hex = 0x50400000` |
| `tray_cannot_close_80045300` | Cabo flat de 4 pinos desconectado / micro-switch aberto | `** can not close tray **`, Sense `04/53/00`, `80045300` | `startup_succeeded = false`, `0x80045300` |
| `laser_apc_failure_80040904` | Falha de calibração APC (`< 0,5 mW` / erro mecha `0x83A7`) | `drive start error(80040904)`, Sense `04/09/04` | `startup_succeeded = false`, `0x80040904` |

---

## 2. Tabela de CDB ATAPI e Injeção de Gravação Protegida

A engenharia reversa de `EH55_DRV_S211` e `DRV_decomp` mapeou tanto os comandos MMC/ATAPI quanto os opcodes proprietários Matsushita:

- **`0x00` `TEST UNIT READY`** (`0x40014C00`) — fechamento de bandeja (`can not close tray`) e detecção de mídia (`Medium Not Present`).
- **`0x12` `INQUIRY`** (`0x40014B00`) — identifica `"MATSHITA R6-MV4          "`.
- **`0x1B` `START STOP UNIT`** (`0x44009CEA`) — acionamento do spindle e calibração APC (`80040904` / `80045300`).
- **`0x1E` `PREVENT ALLOW MEDIUM REMOVAL`** (`0x40015000`), **`0x28` `READ(10)`**, **`0x2A` `WRITE(10)`**, **`0x43` `READ TOC/PMA/ATIP`**.
- **`0x3B` `WRITE BUFFER`** (`0x4002168D`) e **`0x3C` `READ BUFFER`** (`0x40021476`) — gravação de buffer (modo `2` dados, modo `5` microcódigo) com conferência SHA-256 no readback.
- **`0xED` Vendor Write/Diag** (`FUN_400209f3` @ `0x400209F3`) — preparação em 2 estágios Matsushita (`0x91` grava VA alvo ativando `sram_0242_ed_mode`, `0x93` injeta o payload).
- **Vendor `0xFD`** (`0x40020147`) e **Vendor `0xFE`** (`0x4002025E` — `MMODE`/`SLOG`/`PASS`).

### Trava Hardware de Proteção IPL (`E05`)

Toda gravação via `0xED` (`va`) ou via CLI serial (`WB`, `WW`, `WRITE`, `WTLONG`) passa pela validação de faixa:

- **Regiões de escrita permitida**: SDRAM MN103S (ex.: `0x51800000`), SRAM interna do drive (`sram_0cd8_auth`, `sram_0d78_s211`, `sram_0242_ed_mode`) e microsetores NVRAM no topo da NOR (`0x807F0000..=0x807FFFFF`).
- **Somente leitura estrita (bloqueio `E05`)**: Bootloader IPL Spansion S29JL064H (`0x80000000..=0x807EFFFF` cacheado e espelho não-cacheado `0x84000000..=0x847EFFFF`).

---

## 3. CLI Serial Emulada (`DRV`)

O interpretador emulado reproduz a tabela de despacho `<8sIII>` (`0x400DF5C0` em `DRV_decomp`):

- `PASS 0x4d415453 0x48495441` (`"MATSHITA"`, handler `0x4013420A` em `DRV_decomp` / `0x40130EFA` em `EH55_DRV_S211`): habilita `sram_0cd8_auth = 1`, desbloqueando comandos protegidos.
- `MMODE` (`0x40134382`), `SLOGON` (`0x40133ED0`), `SLOGOFF` (`0x40133EDE`), leitura `RD`/`RB`/`RW` e escrita protegida `WB`/`WW` (`WW 0x51800000 0xDEADBEEF`).

---

## 4. Containers de Unlock `.FRM` para SD e CD/DVD

- **SD Stage-1 (`DIGADIAG.FRM`)**: Deve estar na raiz de cartão **FAT16** (`FUN_4400c696` rejeita `> 0xFFF5` clusters), com **`262.144` bytes (`0x40000`) brutos descomprimidos** para **`Loader ver0.61`** real do DMR-EH55 (`0x90400000 -> JMP 0x50400000`), ou **`131.072` bytes (`0x20000`) `EXTRHEAD`** para **`Loader ver0.63`** (`/BOOT.BIN` EZ485V/EH58).
- **SD (`PANA__SD.FRM`)** e **CD/DVD (`PANA_DVD.FRM`)**: Containers (`sd48` para `ver0.61`, `sd64` para `ver0.63`, Feistel para `DVD`) validados por `verify_unlock_frm_container` e sintetizados por `build_sample_unlock_frm`.

---

## 5. Pontos de Entrada no Ecossistema Panadiga

| Camada | Comando / API |
| :--- | :--- |
| **CLI (`panadiga-cli`)** | `panadiga unbrick matshita-emu boot --scenario unlock_disc_frm` |
| **CLI Injeção & Serial** | `panadiga unbrick matshita-emu inject-write --vendor-ed --va 0x51800000 --payload-hex DEADBEEF` · `serial-cmd --unlock-pass "WW 0x51800000 0xDEADBEEF"` |
| **CLI Unlock `.FRM`** | `panadiga unbrick matshita-emu verify-frm --kind digadiag DIGADIAG.FRM` · `pack-unlock-frm --kind sd -o out/DIGADIAG.FRM` |
| **MCP (`panadiga-mcp`)** | `panadiga_sd_unbrick_kit` com `"action": "matshita_emu"` e `"matshita_op": "boot" \| "inject_write" \| "serial_cmd" \| "verify_frm" \| "pack_unlock_frm"` |
| **Tauri IPC & UI Desktop** | `run_matshita_boot_emu`, `inject_matshita_atapi_write`, `run_matshita_serial_cmd`, `verify_frm_unlock_command`, `pack_unlock_frm_command` -> **Centro Unbrick (`MatshitaEmulatorCard`)** |
