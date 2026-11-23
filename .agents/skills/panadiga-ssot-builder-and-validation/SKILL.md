---
name: panadiga-ssot-builder-and-validation
description: >-
  Construção, evolução e validação fail-closed da SSOT declarativa em JSON do Panadiga
  (profiles/, panadiga-profiles, build.rs compile-time gate, 12 JSON Schemas Draft 2020-12,
  panadiga ssot export-all, SSOT_CONTRACT_HASH, tabelas BGO e catálogo de armadilhas).
  Use ao criar novos mapas JSON SSOT, regras de validação em panadiga-profiles ou campos
  compartilhados entre Rust, TypeScript, Python e C11.
---

# Construção & Validação Fail-Closed da SSOT Declarativa (`profiles/` + `panadiga-profiles`)

Esta skill governa a criação e extensão da **Single Source of Truth (SSOT)** relacional de hardware, firmware e memória em `profiles/`, garantindo que toda regra física seja checada em **compile-time**, CLI e testes Python/Rust sem silêncios nem duplicações.

## 1. Quando Usar & Handoffs
- Ative ao adicionar novas coleções/mapas JSON em `profiles/` (`nor_layouts/`, `firmware_trains/`, `flash_chips/`, `traps/`, `forensic_atlas/`, `symbols/`), alterar structs/validators em `crates/panadiga-profiles/src/` ou regenerar projeções cross-stack via `panadiga ssot export-all`.
- Para onboarding de modelos DIGA (tupla obrigatória e portões `T0..T4`), use `.agents/skills/panadiga-device-onboarding/SKILL.md`.
- Para checagem específica de armadilhas BGO `4 MiB` vs `8 MiB` e binários ao vivo, combine com `.agents/skills/panadiga-anti-trap-geometry-verification/SKILL.md`.
- Para compliance de contratos IR (`SSOT_CONTRACT_HASH`) entre camadas, use `.agents/skills/panadiga-code-compliance-and-layer-coherence/SKILL.md` e `.agents/skills/panadiga-contracts-stream-and-journal/SKILL.md`.
- Detalhes da arquitetura de schemas, invariantes e evolução segura em [ssot-schema-validation-and-evolution.md](./references/ssot-schema-validation-and-evolution.md).

## 2. Invariantes Invioláveis da SSOT Panadiga
1. **Compile-Time Bundle Gate (`crates/panadiga-core/build.rs`):** Toda alteração em `profiles/**/*.json` ou em `crates/panadiga-profiles/` é validada durante o build de `panadiga-core` (`load_bundle_from_dir` + `verify_schemas_match_dir`); se houver deriva de schema ou invariante física quebrada, o compilador aborta antes de gerar binário.
2. **Coerência Matemática & Anti-Armadilha (`validate_bgo.rs` + `validate_layout.rs`):**
   - `logic_voltage_mv == 3300`, `p7501_nsw5v_hiz_only == true` e `0xC000100C` na lista letal dos SoCs `MN103`.
   - Somas e alinhamento exato de setores contra `derive_sector_table(flash)`; IPL inicial `>= 256 KiB` (`read_only_permanent`) e região `Bgo` disjunta do IPL e `Eemul`.
   - **Proibição de deriva 4 MiB vs 8 MiB:** `bgo_retarget_spec` / `digadiag_spec.bgo_geometry_target` (`_4MB_`/`_8MB_`/`_16MB_` + base `0x...`) devem bater exatamente com `flash.total_bytes`, `bgo_table_words[12..13, 31]` e o span `Bgo` em `nor_layouts/`.
3. **Sem Tokens Obsoletos:** Nunca grave intervalos pré-calibração de setores (ex.: ranges inválidos de IPL/Upper/NVRAM filtrados por `STALE_SA_PATTERN` em `scripts/tests/test_profiles_ssot.py`) nem o hash revogado em qualquer JSON ou documento ativo. Use sempre `SA0..SA10` (IPL RO), `SA11..SA133` (Upper NOR, BGO `SA26..SA70`) e `SA134..SA141` (NVRAM/EEMUL `64 KiB`).
4. **Evidence Tiering (`T0..T4`):** Nenhum dado de manual (`T1`) ou `.FRM` (`T2`) pode habilitar escrita BGO live ou sobrescrever entidades `T3`/`T4` sem auditoria física.

## 3. Pipeline Canônico para Adicionar ou Evoluir SSOT
1. **Modelagem Tipada em `crates/panadiga-profiles/src/types/`:** Use `#[serde(deny_unknown_fields)]`, serialização hex tipada (`hex_serde`) e `#[derive(JsonSchema)]`.
2. **Validação Cruzada em `validate*.rs`:** Toda relação matemática (ex.: `word[31] * 64KiB`, capacidade `4MB` vs `8MB`, tamanho `DIGADIAG` raw vs `EXTRHEAD`) deve ganhar check explícito + teste negativo em `crates/panadiga-profiles/src/tests.rs`.
3. **Exportação de Schemas & Código Derivado:**
   ```bash
   cargo run -q -p panadiga-cli -- profiles export-schema
   cargo run -q -p panadiga-cli -- ssot export-all --root .
   ```
4. **Atualização dos Mapas Auxiliares & Atlas:** Execute `python scripts/verify_geometry_traps.py --check`, `python scripts/generate_firmware_atlas.py --write` e `python scripts/generate_tool_arsenal.py --write`.

## 4. Verificação Obrigatória
```bash
cargo test -p panadiga-profiles -p panadiga-core
cargo run -q -p panadiga-cli -- profiles validate
python -m pytest scripts/tests/test_profiles_ssot.py scripts/tests/test_geometry_traps.py
```
