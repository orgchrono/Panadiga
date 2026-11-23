---
name: panadiga-forensic-archival
description: >-
  Arquivismo forense, cadeia de custódia SHA-256, sessões segregadas do Vault (panadiga session:
  raw, frm e recompiled com gate Mn103Vm fail-closed), Evidence Ledger (docs/evidence/ledger.json
  N1_STATIC..N5_REPRODUCED), curadoria de logs em docs/debug_logs/, linha do tempo e descobertas,
  paridade documental bilíngue (translation_pairs.json, test_docs_parity.py, git check-ignore)
  e gestão de acervos PDF/releases. Use ao arquivar evidências, atualizar docs ou gerir a timeline.
---

# Arquivismo Forense, Evidências (`ledger.json`), Documentação & Timeline

Esta skill garante preservação imutável de binários/logs, rigor epistêmico nas afirmações técnicas, organização cronológica das descobertas e paridade bilíngue sem links quebrados nem arquivos ignorados pelo Git.

## 1. Quando Usar & Handoffs
- Ative ao armazenar dumps/capturas no Vault, registrar nova descoberta no `ledger.json`, arquivar logs em `docs/debug_logs/`, escrever/traduzir dossiês ou atualizar a timeline histórica do projeto.
- Para onboarding de novos modelos na SSOT `profiles/` ou iniciar projetos novos, combine com `.agents/skills/panadiga-device-onboarding/SKILL.md`.
- Para checagem de links e paridade em CI, combine com `.agents/skills/panadiga-validate/SKILL.md`.
- Contrato completo dos 3 buckets do Vault, schema do `ledger.json`, política de releases e template portátil em [archival-docs-timeline-and-ledger.md](./references/archival-docs-timeline-and-ledger.md).

## 2. Invariantes Não-Negociáveis
1. **Vault Segregado de 3 Buckets (`crates/panadiga-core/src/vault/`):**
   - `raw/`: aceita apenas `.bin`/`.img`/`.nvram`/`.dump` (rejeita `.frm`); **imutável** por SHA-256.
   - `frm/`: aceita apenas `.frm`; **imutável** por SHA-256.
   - `recompiled/`: **falha fechado** se `PreWriteGateReport.passed != true` (`Mn103Vm`).
2. **Regra Epistêmica & Ledger (`docs/evidence/ledger.json`):** Todo documento `docs/pt-BR/*.md` com a tag `[PROVADO EM HARDWARE REAL]` **DEVE** ter ao menos uma entrada `N4_HARDWARE` ou `N5_REPRODUCED` no `ledger.json` (`test_docs_parity.py`), com todos os `sha256_assertions` batendo em disco.
3. **Paridade Bilíngue 1:1 & `git check-ignore`:** `docs/pt-BR/*.md`, `docs/en-US/*.md`, `docs/translation_pairs.json` e `docs/INDEX.md` mantêm exatamente **40 pares** (atualizando os 4 juntos se adicionar novo par), e nenhum link de doc/ledger pode apontar para caminho ignorado pelo `.gitignore`.
4. **Dumps Pesados Fora do Git:** Imagens acima do limite Git vão para Releases (`firmware-pack-v0.1.0`) mantendo apenas `SHA256SUMS.txt` e guias no repositório.

## 3. Workflow Canônico (CLI, RAG & Paridade)
```bash
# 1. Sessão do Cofre Forense (SelectModel -> BuildCleanroomSd -> SetupPicoProbe -> CaptureAndBackup -> OrganizeAndRecompile -> EmulateAndFlash)
cargo run -q -p panadiga-cli -- session init --model DMR-EH55 --loader-profile ver061 --session-id eh55_s01 -j -
cargo run -q -p panadiga-cli -- session ingest --session-dir <VAULT_SESS> --file <DUMP.bin> --bucket raw --origin bench_capture -j -
cargo run -q -p panadiga-cli -- session extract-drv --session-dir <VAULT_SESS> --nor-dump <NOR.bin> -j -
cargo run -q -p panadiga-cli -- session status --session-dir <VAULT_SESS> -j -

# 2. Verificar SHA-256 de artefato e decodificar logs de captura
cargo run -q -p panadiga-cli -- verify-artifact <FILE> -s <EXPECTED_SHA256>
cargo run -q -p panadiga-cli -- pico decode-log -f docs/debug_logs/<LOG> --json

# 3. Pesquisar 32 manuais PDF, corpus RAG e catálogo visual (238 PNGs)
cargo run -q -p panadiga-cli -- manuals -t P7501 -t IC7501
cargo run -q -p panadiga-cli -- rag status --json -

# 4. Validar paridade documental, hashes do ledger e debug_logs
python -m pytest scripts/tests/test_docs_parity.py
```
- **MCP:** `panadiga_search_manuals`, `panadiga_search_visuals`, `panadiga_rag_query`.

## 4. Cuidados com Timeline & Pendências Atuais (`2026-11-22`)
- **Pendência Conhecida:** Os arquivos recentes `docs/evidence/*20261122*` (incluindo o breakthrough `davis_probe_ch1_hdmi_flat_20261122.json`, `ch1_identify_ec_hdmi_flat_20261122.bin` e `boottrace_*_20261122.log`) ainda não foram formalizados em `docs/evidence/ledger.json` nem movidos/curados em `docs/debug_logs/README.md`.
- **Sincronização de Contagens (`test_docs_parity.py`):** Ao criar novo log curado ou par de dossiês, atualize `docs/debug_logs/README.md`, `docs/INDEX.md`, `docs/STATUS.md` (note que `STATUS.md` ainda exibe contagens antigas), `README.md`, `README.pt-BR.md`, `AGENTS.md`, `.github/RELEASE_NOTES.md` e `EXPECTED_PAIRS`/`EXPECTED_LOGS` quando aplicável.

## 5. Verificação
- `python -m pytest scripts/tests/test_docs_parity.py` `100% passed` (valida `40` pares, `28` logs + SHA-256, `ledger.json`, zero `stale hash`, badges EN/PT e `git check-ignore`).
