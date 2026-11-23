---
name: panadiga-git-cicd-and-releases
description: >-
  Controle Git seguro (.gitignore, git check-ignore, offload de dumps pesados e proibição
  de versionar keystores/chaves), engenharia de GitHub Actions (ci.yml, release.yml, pinning
  SHA-40, least-privilege permissions, cache seguro) e empacotamento de Releases multiplataforma
  (Windows/Linux .deb/macOS/Pico UF2/Android APK com SHA256SUMS POSIX e assinatura). Use ao
  alterar workflows CI/CD, build scripts ou pipelines de release.
---

# Controle Git, GitHub Actions CI/CD & Engenharia de Releases

Esta skill protege a cadeia de suprimentos, garante builds reproduzíveis nos 5 alvos (Windows, Linux, macOS, Pico RP2040/RP2350 e Android APK) e previne publicações quebradas ou inseguras.

## 1. Quando Usar & Handoffs
- Ative ao editar `.github/workflows/*.yml`, `.gitignore`, `scripts/build.py`, `scripts/build_lanes/*.py`, `.github/RELEASE_NOTES.md` ou ao preparar tags/releases.
- Para executar os 9 (+1) Quality Gates localmente antes de subir commits, use `.agents/skills/panadiga-validate/SKILL.md`.
- Para sanitização do repositório, sincronia SSOT/Arsenal e plano de commits em 6 ondas (`--commit-plan`), combine com `.agents/skills/panadiga-repo-sanitization-and-commit-sync/SKILL.md`.
- Para checagem de link/arquivo ignorado pelo Git em documentos e `ledger.json`, combine com `.agents/skills/panadiga-forensic-archival/SKILL.md`.
- Inventário técnico e checklist de endurecimento de `ci.yml`, `release.yml`, `dag.py` e `android_verify.py` em [workflows-and-release-hardening.md](./references/workflows-and-release-hardening.md).

## 2. Invariantes Não-Negociáveis de Git, CI & Supply-Chain
1. **Least-Privilege Permissions em Workflows:** Defina `permissions: contents: read` no topo do workflow; conceda `contents: write` **exclusivamente** no job final `publish-release`.
2. **Pinning Completo (Actions + Ferramentas + Runners):** Toda action `uses:` deve estar pinada em hash SHA-1 completo de 40 caracteres (`# vX`), e ferramentas externas (`@tauri-apps/cli`, `mypy`/`pytest`, Rust toolchain via `rust-toolchain.toml`) devem ter versão fixada — evite `npx --yes <pacote-sem-versão>`.
3. **Segurança de Assinatura & Chaves no Git:** Nunca comitar `*.keystore`, `apps/desktop/src-tauri/gen/android/keystore/`, `keys.local.ukf` ou dumps grandes fora da allowlist `.gitignore`. Releases públicas de Android não podem depender de keystore efêmera gerada na hora (quebra upgrade `vN -> vN+1`); use Secret persistente em produção ou rotule builds de dev explicitamente.
4. **Qualidade Antes de Publicar Release:** `release.yml` não pode publicar release final (`draft: false`) apenas com 1 smoke test — deve exigir os Quality Gates ou publicar como `draft: true` até validação.
5. **Checksums & Versões Cross-OS Uniformes:** Todos os `.sha256` (incluindo Windows!) devem sair em formato POSIX `<64-hex-minúsculo>  <arquivo>` compatível com `sha256sum -c`; `DEB_VERSION` em Linux deve iniciar sempre com dígito (`^[0-9]`).

## 3. Workflow Canônico de Build & Verificação CI
```bash
# Testar DAG completo localmente (Desktop, Android, Pico, Caches)
python scripts/build.py --cache-status
python scripts/build.py --desktop --ci
python scripts/build.py --pico --strict-toolchains
python scripts/build.py --android --strict-toolchains

# Verificar assinatura e dual-ABI (arm64-v8a + armeabi-v7a) de APK gerado
python -m scripts.build_lanes.android_verify --verify-only <CAMINHO_APK>

# Garantir que nenhum arquivo de doc/evidência está bloqueado pelo .gitignore
python -m pytest scripts/tests/test_docs_parity.py scripts/tests/test_build_and_validate.py scripts/tests/test_android_verify.py
```

## 4. Armadilhas Críticas Descobertas no Pipeline
- **`scripts/build_lanes/dag.py` + `dist-android`:** `build_frontend` padrão gera apenas `dist/` (desktop); rodar `build.py --android` sem construir `dist-android` antes deixa `skip_frontend=True` apontando para pasta ausente ou obsoleta.
- **Double-Trigger `ci.yml`:** `branches: ["main", "master", "**"]` tanto em `push` quanto em `pull_request` dispara 2 runs simultâneos em PRs do mesmo repositório.
- **`release.yml` `workflow_dispatch` Default:** `default: "v0.2.0"` pode sobrescrever silenciosamente a release existente se disparado sem preencher a nova tag!
- **Gate 7 no `validate_gates.py`:** Sem `git diff --exit-code` após `ssot export-all`, deriva não-commitada passa silenciosa; e rodar `ssot export-all` em paralelo com `oxlint`/`vitest`/`C11` cria condição de corrida em arquivos gerados.

## 5. Verificação
- Após alterar workflows ou scripts de build, rode `python -m mypy --strict scripts`, `python -m pytest scripts/tests/test_build_and_validate.py scripts/tests/test_android_verify.py` e valide sintaxe YAML/shell (`sh -n ./build.sh`).
