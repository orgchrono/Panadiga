---
name: panadiga-device-onboarding
description: >-
  Pipeline completo em 7 estágios (T0 -> T4) para onboarding de novos modelos Panasonic DIGA / OEM
  na SSOT declarativa profiles/ (tupla obrigatória model-region-board_rev-flash_jedec_id,
  12 JSON Schemas, quarentena de manuais PDF em profiles/staging/manual_proposals/, allowlist
  matemática BGO, CapabilityGate C0..C5, profiles promote e ssot export-all) e blueprint portátil
  para iniciar novos projetos de RE e forense de hardware no Dia 0.
---

# Onboarding de Novos Modelos (`T0 -> T4`) & Blueprint para Novos Projetos

Esta skill padroniza a integração de qualquer novo modelo na SSOT declarativa `profiles/` sem código hardcoded e destila a arquitetura Panadiga em um guia de partida para novos projetos.

## 1. Quando Usar & Handoffs
- Ative ao integrar novo gravador DIGA/OEM, cadastrar chips/placas/trens na SSOT, promover evidências `T0..T4` ou estruturar um novo projeto de engenharia reversa/forense do zero.
- Use `.agents/skills/panadiga-hardware-forensics/SKILL.md` (Estágio 2), `.agents/skills/panadiga-firmware-decrypto/SKILL.md` (Estágio 3), `.agents/skills/panadiga-ghidra-workflow/SKILL.md` (Estágio 5) e `.agents/skills/panadiga-validate/SKILL.md` (Estágio 7).
- Esquema completo da SSOT, tiers `T0..T4`, `CapabilityGate` e **Blueprint Portátil de 10 Princípios para Novos Projetos** em [ssot-tiers-and-new-project-blueprint.md](./references/ssot-tiers-and-new-project-blueprint.md).

## 2. Invariantes Não-Negociáveis
1. **Tupla de 4 Campos (Zero Hardcode):** Identifique hardware pela tupla **`(model, region, board_rev, flash_jedec_id)`** e nunca por nome comercial isolado em código Rust/TS/C11.
2. **Quarentena Anti-Envenenamento de PDF (`direct_profile_writes == 0`):** `manual_spec_extractor.py --apply` grava exclusivamente em `profiles/staging/manual_proposals/*.proposal.json` com `"requires_human_review": true` e `page_citations`. Nunca edite `profiles/boards/` automaticamente a partir de PDFs.
3. **Gate Runtime `< T3` & Allowlist Positiva BGO:** `prepare-sd`, `rebuild-from-dump` e `retarget` recusam perfis `< T3` sem `--allow-unverified-profile` (`--allow-unverified-mix` para combinações cross-build). Escrita BGO exige `[start..=end] ⊆ nor_layout.regions[bgo]`, alinhamento aos setores físicos e interseção vazia com `ipl`/`drv`/`nor_fs`/`eemul`.

## 3. Pipeline Canônico em 7 Estágios (`T0 -> T4`)
```bash
# Estágios 1-2: Segurança SMPS/P7501 + Captura NOR + Triagem Anti-ByteSwap/Mirror
cargo run -q -p panadiga-cli -- profiles discover-nor -i tmp/novo_nor.bin --model-id DMR-NOVO --out-dir tmp/drafts -j -
python -m scripts.re.nor_intake_analyzer --offline-only --input tmp/novo_nor.bin --model-id DMR-NOVO --out-dir tmp/drafts

# Estágio 3: .FRM, T2 Train Harvester & rebuild-from-dump
python -m scripts.re.frm_train_harvester --frm <PATH_FRM> --unpack-missing --apply --json -
cargo run -q -p panadiga-cli -- unbrick rebuild-from-dump --model DMR-NOVO --nor-dump tmp/novo_nor.bin -o tmp/sd_kit --allow-unverified-profile -j -

# Estágio 4: Quarentena PDF com citações de página (revisão humana obrigatória antes do merge T1)
python -m scripts.re.manual_spec_extractor --apply

# Estágio 5: Ghidra multi-ISA sobre seções executáveis alcançáveis -> ghidra_fleet_orchestrator.py

# Estágio 6: SSOT Export, Promoção por Evidência (flag real: profiles promote) & Validação (796 checagens)
cargo run -q -p panadiga-cli -- profiles export-schema --out-dir profiles/schema
cargo run -q -p panadiga-cli -- ssot export-all --root .
cargo run -q -p panadiga-cli -- profiles promote --model DMR-NOVO --axis nor_layout --tier T3 --sha256 <64_HEX> --fixture <PATH>
cargo run -q -p panadiga-cli -- profiles completeness -j -
cargo run -q -p panadiga-cli -- profiles validate -j -

# Estágio 7: Mn103Vm + 9 Quality Gates (python scripts/validate.py) + checklist CONTRIBUTING.md §8
```
- **MCP:** `panadiga_profiles_authorize`, `panadiga://profiles/<MODEL_ID>`, `panadiga://models-catalog`.

## 4. Armadilhas & Daqui pra Frente
- **Armadilhas:** Promover para `T4` sem `--bench-log` em `docs/debug_logs/`; usar subcomando inexistente em vez de `profiles promote`; aceitar `LabOverride` offline achando que habilita escrita ao vivo.
- **Fronteiras futuras no Panadiga:** Estender `profiles/schema/` v2.1 para SLC NAND / eMMC (`XW300`, `BS/XS`, `BW/BWT`) e completar manuais Vol. 2 para promover modelos atualmente em `T0`.

## 5. Verificação
- `cargo run -q -p panadiga-cli -- profiles validate` (`0` violações nas `796` checagens relacionais), `profiles completeness` atualizado, e `python -m pytest scripts/tests/test_profiles_ssot.py` + `python scripts/validate.py` passando 100%.
