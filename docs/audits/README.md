> 🌐 **Language / Idioma:** **Bilingual (EN / PT-BR)**
> 🧭 **Portal:** [Docs Index](../INDEX.md) | [Complete Audit (EN)](../../COMPLETE_AUDIT.md) | [Auditoria Completa (PT-BR)](../../AUDITORIA_COMPLETA.md) | [Ghidra Pseudocode](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Debug Logs](../debug_logs/README.md)

# Histórico de Auditorias / Audit History Archive — Panadiga

**PT-BR:** Este diretório preserva as auditorias técnicas originais de **2026-11-11**, produzidas antes da decomposição modular do repositório em `crates/`, `firmware/pico-p7501/`, `apps/desktop/` e `scripts/`.
**EN-US:** This directory preserves the historical **2026-11-11** engineering audits written prior to the modular workspace split into `crates/`, `firmware/pico-p7501/`, `apps/desktop/`, and `scripts/`.

> [!NOTE]
> **PT-BR:** Para o retrato atualizado do workspace após a refatoração modular, os upgrades do firmware C11 (`DIAGWATCH`, `PANARSP REGS`, `DRVPANA`, `SDSNIFF`), o diagnóstico dos trechos `??` no Ghidra e a matriz de paridade Python $\rightarrow$ Rust, consulte [**`AUDITORIA_COMPLETA.md`**](../../AUDITORIA_COMPLETA.md).
> **EN-US:** For the current post-refactor workspace audit covering C11 firmware upgrades, Ghidra `??` undefined-byte diagnostics, and Python $\rightarrow$ Rust parity, read [**`COMPLETE_AUDIT.md`**](../../COMPLETE_AUDIT.md).

## Snapshots Preservados / Preserved Snapshots (`2026-11-11`)

| Documento / Document | Escopo Histórico (`2026-11-11`) / Historical Scope | Estado Atual / Current Workspace Status |
| :--- | :--- | :--- |
| [**`AUDITORIA_COMPLETA_2026-11-11.md`**](./AUDITORIA_COMPLETA_2026-11-11.md) | Auditoria geral pré-modularização (Rust, Python, Pico, Tauri) / Pre-split full workspace audit | Consolidado e atualizado / Consolidated & updated in [`AUDITORIA_COMPLETA.md`](../../AUDITORIA_COMPLETA.md) & [`COMPLETE_AUDIT.md`](../../COMPLETE_AUDIT.md) |
| [**`AUDITORIA_FIRMWARE_PICO_E_PIPELINE_2026-11-11.md`**](./AUDITORIA_FIRMWARE_PICO_E_PIPELINE_2026-11-11.md) | Auditoria de firmware C11, UF2 e pipeline de build / C11 firmware & UF2 build pipeline audit | Refatorado em [`firmware/pico-p7501/`](../../firmware/pico-p7501/p7501_pico_main.c) & [`scripts/build_lanes/pico.py`](../../scripts/build_lanes/pico.py) |
| [**`AUDITORIA_RUNTIME_PANADIGA_2026-11-11.md`**](./AUDITORIA_RUNTIME_PANADIGA_2026-11-11.md) | Auditoria de runtime serial, parser Tauri e segurança elétrica / Serial runtime & electrical safety audit | Endurecido / Hardened via `diagnostics/parser.rs`, tiered timeouts & C11 host tests |
