---
name: panadiga-technical-documentation
description: >-
  Desenvolvimento, organização e governança de documentação técnica de grande porte:
  arquitetura Diátaxis (5 trilhas), paridade bilíngue 1:1 (docs/pt-BR <-> docs/en-US,
  translation_pairs.json, docs/INDEX.md), ADRs, diagramas Mermaid suportados, Rustdoc/JSDoc/
  docstrings sincronizados e combate automatizado a doc-drift via test_docs_parity.py e
  git check-ignore. Use ao criar ou revisar documentação técnica no repositório ou em novos projetos.
---

# Engenharia & Governança de Documentação Técnica (Padrão Grande Porte)

Esta skill define a arquitetura documental para manter dossiês extensos, guias operacionais, referência de código e relatórios livres de contradições internas (*doc-drift*) e links quebrados.

## 1. Quando Usar & Handoffs
- Ative ao criar novos dossiês bilíngues, atualizar índices/contagens, escrever guias Diátaxis, ADRs ou revisar documentação de APIs (Rustdoc/JSDoc/Python).
- Para cadeia de custódia de provas, `ledger.json` e timeline histórica, combine com `.agents/skills/panadiga-forensic-archival/SKILL.md`.
- Para automatizar verificações no CI, combine com `.agents/skills/panadiga-validate/SKILL.md`.
- Estrutura das 5 trilhas Diátaxis, regras Mermaid e checklist anti-drift em [diataxis-bilingual-and-doc-standards.md](./references/diataxis-bilingual-and-doc-standards.md).

## 2. Os 5 Pilares Documentais (Padrão Panadiga)
1. **Organização Diátaxis (`docs/STATUS.md` & `docs/INDEX.md`):** Separe claramente *Tutoriais/Guias de Bancada* (como executar passo a passo), *Referências Fatuais* (pinagens, tabelas hex, CLI), *Explicações/Teoria* (BGO, criptografia, `IC7501`) e *Snapshots/Auditorias Históricas* (`docs/audits/` imutáveis com data).
2. **Paridade Bilíngue 1:1 Verificada por Código:** Todo dossiê sob `docs/pt-BR/` tem par exato em `docs/en-US/`, listado em `docs/translation_pairs.json` e nas duas tabelas bidirecionais de `docs/INDEX.md`.
3. **Fontes de Verdade para Números & Versões (Zero Chute):** Contagens de crates (`Cargo.toml`), versões JS (`apps/desktop/package.json`), número de testes (`cargo test` / `vitest` / `playwright` / `pytest`), pares bilíngues (`translation_pairs.json`) e logs (`docs/debug_logs/*.log`) devem ser **medidos antes de escrever**.
4. **Documentação Integrada ao Código:**
   - **Rust:** `///` (Rustdoc) nas APIs públicas e invariantes de segurança (`//!` de módulo).
   - **TypeScript:** JSDoc (`/** ... */` com `@param`/`@returns` quando não trivial) em contratos, `services/`, `hooks/`/`viewmodels/` e `utils/`.
   - **Python:** docstrings PEP 257 com tipagem `mypy --strict`.
5. **Higiene de Links & `.gitignore`:** Todo link relativo em Markdown deve resolver em disco e passar por `git check-ignore --stdin` (links apontando para artefatos ignorados passam local e quebram no GitHub Actions).

## 3. Workflow Canônico ao Criar ou Atualizar Documentação
```bash
# 1. Conferir contagens reais antes de atualizar docs raiz / INDEX / STATUS
python -m pytest scripts/tests/test_docs_parity.py scripts/tests/test_profiles_ssot.py

# 2. Conferir compilação de Python docstrings / Rustdoc
cargo doc --workspace --no-deps
```
- Sempre crie/edite os arquivos equivalentes em `docs/pt-BR/` e `docs/en-US/` juntos, atualize `docs/translation_pairs.json`, `docs/INDEX.md`, `docs/STATUS.md`, e marque claramente o nível epistêmico (`N1_STATIC` .. `N5_REPRODUCED`, nunca usando badge `[PROVADO EM HARDWARE REAL]` sem entrada correspondente em `docs/evidence/ledger.json`).

## 4. Armadilhas & Anti-Padrões
- Atualizar `README.md` e `docs/INDEX.md` para 40 pares / 28 logs esquecendo `docs/STATUS.md` ou `CONTRIBUTING.md` (causando contradição interna).
- Usar tipos Mermaid não suportados (`gantt`, `mindmap` etc.) ou tags HTML dentro de labels Mermaid; prefira `flowchart`, `sequenceDiagram`, `stateDiagram-v2`, `classDiagram`, `erDiagram` com labels entre aspas.
- Citar endereços hexadecimais ou comandos CLI sem verificar o binário ou `crates/panadiga-cli/src/cli/args/` atual.

## 5. Verificação
- `python -m pytest scripts/tests/test_docs_parity.py` passando 100%, `cargo doc --workspace --no-deps` sem warnings quebrados e revisão visual confirmando ausência de deriva numérica entre `INDEX.md`, `STATUS.md`, `README*` e `CONTRIBUTING.md`.
