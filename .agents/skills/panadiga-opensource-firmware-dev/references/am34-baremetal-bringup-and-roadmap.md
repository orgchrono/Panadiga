# Mapa MMIO Real `AM34`, ABI Bare-Metal, Roadmap `M0 -> M3` & Separação Jurídica

## 1. Mapa MMIO Verificado para Drivers Bare-Metal (`AM34 / MN2DS0009`)

| Subsistema | Endereço Base / Registrador | Valores & Protocolo Verificado | Notas de Segurança |
| :--- | :--- | :--- | :--- |
| **UART0 (`SC0`)** | `SC0CTR = 0xD400_2000` · `SC0TXB = 0xD400_2008` (8-bit) · `SC0STR = 0xD400_200C` | Polling: `while ((*SC0STR & 0xA0) != 0 && spin < 0x40000)` (`TXBUSY \| TXBF`) | Countdown finito obrigatório; `115200 8N1` |
| **Timer Watchdog `TBUS`** | `0xDC01_0000..0xDC01_001F` (`0xDC01_0010` mailbox) | Frame 255s: `[0x8A, 0x02, 0x00, 0xFF, ..., 0x30]` | Sem keepalive, `IC7501` derruba energia em `~44s` |
| **Vídeo `AIOF` (TBC/Sync)** | `0x8800_0000..0x8800_01FF` | `AIOF_V +0x66 = 0x8001`; `VMODESEL`: `0x8020` NTSC, `0x8021` `PAL-M`, `0x8027` PAL | Verificado em `MAIN` (`82` descritores / `98` presets) |
| **Vídeo `AVDD` (VDEC/VENC)** | `0x8980_0000` | Front-end / back-end analógico DAC/ADC | Só configurar após clock/power estabilizado |
| **I²C (`IIC0` / `IIC1`)** | `0xD840_0000` · `0xD840_1000` | Roteia matriz A/V `IC3001` (`0x4A` 7-bit / `0x94` 8-bit) | Necessário para passthrough analógico na placa Main |
| **SD Card (`SDCD`)** | `0x8900_1800` · `0x8900_1850` (4-bit) | Acesso SDSC FAT16 (`<= 2 GiB`) | Base para Stage-2 chainload sem HDD |
| **ATA/IDE `DAVIS`** | `CH0 = 0x8900_0400` (`0x8900_0420=0x02`) · `CH1 = 0x8900_0C00` (`0x8900_0C20=0x00`) | `CH0` reset ativo-BAIXO; `CH1` reset ativo-ALTO | **JAMAIS acessar `0xC000_100C`** (e evitar `0xDC0F_0000`) |
| **Flash NOR `S29JL064H`** | `0x8000_0000..0x807F_FFFF` (cached) · `0x8400_0000..0x847F_FFFF` (uncached) | `SA0..SA10` (`256 KiB`) **RO permanente** | CFI exige **`CLI` + vetores SDRAM + `WP#/ACC` HIGH + `~700 ms`** |

## 2. Convenções de Toolchain, `crt0_am34.S` & Testes Host
- **Toolchain ELF (`mn10300-elf-gcc`):** Verifique se o compilador gera símbolos sem underscore (`main` vs `_main`, via `nm panamon2_tbc.elf`) antes de ligar `crt0_am34.S`.
- **Sem `libgcc` acidental:** Em `-nostdlib -Os`, evite divisão/módulo 64-bit (`uint64_t /`) sem prover `udivdi3` ou implemente helpers 32-bit determinísticos.
- **Testabilidade Host (`-DPANADIGA_FW_HOST_TEST=1`):** Toda função de driver recebe ou chama `am34_mmio_read32`/`am34_mmio_write32` abstratos para validar sequências exatas de inicialização de registradores em CI (sem precisar do cross-compiler instalado no host).

## 3. Roadmap Incremental (`M0 -> M3`)

| Marco | Objetivo Entregável | Critério de Pronto (Gate de Passagem) |
| :--- | :--- | :--- |
| **`M0`** | `crt0` + `SC0` (`0xD4002000`/`2008`/`200C`) + `TBUS 255s` keepalive + `PANAMON2>` | Boot em `0x50400000` via `DIGADIAG.FRM`; prompt vivo > 10 min sem queda `IC7501` |
| **`M1`** | `IIC0` (`IC3001`) + `AIOF` (`PAL-M`/`NTSC`/`PAL`) + `AVDD` | Vídeo E-E e barra/TBC estáveis em saída analógica sem HDD conectado |
| **`M2`** | `SDCD` 4-bit FAT16 + `DAVIS CH0/CH1` bare-metal | Leitura/escrita SD funcional + `0xEC IDENTIFY` e setores HDD LBA sem travamento `80000404` |
| **`M3`** | Stage-2 Loader open-source carregado de SD/HDD para SDRAM principal (`0x5700_0000` / livre) | Sistema aberto completo bootando em SDRAM enquanto `SA0..SA10` IPL original segue intacto |

## 4. Separação Jurídica (Interoperabilidade vs Clean-Room Real)
- **`firmware/panadiga-blackbox-fw/` (no mesmo repo que disassembly):** é legalmente um **Protótipo Open-Source de Interoperabilidade / Feasibility Spike**. Nunca o rotule como `"Clean-Room"` nos cabeçalhos `.S`, `.ld` ou `.c`.
- **Se no futuro desejarem implementação Clean-Room pura:** crie repositório separado com *Chinese-wall* rígido — Time A documenta apenas interface física/MMIO em especificação limpa; Time B implementa do zero sem jamais ler disassembly ou strings proprietárias.
