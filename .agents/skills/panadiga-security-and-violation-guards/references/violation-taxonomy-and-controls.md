# Taxonomia Completa de Violações & Controles Fail-Closed

## 1. Violações Elétricas e de Hardware ao Vivo
- **Risco:** Queima do SoC `AM34`, dano ao Pico ou choque/ground-loop via SMPS.
- **Controles:**
  1. Transformador isolador AC/AC ou isolador USB `ADuM3160` / `ADuM4160` antes de ligar a sonda (`~60–110 VAC` capacitores Y); descarga de capacitores primários (`~170–340 VDC`).
  2. Conferência explícita de tensão lógica (`3.3V`/`1.8V`/`5V`) e pino `p7501_header_present` (`DMR-EH58/EH59/EH68 VEP79183` têm `+5.8V_AV` em `P7501`!).
  3. No `DMR-EH55 P7501`: `GP3` (`NSW5V`) SEMPRE `Hi-Z Input` sem pulls; `GP5` (`XRESET`) Open-Drain; nunca alimentar Pico pelo Pino 1 (`SW_UART < 10 mA`); proibido ler/escrever `0xC000_100C`.
  4. Em C11/Python/Rust: `RESERVED_P7501_MAX_GPIO = 5` (rejeita `GP0..GP5` como I/O de usuário), `validate_safe_bench_command` (`BenchSafetyError`), `panadiga_safety_check`.

## 2. Violações de Escrita em Flash NOR & Storage Físico
- **Flash Spansion `S29JL064H` (`IC5001` 8 MiB):**
  - `SA0..SA10` (`256 KiB` IPL): **bloqueio `E05` permanente** na sonda e **`CapabilityGate::C5`** sempre negada (inclusive com `LabOverride`).
  - `SA11..SA133` (Upper NOR/BGO `SA26..SA70`): bloqueado por padrão (`E05`); exige `T4`, `--backup-dir` prévio SHA-256 e `--confirm-upper-nor` (`U CONFIRM_UPPER_NOR`) com `U LOCK` RAII.
  - `nvram clean` / `rebuild-from-dump` / `prepare-sd`: bloqueia perfis `< T3` sem `--allow-unverified-profile` / `LabOverride` (`I_ACCEPT_UNVERIFIED_HARDWARE_RISK`, que libera **exclusivamente CoW offline**).
- **Discos Físicos (`hdd` / `sd` / `dvd`):**
  - Hard-block de `PhysicalDrive0`, discos `BusTypeNvme (17)`, discos raiz Linux (`/proc/mounts` + `/dev/mapper`, `/dev/sda*`), `/dev/disk0*`/`/dev/rdisk0*` no macOS.
  - Exige `--backup-dir` CoW + token `CONFIRM-<TARGET>` + `Mn103Vm` pre-write gate (`flash-composed`/`sd create`/`dvd create`) + readback SHA-256.

## 3. Violações de Integridade Forense, Quarentena & Epistemologia
- **Mutação In-Place:** Nunca modificar dumps `.bin`/`.img`/`.FRM` originais; bucket `raw/` e `frm/` do Vault falham fechado em colisão de hash para mesmo arquivo; bucket `recompiled/` falha fechado sem `PreWriteGateReport.passed == true`.
- **Quarentena PDF (`manual_spec_extractor.py`):** `direct_profile_writes == 0`; escreve apenas em `profiles/staging/manual_proposals/` com `requires_human_review: true` e `page_citations`.
- **Epistemologia:** Não declarar `[PROVADO EM HARDWARE REAL]` sem entrada `N4_HARDWARE`/`N5_REPRODUCED` no `docs/evidence/ledger.json`; não vender `Mn103Vm` (`N2_EMULATED`) como prova de silício; não descrever `firmware/panadiga-blackbox-fw` como clean-room; manter `0` tokens banidos (`test_profiles_ssot.py`, `test_docs_parity.py`).

## 4. Violações de Código, AppSec & Paridade
- `#![deny(unsafe_code)]` em módulos não-FFI; `#![cfg_attr(not(test), deny(clippy::print_stdout, clippy::print_stderr))]`.
- Frontend: zero `any`, `as Type`, `eval`, `console.*`, `dangerouslySetInnerHTML`, `eslint-disable`/`@ts-ignore`, ou globals de browser fora da allowlist.
- Path traversal bloqueado por `validate_safe_segment` (Vault) e guarda de diretório em `panadiga-mcp/src/resources.rs`.
- Android-only code (`IrCommanderView`, `panasonic_ir_catalog`, `__PANADIGA_IR_BRIDGE__`) estritamente isolado em `--mode android`.
