---
name: panadiga-bench-automation-and-headless
description: >-
  Orquestração de testes automatizados em bancada e execução headless de todo o ecossistema
  (scripts.bench.runner com --mock/RAII ISOLATE, pico bench-wizard --non-interactive, ab-boot,
  Playwright Chromium headless, Ghidra headless, CLI --json -, Mn103Vm e matshita-emu) com
  caminhos de raciocínio end-to-end para teste em bancada, dump, patch, testes diversos,
  unbrick e regravação. Use ao automatizar bancada, rodar suites headless ou planejar fluxos E2E.
---

# Automação de Bancada, Modos Headless & Caminhos de Raciocínio

Esta skill padroniza a automação reproduzível (em bancada física ou simulada/`--mock`), explica como abrir e operar cada subsistema em **modo headless** e define as árvores de decisão ponta a ponta.

## 1. Quando Usar & Handoffs
- Ative para rodar baterias automatizadas no Pico sem supervisão manual, executar testes E2E/Ghidra/CLI headless ou escolher a rota completa para um objetivo (teste, dump, patch, unbrick, regravação).
- Combine com `.agents/skills/panadiga-bench-operations/SKILL.md` para pré-condições elétricas e `.agents/skills/panadiga-validate/SKILL.md` para os 9 Quality Gates.
- Fluxogramas detalhados dos **6 Caminhos de Raciocínio** (Bancada, Dump, Patch, Testes Diversos, Unbrick e Regravação) e matriz headless em [headless-pipelines-and-decision-trees.md](./references/headless-pipelines-and-decision-trees.md).

## 2. Como Abrir e Executar Cada Subsistema em Modo Headless
1. **Bancada Automatizada (`scripts.bench.runner` — garante `ISOLATE` via RAII mesmo em erro):**
   ```bash
   # Simulação offline/CI (--mock) ou porta física real (--port COM10)
   python -m scripts.bench.runner health --mock
   python -m scripts.bench.runner smoke-suite --mock
   python -m scripts.bench.runner capture BOOTCAP --port COM10 --out tmp/bootcap.log
   python -m scripts.bench.runner ab-boot --pairs 5 --window-ms 6000 --csv tmp/ab_boot.csv --mock
   ```
2. **CLI Pico & Diagnóstico Headless (`--non-interactive` & `--json -`):**
   ```bash
   cargo run -q -p panadiga-cli -- pico bench-wizard -p COM10 --profile all --non-interactive --out-dir tmp/wizard_out
   cargo run -q -p panadiga-cli -- pico validate-bench-cmd -c "PINCHECK" --json
   cargo run -q -p panadiga-cli -- pico parse-diag --kind pincheck -f tmp/pincheck.txt --json
   ```
3. **Frontend Tauri/Web Headless (Playwright Chromium `12` specs, sobe Vite em `:1420` sozinho):**
   ```bash
   npm --prefix apps/desktop run test:e2e
   ```
4. **Ghidra Headless Multi-ISA & Smoke:**
   ```bash
   python -m scripts.re.ghidra_fleet_orchestrator --catalog tmp/decrypted_fleet/fleet_decrypted_catalog.json --shard small --workers 6 --java-audit
   python scripts/validate.py --ghidra-smoke
   ```
5. **Emulação Headless (`Mn103Vm` & `Matshita R6-MV4`):**
   ```bash
   cargo run -q -p panadiga-cli -- re verify-patches --json
   cargo run -q -p panadiga-cli -- unbrick matshita-emu boot --scenario healthy_with_disc -j -
   ```

## 3. Resumo dos 6 Caminhos de Raciocínio End-to-End
- **Caminho A — Teste em Bancada:** Isolamento SMPS -> Standby `P-OFF` -> `health`/`PINCHECK` -> `PASSIVE`/`DIAGWATCH WAKE` -> `PREJIG`/`PANAMON1>` -> `ISOLATE` + log SHA-256.
- **Caminho B — Dump Imutável:** Definir mídia (`NOR` vs `HDD` vs `SD`) -> `nor-stub`/`capture-nor --slice` ou `hdd extract` -> `verify-artifact` -> `nor_intake_analyzer.py` -> `session ingest --bucket raw`.
- **Caminho C — Patch Seguro:** `re audit`/`ghidra` (seções alcançáveis) -> patch CoW (`bgo`/`patch-capacity`/`passthrough`/`region-compat`/`frm repack`) -> `re verify-patches` (`Mn103Vm`) -> `session ingest --bucket recompiled` -> auditoria adversarial.
- **Caminho D — Testes Diversos:** Unidade (`cargo test`, `vitest`, C11 host, `pytest`) -> `--mock` (`scripts.bench.runner`) -> Emulação (`Mn103Vm` / `matshita-emu`) -> Headless (`playwright` / `ghidra-smoke`) -> Probe passivo -> Hardware real.
- **Caminho E — Unbrick `U99`:** Triagem de perfil (`PLEASE WAIT` ~2s vs `BGO init failed`) -> Backup 8 MiB -> `bgo-audit --nor` / `bgo-repair-plan` vs `nvram clean` -> `bgo-live-apply --backup-dir ... --confirm-upper-nor` + `prepare-sd --loader-profile ver061`.
- **Caminho F — Regravação Física:** Checar `CapabilityGate` (`T4` para live) -> Backup CoW prévio -> Lockout multi-SO / token `CONFIRM-*` -> Gravação (`flash-eh55`, `sd create`, `dvd create`, `bgo-live-apply`) -> Readback SHA-256 + `BOOTTRACE`.

## 4. Invariantes & Armadilhas
- **Allowlist `SAFE_BENCH_COMMANDS`:** `scripts.bench.protocol` rejeita comandos mutantes (como `FLASH_ERASE`) com `BenchSafetyError`.
- **Nunca travar o alvo em modo ativo:** Sempre execute blocos de bancada com `BenchSession` (`ISOLATE` garantido em `finally`) e valide comandos antes com `pico validate-bench-cmd`.

## 5. Daqui pra Frente & Verificação
- **Novos aparelhos/projetos:** Reaproveite `scripts/bench/runner.py` (`MockBenchTransport` + RAII `ISOLATE`) para construir bateria automatizada antes mesmo de plugar na placa real.
- **Verificação:** `python -m scripts.bench.runner smoke-suite --mock` e `python -m pytest scripts/tests/test_bench_protocol.py` passando 100%.
