---
name: panadiga-opensource-firmware-dev
description: >-
  Desenvolvimento completo do DigaOS bare-metal para Panasonic DIGA AM34 / MN103S
  (firmware/digaos/ M0->M4, com multi-SoC HAL fail-closed UNVERIFIED_MMIO e SSOT digaos_ssot.h):
  boot seguro em SDRAM volátil (Stage-1.5 0x50400000 512 KiB -> Stage-2 0x57000000 16 MiB)
  sem tocar IPL SA0..SA10, mapa MMIO real (SC0, TBUS 255s, AIOF NTSC/PAL-M/PAL/SECAM, AVDD,
  IIC1, SDCD, DAVIS CH1 1000ms IDE2SAT2), toolchain mn10300-elf-gcc C11 freestanding, drivers,
  ISRs/DMA, descompressão zlib/LZSS sem heap e testes host -DPANADIGA_DIGAOS_HOST_TEST=1.
  Use ao desenvolver o kernel, HAL ou drivers livres do DIGA.
---

# Desenvolvimento do `DigaOS` Bare-Metal (`AM34 / MN103S`)

Esta skill governa a evolução de **`firmware/digaos/`** (`DIGAOS>`, roadmap `M0 -> M4`, incluindo os módulos `SW_GUARDS`, `DIGAOS_TV_SECAM` e `probe_tbc_aiof_baseline` absorvidos) de forma modular em ISO C11 + GNU ASM, testável no host, verificável em `Mn103Vm` e segura para execução em SDRAM volátil no silício real (`VEP79138`).

## 1. Quando Usar & Handoffs
- Ative ao projetar ou revisar arquitetura (`M1..M4`), drivers bare-metal (`uart_sc0.c`, `tbus_ic7501.c`, `aiof_video.c`, `avdd_iic_video.c`, `davis_sata.c`, `sdcd_sd.c`, `nor_bgo_ro.c`, `storage_provisioner.c`), HAL (`am34_mmio.h`, `digaos_soc_stubs.{h,c}`), rotinas de ISR/DMA, descompressão Stage-1.5 -> Stage-2 sem heap, `boot/crt0_am34.S`, `boot/am34_sdram.ld` ou `kernel/digaos_main.c`.
- Para empacotar em `DIGADIAG.FRM` (`262.144 B` raw) ou `PANA__SD.FRM` (`sd48`), use `.agents/skills/panadiga-nvram-unbrick/SKILL.md` (`panadiga frm pack --mode digadiag`) e `.agents/skills/panadiga-emulation-and-patching/SKILL.md`.
- Para executar e capturar no silício real com segurança SMPS/`ADuM`, use `.agents/skills/panadiga-bench-operations/SKILL.md`.
- Para FSMs puras em C11, aritmética ponto fixo/modular e simplificação de drivers determinísticos, combine com `.agents/skills/panadiga-functional-math-and-patterns/SKILL.md`.
- **Referências detalhadas:**
  - Mapa MMIO real, layout `firmware/digaos/`, roadmap `M0 -> M4` e separação jurídica em [am34-baremetal-bringup-and-roadmap.md](./references/am34-baremetal-bringup-and-roadmap.md).
  - Revisão arquitetural em 2 fases, estilo C11 freestanding, design de drivers, disciplina de ISRs/DMA/WDT, descompressão `zlib`/LZSS sem heap e fault injection host em [openos-c11-drivers-isr-and-compression.md](./references/openos-c11-drivers-isr-and-compression.md).
  - Registradores, `PSW`/`EPSW`, convenção `ABI`, máscara `MOVM`, opcodes e padrões GNU `as` `MN103S/AM34` em [mn103s-am34-assembly-and-abi-cheatsheet.md](./references/mn103s-am34-assembly-and-abi-cheatsheet.md).

## 2. Invariantes de Silício, Arquitetura & Proveniência (NÃO NEGOCIÁVEIS)
1. **Execução Exclusiva em SDRAM Volátil (`Stage-1.5 0x50400000` / `Stage-2 0x57000000`):** Stage-1.5 (`512 KiB`, `__stack_top = 0x5047F000`) nasce via `Loader ver0.61` (`DIGADIAG.FRM` `262.144 B` raw) e carrega Stage-2 (`16 MiB`) sem escrever na Flash NOR — zero risco de brick.
2. **Barreiras Letais & IPL RO (`hal/am34_mmio.h`):** **JAMAIS** ler/escrever `0xC000_100C..0xC000_100F` nem `0xDC0F_0000..0xDC0F_0FFF`; **JAMAIS** escrever em `SA0..SA10` (`0x8000_0000..0x8003_FFFF` e `0x8400_0000..0x8403_FFFF`).
3. **UART0 `SC0` & `TBUS` Keepalive:** `SC0CTR = 0xD400_2000`, `SC0TXB = 0xD400_2008` (8-bit), `SC0STR = 0xD400_200C` (polling `(SC0STR & 0xA0) != 0` com spin finito em `am34_mmio.h`). `IC7501 TBUS` (`0xDC01_0010..0xDC01_001F`) precisa de keepalive `255s` orientado a progresso antes de `~44s`.
4. **Padrão `AIOF` & Espera `DAVIS CH1`:** `AIOF` inicia por padrão em **`DIGAOS_TV_NTSC` (`0x8020`)** com troca em runtime para `DIGAOS_TV_PAL_M` (`0x8021`), `DIGAOS_TV_PAL` (`0x8027`) e `DIGAOS_TV_SECAM`. `DAVIS CH1` (`0x8900_0C00`, reset ativo-ALTO) exige **`1000 ms`** de espera para a ponte StarTech `IDE2SAT2` antes de consultar `BSY`/`DRDY`.
5. **Toolchain C11 + Sem MPU/Heap:** `AM34/MN103S` **não tem backend LLVM/Rust** — use `mn10300-elf-gcc` C11 freestanding sem `malloc`; em futura CFI NOR (`S29JL064H`), use **`CLI` + vetores em SDRAM + `WP#/ACC` HIGH (`~700 ms`)**.
6. **Higiene Jurídica (`NOT LEGALLY CLEAN-ROOM`):** Todo arquivo `.c`, `.S`, `.ld` e `README.md` em `firmware/digaos/` deve declarar explicitamente protótipo de interoperabilidade (`NOT LEGALLY CLEAN-ROOM`), nunca `"Clean-Room"`.

## 3. Arquitetura Modular (`firmware/digaos/`)
- `boot/`: `am34_sdram.ld` (`SDRAM_STAGE15` + `SDRAM_STAGE2`), `crt0_am34.S` (`_start` -> `.bss` zero -> `digaos_kernel_entry`).
- `hal/`: `am34_mmio.h` (`_Static_assert` + barreiras via `generated/digaos_ssot.h` + `-DPANADIGA_DIGAOS_HOST_TEST=1`), `digaos_soc_stubs.{h,c}` (fail-closed `UNVERIFIED_MMIO` multi-SoC).
- `generated/`: `digaos_ssot.h` e `digaos_ssot_tables.c` gerados pela SSOT Rust.
- `drivers/`: `digaos_drivers.h`, `uart_sc0.c`, `tbus_ic7501.c`, `aiof_video.c`, `avdd_iic_video.c`, `davis_sata.c`, `sdcd_sd.c`, `nor_bgo_ro.c`, `storage_provisioner.c`.
- `kernel/`: `digaos_main.c` (`digaos_kernel_entry`, shell interativo `DIGAOS>`).
- `tests/`: `test_digaos_host.c` (suíte host C11 negativa + positiva).

## 4. Fluxo de Build, Simulação & Bring-Up em Bancada
```bash
# 1. Testes host C11 (barreiras MMIO, TBUS 255s, NTSC/PAL-M/PAL/SECAM, espera 1000ms DAVIS CH1, SSOT, DIGAOS>)
python scripts/validate_c11.py

# 2. Cross-compilação reproduzível em container mn10300-elf-gcc
mn10300-elf-gcc -std=c11 -Os -Wall -Wextra -Werror -ffreestanding -nostdlib \
  -Ifirmware/digaos -T firmware/digaos/boot/am34_sdram.ld \
  firmware/digaos/boot/crt0_am34.S firmware/digaos/drivers/*.c \
  firmware/digaos/hal/digaos_soc_stubs.c firmware/digaos/generated/digaos_ssot_tables.c \
  firmware/digaos/kernel/digaos_main.c -o tmp/digaos.elf
mn10300-elf-objcopy -O binary tmp/digaos.elf tmp/digaos.bin

# 3. Empacotar como DIGADIAG.FRM (262.144 B raw para Loader ver0.61) e validar limites em Mn103Vm
cargo run -q -p panadiga-cli -- frm pack --mode digadiag -i tmp/digaos.bin -o tmp/DIGADIAG.FRM --dst-addr 0x50400000 -j -
cargo run -q -p panadiga-cli -- unbrick tbc-plan --mode digadiag_ram_tbc --standard pal-m -j -
cargo run -q -p panadiga-cli -- re verify-patches --json

# 4. Execução real sob isolamento ADuM3160 + captura PicoFW P7501 (115200 8N1)
```

## 5. Armadilhas & Verificação
- **Armadilhas:** Usar `volatile` achando que garante atomicidade entre ISR e loop principal; bloquear a janela `0x8900_0080..0x8A00_0000` (`DAVIS`/`SDCD`/`AVDD`); consultar `DAVIS CH1` sem aguardar `1000 ms` atrás do `IDE2SAT2`; alimentar `IC7501 TBUS` de dentro de um timer ISR cego; testar sem `ADuM3160`.
- **Verificação:** `test_digaos_host` (`100%` PASS em `-Wall -Wextra -Werror`), ELF sem símbolos indefinidos nem extrapolar `512 KiB` em `DIAG_PROG_RAM`, `panadiga re verify-patches` verde em `Mn103Vm` (`N2_EMULATED`) e `DIGAOS>` + `TBUS` estável > 10 min na `COM10` antes de declarar avanço `M0..M4` (`N4_HARDWARE`).
