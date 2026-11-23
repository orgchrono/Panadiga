---
name: panadiga-opensource-firmware-dev
description: >-
  Desenvolvimento completo da firmware open-source bare-metal para Panasonic DIGA AM34 /
  MN103S (firmware/panadiga-blackbox-fw/ e evolução Stage-1.5/Stage-2): bring-up seguro em
  SDRAM volátil 0x50400000 sem tocar IPL SA0..SA10, mapa MMIO real verificado (SC0 UART0
  0xD4002008/0xD400200C, IC7501 TBUS 0xDC010000 keepalive 255s, AIOF 0x88000000, AVDD
  0x89800000, IIC0 0xD8400000, DAVIS 0x89000400/0x89000C00, SDCD 0x89001800), toolchain
  mn10300-elf-gcc, HAL + mock MMIO host tests, validação Mn103Vm, roadmap M0->M3 e higiene
  jurídica (interoperabilidade vs clean-room). Use ao desenvolver a firmware livre do DIGA.
---

# Desenvolvimento da Firmware Open-Source Bare-Metal (`AM34 / MN103S`)

Esta skill governa a evolução de `firmware/panadiga-blackbox-fw/` de um protótipo mínimo em SDRAM para uma firmware livre modular, testável no host, verificável em `Mn103Vm` e segura para executar em silício real (`VEP79138`).

## 1. Quando Usar & Handoffs
- Ative ao projetar drivers bare-metal do AM34 (`SC0`, `TBUS`, `IIC0`, `AIOF`, `AVDD`, `SDCD`, `DAVIS`, `S29JL064H`), evoluir o monitor `PANAMON2>`, ajustar `crt0_am34.S`/`am34_sdram.ld` ou construir o Stage-2 open-source.
- Para empacotar o binário gerado em `DIGADIAG.FRM` (`262.144 B` raw) ou `PANA__SD.FRM` (`sd48`), use `.agents/skills/panadiga-nvram-unbrick/SKILL.md` (`panadiga frm pack --mode digadiag`) e `.agents/skills/panadiga-emulation-and-patching/SKILL.md`.
- Para executar e capturar no silício real com segurança SMPS/`ADuM`, use `.agents/skills/panadiga-bench-operations/SKILL.md`.
- Mapa MMIO real completo, convenções ABI/CRT0/Linker, harness de testes host, roadmap `M0 -> M3` e separação jurídica em [am34-baremetal-bringup-and-roadmap.md](./references/am34-baremetal-bringup-and-roadmap.md).

## 2. Invariantes de Silício & Proveniência (NÃO NEGOCIÁVEIS)
1. **Execução Exclusiva em SDRAM Volátil (`0x50400000`, `256 KiB`):** O código nasce na RAM via `DIGADIAG.FRM` / Porta #1 (`JMP 0x50400000`) sem escrever na Flash NOR. Se algo travar, basta ciclar energia — **zero risco de brick permanente**.
2. **Proibição Absoluta de MMIO Letal & IPL RO:**
   - **JAMAIS** ler ou escrever **`0xC000_100C..0xC000_100F`** (trava barramento/WDT) nem `0xDC0F_0000..0xDC0F_0FFF`.
   - **JAMAIS** escrever em **`SA0..SA10`** (`0x8000_0000..0x8003_FFFF` e `0x8400_0000..0x8403_FFFF`).
3. **Endereços Reais UART0 `SC0` (Nunca Inventar MMIO):** Base `SC0CTR = 0xD400_2000`, `SC0TXB = 0xD400_2008` (escrita 8-bit `uint8_t`/`MOVBU` para não invadir `SC0RXB` em `+0x09`), `SC0STR = 0xD400_200C` (aguardar enquanto `(SC0STR & 0xA0) != 0` com countdown finito `0x0004_0000`, `TXBUSY | TXBF`) — **não** `0x1000_0000` nem inversão de flag `== 0`.
4. **Sem Hardware MPU + Requisitos para CFI NOR Futura:** Stage-1.5 roda sem MPU de hardware (use wrappers `am34_safe_read/write` com `_Static_assert`). Qualquer futuro driver de Flash CFI (`S29JL064H`) **DEVE** desligar IRQs (`CLI`) e apontar tabela de vetores para SDRAM antes de tirar a NOR de `Read Array` (~`700 ms` por erase + Pino 14 `WP#/ACC` HIGH).
5. **Keepalive `IC7501 TBUS` (`~44s`):** O supervisor `IC7501` derruba os rails em `~44s` se a firmware livre não enviar os quadros keepalive `255s` em `0xDC01_0010..0xDC01_001F`.
6. **Higiene Jurídica & Epistêmica:** Todo código em `firmware/panadiga-blackbox-fw/` neste repositório deve ser rotulado honestamente como **Protótipo Open-Source de Interoperabilidade (NOT LEGALLY CLEAN-ROOM)** — jamais usar comentários `"Clean-Room"` indevidos!

## 3. Arquitetura Modular Alvo (`firmware/panadiga-blackbox-fw/`)
Separe a firmware livre em camadas limpas e testáveis no host (sem god-file único):
- `boot/`: `am34_sdram.ld` (`0x50400000..0x5043FFFF`, stack `0x5043C000..0x5043FFFF`), `crt0_am34.S` (alinha `SP`, zera `.bss`, respeita ABI `CALL target, [], 0x00` e convenção de símbolo `main` vs `_main`).
- `hal/`: `am34_mmio.h` (barramento virtualizável: em silício lê/escreve `volatile uint32_t *`/`uint8_t *` com guardas `0xC000_100C` + IPL; em `-DPANADIGA_FW_HOST_TEST=1` chama mock bus auditável!).
- `drivers/`: um módulo por periférico (`uart_sc0.c`, `tbus_wdt.c`, `iic_bus.c`, `aiof_tbc.c`, `avdd_video.c`, `sdcd_sd.c`, `davis_ide.c`).
- `shell/`: monitor `PANAMON2>` extensível, comandos diagnósticos e script de boot M0..M3.
- `tests/`: suíte C11 host testando guardas MMIO, construção do frame `TBUS` 255s, sequência de inicialização `AIOF`/`IC3001`/`DAVIS` antes de tocar silício.

## 4. Fluxo de Build, Simulação & Bring-Up em Bancada
```bash
# 1. Testes host C11 (mock MMIO bus verificando zero acessos a 0xC000_100C e IPL)
# Compilar testes de drivers com -std=c11 -Wall -Wextra -Werror -DPANADIGA_FW_HOST_TEST=1

# 2. Cross-compilação reproduzível em container mn10300-elf-gcc
mn10300-elf-gcc -std=c11 -Os -Wall -Wextra -Werror -ffreestanding -nostdlib \
  -T am34_sdram.ld crt0_am34.S <modulos.c> -o panamon2_tbc.elf
mn10300-elf-objcopy -O binary panamon2_tbc.elf panamon2_tbc.bin

# 3. Empacotar como DIGADIAG.FRM (262.144 B raw para Loader ver0.61) e validar limites em Mn103Vm
cargo run -q -p panadiga-cli -- frm pack --mode digadiag -i tmp/panamon2_tbc.bin -o tmp/DIGADIAG.FRM --dst-addr 0x50400000 -j -
cargo run -q -p panadiga-cli -- unbrick tbc-plan --mode stage15_digadiag_tbc --standard pal-m -j -
cargo run -q -p panadiga-cli -- re verify-patches --json

# 4. Execução real sob isolamento ADuM3160 + captura Pico P7501 (115200 8N1)
```

## 5. Armadilhas & Verificação
- **Armadilhas:** Bloquear por engano a janela `0x8900_0080..0x8A00_0000` (impede acesso legítimo a `DAVIS CH0/CH1`, `SDCD` e `AVDD`); esquecer tamanho exato `262.144 B` raw em `DIGADIAG.FRM` para `Loader ver0.61`; testar no hardware sem `ADuM3160` ou sem watchdog `TBUS` armado.
- **Verificação:** `100%` dos testes host MMIO passando (`-Wall -Wextra -Werror`), ELF sem símbolos indefinidos nem extrapolar `256 KiB`, `panadiga re verify-patches` verde em `Mn103Vm` e banner `PANAMON2>` + keepalive estável > 10 min na captura `COM10` antes de avançar de `M0` para `M1..M3`.
