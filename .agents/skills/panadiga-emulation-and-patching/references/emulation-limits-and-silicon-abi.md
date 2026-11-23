# Limites de Emulação, ABI Real de Silício & Sítios Verificados

## 1. O que `Mn103Vm` Valida vs O que Exige Silício Real
| Aspecto | Coberto pelo `Mn103Vm` (`N2_EMULATED`) | Exige Hardware Real (`N4_HARDWARE`) |
| :--- | :--- | :--- |
| Decodificação ISA `MN103S` & desvios básicos | Sim (ex.: `0xC6 bls -> 0xCA bra`) | Confirmação após carga completa em SDRAM |
| Fronteiras estáticas Flash & bloqueio `0xC000_100C` | Sim (`E05` em `SA0..SA10`) | Sem hardware MPU real: ponteiros soltos travam o SoC |
| Apagamento/gravação CFI Spansion `S29JL064H` | Sequência sintética `CfiStage` | **`CLI` + vetores em SDRAM obrigatórios** antes de sair de `Read Array` + polling real `~700 ms` + Pino 14 `WP#/ACC=HIGH` |
| Watchdog Supervisor `IC7501` (`TBUS`) | Simula pacote `M0` (`[0x8A, 0x02, 0x00, 0xFF, ..., 0x30]`) | Reset real de rails em ~`44s` se `TBUS` não for atendido |
| Coerência I/D-Cache, Wait-States & DMA | Não modelado | Obrigatório validar em hardware com isolamento SMPS/`ADuM` |

## 2. Regras de ABI para Stubs MN103S Reais
- **Chamadas:** `100%` das chamadas observadas (`1.386` em `PROG061`, `1.065` em `BOOT.BIN`) usam `CALL target, [], 0x00` / `RET ..., 0x00|0x20`, gerenciando stack explicitamente (`ADD -N, SP` / `ADD +N, SP`).
- **UART0 `SC0STR` (`0xD400200C`):** Máscara `0xA0` (`TXBUSY | TXBF`) exige countdown finito (`0x1400000` fábrica / `0x00040000` em `PUTC_D3_BYTES`), jamais loop infinito.
- **Buffer RX `SC0RXB`:** Apenas 1 byte de profundidade; protocolo cliente sobre `PANAMON1>` deve espaçar envio em `300–350 µs`/byte drenando eco.

## 3. Endereços & Assinaturas Verificadas (`3410PQ` / `3060NP` / `PROG061`)
| Alvo / Módulo | VA / Endereço Verificado | Comportamento & Observação |
| :--- | :--- | :--- |
| **`PROG061` -> `PANAMON1>`** | `0x5040A2BC` (`BRA +17 -> 0x5040A2CD`) -> `0x50439494` | `PANAMON1>` em UART0 (`B/R/W/D/F/J/E/U/Q`; `U CONFIRM_UPPER_NOR` / `U LOCK`; `F` rejeitado fail-closed) |
| **`MAIN` `MaxProductLBA`** | `3410PQ: 0x574DD1FC` · `3060NP: 0x574E499C` | `site_va + 0x4E..0x6F`: `0xC6 (bls +12) -> 0xCA (bra +12)` (`5 GB..2 TB` / `4 TiB`) |
| **`MAIN` `<deviceinfo>` (21 B)** | `3410PQ: 0x57498738` · `3060NP: 0x574A01AA` | `pre_update_firm.c` (`"[fmu] <deviceinfo> model = %s, oem = %c"`) |
| **`MAIN` `main_passthrough`** | `3410PQ`: `0x5771ABFB`, `0x574F7B8C`, `0x574F7C5C` | Só roda pós-carga do `MAIN`; ainda não contorna `VDM_Init` (`0x57323EC3`), `SDM_Mount` (`0x574f5c6a`) e `"HDD Err -> P-OFF!"` (`0x574f469d`) |
| **`AIOF` / `AVDD` / `IC3001`** | `AIOF 0x8800_0000` · `AVDD 0x8980_0000` · `IIC0 0xD840_0000` | `82` descritores (`0x578935FC`), `98` presets (`0x578957D8`): `cps_ntsc=0x8020`, `cps_pal_m=0x8021`, `cps_pal=0x8027`; `IC3001` I²C `0x4A` (7-bit) / `0x94` (8-bit) |
| **`DAVIS` CH0/CH1 & MMIO Letal** | `CH0 0x89000420` · `CH1 0x89000C20` · **`0xC000_100C` PROIBIDO** | `CH0` ativo-BAIXO (`0x02`); `CH1` ativo-ALTO (`0x00`); jamais ler/escrever `0xC000_100C` |
