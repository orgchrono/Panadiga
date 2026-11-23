# Referência: Ondas de Commit Atômico & Regras de Higiene

## 1. Precedência das 6 Ondas (`--commit-plan`)

1. **Wave 1 (`chore(contracts)`):** `Cargo.toml`, `Cargo.lock`, `.jscpd.json`, `.gitignore`, `package.json`, `package-lock.json`, `crates/panadiga-constants/**`, `crates/panadiga-profiles/**`, `profiles/**`, `crates/panadiga-core/src/ssot/**`.
2. **Wave 2 (`refactor(core)`):** `crates/panadiga-bench/**`, `crates/panadiga-vault/**`, `crates/panadiga-crypto/**`, `crates/panadiga-ir/**`, `crates/panadiga-core/**`.
3. **Wave 3 (`feat(cli-mcp-rag)`):** `crates/panadiga-cli/**`, `crates/panadiga-mcp/**`, `crates/panadiga-rag/**`.
4. **Wave 4 (`fix(desktop-fw)`):** `apps/desktop/**`, `firmware/**`, `tools/**`, `packages/**`.
5. **Wave 5 (`feat(scripts)`):** `scripts/**`.
6. **Wave 6 (`docs(agents)`):** `.agents/**`, `docs/**`, `.github/**` e arquivos Markdown raiz (`README*.md`, `AGENTS.md`, `GEMINI.md`, auditorias raiz).

- Qualquer caminho passado via `--exclude <PATH>` (arquivo exato ou prefixo de diretório) vai estritamente para `excluded_concurrent` e fica fora de todas as ondas.

## 2. Regras de Higiene (`--check`)

- **Segredos / Assinaturas:** Falha (`FAIL`, exit `1`) se existir arquivo `.jks`, `.keystore`, `.p12` ou `.pem`.
- **Restos Temporários:** Falha (`FAIL`) para `.tmp`, `.bak` ou `.orig` no repositório ativo.
- **Binários Não Rastreáveis:** Falha (`FAIL`) para arquivos não rastreados `> 1 MiB` (`.bin`, `.frm`, `.img`, `.iso`, `.uf2`, `.apk`, `.exe`, `.dll`, `.so`).
- **God-Modules (LOC):** `WARN` a partir de `450` linhas e `FAIL` a partir de `650` linhas em arquivos de produção (`crates/`, `apps/desktop/src-tauri/src/`, `scripts/`, excluindo `generated/`, `tests/` e `target/`).
- **Drift do Arsenal:** Exige que `scripts/generate_tool_arsenal.py --check` retorne código `0`.
