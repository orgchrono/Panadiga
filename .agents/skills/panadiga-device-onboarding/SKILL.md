---
name: panadiga-device-onboarding
description: >-
  Construção, evolução e validação fail-closed da SSOT declarativa em JSON (profiles/,
  crates/panadiga-profiles/, build.rs compile-time gate, 12 JSON Schemas Draft 2020-12,
  panadiga ssot export-all, SSOT_CONTRACT_HASH, tabelas BGO e catálogo de armadilhas) e
  pipeline em 7 estágios (T0 -> T4) para onboarding de novos modelos Panasonic DIGA / OEM
  (tupla model-region-board_rev-flash_jedec_id, quarentena PDF em manual_proposals/, allowlist
  BGO, CapabilityGate C0..C5, profiles promote) além do blueprint portátil para novos projetos.
  Use ao criar mapas SSOT, regras em panadiga-profiles ou integrar novos dispositivos.
---

# Construção SSOT Fail-Closed, Onboarding (`T0 -> T4`) & Blueprint para Novos Projetos

Esta skill governa a criação, extensão e validação em tempo de compilação e runtime da **Single Source of Truth (SSOT)** relacional em `profiles/` + `crates/panadiga-profiles/`, o onboarding completo de novos modelos DIGA/OEM sem hardcode e o blueprint portátil para novos projetos.

## 1. Quando Usar & Handoffs
- Ative ao adicionar/evoluir mapas JSON em `profiles/` (`soc_cores/`, `socs/`, `flash_chips/`, `eeprom_chips/`, `firmware_trains/`, `nor_layouts/`, `media_formats/`, `boards/`, `models/`, `traps/`, `forensic_atlas/`, `symbols/`), alterar structs/validators em `crates/panadiga-profiles/src/`, regenerar projeções `panadiga ssot export-all`, integrar novo gravador DIGA/OEM ou promover evidências `T0..T4`.
- Combine com `.agents/skills/panadiga-hardware-forensics/SKILL.md` (Estágio 2), `.agents/skills/panadiga-firmware-decrypto/SKILL.md` (Estágio 3), `.agents/skills/panadiga-ghidra-workflow/SKILL.md` (Estágio 5), `.agents/skills/panadiga-anti-trap-geometry-verification/SKILL.md` (`4 MiB` vs `8 MiB`), `.agents/skills/panadiga-contracts-stream-and-journal/SKILL.md` (`SSOT_CONTRACT_HASH`) e `.agents/skills/panadiga-validate/SKILL.md` (Estágio 7).
- **Referências:**
  - Esquema completo SSOT, tiers `T0..T4`, `CapabilityGate` e **Blueprint Portátil de 10 Princípios** em [ssot-tiers-and-new-project-blueprint.md](./references/ssot-tiers-and-new-project-blueprint.md).
  - Arquitetura de validação fail-closed (`build.rs`, `validate*.rs`), modeling Rust e evolução segura em [ssot-schema-validation-and-evolution.md](./references/ssot-schema-validation-and-evolution.md).

## 2. Invariantes Não-Negociáveis da SSOT & Onboarding
1. **Tupla de 4 Campos & Compile-Time Bundle Gate (`build.rs`):** Identifique hardware por **`(model, region, board_rev, flash_jedec_id)`** (nunca nome comercial solto). Toda alteração em `profiles/**/*.json` ou `crates/panadiga-profiles/` é validada em build-time pelo `crates/panadiga-core/build.rs` (`load_bundle_from_dir` + `verify_schemas_match_dir`).
2. **Coerência Matemática & Anti-Armadilha (`validate_bgo.rs` + `validate_layout.rs`):** `logic_voltage_mv == 3300`, `p7501_nsw5v_hiz_only == true`, `0xC000100C` na lista letal `MN103`, alinhamento exato com `derive_sector_table(flash)`, IPL inicial `>= 256 KiB` (`read_only_permanent`) e `Bgo` disjunto de IPL/Eemul. `bgo_retarget_spec` / `digadiag_spec.bgo_geometry_target` (`_4MB_`/`_8MB_`/`_16MB_` + base `0x...`) devem bater exatamente com `flash.total_bytes`, `bgo_table_words[12..13, 31]` e o span `Bgo`.
3. **Sem Tokens Obsoletos & Quarentena PDF (`direct_profile_writes == 0`):** Nunca grave ranges pré-calibração (`STALE_SA_PATTERN`) ou hashes revogados; use sempre `SA0..SA10` (IPL RO), `SA11..SA133` (Upper NOR, BGO `SA26..SA70`) e `SA134..SA141` (NVRAM/EEMUL `64 KiB`). `manual_spec_extractor.py --apply` grava exclusivamente em `profiles/staging/manual_proposals/*.proposal.json` (`requires_human_review: true`).
4. **Gate Runtime `< T3` & Allowlist BGO:** `prepare-sd`, `rebuild-from-dump` e `retarget` recusam `< T3` sem `--allow-unverified-profile` (`--allow-unverified-mix` para cross-build). Escrita BGO exige `[start..=end] ⊆ nor_layout.regions[bgo]`. Dados `T1`/`T2` nunca habilitam escrita live nem sobrescrevem `T3`/`T4`.

## 3. Pipeline Canônico (`T0 -> T4`) & Evolução Tipada
1. **Modelagem Tipada (`crates/panadiga-profiles/src/types/`):** Use `#[serde(deny_unknown_fields)]`, `hex_serde` e `#[derive(JsonSchema)]`; toda regra matemática nova ganha check em `validate*.rs` + teste negativo em `crates/panadiga-profiles/src/tests.rs`.
```bash
# Estágios 1-4: Triagem NOR -> .FRM Train Harvester -> Rebuild -> Quarentena PDF
cargo run -q -p panadiga-cli -- profiles discover-nor -i tmp/novo_nor.bin --model-id DMR-NOVO --out-dir tmp/drafts -j -
python -m scripts.re.nor_intake_analyzer --offline-only --input tmp/novo_nor.bin --model-id DMR-NOVO --out-dir tmp/drafts
python -m scripts.re.frm_train_harvester --frm <PATH_FRM> --unpack-missing --apply --json -
cargo run -q -p panadiga-cli -- unbrick rebuild-from-dump --model DMR-NOVO --nor-dump tmp/novo_nor.bin -o tmp/sd_kit --allow-unverified-profile -j -
python -m scripts.re.manual_spec_extractor --apply

# Estágios 5-7: Ghidra alcançável -> Export Schemas & SSOT -> Promoção -> Validação & Sync Auxiliar
cargo run -q -p panadiga-cli -- profiles export-schema --out-dir profiles/schema
cargo run -q -p panadiga-cli -- ssot export-all --root .
cargo run -q -p panadiga-cli -- profiles promote --model DMR-NOVO --axis nor_layout --tier T3 --sha256 <64_HEX> --fixture <PATH>
cargo run -q -p panadiga-cli -- profiles completeness -j -
cargo run -q -p panadiga-cli -- profiles validate -j -
python scripts/verify_geometry_traps.py --check
python scripts/generate_firmware_atlas.py --write
python scripts/generate_tool_arsenal.py --write
```
- **MCP:** `panadiga_profiles_authorize`, `panadiga://profiles/<MODEL_ID>`, `panadiga://models-catalog`.

## 4. Armadilhas, Fronteiras Futuras & Verificação
- **Armadilhas:** Promover para `T4` sem `--bench-log` em `docs/debug_logs/`, usar subcomando inexistente em vez de `profiles promote`, ou achar que `LabOverride` offline habilita escrita ao vivo.
- **Fronteiras futuras:** Estender `profiles/schema/` v2.1 para SLC NAND / eMMC (`XW300`, `BS/XS`, `BW/BWT`) e completar manuais Vol. 2 para modelos em `T0`.
- **Verificação:**
  ```bash
  cargo test -p panadiga-profiles -p panadiga-core
  cargo run -q -p panadiga-cli -- profiles validate
  python -m pytest scripts/tests/test_profiles_ssot.py scripts/tests/test_geometry_traps.py
  ```
