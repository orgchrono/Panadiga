---
name: panadiga-security-and-violation-guards
description: >-
  Guarda fail-closed contra violações em 5 domínios: segurança elétrica/silício (<=3.3V,
  GP3 NSW5V Hi-Z, XRESET Open-Drain, SMPS/ADuM, EH58/EH59 +5.8V_AV, 0xC000_100C), proteção
  Flash/HDD (IPL SA0..SA10 E05, CONFIRM_UPPER_NOR, CapabilityGate C0..C5, lockouts de disco),
  integridade forense/epistêmica (quarentena PDF, N1..N5, T0..T4), AppSec/código (unsafe,
  any, traversal, Android-only leak) e protocolo Stop-Block-Report. Use antes de operações
  arriscadas ou como auditoria de conformidade.
---

# Guarda Fail-Closed Contra Violações (Silício, Forense, Epistemologia & AppSec)

Esta skill atua como barreira não negociável contra ações capazes de destruir hardware, corromper provas forenses, envenenar a SSOT ou abrir brechas de segurança no código.

## 1. Quando Usar & Handoffs
- Suba na **Fase 0** de qualquer operação em bancada, gravação de Flash/HDD/SD/DVD, promoção `T0..T4` / `N1..N5`, extração de PDFs ou alteração de código crítico.
- Para auditorias adversariais formais e templates de prompts red-team, combine com `.agents/skills/panadiga-adversarial-and-hallucination-auditor/SKILL.md`.
- Catálogo completo das 5 classes de violação e comandos de verificação em [violation-taxonomy-and-controls.md](./references/violation-taxonomy-and-controls.md).

## 2. Protocolo Obrigatório `STOP -> BLOCK -> REPORT -> REMEDIATE`
Se **qualquer** pré-condição abaixo falhar ou for contradita por medição/ferramenta:
1. **PARE** imediatamente (não improvise fallback inseguro nem tente desativar travas sem confirmação explícita).
2. **BLOQUEIE** o comando mutante/escrita.
3. **REPORTE** o risco exato, a evidência medida e o caminho oficial sancionado.
4. **REMEDIE** apenas após cumprir a pré-condição (ex.: backup SHA-256, `ADuM`, token `CONFIRM-*`, revisão humana de proposta PDF).

## 3. Resumo das 5 Barreiras Invioláveis
1. **Silício & Eletricidade:** `NSW5V` (`GP3`) sempre `Hi-Z` sem pulls; `XRESET` (`GP5`) Open-Drain; nunca alimentar Pico por `P7501`; isolamento galvânico SMPS (`ADuM3160`/transformador isolador); checar `p7501_header_present` (`EH58/EH59 P7501` = `+5.8V_AV`!); proibido tocar `0xC000_100C`.
2. **Flash, HDD & Mídias Físicas:** `SA0..SA10` (`256 KiB` IPL) permanente `E05` (e `CapabilityGate C5` sempre negado mesmo com `LabOverride`); Upper NOR `SA11..SA133` só ao vivo com `--backup-dir` + `--confirm-upper-nor` (`U CONFIRM_UPPER_NOR`) + `T4`; HDD/SD só com lockout multi-SO intacto (`PhysicalDrive0`/NVMe/`/dev/sda*`/`/dev/disk0*` bloqueados), `--backup-dir` e `--confirm-*`.
3. **Forense & Quarentena SSOT:** Jamais alterar dumps originais *in-place*; `manual_spec_extractor.py` grava apenas em `profiles/staging/manual_proposals/` (`requires_human_review: true`); `nvram clean`/kit `< T3` falha fechado sem `LabOverride` (`I_ACCEPT_UNVERIFIED_HARDWARE_RISK` só offline CoW) ou `--allow-unverified-profile`.
4. **Epistemologia & Legalidade:** Proibido afirmar `[PROVADO EM HARDWARE REAL]` sem entrada `N4_HARDWARE`/`N5_REPRODUCED` no `ledger.json` (`test_docs_parity.py`); `Mn103Vm` = `N2_EMULATED`; `PANAMON2` = protótipo experimental de interoperabilidade (não clean-room); zero tokens banidos em `.agents/` / `profiles/`.
5. **Código & AppSec:** `#![deny(unsafe_code)]`; zero `println!` em MCP/core; zero `any`/`as Type`/`console.*`/`eval`/`no-danger`; proteção contra path traversal (`validate_safe_segment`, `resources.rs`); zero vazamento de código `--mode android` (`IrCommanderView`/`__PANADIGA_IR_BRIDGE__`) no build desktop.

## 4. Comandos de Verificação de Segurança
```bash
# Checagem de autoridade e política T0..T4 (sai != 0 se negado)
cargo run -q -p panadiga-cli -- profiles authorize DMR-EH55 --capability nvram_write_live --dump-size 8388608 --wp-deasserted true -j -
cargo run -q -p panadiga-cli -- profiles validate -j -
cargo run -q -p panadiga-cli -- pico validate-bench-cmd -c "STATUS" --json
cargo run -q -p panadiga-cli -- pico rsp-codec --kind is-ipl-readonly --addr 0x80000000 --json
```
- **MCP:** `panadiga_safety_check`, `panadiga_profiles_authorize`.

## 5. Armadilhas & Verificação
- **Armadilha:** Tratar `LabOverride` (`--lab-override-unverified`) como liberação para hardware ao vivo — ele destrava **exclusivamente CoW offline**; escrita ao vivo continua exigindo `T4` (`board>=T4`).
- **Verificação:** Qualquer operação destrutiva exige resposta explícita `allowed: true` de `profiles authorize` (ou confirmação manual documentada em `T4`), backup SHA-256 gravado e zero alertas de `panadiga_safety_check`.
