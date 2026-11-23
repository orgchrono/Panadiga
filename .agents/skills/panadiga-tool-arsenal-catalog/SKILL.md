---
name: panadiga-tool-arsenal-catalog
description: >-
  Catálogo auto-atualizável do arsenal completo do ecossistema Panadiga (11 crates Rust,
  subcomandos CLI, 26 tools/9 famílias de resources/4 prompts MCP, comandos IPC Tauri 2,
  catálogo Pico P7501, scripts Python/Ghidra, 32 skills e 6 subagentes) com detecção de
  deriva (--check). Use para descobrir ferramentas existentes ou atualizar após mudanças.
---

# Catálogo Auto-Atualizável do Arsenal de Ferramentas Panadiga

Esta skill indexa dinamicamente todas as ferramentas do repositório via `scripts/generate_tool_arsenal.py`, evitando invenção de flags inexistentes ou deriva documental.

## 1. Quando Usar & Handoffs
- Ative para localizar rapidamente qual crate, subcomando `panadiga`, tool MCP, comando IPC Tauri, script Python/Ghidra ou skill cobre uma tarefa, ou logo após criar/remover qualquer recurso.
- O inventário completo gerado automaticamente vive em [ARSENAL_INDEX.md](./references/ARSENAL_INDEX.md).
- Para orquestração e cadeias multi-skill, use `.agents/skills/panadiga-skill-orchestrator/SKILL.md`.
- Para sincronização completa (SSOT + Ghidra + Arsenal) + plano de commit, use `.agents/skills/panadiga-repo-sanitization-and-commit-sync/SKILL.md`.

## 2. Regras de Sincronização Automática
1. **Zero Inventário Manual:** Nunca edite `references/ARSENAL_INDEX.md` à mão; o gerador lê diretamente `Cargo.toml`, `crates/panadiga-cli/src/cli/args/mod.rs`, `crates/panadiga-mcp/src/`, `apps/desktop/src-tauri/src/lib.rs`, `hardware_catalog.json`, `scripts/`, `tools/ghidra/`, `.agents/skills/` e `.agents/agents/`.
2. **Detecção de Deriva (`--check`):** Retorna código `1` sempre que uma nova tool MCP, crate, comando Tauri, script Python ou skill entrar sem regeneração do catálogo.
3. **Execução via `repo_sanitize_and_sync.py`:** `python scripts/repo_sanitize_and_sync.py --sync` dispara `--write` automaticamente ao lado de `ssot export-all` e `ghidra export-all`.

## 3. Comandos (`scripts/generate_tool_arsenal.py`)
```bash
# Regenerar ./references/ARSENAL_INDEX.md de forma determinística
python scripts/generate_tool_arsenal.py --write

# Verificar deriva sem modificar arquivos (exit 0 = sincronizado, exit 1 = deriva)
python scripts/generate_tool_arsenal.py --check

# Exportar inventário estruturado em JSON para automações/inspeção
python scripts/generate_tool_arsenal.py --json -
```
- Wrapper local na skill: `.agents/skills/panadiga-tool-arsenal-catalog/scripts/sync_arsenal.py` delega diretamente para `scripts.generate_tool_arsenal`.

## 4. Verificação
```bash
python -m mypy --strict scripts
python -m pytest scripts/tests/test_tool_arsenal.py
```
