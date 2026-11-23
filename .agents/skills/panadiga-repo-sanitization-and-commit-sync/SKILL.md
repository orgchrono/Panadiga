---
name: panadiga-repo-sanitization-and-commit-sync
description: >-
  Sanitização programática do repositório, sincronia SSOT/Ghidra/Arsenal (--sync),
  auditoria de higiene, keystores e god-modules (--check), e planejamento read-only
  de commits atômicos em 6 ondas (--commit-plan com --exclude para arquivos concorrentes).
  Nunca executa git commit ou git push automaticamente.
---

# Sanitização Programática, Sincronia SSOT & Plano de Commits em 6 Ondas

Esta skill garante higiene do repositório, sincronização automática entre projeções derivadas e agrupamento determinístico de mudanças em ondas atômicas via `scripts/repo_sanitize_and_sync.py`.

## 1. Quando Usar & Handoffs
- Ative após grandes refatorações, extrações de crates, mudanças em `profiles/`, criação de novas ferramentas/skills ou antes de preparar commits de revisão.
- Para hardening de `.gitignore` e GitHub Actions CI/CD, combine com `.agents/skills/panadiga-git-cicd-and-releases/SKILL.md`.
- Para rodar os 9 Quality Gates completos, use `.agents/skills/panadiga-validate/SKILL.md`.
- Ordem detalhada das 6 ondas e critérios de bloqueio em [commit-wave-order-and-hygiene-rules.md](./references/commit-wave-order-and-hygiene-rules.md).

## 2. Guardrails Invioláveis
1. **100% Read-Only em Git:** Este fluxo **NUNCA** executa `git add`, `git commit` ou `git push`; `--commit-plan` apenas classifica `git status --porcelain=v1 -uall` e sugere comandos explícitos para revisão humana.
2. **Isolamento de Tarefas Concorrentes (`--exclude`):** Arquivos pertencentes a outro agente/tarefa são isolados em `excluded_concurrent` para nunca misturar diffs parciais nas ondas.
3. **Zero Segredos & Binários Pesados:** Detecta `.jks`, `.keystore`, `.p12`, `.pem`, `.tmp`, `.bak`, `.orig` e binários não rastreados `> 1 MiB` fora dos diretórios ignorados de dumps.

## 3. Comandos Principais (`scripts/repo_sanitize_and_sync.py`)
```bash
# 1. Sincronizar projeções SSOT (TS/Python/C11), scripts Ghidra e Arsenal
python scripts/repo_sanitize_and_sync.py --sync

# 2. Auditoria read-only (artefatos proibidos, god-modules >450/650 LOC, drift do arsenal)
python scripts/repo_sanitize_and_sync.py --check --json -

# 3. Gerar plano de commit atômico em 6 ondas (dry-run, sem mutar git)
python scripts/repo_sanitize_and_sync.py --commit-plan --json -

# Exemplo isolando arquivos de uma frente paralela:
python scripts/repo_sanitize_and_sync.py --commit-plan \
  --exclude crates/panadiga-core/src/re/sata_bridge_stage2.rs \
  --exclude crates/panadiga-core/src/re/unbrick_sd_kit/verify/mod.rs
```
- Wrapper local na skill: `.agents/skills/panadiga-repo-sanitization-and-commit-sync/scripts/sanitize_sync.py` redireciona para `scripts.repo_sanitize_and_sync`.

## 4. Verificação
```bash
python -m mypy --strict scripts
python -m pytest scripts/tests/test_repo_sanitize_and_sync.py
```
