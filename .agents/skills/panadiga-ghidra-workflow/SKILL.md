---
name: panadiga-ghidra-workflow
description: >-
  Orquestração Ghidra multi-ISA (MN103S/AM34, SPARC V8, ARMv7, AArch64, MIPS), geração de
  scripts via SSOT Rust (panadiga ghidra), auditoria estática (panadiga re), correção das
  10 Armadilhas MN103S, aplicação do patch SLEIGH de 12 defeitos em projetos clonados e
  exportação de pseudocódigo em seções executáveis alcançáveis. Use ao fazer engenharia
  reversa estática, atualizar símbolos ou auditar o pipeline Ghidra.
---

# Fluxo Ghidra Multi-ISA & Auditoria Estática (`AM34 / MN103S`)

Esta skill governa a engenharia reversa estática em Ghidra Headless/GUI e nos analisadores nativos Rust (`panadiga re`), unificando a SSOT `profiles/` com o patch SLEIGH do `GhidraMn10300`.

## 1. Quando Usar & Handoffs
- Ative ao iniciar análises em Ghidra, resolver `??` / decompilação quebrada no `MN103S`, exportar mapas/símbolos ou varrer frotas `.FRM`/`MAIN`/`PROG`/`DRV`.
- Para transformar achados em patches CoW e validá-los em `Mn103Vm` ou GDB, use `.agents/skills/panadiga-emulation-and-patching/SKILL.md`.
- Para extrair binários a partir de `.FRM`, use `.agents/skills/panadiga-firmware-decrypto/SKILL.md`.
- Catálogo completo das **10 Armadilhas MN103S**, **12 defeitos SLEIGH**, errata de 3 vias e receitas detalhadas em [mn103s-traps-and-sleigh.md](./references/mn103s-traps-and-sleigh.md).

## 2. Invariantes Não-Negociáveis
1. **Cobertura sobre Seções Alcançáveis (NUNCA `0 ??` Cego em Novos Alvos):** Meça cobertura exclusivamente sobre seções executáveis delimitadas e funções alcançáveis a partir de vetores e ponteiros verificados. Jamais force disassembly em *literal pools*, *jump tables*, strings, tabelas TBC ou padding `0xFF` — isso cria *xrefs* falsos para MMIO.
2. **Clones Isolados & Java Read-Only:** Nunca modifique o projeto original `Panadiga_MN103S`; rode `ghidra_zero_undef_pipeline.py` apenas em clones (`Panadiga_MN103S_ZeroUndef` / `Fleet_*`) e execute analistas Java headless com `-noanalysis -readOnly` (jamais `-save`).
3. **SSOT Unidirecional:** Os scripts `tools/ghidra/` são gerados por `panadiga ssot export-all` e `panadiga ghidra export-all`; não faça fork manual de tabelas de memória.
4. **Higiene de Correspondência:** Jamais misture offsets de `Loader ver0.63` (`/BOOT.BIN`) com `Loader ver0.61` (`eh55_loader_ver061_256k.bin`) ou `PROG061`.

## 3. Workflow Canônico (Rust CLI, PyGhidra & MCP)

### A. Sincronia SSOT & Analisadores Estáticos Nativos (Sem JVM)
```bash
# 1. Regenerar mapas JSON SSOT e scripts Jython em tools/ghidra/
cargo run -q -p panadiga-cli -- ghidra export-all --dir tools/ghidra

# 2. Auditoria de Armadilhas, cross-references e tabelas DRV/ATAPI em Rust
cargo run -q -p panadiga-cli -- re audit -i <BIN> --base 0x44000000 --json
cargo run -q -p panadiga-cli -- re xrefs -i <BIN> -t <HEX_VA> --base 0x44000000 --json
cargo run -q -p panadiga-cli -- re drv-cli -i <DRV_DECOMP> --json
cargo run -q -p panadiga-cli -- re drv-errors -i <DRV_DECOMP> --json
# Primitivas granulares JSON: re mn103 | re traps | re pana-frm | re bench-runtime
```

### B. Orquestração Multi-ISA & Exportação de Pseudocódigo
```bash
# 1. Checar deriva contra fleet_verify.json (< 1s, sem abrir JVM)
python -m scripts.re.ghidra_fleet_orchestrator --catalog tmp/decrypted_fleet/fleet_decrypted_catalog.json --check-drift --json -

# 2. Analisar frota (MN10300, SPARC V8, ARMv7, AArch64, MIPS) com preflight *.ldefs e Java TrapAudit
python -m scripts.re.ghidra_fleet_orchestrator --catalog tmp/decrypted_fleet/fleet_decrypted_catalog.json --out-c-dir tmp/pseudocode_full --shard small --workers 6 --java-audit --assemble

# 3. Smoke test do patch SLEIGH (micro-fixture Traps #1..#8) e exportação de bundles de pseudocódigo
python scripts/validate.py --ghidra-smoke
python -m tools.ghidra.ghidra_export_pseudocode_bundle --model DMR-EH55 --raw-out tmp/eh55_pseudo.txt --map-out tmp/eh55_maps.json
```
- **MCP:** Use `panadiga_ghidra_symbols`, `panadiga_memory_map`, `panadiga_search_code` e bridge `ghidra-mcp`.
- **GUI Windows:** `tools/iniciar_ghidra.cmd` espelha scripts para `%USERPROFILE%\ghidra_scripts\` e abre o Ghidra; ordem na GUI: `ghidra_am34_memory_map.py` -> `ghidra_import_symbols_panadiga.py` -> `ghidra_mn103_fix_decompilation.py` -> `ghidra_auto_unbrick_audit.py`.

## 4. Armadilhas & Anti-Padrões
- Confiar em decompilação stock do `GhidraMn10300` sem o patch `am33_panadiga_zero_undef.patch`: `SUB fb 9c` sai invertido (`src - dst`), `BCLR` vira no-op, `NOT f2 3x` zera quando `Dm == Dn`, e `0xF9 0xEA` gera milhares de `??`.
- Tratar `+5B` mid-instruction overlaps (`0x44008020` em `ver0.61`, `0x504238a5` em `PROG061`) como início de função.

## 5. Daqui pra Frente & Reuso em Novos Projetos
- **Em novos SoCs/ISAs:** Cadastre `ghidra_language_id` na SSOT `profiles/soc_cores/`, aplique `ghidra_fleet_orchestrator.py` (preflight `*.ldefs` + fallback ELF `e_machine`) e versionamento de patches SLEIGH em `tools/ghidra/sleigh_patches/` testados contra micro-fixtures sintéticas.

## 6. Verificação
- `cargo run -q -p panadiga-cli -- ghidra export-all --dir tools/ghidra` sem diffs sujos inesperados; `python -m scripts.re.ghidra_fleet_orchestrator --check-drift` limpo; quando `PANADIGA_GHIDRA_PATCHED` estiver presente, `python scripts/validate.py --ghidra-smoke` passando.
