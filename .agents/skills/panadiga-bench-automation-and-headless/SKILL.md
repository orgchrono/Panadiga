---
name: panadiga-bench-automation-and-headless
description: >-
  Orquestração de testes automatizados em bancada, monitoramento heartbeat da sonda Pico P7501
  (autodescoberta USB CDC VID 2E8A, classificação VDD/WDT/DCSR e disparador guiado read-only
  com bloqueio rígido SAFETY_DENIED / OVERVOLTAGE_ABORT) e execução headless (scripts.bench.runner
  com --mock/RAII ISOLATE, pico bench-wizard --non-interactive, ab-boot, Playwright, Ghidra,
  CLI --json -, Mn103Vm e matshita-emu) com 6 caminhos de raciocínio E2E. Use ao automatizar
  bancada, checar heartbeat Pico ou rodar fluxos headless.
---

# Automação de Bancada, Heartbeat Pico `P7501`, Modos Headless & Caminhos E2E

Esta skill padroniza o monitoramento passivo/guiado da sonda Pico `P7501`, a automação reproduzível em bancada (física ou simulada/`--mock`), a operação **headless** dos subsistemas e as árvores de decisão ponta a ponta.

## 1. Quando Usar & Handoffs
- Ative para checar saúde da sonda Pico (`PING`/`STATUS` via `VID:2E8A`), classificar `VDD`/`DCSR`, validar macros somente-leitura, rodar baterias automatizadas no Pico sem supervisão, executar suites headless ou planejar rotas ponta a ponta (teste, dump, patch, unbrick, regravação).
- Para pré-condições elétricas físicas (SMPS/`ADuM`, `Q7510`/`Q7514`, `DAVIS`), combine com `.agents/skills/panadiga-bench-operations/SKILL.md`.
- Para desenvolvimento C11/UF2 e gravação da sonda Pico, use `.agents/skills/panadiga-pico-flashing/SKILL.md`; para os 9 Quality Gates, use `.agents/skills/panadiga-validate/SKILL.md`.
- **Referências:**
  - Fluxogramas dos **6 Caminhos de Raciocínio** e matriz headless em [headless-pipelines-and-decision-trees.md](./references/headless-pipelines-and-decision-trees.md).
  - Tabela de estados heartbeat e macros guiadas seguras em [heartbeat-states-and-guided-macros.md](./references/heartbeat-states-and-guided-macros.md).

## 2. Heartbeat Pico `P7501` & Disparador Guiado Read-Only (`scripts/pico_heartbeat_ops.py`)
1. **Autodescoberta USB Dinâmica (Sem Hardcode de Porta):** `discover_pico_port` consulta `panadiga pico list` e prioriza `VID:2E8A`; `--port <PORT>` (`COM*`, `/dev/ttyACM*`, `/dev/cu.usbmodem*`) é apenas override explícito.
2. **Bloqueio Rígido `SAFETY_DENIED` (Exit Code `2`):** `--mode guided` bloqueia tokens destrutivos/restritos (`ERASE`, `WRITE`, `FLASH`, `POKE`, `PATCH`, `MEMW`, `W32`/`W16`/`W8`, `CONFIRM_UPPER_NOR`, `0xC000100C`, `0x50409DF2`, `0x50409E65`, `BGO_WRITE`, `NVRAM_WRITE`, `M80`, `X80`, `JOB`) e valida contra Rust `SAFE_BENCH_COMMANDS` via `panadiga pico validate-bench-cmd`.
3. **Alarme de Sobretensão (`OVERVOLTAGE_ABORT`):** Qualquer `VDD > 3.65V` dispara abort imediato (`GP3`/`NSW5V` permanece sempre `Hi-Z Input` sem pulls).
4. **Portão `TARGET_OFF_OR_STANDBY` (Pausa Obrigatória / Zero Desperdício de Tokens):** Ao detectar `TARGET_OFF_OR_STANDBY` (`VDD < 2.50V`, `NSW5V=0` ou `fault_latch=1`, mesmo com VFD frontal congelado em `LD FVU`), **PROIBIDO** disparar `XRESET`, `BOOTTRACE NORMAL`, `XRAY BOOT` ou baterias ativas; execute apenas `ISOLATE` e **PARE aguardando interação explícita do usuário** (armando `XRAY WAKE` / `BOOTTRACE ... WAKE` somente após confirmação humana).
```bash
python scripts/pico_heartbeat_ops.py --mode heartbeat --json -
python scripts/pico_heartbeat_ops.py --mode heartbeat --port COM10 --count 3 --interval-s 1 --json -
python scripts/pico_heartbeat_ops.py --mode guided --profile passive_triage --dry-run --json -
# Wrapper local na skill: .agents/skills/panadiga-bench-automation-and-headless/scripts/pico_heartbeat.py
```

## 3. Execução Headless por Subsistema
1. **Bancada Automatizada (`scripts.bench.runner` — garante `ISOLATE` via RAII mesmo em erro):**
   ```bash
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
3. **Frontend Playwright / Ghidra Fleet / Emulação Headless:**
   ```bash
   npm --prefix apps/desktop run test:e2e
   python -m scripts.re.ghidra_fleet_orchestrator --catalog tmp/decrypted_fleet/fleet_decrypted_catalog.json --shard small --workers 6 --java-audit
   python scripts/validate.py --ghidra-smoke
   cargo run -q -p panadiga-cli -- re verify-patches --json
   cargo run -q -p panadiga-cli -- unbrick matshita-emu boot --scenario healthy_with_disc -j -
   ```

## 4. Os 6 Caminhos E2E, Invariantes & Verificação
- **Caminho A (Bancada):** SMPS -> Standby `P-OFF` -> `health`/`PINCHECK` -> `PASSIVE`/`DIAGWATCH WAKE` -> `PREJIG`/`PANAMON1>` -> `ISOLATE` + log SHA-256.
- **Caminho B (Dump Imutável):** Mídia (`NOR`/`HDD`/`SD`) -> `nor-stub`/`capture-nor --slice` ou `hdd extract` -> `verify-artifact` -> `nor_intake_analyzer.py` -> `session ingest --bucket raw`.
- **Caminho C (Patch Seguro):** `re audit`/`ghidra` (seções alcançáveis) -> patch CoW -> `re verify-patches` (`Mn103Vm`) -> `session ingest --bucket recompiled` -> auditoria adversarial.
- **Caminho D (Testes Diversos):** Unidade -> `--mock` (`scripts.bench.runner`) -> Emulação (`Mn103Vm`/`matshita-emu`) -> Headless (`playwright`/`ghidra-smoke`) -> Probe passivo -> Hardware real.
- **Caminho E (Unbrick `U99`):** Triagem (`PLEASE WAIT` ~2s vs `BGO init failed`) -> Backup 8 MiB -> `bgo-audit --nor` / `bgo-repair-plan` vs `nvram clean` -> `bgo-live-apply --backup-dir ... --confirm-upper-nor` + `prepare-sd --loader-profile ver061`.
- **Caminho F (Regravação Física):** `CapabilityGate` (`T4` para live) -> Backup CoW -> Lockout multi-SO / token `CONFIRM-*` -> Gravação -> Readback SHA-256 + `BOOTTRACE`.
- **Invariante:** `scripts.bench.protocol` rejeita comandos mutantes com `BenchSafetyError`; sempre execute blocos em `BenchSession` (`ISOLATE` em `finally`).
- **Verificação:** `python -m mypy --strict scripts` + `python -m pytest scripts/tests/test_bench_ecosystem.py scripts/tests/test_pico_heartbeat.py` + `python -m scripts.bench.runner smoke-suite --mock` 100% PASS.
