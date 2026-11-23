# Engenharia de Workflows (`ci.yml`, `release.yml`, `dag.py`) & Hardening

## 1. Mapa dos Workflows Atuais (`.github/workflows/`)
- **`ci.yml` (5 jobs):** `frontend-bundle` (`ubuntu`, check de vazamento Android no bundle desktop) -> `quality-gates-windows` (`windows-latest`, `.\build.cmd --ci --desktop --strict-toolchains`), `cross-platform` (`ubuntu-latest`/`macos-latest`), `pico-firmware` (`ubuntu-latest`, `gcc-arm-none-eabi` + Pico SDK `2.1.1`), `android-apk` (`ubuntu-latest`, `aarch64` + `armv7` dual-ABI).
- **`release.yml` (5 jobs):** `frontend-bundle` + `pico-firmware` -> `build-release` (`windows-latest`/`ubuntu-22.04`/`macos-latest`), `release-android-apk` -> `publish-release` (`softprops/action-gh-release`).

## 2. Checklist de Hardening Priorizado (Achados Verificados)
1. **`release.yml` Permissions:** Manter `contents: read` no topo; colocar `permissions: { contents: write }` apenas no job `publish-release`.
2. **`release.yml` Windows `.sha256`:** Substituir `Get-FileHash | Out-File` (tabela multi-linha PowerShell) por saída POSIX `<64-hex>  <filename>` para compatibilidade com `SHA256SUMS.txt` / `sha256sum -c`.
3. **`release.yml` `DEB_VERSION` & `workflow_dispatch`:** Extrair versão de `${{ inputs.tag_name || github.ref_name }}` e fazer fallback para `Cargo.toml` sempre que o resultado não bater `^[0-9]`; evitar sobrescrita acidental de tag existente (`draft: true` ou checagem de tag).
4. **Assinatura Android Persistente:** Substituir geração efêmera de keystore via `keytool` em CI (`android_verify.py`) por secrets `ANDROID_KEYSTORE_BASE64` + senhas quando publicar release oficial; renomear APK dual-ABI de `panadiga-android-arm64.apk` para `panadiga-android-universal.apk` (contém `arm64-v8a` e `armeabi-v7a`).
5. **Pinning de Dependências CI:** Substituir `npx --yes @tauri-apps/cli` flutuante por pacote pinado em `apps/desktop/package.json` (`npx tauri ...`) e pinagem de `mypy`/`pytest`/`matplotlib` + `rust-toolchain.toml`.
6. **Correção do DAG (`scripts/build_lanes/dag.py`):** Construir `dist-android` na Wave 1 sempre que `run_android` estiver ativo antes de invocar `build_android(..., skip_frontend=True)`.
7. **Sincronia do Gate 7 (`scripts/validate_gates.py`):** Nunca executar `ssot export-all` concorrendo com lanes leitoras (`oxlint`/`vitest`/`jscpd`/`C11`), e verificar deriva no CI via `git diff --exit-code`.
