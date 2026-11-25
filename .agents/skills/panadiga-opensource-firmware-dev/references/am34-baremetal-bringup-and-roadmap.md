# Mapa MMIO Real `AM34`, ABI Bare-Metal, `DigaOS` (`M0 -> M4`) & Separação Jurídica

## 1. Mapa MMIO Verificado para Drivers Bare-Metal (`AM34 / MN2DS0009`)

| Subsistema | Endereço Base / Registrador | Valores & Protocolo Verificado | Notas de Segurança |
| :--- | :--- | :--- | :--- |
| **UART0 (`SC0`)** | `SC0CTR = 0xD400_2000` · `SC0TXB = 0xD400_2008` (8-bit) · `SC0RXB = 0xD400_2009` · `SC0STR = 0xD400_200C` | Init `SC0CTR = 0x0080`; polling `while ((*SC0STR & 0xA0) != 0 && spin < spin_limit)` (`TXBUSY \| TXBF`) | Countdown finito obrigatório; `115200 8N1` |
| **Timer Watchdog `TBUS`** | `0xDC01_0000..0xDC01_001F` (`0xDC01_0010` mailbox) | Frame 255s: `[0x8A, 0x02, 0x00, 0xFF, ..., 0x30]`; VFD cmd `0xA3`; strobe `0x10`; checksum `(sum(body15) - 0x5B) & 0xFF` | Sem keepalive, `IC7501` derruba energia em `~44s` |
| **Vídeo `AIOF` (TBC/Sync)** | `0x8800_0000..0x8800_01FF` | `AIOF_V +0x66 = 0x8001`; `VMODESEL (+0x68)`: `0x8020` `NTSC` (padrão DigaOS), `0x8021` `PAL-M`, `0x8027` `PAL` | Verificado em `MAIN` (`82` descritores / `98` presets) |
| **Vídeo `AVDD` (VDEC/VENC)** | `0x8980_0000` | Front-end / back-end analógico DAC/ADC (`avdd_iic_video.c`) | Só configurar após clock/power estabilizado |
| **I²C (`IIC0` / `IIC1`)** | `0xD840_0000` · `0xD840_1000` | Roteia matriz A/V `IC3001` (`0x94` `C1AB2379` EC / `0x90`/`0x92` `BH7629KS2` BR/NA) | Necessário para passthrough analógico na placa Main |
| **SD Card (`SDCD`)** | `0x8900_1800` · `0x8900_1850` (4-bit) | Acesso SDSC FAT16 (`<= 2 GiB`) | Base para Stage-2 chainload sem HDD |
| **ATA/IDE `DAVIS`** | `CH0 = 0x8900_0400` (`+0x20=0x02`) · `CH1 = 0x8900_0C00` (`+0x20=0x00`) | `CH0` reset ativo-BAIXO (`2 ms`); `CH1` reset ativo-ALTO + **`1000 ms`** de espera (`IDE2SAT2`) antes de consultar `BSY`/`DRDY` | **JAMAIS acessar `0xC000_100C`** nem `0xDC0F_0000..0xDC0F_0FFF` |
| **Flash NOR `S29JL064H`** | `0x8000_0000..0x807F_FFFF` (cached) · `0x8400_0000..0x847F_FFFF` (uncached) | `SA0..SA10` (`256 KiB`) **RO permanente (`E05`)** | CFI exige **`CLI` + vetores SDRAM + `WP#/ACC` HIGH + `~700 ms`** |

## 2. Layout de Código & Boot SDRAM (`firmware/digaos/`)

- **Alvo Principal (`firmware/digaos/`):** substitui o `PiE-OS` proprietário apenas em SDRAM volátil via `Loader ver0.61` (incorporando `SW_GUARDS`, `DIGAOS_TV_SECAM` e `probe_tbc_aiof_baseline` no núcleo unificado):
  - `boot/am34_sdram.ld`: define `SDRAM_STAGE15` (`0x5040_0000`, `512 KiB`), `SDRAM_STAGE2` (`0x5700_0000`, `16 MiB`) e `__stack_top = 0x5047F000`.
  - `boot/crt0_am34.S`: configura `SP = __stack_top`, zera `.bss` e chama `digaos_kernel_entry()` via ABI `CALL target, [], 0x00`.
  - `hal/am34_mmio.h` & `hal/digaos_soc_stubs.{h,c}`: impõem `_Static_assert` e guardas de runtime (`DIGAOS_SSOT_LETHAL_MMIO_START`, `DIGAOS_SSOT_SYSCON_DANGER_*`, IPL RO, fail-closed `UNVERIFIED_MMIO` multi-SoC); chaveiam entre MMIO real e hooks mock via `-DPANADIGA_DIGAOS_HOST_TEST=1`.
  - `generated/digaos_ssot.h` & `digaos_ssot_tables.c`: tabelas de menus, temas RGB565, power profiles e multi-SoC targets projetadas via SSOT Rust.
  - `drivers/`: `digaos_drivers.h`, `uart_sc0.c`, `tbus_ic7501.c`, `aiof_video.c`, `avdd_iic_video.c`, `davis_sata.c`, `sdcd_sd.c`, `nor_bgo_ro.c`, `storage_provisioner.c`.
  - `kernel/digaos_main.c`: executa bring-up inicial e shell interativo `DIGAOS>`.
  - `tests/test_digaos_host.c`: suíte host C11 validando barreiras `0xC000_100C`/`0xDC0F_0000..0xDC0F_0FFF`/`SA0..SA10`, checksum `0x30`, banner `DIGAOS>`, `NTSC` padrão + alternância `PAL-M`/`PAL`/`SECAM`, `IC3001`, storage provisioner e espera de `1000 ms` no `DAVIS CH1`.
- **Toolchain ELF (`mn10300-elf-gcc`):** Verifique símbolos com `nm` (`digaos_kernel_entry` vs `_digaos_kernel_entry`) antes de ligar `crt0_am34.S`; em `-nostdlib -Os`, evite divisão 64-bit (`libgcc`) e mantenha stubs determinísticos.

## 3. Roadmap Incremental `DigaOS` (`M0 -> M4`)

| Marco | Objetivo Entregável | Critério de Pronto (Gate de Passagem) |
| :--- | :--- | :--- |
| **`M0`** | `boot/am34_sdram.ld`, `crt0_am34.S`, `SC0` UART0 (`115200 8N1`), `hal/am34_mmio.h` | Boot em `0x5040_0000` via `DIGADIAG.FRM`; banner `DIGAOS>` vivo na `P7501` |
| **`M1`** | `IC7501` `TBUS` keepalive `255s` + display VFD (`0xA3`) + decode IR RX | Estabilidade dos rails > 10 min sem desligamento de `~44 s` pelo `IC7501` |
| **`M2`** | `IIC1` (`IC3001`) + `AIOF` (`NTSC` padrão `0x8020`, `PAL-M` `0x8021`, `PAL` `0x8027`, `SECAM`) + `AVDD` | Vídeo E-E, color bar S-Video e TBC estáveis sem HDD conectado |
| **`M3`** | `SDCD` 4-bit FAT16 + `DAVIS` CH0/CH1 ATA (`1000 ms` `IDE2SAT2`) + Open-FS/UDF + Stage-2 (`zlib`/LZSS sem heap) | `0xEC IDENTIFY` confiável atrás da ponte `IDE2SAT2` + boot Stage-2 em `0x5700_0000` |
| **`M4`** | Codec `AVFE` / `MPE4` MPEG-2 encoder/decoder + DSP de áudio | Captura e playback DVR open-source sem blobs proprietários Panasonic |

## 4. Separação Jurídica (Interoperabilidade vs Clean-Room Real)
- **`firmware/digaos/` (no mesmo repo que disassembly):** é legalmente **Implementação Open-Source de Interoperabilidade / Substituição Livre (`NOT LEGALLY CLEAN-ROOM`)** — escrita do zero em ISO C11 + GNU ASM sem blobs binários Panasonic. Nunca use comentários `"Clean-Room"` nos cabeçalhos `.S`, `.ld`, `.c` ou `.h`.
- **Se no futuro desejarem implementação Clean-Room pura:** crie repositório separado com *Chinese-wall* rígido — Time A documenta apenas interface física/MMIO em especificação limpa; Time B implementa do zero sem jamais ler disassembly ou strings proprietárias.
