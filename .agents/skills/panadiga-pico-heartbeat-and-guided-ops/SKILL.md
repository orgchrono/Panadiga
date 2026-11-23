---
name: panadiga-pico-heartbeat-and-guided-ops
description: >-
  Monitoramento heartbeat da sonda Pico P7501 (autodescoberta de porta USB CDC VID 2E8A,
  sem porta fixa obrigatória), classificação de VDD/WDT/DCSR e disparador guiado read-only
  com bloqueio rígido SAFETY_DENIED para comandos de escrita em NOR/NVRAM/BGO.
  Use antes e durante triagem passiva e operação guiada segura na bancada.
---

# Heartbeat da Sonda Pico P7501 & Disparador Guiado Read-Only

Esta skill automatiza a verificação contínua de saúde da sonda RP2040/RP2350 (`P7501` `115200 8N1`) e o disparo de macros somente-leitura via `scripts/pico_heartbeat_ops.py`.

## 1. Quando Usar & Handoffs
- Ative para checar se a sonda Pico está respondendo (`PING` / `STATUS`), classificar estado `VDD` / `DCSR` do alvo AM34 ou validar macros passivas antes de sessões de depuração.
- Para desenvolvimento C11/UF2 e gravação do Pico, use `.agents/skills/panadiga-pico-flashing/SKILL.md`.
- Para procedimentos físicos SMPS/ADuM/`Q7510`/`DAVIS`, use `.agents/skills/panadiga-bench-operations/SKILL.md`.
- Tabela de estados heartbeat e macros seguras em [heartbeat-states-and-guided-macros.md](./references/heartbeat-states-and-guided-macros.md).

## 2. Regras de Ouro (Descoberta Dinâmica & Read-Only Estrito)
1. **Sem Hardcode de Porta:** `discover_pico_port` consulta `panadiga pico list` dinamicamente e prioriza `VID:2E8A`; `--port <PORT>` funciona apenas como override explícito (`COM*`, `/dev/ttyACM*`, `/dev/cu.usbmodem*`).
2. **Bloqueio Rígido `SAFETY_DENIED`:** `--mode guided` rejeita com código de saída `2` qualquer token destrutivo ou restrito (`ERASE`, `WRITE`, `FLASH`, `POKE`, `PATCH`, `MEMW`, `W32`/`W16`/`W8`, `CONFIRM_UPPER_NOR`, `0xC000100C`, `0x50409DF2`, `0x50409E65`, `BGO_WRITE`, `NVRAM_WRITE`, `M80`, `X80`, `JOB`) e valida a cabeça do comando contra Rust `SAFE_BENCH_COMMANDS` via `panadiga pico validate-bench-cmd`.
3. **Alarme de Sobretensão (`OVERVOLTAGE_ABORT`):** Qualquer leitura `VDD > 3.65V` dispara abort imediato (`GP3`/`NSW5V` permanece sempre `Hi-Z Input` sem pulls).

## 3. Uso Rápido (`scripts/pico_heartbeat_ops.py`)
```bash
# Amostra única de heartbeat com autodescoberta USB CDC (JSON)
python scripts/pico_heartbeat_ops.py --mode heartbeat --json -

# Override explícito de porta e 3 amostras com intervalo de 1s
python scripts/pico_heartbeat_ops.py --mode heartbeat --port COM10 --count 3 --interval-s 1 --json -

# Macro guiada somente-leitura em dry-run (valida contra Rust SAFE_BENCH_COMMANDS)
python scripts/pico_heartbeat_ops.py --mode guided --profile passive_triage --dry-run --json -

# Teste de bloqueio SAFETY_DENIED (deve retornar exit=2 e status=SAFETY_DENIED)
python scripts/pico_heartbeat_ops.py --mode guided --cmd "ERASE 0x807F0000" --json -
```
- Wrapper local na skill: `.agents/skills/panadiga-pico-heartbeat-and-guided-ops/scripts/pico_heartbeat.py` redireciona diretamente para `scripts.pico_heartbeat_ops`.

## 4. Subcomandos Rust Diretos Utilizados
- Descoberta USB CDC: `panadiga pico list`
- Pulso & VDD/DCSR: `panadiga pico ping -p <PORT>` e `panadiga pico status -p <PORT>`
- Validação SSOT: `panadiga pico validate-bench-cmd --cmd <CMD> --json -`

## 5. Verificação
```bash
python -m mypy --strict scripts
python -m pytest scripts/tests/test_pico_heartbeat.py
```
