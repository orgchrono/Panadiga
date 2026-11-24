---
name: panadiga-opensource-firmware-dev
description: >-
  Desenvolvimento completo do Panadiga OpenOS bare-metal para Panasonic DIGA AM34 / MN103S
  (firmware/panadiga-openos/ M0->M4 e predecessor firmware/panadiga-blackbox-fw/ PANAMON2>):
  boot seguro em SDRAM volátil (Stage-1.5 0x50400000 512 KiB -> Stage-2 0x57000000 16 MiB)
  sem tocar IPL SA0..SA10, mapa MMIO real (SC0, TBUS 255s, AIOF NTSC/PAL-M/PAL, AVDD, IIC0,
  SDCD, DAVIS CH1 1000ms IDE2SAT2), toolchain mn10300-elf-gcc C11 freestanding, drivers,
  ISRs/DMA, descompressão zlib/LZSS sem heap e testes host -DPANADIGA_OPENOS_HOST_TEST=1.
  Use ao desenvolver o kernel, HAL ou drivers livres do DIGA.
---

# Desenvolvimento do `Panadiga OpenOS` Bare-Metal (`AM34 / MN103S`)

Esta skill governa a evolução de **`firmware/panadiga-openos/`** (`OPENOS>`, roadmap `M0 -> M4`) e de seu protótipo mínimo **`firmware/panadiga-blackbox-fw/`** (`PANAMON2>`) de forma modular em ISO C11 + GNU ASM, testável no host, verificável em `Mn103Vm` e segura para execução em SDRAM volátil no silício real (`VEP79138`).

## 1. Quando Usar & Handoffs
- Ative ao projetar ou revisar arquitetura (`M1..M4`), drivers bare-metal (`uart_sc0.c`, `tbus_ic7501.c`, `aiof_video.c`, `davis_sata.c`, `SDCD`, `IIC0`, `AVDD`, `AVFE`/`MPE4`), rotinas de ISR/DMA, descompressão Stage-1.5 -> Stage-2 sem heap, `boot/crt0_am34.S`, `boot/am34_sdram.ld` ou `kernel/openos_main.c`.
- Para empacotar em `DIGADIAG.FRM` (`262.144 B` raw) ou `PANA__SD.FRM` (`sd48`), use `.agents/skills/panadiga-nvram-unbrick/SKILL.md` (`panadiga frm pack --mode digadiag`) e `.agents/skills/panadiga-emulation-and-patching/SKILL.md`.
- Para executar e capturar no silício real com segurança SMPS/`ADuM`, use `.agents/skills/panadiga-bench-operations/SKILL.md`.
- Para FSMs puras em C11, aritmética ponto fixo/modular e simplificação de drivers determinísticos, combine com `.agents/skills/panadiga-functional-math-and-patterns/SKILL.md`.
- **Referências detalhadas:**
  - Mapa MMIO real, layout `firmware/panadiga-openos/`, roadmap `M0 -> M4` e separação jurídica em [am34-baremetal-bringup-and-roadmap.md](./references/am34-baremetal-bringup-and-roadmap.md).
  - Revisão arquitetural em 2 fases, estilo C11 freestanding, design de drivers, disciplina de ISRs/DMA/WDT, descompressão `zlib`/LZSS sem heap e fault injection host em [openos-c11-drivers-isr-and-compression.md](./references/openos-c11-drivers-isr-and-compression.md).
  - Registradores, `PSW`/`EPSW`, convenção `ABI`, máscara `MOVM`, opcodes e padrões GNU `as` `MN103S/AM34` em [mn103s-am34-assembly-and-abi-cheatsheet.md](./references/mn103s-am34-assembly-and-abi-cheatsheet.md).

## 2. Invariantes de Silício, Arquitetura & Proveniência (NÃO NEGOCIÁVEIS)
1. **Execução Exclusiva em SDRAM Volátil (`Stage-1.5 0x50400000` / `Stage-2 0x57000000`):** Stage-1.5 (`512 KiB`, `__stack_top = 0x5047F000`) nasce via `Loader ver0.61` (`DIGADIAG.FRM` `262.144 B` raw) e carrega Stage-2 (`16 MiB`) sem escrever na Flash NOR — zero risco de brick.
2. **Barreiras Letais & IPL RO (`hal/am34_mmio.h`):** **JAMAIS** ler/escrever `0xC000_100C..0xC000_100F` nem `0xDC0F_0000..0xDC0F_0FFF`; **JAMAIS** escrever em `SA0..SA10` (`0x8000_0000..0x8003_FFFF` e `0x8400_0000..0x8403_FFFF`).
3. **UART0 `SC0` & `TBUS` Keepalive:** `SC0CTR = 0xD400_2000`, `SC0TXB = 0xD400_2008` (8-bit), `SC0STR = 0xD400_200C` (polling `(SC0STR & 0xA0) != 0` com `AM34_OPENOS_SC0_SPIN_LIMIT = 0x00040000UL`). `IC7501 TBUS` (`0xDC01_0010..0xDC01_001F`) precisa de keepalive `255s` orientado a progresso antes de `~44s`.
4. **Padrão `AIOF` & Espera `DAVIS CH1`:** `AIOF` inicia por padrão em **`OPENOS_TV_NTSC` (`0x8020`)** com troca em runtime para `PAL-M` (`0x8021`) e `PAL` (`0x8027`). `DAVIS CH1` (`0x8900_0C00`, reset ativo-ALTO) exige **`OPENOS_SATA_BRIDGE_WAIT_MS = 1000 ms`** para a ponte StarTech `IDE2SAT2` antes de consultar `BSY`/`DRDY`.
5. **Toolchain C11 + Sem MPU/Heap:** `AM34/MN103S` **não tem backend LLVM/Rust** — use `mn10300-elf-gcc` C11 freestanding sem `malloc`; em futura CFI NOR (`S29JL064H`), use **`CLI` + vetores em SDRAM + `WP#/ACC` HIGH (`~700 ms`)**.
6. **Higiene Jurídica (`NOT LEGALLY CLEAN-ROOM`):** Todo arquivo em `firmware/panadiga-openos/` e `firmware/panadiga-blackbox-fw/` deve declarar explicitamente protótipo de interoperabilidade (`NOT LEGALLY CLEAN-ROOM`), nunca `"Clean-Room"`.

## 3. Arquitetura Modular (`firmware/panadiga-openos/`)
- `boot/`: `am34_sdram.ld` (`SDRAM_STAGE15` + `SDRAM_STAGE2`), `crt0_am34.S` (`_start` -> `.bss` zero -> `openos_kernel_entry`).
- `hal/`: `am34_mmio.h` (`_Static_assert` + barreiras `am34_mmio_is_forbidden` / `am34_mmio_write_allowed` + seam `-DPANADIGA_OPENOS_HOST_TEST=1`).
- `drivers/`: `openos_drivers.h`, `uart_sc0.c`, `tbus_ic7501.c`, `aiof_video.c`, `davis_sata.c` (e próximos `iic_bus.c`, `avdd_video.c`, `sdcd_sd.c`, `avfe_mpeg.c`).
- `kernel/`: `openos_main.c` (`openos_kernel_entry`, futuro scheduler/shell `OPENOS>` e chainloader Stage-2).
- `tests/`: `test_openos_host.c` (suíte host C11 negativa + positiva).

## 4. Fluxo de Build, Simulação & Bring-Up em Bancada
```bash
# 1. Testes host C11 (barreiras MMIO, TBUS 255s, NTSC/PAL-M, espera 1000ms DAVIS CH1)
gcc -std=c11 -Wall -Wextra -Werror -DPANADIGA_OPENOS_HOST_TEST=1 -Ifirmware/panadiga-openos \
  firmware/panadiga-openos/drivers/*.c firmware/panadiga-openos/kernel/openos_main.c \
  firmware/panadiga-openos/tests/test_openos_host.c -o /tmp/test_openos_host && /tmp/test_openos_host

# 2. Cross-compilação reproduzível em container mn10300-elf-gcc
mn10300-elf-gcc -std=c11 -Os -Wall -Wextra -Werror -ffreestanding -nostdlib \
  -Ifirmware/panadiga-openos -T firmware/panadiga-openos/boot/am34_sdram.ld \
  firmware/panadiga-openos/boot/crt0_am34.S firmware/panadiga-openos/drivers/*.c \
  firmware/panadiga-openos/kernel/openos_main.c -o tmp/panadiga_openos.elf
mn10300-elf-objcopy -O binary tmp/panadiga_openos.elf tmp/panadiga_openos.bin

# 3. Empacotar como DIGADIAG.FRM (262.144 B raw para Loader ver0.61) e validar limites em Mn103Vm
cargo run -q -p panadiga-cli -- frm pack --mode digadiag -i tmp/panadiga_openos.bin -o tmp/DIGADIAG.FRM --dst-addr 0x50400000 -j -
cargo run -q -p panadiga-cli -- unbrick tbc-plan --mode digadiag_ram_tbc --standard pal-m -j -
cargo run -q -p panadiga-cli -- re verify-patches --json

# 4. Execução real sob isolamento ADuM3160 + captura Pico P7501 (115200 8N1)
```

## 5. Armadilhas & Verificação
- **Armadilhas:** Usar `volatile` achando que garante atomicidade entre ISR e loop principal; bloquear a janela `0x8900_0080..0x8A00_0000` (`DAVIS`/`SDCD`/`AVDD`); consultar `DAVIS CH1` sem aguardar `1000 ms` atrás do `IDE2SAT2`; alimentar `IC7501 TBUS` de dentro de um timer ISR cego; testar sem `ADuM3160`.
- **Verificação:** `test_openos_host` (`100%` PASS em `-Wall -Wextra -Werror`), ELF sem símbolos indefinidos nem extrapolar `512 KiB` em `DIAG_PROG_RAM`, `panadiga re verify-patches` verde em `Mn103Vm` (`N2_EMULATED`) e `OPENOS>` + `TBUS` estável > 10 min na `COM10` antes de declarar avanço `M0..M4` (`N4_HARDWARE`).
