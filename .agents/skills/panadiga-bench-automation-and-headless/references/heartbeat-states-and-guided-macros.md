# Referência: Estados de Heartbeat & Macros Guiadas `P7501`

## 1. Máquina de Estados de Saúde (`grade_heartbeat`)

| Estado | Critério Determinístico | Significado Físico / Ação |
| :--- | :--- | :--- |
| `ONLINE_HEALTHY` | `ping_ok == True` e `2.95 <= VDD <= 3.60 V` | Sonda Pico e alvo AM34 energizados na janela 3.3V LVTTL segura. |
| `TARGET_OFF_OR_STANDBY` | `ping_ok == True` e (`DCSR == 0xFFFFFFFF`, `VDD < 2.50V`, `NSW5V=0`, `fault_latch=1` ou sem senso VDD) | Alvo em Standby (`IC7501` `P-OFF`, mesmo com VFD congelado em `LD FVU`), AC desligado ou pinos soltos. **PARAR IMEDIATAMENTE, executar `ISOLATE` e AGUARDAR interação do usuário** (jamais rodar `BOOTTRACE NORMAL`/`XRESET`/retries em loop). |
| `OVERVOLTAGE_ABORT` | `VDD > 3.65 V` | **ABORTAR IMEDIATAMENTE.** Risco elétrico no barramento `P7501`; verificar regulador e isolar `NSW5V` (`GP3` Hi-Z). |
| `WDT_STALL` | Indício `WDT`/`STALL`/`TIMEOUT` ou `status` falhou sem leitura `VDD` | Watchdog ou barramento debug AM34 travado; aguardar reset ou rodar `passive_triage`. |
| `DISCONNECTED` | Porta não encontrada ou sem resposta `PONG` | Verificar cabo USB-CDC na sonda Pico (nunca alimentar via `P7501`). |

## 2. Macros Guiadas Somente-Leitura (`SAFE_GUIDED_PROFILES`)

| Profile | Sequência Validada (`SAFE_BENCH_COMMANDS`) | Finalidade |
| :--- | :--- | :--- |
| `passive_triage` | `PING` → `STATUS` → `PINCHECK` → `BUSDIAG` | Diagnóstico completo de saúde elétrica (`VDD`, `Hi-Z`/`Pull` `GP0..GP5`, estado do barramento). |
| `status_dcsr` | `PING` → `STATUS` | Verificação enxuta de pulso CDC + tensão + `DCSR` sem alterar NVRAM. |
| `bus_health` | `PING` → `PINCHECK` → `BUSDIAG` | Auditoria rápida após conectar/desconectar JIG `P7501`. |

## 3. Lista de Bloqueio Incondicional (`SAFETY_DENIED`)

Qualquer chamada em `--mode guided` contendo estes tokens é bloqueada antes de alcançar a porta serial:
- Escrita/mutação NOR/NVRAM/SRAM: `ERASE`, `WRITE`, `FLASH`, `PROGRAM`, `WRITEMEM`, `POKE`, `PATCH`, `MEMW`, `W32`, `W16`, `W8`, `BGO_WRITE`, `NVRAM_WRITE`, `CONFIRM_UPPER_NOR`, `JOB`.
- Pacotes GDB de escrita: prefixos `M80...`, `X80...`.
- Endereço MMIO letal & mutadores BGO conhecidos: `0xC000100C`, `0xC000_100C`, `0x50409DF2`, `0x50409E65`.
