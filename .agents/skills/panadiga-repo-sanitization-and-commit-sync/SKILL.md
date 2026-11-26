---
name: panadiga-repo-sanitization-and-commit-sync
description: >-
  Sanitização programática do repositório, catálogo auto-atualizável do arsenal completo
  (crates Rust, subcomandos CLI, tools/resources/prompts MCP, comandos IPC Tauri 2, catálogo
  Pico P7501, scripts Python/Ghidra, 29 skills e 7 subagentes), sincronia SSOT/Ghidra/Arsenal
  (--sync), auditoria de higiene/keystores/god-modules (--check) e planejamento read-only de
  commits atômicos em 6 ondas (--commit-plan com --exclude). Nunca executa git commit/push.
---

# Sanitização Programática, Catálogo do Arsenal & Plano de Commits em 6 Ondas

Esta skill garante higiene do repositório, mantém o inventário vivo completo do arsenal em `./references/ARSENAL_INDEX.md` (detectando deriva sem edição manual), sincroniza projeções derivadas e agrupa mudanças em 6 ondas atômicas read-only via `scripts/repo_sanitize_and_sync.py` e `scripts/generate_tool_arsenal.py`.

## 1. Quando Usar & Handoffs
- Ative para localizar qual crate, subcomando `panadiga`, tool MCP, comando IPC Tauri, script Python/Ghidra ou skill cobre uma tarefa, após grandes refatorações/extrações de crates, mudanças em `profiles/`, criação/remoção de ferramentas/skills, ou antes de preparar commits de revisão.
- Para hardening de `.gitignore` e GitHub Actions CI/CD, combine com `.agents/skills/panadiga-git-cicd-and-releases/SKILL.md`.
- Para rodar os 9 Quality Gates completos, use `.agents/skills/panadiga-validate/SKILL.md`.
- **Referências:**
  - Ordem detalhada das 6 ondas e critérios de bloqueio em [commit-wave-order-and-hygiene-rules.md](./references/commit-wave-order-and-hygiene-rules.md).
  - Inventário completo gerado automaticamente em [ARSENAL_INDEX.md](./references/ARSENAL_INDEX.md).

## 2. Guardrails Invioláveis
1. **100% Read-Only em Git:** Este fluxo **NUNCA** executa `git add`, `git commit` ou `git push`; `--commit-plan` apenas classifica `git status --porcelain=v1 -uall` e sugere comandos explícitos para revisão humana.
2. **Isolamento de Tarefas Concorrentes (`--exclude`):** Arquivos pertencentes a outro agente/tarefa são isolados em `excluded_concurrent` para nunca misturar diffs parciais nas ondas.
3. **Zero Segredos & Binários Pesados:** Detecta `.jks`, `.keystore`, `.p12`, `.pem`, `.tmp`, `.bak`, `.orig` e binários não rastreados `> 1 MiB` fora dos diretórios ignorados de dumps.
4. **Zero Inventário Manual do Arsenal:** Nunca edite `./references/ARSENAL_INDEX.md` à mão; o gerador lê diretamente `Cargo.toml`, `crates/panadiga-cli/src/cli/args/mod.rs`, `crates/panadiga-mcp/src/`, `apps/desktop/src-tauri/src/lib.rs`, `hardware_catalog.json`, `scripts/`, `tools/ghidra/`, `.agents/skills/` e `.agents/agents/`. `--check` falha com código `1` se houver deriva.

## 3. Comandos Principais (`repo_sanitize_and_sync.py` & `generate_tool_arsenal.py`)
```bash
# 1. Sincronizar projeções SSOT (TS/Python/C11), scripts Ghidra, Arsenal e Atlas
python scripts/repo_sanitize_and_sync.py --sync

# 2. Auditoria read-only (artefatos proibidos, god-modules >450/650 LOC, drift arsenal/atlas)
python scripts/repo_sanitize_and_sync.py --check --json -

# 3. Gerar plano de commit atômico em 6 ondas (dry-run, sem mutar git; suporta --exclude)
python scripts/repo_sanitize_and_sync.py --commit-plan --json -

# 4. Catálogo do Arsenal isolado (regenerar, checar deriva ou exportar JSON)
python scripts/generate_tool_arsenal.py --write
python scripts/generate_tool_arsenal.py --check
python scripts/generate_tool_arsenal.py --json -
```
- Wrappers locais na skill: `.agents/skills/panadiga-repo-sanitization-and-commit-sync/scripts/sanitize_sync.py` e `.agents/skills/panadiga-repo-sanitization-and-commit-sync/scripts/sync_arsenal.py`.

## 4. Verificação
```bash
python -m mypy --strict scripts
python -m pytest scripts/tests/test_repo_sanitize_and_sync.py scripts/tests/test_tool_arsenal.py
```
