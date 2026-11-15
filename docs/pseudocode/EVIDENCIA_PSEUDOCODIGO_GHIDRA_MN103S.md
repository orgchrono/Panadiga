> 🌐 **Language / Idioma:** **Bilingual (PT-BR / EN-US Navigation)**
> 🧭 **Portal:** [Docs Index](../INDEX.md) | [Complete Audit (EN)](../../COMPLETE_AUDIT.md) | [Auditoria Completa (PT-BR)](../../AUDITORIA_COMPLETA.md) | [Pseudocode Index](./INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [`??` Audit](./GHIDRA_UNDEF_AUDIT_REPORT.md) | [Debug Logs](../debug_logs/README.md)

# Evidência Forense Completa de Pseudocódigo C e Disassembly — Ghidra MN103S (Panasonic DMR-EH55)

> **Escopo e Integridade:** Este dossiê consolida **100% do pseudocódigo C decompilado, listagens de disassembly MN103S (`mn10300:LE:32:default`), constantes, strings e divergências de revisão** extraídos em modo estritamente **read-only** (`endTransaction(tx, False)`) do projeto Ghidra `C:\ghidra\projects\Panadiga_MN103S`. Nenhum código C ou mnemônico aqui reproduzido foi resumido, editado ou sintetizado: todos os blocos reproduzem literalmente a saída verificada do Ghidra 12.0.3 / PyGhidra.

---

## Sumário

- [Inventário Verificado, Memória e SHA-256 dos 5 Binários](#inventário-verificado-memória-e-sha-256-dos-5-binários)
- [Cadeia Arquitetural End-to-End (BOOT -> PROG061 -> MAIN <-> DRV)](#cadeia-arquitetural-end-to-end-boot---prog061---main---drv)
- [Índice Geral de Funções Decompiladas e Blocos Verificados](#índice-geral-de-funções-decompiladas-e-blocos-verificados)
- [Parte I — `/BOOT.BIN` (Estágio 1 / NOR Flash XIP)](#parte-i--bootbin-estágio-1--nor-flash-xip)
  - [1.1 Vetores de Reset, CRT0 (`.bss` + `.data` -> `0x50480000`), Nomes BGO & Strings](#11-vetores-de-reset-crt0-bss--data---0x50480000-nomes-bgo--strings)
  - [1.2 Fluxo Principal de Boot, GPIOs, UART0 e Handshake Timer MCU / VFD](#12-fluxo-principal-de-boot-gpios-uart0-e-handshake-timer-mcu--vfd)
  - [1.3 Subsistema BGO / NFROM / NOR Flash & NVRAM](#13-subsistema-bgo--nfrom--nor-flash--nvram)
  - [1.4 Subsistema SD/MMC (`FMU`, `SetBusWidth`, erro `-91006`)](#14-subsistema-sdmmc-fmu-setbuswidth-erro--91006)
  - [1.5 Optical Drive (`CD`/`DVD-RAM`), Joliet, `feonly_mode` & HDD RTSD/DAVIS IDE](#15-optical-drive-cddvd-ram-joliet-feonly_mode--hdd-rtsddavis-ide)
  - [1.6 Criptografia Feistel (`0x44039aa8` + S-Box `0x44035d84`), `ExtrHead`/`ExtrFoot` & LZ (`uncompress`)](#16-criptografia-feistel-0x44039aa8--s-box-0x44035d84-extrheadextrfoot--lz-uncompress)
- [Parte II — `/eh55_frm_PROG061_BOOT.bin` (Estágio 2 / DIGADIAG & FVU Loader)](#parte-ii--eh55_frm_prog061_bootbin-estágio-2--digadiag--fvu-loader)
  - [2.1 Inicialização, Menu DIGADIAG, `FMU_list_sum` (`0x5040ea30`) & `TimerMCU_update`](#21-inicialização-menu-digadiag-fmu_list_sum-0x5040ea30--timermcu_update)
  - [2.2 Drivers BGO, Controles NOR & Os 3 Patch Sites Exatos do PROG061](#22-drivers-bgo-controles-nor--os-3-patch-sites-exatos-do-prog061)
- [Parte III — `/EH55_MAIN_3410PQ.bin` (Estágio 3 / Main OS)](#parte-iii--eh55_main_3410pqbin-estágio-3--main-os)
  - [3.1 Console `_dev_SERIAL` / GDB RSP, Gates de UART0 & Loop de Menu `ENC`](#31-console-_dev_serial--gdb-rsp-gates-de-uart0--loop-de-menu-enc)
  - [3.2 Front Panel Test Mode (`FPC_TestModeControl`), `TEST+00` & ODD / `ATIH` / `NFROM BGO FAIL`](#32-front-panel-test-mode-fpc_testmodecontrol-test00--odd--atih--nfrom-bgo-fail)
  - [3.3 Subsistema HDD LBA48 (`FUN_574dd010..0x574dd273`), `400GB` vs `200GB` & Gate `MAIN`](#33-subsistema-hdd-lba48-fun_574dd0100x574dd273-400gb-vs-200gb--gate-main)
- [Parte IV — `/DRV_decomp.bin` & `/EH55_DRV_S211.bin` (Drive Front-End S211)](#parte-iv--drv_decompbin--eh55_drv_s211bin-drive-front-end-s211)
  - [4.1 Serial CLI (`0x400DF55C..0x400DF940`) & Comandos (`PASS`, `MMODE`, `SMODE`, `RB`, `WB`, `EEP`)](#41-serial-cli-0x400df55c0x400df940--comandos-pass-mmode-smode-rb-wb-eep)
  - [4.2 Comparativo `/DRV_decomp.bin` vs `/EH55_DRV_S211.bin` (`FUN_40130efa` vs `FUN_4013420a`)](#42-comparativo-drv_decompbin-vs-eh55_drv_s211bin-fun_40130efa-vs-fun_4013420a)
  - [4.3 Despachante ATAPI (`0x3c` Write Buffer), Sense Table (`0x400089dc`) & Strings Shift-JIS (`DRV12`)](#43-despachante-atapi-0x3c-write-buffer-sense-table-0x400089dc--strings-shift-jis-drv12)
- [Parte V — Catálogo de Armadilhas do Decompilador Ghidra no MN103S](#parte-v--catálogo-de-armadilhas-do-decompilador-ghidra-no-mn103s)

---

## Inventário Verificado, Memória e SHA-256 dos 5 Binários

| Programa no Projeto Ghidra | Papel no DMR-EH55 | Faixa Inicializada (`ram`) | Tamanho Inicializado | `funcs` | `defined_strings` | SHA-256 do Bloco Inicializado |
|---|---|---|---:|---:|---:|---|
| `/BOOT.BIN` | Estágio 1: IPL / Bootloader NOR XIP | `0x44000000..0x4403ffff` | `0x40000` (262.144 B) | 843 | 512 | `7c13c72a54c77387f3f7df0ddc213e4800e20b27eecbfd225cbc05a8fc73e377` |
| `/eh55_frm_PROG061_BOOT.bin` | Estágio 2: DIGADIAG / FVU Loader (`PROG061`) | `0x50400000..0x5043ffff` | `0x40000` (262.144 B) | 5 | 0 | `c7da9121174fcf4c54f35f5bb1f7b23a3dc0c93ab227158ad4fdd90a1ea24a81` |
| `/EH55_MAIN_3410PQ.bin` | Estágio 3: Main OS / `_dev_SERIAL` / LBA48 | `0x57000000..0x57c6692b` | `0xc6692c` (13.003.052 B) | 33 | 0 | `c3275d39549b68f48db16bc5fd043c6f8a373d4ba416826de09be5c55dc991b1` |
| `/DRV_decomp.bin` | FE S211 Drive Firmware (imagem principal analisada) | `0x40000000..0x4015ffff` | `0x160000` (1.441.792 B) | 2.346 | 1.913 | `694a4ab8aa29fcc45350dc0712047663602bdee7e4bff313b8ae62b057efd052` |
| `/EH55_DRV_S211.bin` | FE S211 Drive Firmware (revisão alterna comparada) | `0x40000000..0x4015ffff` | `0x160000` (1.441.792 B) | 15 | 0 | `6f2b61ede80ea9e3e9185d3d677813593f3d361446678e437c905a0037060243` |

Saída literal da inspeção PyGhidra (`verified_extra_facts.txt` — Seção 1):

```text
[BOOT.BIN] lang=mn10300:LE:32:default compiler=default imageBase=0x00000000 funcs=843 defined_strings=512
  BLK SRAM_INTERNAL: [0x00000000..0x00007fff] size=0x8000 (32768 B) init=False
  BLK MMIO_REGS: [0x10000000..0x100fffff] size=0x100000 (1048576 B) init=False
  BLK ram: [0x44000000..0x4403ffff] size=0x40000 (262144 B) init=True sha256=7c13c72a54c77387f3f7df0ddc213e4800e20b27eecbfd225cbc05a8fc73e377
  BLK NOR_FLASH_XIP: [0x80000000..0x807fffff] size=0x800000 (8388608 B) init=False
  BLK SDRAM_MAIN: [0x90000000..0x93ffffff] size=0x4000000 (67108864 B) init=False
  BLK mmio: [0xc0000000..0xdfffffff] size=0x20000000 (536870912 B) init=False
[eh55_frm_PROG061_BOOT.bin] lang=mn10300:LE:32:default compiler=default imageBase=0x00000000 funcs=5 defined_strings=0
  BLK ram: [0x50400000..0x5043ffff] size=0x40000 (262144 B) init=True sha256=c7da9121174fcf4c54f35f5bb1f7b23a3dc0c93ab227158ad4fdd90a1ea24a81
  BLK mmio: [0xc0000000..0xdfffffff] size=0x20000000 (536870912 B) init=False
[EH55_MAIN_3410PQ.bin] lang=mn10300:LE:32:default compiler=default imageBase=0x00000000 funcs=33 defined_strings=0
  BLK ram: [0x57000000..0x57c6692b] size=0xc6692c (13003052 B) init=True sha256=c3275d39549b68f48db16bc5fd043c6f8a373d4ba416826de09be5c55dc991b1
  BLK mmio: [0xc0000000..0xdfffffff] size=0x20000000 (536870912 B) init=False
[DRV_decomp.bin] lang=mn10300:LE:32:default compiler=default imageBase=0x00000000 funcs=2346 defined_strings=1913
  BLK DRV_SRAM_INT: [0x00000000..0x00007fff] size=0x8000 (32768 B) init=False
  BLK DRV_MMIO_10: [0x10000000..0x100fffff] size=0x100000 (1048576 B) init=False
  BLK ram: [0x40000000..0x4015ffff] size=0x160000 (1441792 B) init=True sha256=694a4ab8aa29fcc45350dc0712047663602bdee7e4bff313b8ae62b057efd052
  BLK DRV_SERVO_MMIO: [0x88000000..0x88ffffff] size=0x1000000 (16777216 B) init=False
  BLK mmio: [0xc0000000..0xdfffffff] size=0x20000000 (536870912 B) init=False
[EH55_DRV_S211.bin] lang=mn10300:LE:32:default compiler=default imageBase=0x40000000 funcs=15 defined_strings=0
  BLK mmio: [0x00000000..0x1fffffff] size=0x20000000 (536870912 B) init=False
  BLK ram: [0x40000000..0x4015ffff] size=0x160000 (1441792 B) init=True sha256=6f2b61ede80ea9e3e9185d3d677813593f3d361446678e437c905a0037060243
```

---

## Cadeia Arquitetural End-to-End (BOOT -> PROG061 -> MAIN <-> DRV)

1. **Reset e Estágio 1 (`/BOOT.BIN` @ `0x44000000`)**:
   - O vetor de reset em `0x44000000` salta (`JMP 0x440000a0`) para o código de inicialização de barramento/SDRAM (`0x440000a0..`), executa `crt0` (`0x4400133e`, limpando `.bss` de `0x50480b80` até `0x505044d0` e copiando `0x0b80` bytes de `.data` de `0x44039830` para `0x50480000`) e entra em `boot_main` (`0x4400492e`).
   - **Chave Feistel derivada em memória**: o ponteiro `0x50480278` passado para `FUN_4400a666` / `FUN_44011d42` corresponde a `0x44039830 + 0x278 = 0x44039aa8` na ROM (`72 eb ca 91 30 45 68` como os 7 bytes de `param_1`, e `0x44039aaf` = `0x5048027f` começando em `ad 99 df 56 72 3a b8 10` como os 8 bytes de `param_2`).
   - `boot_main` inicializa BGO (`bgo_driver_init`), lê flags em NVRAM/EEPROM, consulta `FUN_440046e4` (GPIOs/switches), dispara recuperação de mapa NFROM se necessário (`FUN_440041dc` -> `FUN_44003fd0` -> `FUN_440018bb`) e chama `FUN_44006cd9` / `FUN_44006fe7` para rotear o boot normal (`MAINSOFT` no HDD via `FUN_440068f5` + `ExtrHead`/`uncompress` em RAM -> salto) ou modo de diagnóstico/atualização via **CD/DVD-RAM** (`cd_diag_boot_mode`) ou **Cartão SD** (`fmu_sd_dispatcher` / `sd_fvu_load_7a12` -> `PROG061` em `0x50400000` -> salto em `0x50400014`).
2. **Estágio 2 (`/eh55_frm_PROG061_BOOT.bin` @ `0x50400000`)**:
   - Executável carregado em RAM (`PROG061`) que apresenta o menu interativo serial DIGADIAG/FVU (`FUN_50404fa7`, `FUN_50405416`), soma listas `.FMU` (`FUN_5040ea30` com loop overlapping em `0x5040ea2f`), controla VFD/Timer MCU (`FUN_5040df40`), opera gravação BGO NOR (`FUN_50421ebe`, `FUN_5042201e`, `FUN_50422359`, `FUN_50423058`) e exibe **3 sítios de patch exatos** que explicam bloqueios/assinaturas na atualização de partições.
3. **Estágio 3 (`/EH55_MAIN_3410PQ.bin` @ `0x57000000`)**:
   - Sistema operacional principal descompactado. Mantém console `_dev_SERIAL` com suporte a protocolo GDB Remote Serial (`+$`, `-$`, `#cs`, checksum mod 256 em `FUN_574f0590`), gate `SC0` em `FUN_57000e1c`, loop de diagnóstico de encoder (`FUN_574f8354`/`FUN_574f862b`), controle de painel frontal em Test Mode (`FUN_5771abfb` / `FUN_5749efd3` / `FPC_TestModeControl`), interface ATAPI/ODD (`FUN_574f77b0`..`FUN_574f7b6a`, `FUN_5749859a` `'ATIH'`) e comandos LBA48 para HDD (`FUN_574dd010`..`FUN_574dd273`, revelando inclusive a seleção de geometria `200GB` vs `400GB` em `0x574dd208`).
4. **Front-End do Drive (`/DRV_decomp.bin` & `/EH55_DRV_S211.bin` @ `0x40000000`)**:
   - Firmware do controlador de drive óptico (MN103S próprio). Gerencia despachante ATAPI (`FUN_400209f3` incluindo opcode `0x3c` Write Buffer, `FUN_40021476`, `FUN_4002168d`), tabela de sense codes (`0x400089dc`), parser serial CLI (`0x400DF55C..0x400DF940`), comando de desbloqueio `PASS` com assinaturas `0x4D415453` (`"MATS"`) e `0x48495441` (`"HITA"`) e comandos `MMODE`/`SMODE`/`RB`/`WB`/`EEP`.

---

## Índice Geral de Funções Decompiladas e Blocos Verificados

| Binário | Endereço de Entrada | Símbolo Ghidra | Alias / Subsistema | Tamanho | Resumo Verificado |
|---|---|---|---|---:|---|
| `/BOOT.BIN` | 0x4400492e | `boot_main` | `boot_main` | 13 B | Função principal de boot (`boot_main`); inicializa BGO, verifica estado NFROM e decide rota de inicialização. |
| `/BOOT.BIN` | 0x44006454 | `detect_boot_mode` | `detect_boot_mode` | 116 B | Leitura de modo de boot via flags NVRAM/hardware e cartão de suporte. |
| `/BOOT.BIN` | 0x440064c8 | `parse_support_card_hdr` | `parse_support_card_hdr` | 197 B | Valida cabeçalho de cartão de suporte SD para modo especial de boot. |
| `/BOOT.BIN` | 0x4400658d | `FUN_4400658d` | `support_card_jump_658d` | 47 B | Salto para endereço de execução do cartão de suporte (`support_card_jump_658d`). |
| `/BOOT.BIN` | 0x44006cd9 | `FUN_44006cd9` | `boot_route_cd_sd_6cd9` | 777 B | Roteamento principal entre boot via CD e via cartão SD (`boot_route_cd_sd_6cd9`). |
| `/BOOT.BIN` | 0x44006fe7 | `FUN_44006fe7` | `media_update_gate_6fe7` | 1093 B | Portão de decisão para atualização de firmware por mídia (`media_update_gate_6fe7`). |
| `/BOOT.BIN` | 0x44006822 | `FUN_44006822` | `hdd_hdr_check_6822` | 211 B | Verifica cabeçalho de partição/imagem em HDD (`hdd_hdr_check_6822`). |
| `/BOOT.BIN` | 0x440068f5 | `FUN_440068f5` | `hdd_load_mainsoft_68f5` | 495 B | Carrega `mainsoft` do HDD, descompacta e salta para execução (`hdd_load_mainsoft_68f5`). |
| `/BOOT.BIN` | 0x44006ae4 | `FUN_44006ae4` | `hdd_inner_recovery_6ae4` | 501 B | Rotina interna de recuperação/fallback de carga do HDD (`hdd_inner_recovery_6ae4`). |
| `/BOOT.BIN` | 0x440046e4 | `FUN_440046e4` | `gpio_model_flag_46e4` | 101 B | Leitura de GPIOs e flag de modelo/hardware (`gpio_model_flag_46e4`). |
| `/BOOT.BIN` | 0x44004749 | `uart0_init` | `uart0_init` | 86 B | Inicialização da serial UART0 / `SC0`. |
| `/BOOT.BIN` | 0x4400479f | `uart0_getc` | `uart0_getc_dead` | 40 B | Leitura de caractere da UART0 (inalcançável no fluxo padrão de varejo). |
| `/BOOT.BIN` | 0x440047c7 | `uart0_putc` | `uart0_putc_poll` | 65 B | Envio de caractere com polling na UART0. |
| `/BOOT.BIN` | 0x4400488a | `console_getline` | `console_getline_dead` | 164 B | Leitura de linha no console serial (`console_getline_dead`). |
| `/BOOT.BIN` | 0x44005086 | `FUN_44005086` | `timer_mcu_5086` | 93 B | Comunicação inicial com o Timer MCU (`timer_mcu_5086`). |
| `/BOOT.BIN` | 0x44005114 | `FUN_44005114` | `timer_rx_handler_5114` | 492 B | Tratador de recepção serial do Timer MCU (`timer_rx_handler_5114`). |
| `/BOOT.BIN` | 0x4400599a | `FUN_4400599a` | `vfd_msg_599a` | 125 B | Envia mensagem para exibição no display VFD (`vfd_msg_599a`). |
| `/BOOT.BIN` | 0x44005cd0 | `FUN_44005cd0` | `vfd_wait_5cd0` | 70 B | Aguarda conclusão/ACK do Timer MCU / VFD (`vfd_wait_5cd0`). |
| `/BOOT.BIN` | 0x44007812 | `boot_panic_or_halt_7812` | `boot_panic_or_halt_7812` | 36 B | Halt / pânico de boot. |
| `/BOOT.BIN` | 0x44007836 | `FUN_44007836` | `vfd_u99_open_tray_7836` | 366 B | Exibe `U99` no VFD e aciona abertura de bandeja em falha crítica (`vfd_u99_open_tray_7836`). |
| `/BOOT.BIN` | 0x440079a4 | `FUN_440079a4` | `power_off_or_halt_79a4` | 110 B | Desliga alimentação ou entra em parada definitiva (`power_off_or_halt_79a4`). |
| `/BOOT.BIN` | 0x4400b34d | `wait_1_sec_timer_sync` | `wait_1_sec_timer_sync_b34d` | 140 B | Aguarda 1 segundo sincronizado com o Timer MCU (`wait_1_sec_timer_sync_b34d`). |
| `/BOOT.BIN` | `0x440041dc` | `FUN_440041dc` | `nfrom_caller_41dc` | 273 B | Função verificada em `verify_ghidra_doc_facts_out.txt` (decomp + disasm). |
| `/BOOT.BIN` | 0x44003fd0 | `FUN_44003fd0` | `nfrom_power_fail_recovery` | 513 B | Recuperação de estado/power-fail em NOR (`nfrom_power_fail_recovery`). |
| `/BOOT.BIN` | 0x440018bb | `FUN_440018bb` | `nfrom_recover_map` | 810 B | Recuperação e remapeamento de tabela NFROM (`nfrom_recover_map`). |
| `/BOOT.BIN` | 0x44001c49 | `FUN_44001c49` | `bgo_ioctl_dispatch` | 260 B | Despacho `ioctl` do driver BGO (`bgo_ioctl_dispatch`). |
| `/BOOT.BIN` | 0x44007ea9 | `FUN_44007ea9` | `bgo_flag_clear_7ea9` | 115 B | Limpeza de flag BGO (`bgo_flag_clear_7ea9`). |
| `/BOOT.BIN` | 0x44007f1c | `FUN_44007f1c` | `bgo_flag_clear2_7f1c` | 65 B | Limpeza secundária de flag BGO (`bgo_flag_clear2_7f1c`). |
| `/BOOT.BIN` | 0x44007fa1 | `FUN_44007fa1` | `bgo_status_7fa1` | 94 B | Consulta de status de partição BGO (`bgo_status_7fa1`). |
| `/BOOT.BIN` | 0x44008075 | `FUN_44008075` | `bgo_update_loop_8075` | 409 B | Loop de atualização BGO entre partições (`bgo_update_loop_8075`). |
| `/BOOT.BIN` | 0x4400825c | `FUN_4400825c` | `bgo_flag_update_825c` | 275 B | Atualização de flags/estado BGO (`bgo_flag_update_825c`). |
| `/BOOT.BIN` | 0x4400836f | `bgo_driver_init` | `bgo_driver_init` | 140 B | Inicialização principal do driver BGO (`bgo_driver_init`). |
| `/BOOT.BIN` | 0x44008634 | `nvram_read_8634` | `nvram_read_8634` | 864 B | Leitura de parâmetros e estrutura da NVRAM (`nvram_read_8634`). |
| `/BOOT.BIN` | 0x44008b02 | `FUN_44008b02` | `bgo_ctrl_init_8b02` | 455 B | Inicialização do controlador BGO (`bgo_ctrl_init_8b02`). |
| `/BOOT.BIN` | 0x44008cc9 | `FUN_44008cc9` | `bgo_device_info_check_8cc9` | 989 B | Validação de Device Info / cabeçalho de controle BGO (`bgo_device_info_check_8cc9`). |
| `/BOOT.BIN` | 0x440090a6 | `FUN_440090a6` | `bgo_write_part_90a6` | 153 B | Gravação de partição BGO (`bgo_write_part_90a6`). |
| `/BOOT.BIN` | 0x4400920b | `FUN_4400920b` | `bgo_read_part_920b` | 324 B | Leitura de partição BGO (`bgo_read_part_920b`). |
| `/BOOT.BIN` | 0x4400934f | `FUN_4400934f` | `conveyer_check_934f` | 133 B | Verificação de consistência / conveyer check BGO (`conveyer_check_934f`). |
| `/BOOT.BIN` | 0x4400f5cb | `nvram_read_id` | `nvram_read_id` | 196 B | Leitura de identificador / assinatura da NVRAM (`nvram_read_id`). |
| `/BOOT.BIN` | 0x4401f044 | `FUN_4401f044` | `bgo_nor_part_check_1f044` | 61 B | Checagem de partição BGO em NOR Flash (`bgo_nor_part_check_1f044`). |
| `/BOOT.BIN` | 0x4401f081 | `FUN_4401f081` | `bgo_err_log_1f081` | 59 B | Registro de erro BGO em NOR Flash (`bgo_err_log_1f081`). |
| `/BOOT.BIN` | 0x4401f804 | `FUN_4401f804` | `bgo_nor_sum_1f804` | 292 B | Cálculo de checksum BGO em NOR Flash (`bgo_nor_sum_1f804`). |
| `/BOOT.BIN` | 0x440237ba | `FUN_440237ba` | `bgo_nor_erase_write_237ba` | 395 B | Apagamento/gravação de setor em NOR Flash (`bgo_nor_erase_write_237ba`). |
| `/BOOT.BIN` | 0x440236ed | `FUN_440236ed` | `bgo_nor_cfi_cmd_236ed` | 205 B | Sequência CFI `0xAA`/`0x55` para NOR Flash — exemplo clássico de pruning MMIO pelo decompilador (`bgo_nor_cfi_cmd_236ed`). |
| `/BOOT.BIN` | 0x440065bc | `fmu_sd_dispatcher` | `fmu_sd_dispatcher` | 614 B | Despachante principal de carga `.FMU`/`.FVU` via SD (`fmu_sd_dispatcher`). |
| `/BOOT.BIN` | 0x44007a12 | `FUN_44007a12` | `sd_fvu_load_7a12` | 403 B | Carregamento de arquivo `.FVU` / `PROG061` via SD (`sd_fvu_load_7a12`). |
| `/BOOT.BIN` | 0x44007ba5 | `FUN_44007ba5` | `sd_diag_boot_7ba5` | 180 B | Boot de diagnóstico via SD (`sd_diag_boot_7ba5`). |
| `/BOOT.BIN` | 0x44007e48 | `FUN_44007e48` | `sd_card_read_test_7e48` | 97 B | Teste de leitura do cartão SD (`sd_card_read_test_7e48`). |
| `/BOOT.BIN` | 0x4400a247 | `FUN_4400a247` | `sd_mount_and_read_a247` | 163 B | Monta o cartão SD e abre fluxo de leitura (`sd_mount_and_read_a247`). |
| `/BOOT.BIN` | 0x4400a32d | `FUN_4400a32d` | `sd_read_file_stream_a32d` | 216 B | Leitura em stream de arquivo SD (`sd_read_file_stream_a32d`). |
| `/BOOT.BIN` | 0x4400a8c7 | `FUN_4400a8c7` | `sd_load_file_a8c7` | 181 B | Carregamento de arquivo nomeado do SD para buffer/RAM (`sd_load_file_a8c7`). |
| `/BOOT.BIN` | 0x4400a97c | `FUN_4400a97c` | `sd_diag_load_a97c` | 227 B | Carga de binário de diagnóstico a partir do SD (`sd_diag_load_a97c`). |
| `/BOOT.BIN` | 0x4400aa5f | `FUN_4400aa5f` | `sd_read_firm_list_aa5f` | 99 B | Leitura de arquivo lista `.FMU` / firmware list no SD (`sd_read_firm_list_aa5f`). |
| `/BOOT.BIN` | 0x4402618b | `FUN_4402618b` | `sd_get_info_2618b` | 88 B | Consulta informações e estado atual do controlador SD (`sd_get_info_2618b`). |
| `/BOOT.BIN` | 0x44026278 | `FUN_44026278` | `sd_slot_init_26278` | 83 B | Inicialização básica de slot SD (`sd_slot_init_26278`). |
| `/BOOT.BIN` | 0x440264a6 | `FUN_440264a6` | `sd_set_state_264a6` | 163 B | Configuração de máquina de estados SD (`sd_set_state_264a6`). |
| `/BOOT.BIN` | 0x44026549 | `FUN_44026549` | `sd_clock_cfg_26549` | 221 B | Configuração de divisor/clock SD (`sd_clock_cfg_26549`). |
| `/BOOT.BIN` | 0x4402724f | `FUN_4402724f` | `sd_send_ocr_rca_2724f` | 776 B | Negociação de OCR e obtenção de RCA do cartão SD (`sd_send_ocr_rca_2724f`). |
| `/BOOT.BIN` | 0x44027826 | `GetSD_Status` | `GetSD_Status_27826` | 244 B | Obtém registrador de status SD (`ACMD13` / `GetSD_Status_27826`). |
| `/BOOT.BIN` | 0x4402791a | `FUN_4402791a` | `SetBusWidth_2791a` | 1774 B | Comutação de largura de barramento 1-bit/4-bit (`SetBusWidth_2791a`, origem das strings `SetBusWidth` e código `-91006`). |
| `/BOOT.BIN` | 0x44028008 | `FUN_44028008` | `sd_init_entry_28008` | 492 B | Ponto de entrada da inicialização do controlador SD (`sd_init_entry_28008`). |
| `/BOOT.BIN` | 0x440281f4 | `FUN_440281f4` | `sd_card_mount_281f4` | 99 B | Montagem de volume/sistema de arquivos no SD (`sd_card_mount_281f4`). |
| `/BOOT.BIN` | 0x44028348 | `FUN_44028348` | `sd_card_check_28348` | 75 B | Verificação de presença/prontidão do cartão SD (`sd_card_check_28348`). |
| `/BOOT.BIN` | 0x44004b06 | `feonly_mode` | `feonly_mode` | 214 B | Modo Front-End Only de depuração/gravação direta com o drive. |
| `/BOOT.BIN` | 0x44004bde | `fe_drive_mailbox_write` | `fe_drive_mailbox_write` | 300 B | Escrita via mailbox/registradores de comunicação com o drive FE. |
| `/BOOT.BIN` | 0x4400742c | `FUN_4400742c` | `dvdram_diag_boot_742c` | 382 B | Fluxo de boot de diagnóstico via DVD-RAM (`dvdram_diag_boot_742c`). |
| `/BOOT.BIN` | 0x440075aa | `FUN_440075aa` | `cdr_diag_boot_75aa` | 376 B | Fluxo de boot de diagnóstico via CD-R (`cdr_diag_boot_75aa`). |
| `/BOOT.BIN` | 0x44007722 | `FUN_44007722` | `drive_key_poll_7722` | 55 B | Polling de teclas do painel durante checagem do drive (`drive_key_poll_7722`). |
| `/BOOT.BIN` | 0x44007789 | `FUN_44007789` | `drive_key_abort_7789` | 137 B | Aborta espera do drive por combinação de teclas (`drive_key_abort_7789`). |
| `/BOOT.BIN` | 0x44007c59 | `cd_diag_boot_mode` | `cd_diag_boot_mode_7c59` | 495 B | Modo completo de boot/diagnóstico via disco óptico (`cd_diag_boot_mode_7c59`). |
| `/BOOT.BIN` | 0x44009835 | `FUN_44009835` | `drive_get_status_9835` | 199 B | Consulta status ATAPI/ATA do drive (`drive_get_status_9835`). |
| `/BOOT.BIN` | 0x44009a3e | `FUN_44009a3e` | `drive_read_tur_9a3e` | 211 B | Envia `TEST UNIT READY` / leitura de prontidão do drive (`drive_read_tur_9a3e`). |
| `/BOOT.BIN` | 0x44009b11 | `atapi_issue_pkt` | `atapi_issue_pkt_9b11` | 142 B | Envio de CDB ATAPI de pacote (`atapi_issue_pkt_9b11`). |
| `/BOOT.BIN` | 0x44009bd7 | `FUN_44009bd7` | `open_tray_9bd7` | 174 B | Comando ATAPI para abrir bandeja do drive (`open_tray_9bd7`). |
| `/BOOT.BIN` | 0x44009c85 | `FUN_44009c85` | `close_tray_9c85` | 101 B | Comando ATAPI para fechar bandeja do drive (`close_tray_9c85`). |
| `/BOOT.BIN` | 0x44009cea | `startup_drive` | `startup_drive_9cea` | 295 B | Inicialização sequencial dos drives IDE/ATAPI (HDD e ODD, `startup_drive_9cea`). |
| `/BOOT.BIN` | 0x44009ea7 | `FUN_44009ea7` | `hdd_max_lba_9ea7` | 355 B | Obtém `MAX LBA` / capacidade do HDD (`hdd_max_lba_9ea7`). |
| `/BOOT.BIN` | 0x4400a86d | `FUN_4400a86d` | `cdr_search_fvu_a86d` | 90 B | Procura arquivo `.FVU` em mídia CD-R (`cdr_search_fvu_a86d`). |
| `/BOOT.BIN` | 0x4400aac2 | `FUN_4400aac2` | `dvdram_read_list_aac2` | 72 B | Leitura de lista de arquivos/firmware em DVD-RAM (`dvdram_read_list_aac2`). |
| `/BOOT.BIN` | 0x4400ab0a | `FUN_4400ab0a` | `cd_load_file_ab0a` | 1562 B | Parser ISO9660/Joliet e carregamento de arquivo por setor em CD (`cd_load_file_ab0a`). |
| `/BOOT.BIN` | 0x4400b124 | `FUN_4400b124` | `cd_load_pana_dvd_b124` | 41 B | Wrapper para carregar `PANA_DVD.FRM` de CD (`cd_load_pana_dvd_b124`). |
| `/BOOT.BIN` | 0x4400b14d | `FUN_4400b14d` | `cd_read_sectors_chunked_b14d` | 189 B | Leitura em blocos/chunks de setores de CD (`cd_read_sectors_chunked_b14d`). |
| `/BOOT.BIN` | 0x4400b20a | `FUN_4400b20a` | `joliet_utf16_to_ascii_b20a` | 276 B | Conversão de nomes UTF-16BE Joliet para ASCII (`joliet_utf16_to_ascii_b20a`). |
| `/BOOT.BIN` | 0x4400b31e | `FUN_4400b31e` | `cd_load_digadiag_b31e` | 38 B | Wrapper para carregar pacote `DIGADIAG` de CD (`cd_load_digadiag_b31e`). |
| `/BOOT.BIN` | 0x4400fb67 | `FUN_4400fb67` | `rtsd_cmd_wait_fb67` | 216 B | Espera conclusão de comando no controlador IDE RTSD (`rtsd_cmd_wait_fb67`). |
| `/BOOT.BIN` | 0x4400fc3f | `FUN_4400fc3f` | `rtsd_channel_ready_fc3f` | 942 B | Prepara canal RTSD / espera `DRDY`/prontidão IDE (`rtsd_channel_ready_fc3f`). |
| `/BOOT.BIN` | 0x4400ffed | `FUN_4400ffed` | `rtsd_ide_exec_ffed` | 1382 B | Executor baixo nível de comandos ATA/ATAPI no controlador RTSD (`rtsd_ide_exec_ffed`). |
| `/BOOT.BIN` | 0x4401a5cd | `FUN_4401a5cd` | `davis_probe_master_slave_a5cd` | 896 B | Detecção Master/Slave no barramento IDE DAVIS (`davis_probe_master_slave_a5cd`). |
| `/BOOT.BIN` | 0x4401aa1f | `FUN_4401aa1f` | `davis_bus_init_aa1f` | 1214 B | Inicialização completa do barramento DAVIS (`davis_bus_init_aa1f`). |
| `/BOOT.BIN` | 0x4400a666 | `FUN_4400a666` | `feistel_decrypt_buf_a666` | 106 B | Descriptografa buffer com cifra Feistel usando chaves em `0x50480278` (`feistel_decrypt_buf_a666`). |
| `/BOOT.BIN` | 0x4400a6d0 | `FUN_4400a6d0` | `extrhead_is_encrypted_a6d0` | 99 B | Verifica se cabeçalho `ExtrHead` indica payload criptografado (`extrhead_is_encrypted_a6d0`). |
| `/BOOT.BIN` | 0x4400a733 | `FUN_4400a733` | `find_firm_entry_a733` | 254 B | Busca entrada de firmware em tabela de diretório `.FRM` (`find_firm_entry_a733`). |
| `/BOOT.BIN` | 0x4400a831 | `FUN_4400a831` | `bytes_sum32_a831` | 60 B | Soma linear de 32 bits em bytes (`bytes_sum32_a831`). |
| `/BOOT.BIN` | 0x4400b4ab | `FUN_4400b4ab` | `extrhead_hdr_check_b4ab` | 81 B | Validação de cabeçalho `ExtrHead` (`extrhead_hdr_check_b4ab`). |
| `/BOOT.BIN` | 0x4400b52e | `FUN_4400b52e` | `extrhead_magic_check_b52e` | 61 B | Verifica magic string `"ExtrHead"` (`extrhead_magic_check_b52e`). |
| `/BOOT.BIN` | 0x4400b56b | `FUN_4400b56b` | `extrfoot_magic_check_b56b` | 74 B | Verifica magic string de rodapé `"ExtrFoot"` (`extrfoot_magic_check_b56b`). |
| `/BOOT.BIN` | 0x4400b5b5 | `FUN_4400b5b5` | `extrhead_get_chk_b5b5` | 44 B | Lê campo de checksum do `ExtrHead` (`extrhead_get_chk_b5b5`). |
| `/BOOT.BIN` | 0x4400b61a | `FUN_4400b61a` | `extrhead_get_dst_b61a` | 30 B | Lê endereço de destino de carga do `ExtrHead` (`extrhead_get_dst_b61a`). |
| `/BOOT.BIN` | 0x4400b652 | `FUN_4400b652` | `extrhead_verify_adler_b652` | 116 B | Verifica checksum Adler-32 strided do payload `ExtrHead` (`extrhead_verify_adler_b652`). |
| `/BOOT.BIN` | 0x4400b6c6 | `FUN_4400b6c6` | `extrhead_unpack_b6c6` | 828 B | Desempacota payload `ExtrHead`: decifra (se cifrado), valida Adler-32 e descomprime (`extrhead_unpack_b6c6`). |
| `/BOOT.BIN` | 0x4400bb14 | `sub_4400bb14` | `sub_4400bb14` | 430 B | Sub-rotina interna de decodificação de tokens/cópias LZ77/LZSS (`sub_4400bb14`). |
| `/BOOT.BIN` | 0x4400bcc2 | `uncompress` | `uncompress_fe_bcc2` | 192 B | Descompressor principal utilizado pelo loader de firmware (`uncompress_fe_bcc2`). |
| `/BOOT.BIN` | 0x44011d42 | `FUN_44011d42` | `feistel_key_schedule_11d42` | 383 B | Key schedule da cifra Feistel a partir de `0x50480278`/`0x5048027f` (`feistel_key_schedule_11d42`). |
| `/BOOT.BIN` | 0x44012029 | `FUN_44012029` | `feistel_block_dec_12029` | 360 B | Decifração de bloco de 64 bits (8 bytes) Feistel (`feistel_block_dec_12029`). |
| `/BOOT.BIN` | 0x44012191 | `FUN_44012191` | `feistel_round_f_12191` | 383 B | Função de rodada Feistel usando S-Box `0x44035d84` (`feistel_round_f_12191`). |
| `/BOOT.BIN` | 0x4401e5a3 | `FUN_4401e5a3` | `strided_adler32_1e5a3` | 147 B | Checksum estilo Adler-32 com step configurável (`strided_adler32_1e5a3`). |
| `/eh55_frm_PROG061_BOOT.bin` | 0x50400014 | `FUN_50400014` | `p61_startup_0014` | 101 B | Ponto de entrada pós-carga de `PROG061` em `0x50400014` (`p61_startup_0014`). |
| `/eh55_frm_PROG061_BOOT.bin` | 0x50400810 | `FUN_50400810` | `p61_data_init_0810` | 150 B | Inicialização de dados e variáveis globais de `PROG061` (`p61_data_init_0810`). |
| `/eh55_frm_PROG061_BOOT.bin` | 0x50401f3c | `FUN_50401f3c` | `p61_nfrom_format_orphan_1f3c` | 483 B | Rotina órfã/especial de formatação NFROM (`p61_nfrom_format_orphan_1f3c`). |
| `/eh55_frm_PROG061_BOOT.bin` | 0x5040211f | `FUN_5040211f` | `p61_cmd_dispatch_211f` | 788 B | Despachante de comandos DIGADIAG/serviço (`p61_cmd_dispatch_211f`). |
| `/eh55_frm_PROG061_BOOT.bin` | 0x5040248e | `FUN_5040248e` | `p61_fmu_sd_diag_248e` | 74 B | Diagnóstico de cartão SD / FMU em `PROG061` (`p61_fmu_sd_diag_248e`). |
| `/eh55_frm_PROG061_BOOT.bin` | 0x50404dcd | `FUN_50404dcd` | `p61_print_helper_4dcd` | 144 B | Helper de impressão formatada no console `PROG061` (`p61_print_helper_4dcd`). |
| `/eh55_frm_PROG061_BOOT.bin` | 0x50404ec0 | `FUN_50404ec0` | `p61_hit_any_key_4ec0` | 148 B | Leitura de linha / espera de tecla (`Hit any key`, `p61_hit_any_key_4ec0`). |
| `/eh55_frm_PROG061_BOOT.bin` | `0x50404f12` | `FUN_50404f12` | `HitAnyKey_xref` | 32 B | Exemplo de `in_LAR` / loop de hardware MN103S. |
| `/eh55_frm_PROG061_BOOT.bin` | 0x50404fa7 | `FUN_50404fa7` | `p61_main_menu_4fa7` | 1126 B | Menu principal DIGADIAG de `PROG061` (`p61_main_menu_4fa7`). |
| `/eh55_frm_PROG061_BOOT.bin` | 0x50405416 | `FUN_50405416` | `p61_diag_loop_5416` | 653 B | Loop de diagnóstico DIGADIAG (`p61_diag_loop_5416`). |
| `/eh55_frm_PROG061_BOOT.bin` | 0x5040689d | `FUN_5040689d` | `p61_halt_689d` | 32 B | Rotina de parada em `PROG061` (`p61_halt_689d`). |
| `/eh55_frm_PROG061_BOOT.bin` | 0x50406d30 | `FUN_50406d30` | `p61_state_init_6d30` | 122 B | Inicialização de estados internos em `PROG061` (`p61_state_init_6d30`). |
| `/eh55_frm_PROG061_BOOT.bin` | 0x5040df40 | `FUN_5040df40` | `p61_TimerMCU_update_df40` | 1058 B | Atualização de firmware do Timer MCU via `PROG061` (`p61_TimerMCU_update_df40`). |
| `/eh55_frm_PROG061_BOOT.bin` | `0x5040ea30` | `FUN_5040ea30` | `FMU_list_sum` | 75 B | Decompilação + disasm de instrução sobreposta `0x5040ea2f`/`0x5040ea30`. |
| `/eh55_frm_PROG061_BOOT.bin` | 0x5040a294 | `FUN_5040a294` | `p61_bgo_init_hook_a294` | 130 B | Hook de inicialização BGO em `PROG061` (`p61_bgo_init_hook_a294`). |
| `/eh55_frm_PROG061_BOOT.bin` | 0x50421ebe | `FUN_50421ebe` | `p61_bgo_ctrl_21ebe` | 101 B | Controle BGO equivalente em `PROG061` (`p61_bgo_ctrl_21ebe`). |
| `/eh55_frm_PROG061_BOOT.bin` | 0x5042201e | `FUN_5042201e` | `p61_bgo_read_2201e` | 165 B | Leitura BGO em `PROG061` (`p61_bgo_read_2201e`). |
| `/eh55_frm_PROG061_BOOT.bin` | 0x50422359 | `FUN_50422359` | `p61_bgo_nor_22359` | 50 B | Operação BGO em NOR Flash no `PROG061` (`p61_bgo_nor_22359`). |
| `/eh55_frm_PROG061_BOOT.bin` | 0x50423058 | `FUN_50423058` | `p61_bgo_sum_23058` | 292 B | Soma/checksum BGO em `PROG061` (`p61_bgo_sum_23058`). |
| `/EH55_MAIN_3410PQ.bin` | 0x57000e1c | `FUN_57000e1c` | `main_uart0_gate_000e1c` | 127 B | Gate de inicialização/ativação da UART0 no `MAIN` (`main_uart0_gate_000e1c`). |
| `/EH55_MAIN_3410PQ.bin` | 0x574ef813 | `FUN_574ef813` | `main_sc0txb_ring_4ef813` | 94 B | Envio por ring-buffer de transmissão serial `SC0TXB` (`main_sc0txb_ring_4ef813`). |
| `/EH55_MAIN_3410PQ.bin` | 0x574f0538 | `FUN_574f0538` | `main_dev_serial_init_4f0538` | 73 B | Inicialização do driver `_dev_SERIAL` (`main_dev_serial_init_4f0538`). |
| `/EH55_MAIN_3410PQ.bin` | 0x574f0590 | `FUN_574f0590` | `main_dev_serial_rsp_4f0590` | 742 B | Parser completo de pacotes GDB Remote Serial (`+$`, `-$`, `#cs`, checksum módulo 256) em `_dev_SERIAL` (`main_dev_serial_rsp_4f0590`). |
| `/EH55_MAIN_3410PQ.bin` | 0x574f8354 | `FUN_574f8354` | `main_enc_menu_read_4f8354` | 48 B | Leitura de entrada no menu de diagnóstico do encoder `ENC` (`main_enc_menu_read_4f8354`). |
| `/EH55_MAIN_3410PQ.bin` | 0x574f862b | `FUN_574f862b` | `main_enc_menu_loop_4f862b` | 142 B | Loop de seleção do menu `ENC` (`main_enc_menu_loop_4f862b`). |
| `/EH55_MAIN_3410PQ.bin` | 0x574f77b0 | `FUN_574f77b0` | `main_odd_gate_4f77b0` | 133 B | Gate do subsistema de drive óptico ODD (`main_odd_gate_4f77b0`). |
| `/EH55_MAIN_3410PQ.bin` | 0x574f786a | `FUN_574f786a` | `main_odd_state_4f786a` | 8 B | Leitura de estado ODD (`main_odd_state_4f786a`). |
| `/EH55_MAIN_3410PQ.bin` | 0x574f7b00 | `FUN_574f7b00` | `main_odd_loop_4f7b00` | 106 B | Loop de comunicação ODD (`main_odd_loop_4f7b00`). |
| `/EH55_MAIN_3410PQ.bin` | 0x574f7b6a | `FUN_574f7b6a` | `main_odd_fn_4f7b6a` | 318 B | Função de controle ODD (`main_odd_fn_4f7b6a`). |
| `/EH55_MAIN_3410PQ.bin` | 0x5771abfb | `FUN_5771abfb` | `main_fpc_status_ctrl_71abfb` | 174 B | Controle de status/painel frontal `FPC_TestModeControl` — contém `0x5771ac87: BCLR 0xff,(0x0,A2)` (`main_fpc_status_ctrl_71abfb`). |
| `/EH55_MAIN_3410PQ.bin` | 0x5749efd3 | `FUN_5749efd3` | `main_test_mode_49efd3` | 179 B | Tratamento de entrada em modo de teste e `TEST+00` (`main_test_mode_49efd3`). |
| `/EH55_MAIN_3410PQ.bin` | 0x574a00cc | `FUN_574a00cc` | `main_nfrom_bgo_fail_4a00cc` | 270 B | Manipulador de falha BGO/NFROM no `MAIN` (`main_nfrom_bgo_fail_4a00cc`). |
| `/EH55_MAIN_3410PQ.bin` | 0x5749859a | `FUN_5749859a` | `main_atih_caller_49859a` | 805 B | Chamador/handshake de autenticação/comando `'ATIH'` (`main_atih_caller_49859a`). |
| `/EH55_MAIN_3410PQ.bin` | `0x574dd010` | `FUN_574dd010` | `main_hdd_cmd_010` | - | Decompilação LBA48 verificada após `createFunction`. |
| `/EH55_MAIN_3410PQ.bin` | 0x574dd0fd | `FUN_574dd0fd` | `main_hdd_cmd_0fd` | 74 B | Sub-rotina LBA48 do subsistema HDD (`main_hdd_cmd_0fd`). |
| `/EH55_MAIN_3410PQ.bin` | 0x574dd156 | `thunk_FUN_574dd15b` | `main_hdd_cmd_156` | 2 B | Thunk para `FUN_574dd15b` (`main_hdd_cmd_156`). |
| `/EH55_MAIN_3410PQ.bin` | 0x574dd15b | `FUN_574dd15b` | `main_hdd_cmd_1fc_containing` | 281 B | Função LBA48 contendo `0x574dd1fc` e o ramo `0x574dd208` ocultado pelo decompilador (`[RTSD] M7920(HDD 400GB)` vs `200GB`, `main_hdd_cmd_1fc_containing`). |
| `/DRV_decomp.bin` | 0x400277de | `FUN_400277de` | `drv_serial_rx_277de` | 197 B | Receptor serial de linha da CLI do drive (`drv_serial_rx_277de`). |
| `/DRV_decomp.bin` | 0x4002803a | `FUN_4002803a` | `drv_serial_cli_dispatch_2803a` | 144 B | Despachante da tabela de comandos da CLI serial (`drv_serial_cli_dispatch_2803a`). |
| `/DRV_decomp.bin` | 0x40029266 | `FUN_40029266` | `drv_unlock_helper_29266` | 220 B | Helper de validação/desbloqueio na CLI do drive (`drv_unlock_helper_29266`). |
| `/DRV_decomp.bin` | 0x40133ed0 | `FUN_40133ed0` | `drv_cli_slogon_133ed0` | 14 B | Comando CLI `SLOGON` (`drv_cli_slogon_133ed0`). |
| `/DRV_decomp.bin` | 0x40133ede | `FUN_40133ede` | `drv_cli_slogoff_133ede` | 13 B | Comando CLI `SLOGOFF` (`drv_cli_slogoff_133ede`). |
| `/DRV_decomp.bin` | 0x4013420a | `FUN_4013420a` | `drv_cli_pass_13420a` | 57 B | Comando CLI `PASS` em `/DRV_decomp.bin`: exige `0x4d415453` (`"MATS"`) / `0x48495441` (`"HITA"`) e seta `DAT_00000cd8 |= 1` (`drv_cli_pass_13420a`). |
| `/DRV_decomp.bin` | 0x40134243 | `FUN_40134243` | `drv_cli_ver_134243` | 3 B | Comando CLI `VER` (`drv_cli_ver_134243`). |
| `/DRV_decomp.bin` | 0x40134382 | `FUN_40134382` | `drv_cli_mmode_134382` | 64 B | Comando CLI `MMODE` (`drv_cli_mmode_134382`). |
| `/DRV_decomp.bin` | 0x401343c2 | `FUN_401343c2` | `drv_cli_smode_1343c2` | 64 B | Comando CLI `SMODE` (`drv_cli_smode_1343c2`). |
| `/DRV_decomp.bin` | 0x40134483 | `FUN_40134483` | `drv_cli_rb_134483` | 44 B | Comando CLI `RB` — Read Byte (`drv_cli_rb_134483`). |
| `/DRV_decomp.bin` | 0x401344e0 | `FUN_401344e0` | `drv_cli_wb_1344e0` | 85 B | Comando CLI `WB` — Write Byte (`drv_cli_wb_1344e0`). |
| `/DRV_decomp.bin` | 0x401345de | `FUN_401345de` | `drv_cli_eep_1345de` | 626 B | Comando CLI `EEP` — leitura/gravação EEPROM do drive (`drv_cli_eep_1345de`). |
| `/EH55_DRV_S211.bin` | `0x40130efa` | `FUN_40130efa` | `drv_s211_pass_130efa` | 57 B | Handler `PASS` (`MATS`/`HITA`, `BSET 0x1,(0xd78)`). |
| `/DRV_decomp.bin` | 0x40004852 | `FUN_40004852` | `drv_sense_lookup_4852` | 150 B | Lookup na tabela de Sense (`0x400089dc`) do drive (`drv_sense_lookup_4852`). |
| `/DRV_decomp.bin` | 0x40005772 | `FUN_40005772` | `drv_mecha_helper_5772` | 114 B | Helper de erro/estado mecânico com uso de strings em tabela (`drv_mecha_helper_5772`). |
| `/DRV_decomp.bin` | 0x40005aba | `FUN_40005aba` | `drv_atapi_sense_5aba` | 570 B | Construção de resposta `REQUEST SENSE` ATAPI (`drv_atapi_sense_5aba`). |
| `/DRV_decomp.bin` | 0x40005d46 | `FUN_40005d46` | `drv_atapi_err_5d46` | 243 B | Registro/propagação de erro ATAPI (`drv_atapi_err_5d46`). |
| `/DRV_decomp.bin` | 0x4001fcaf | `FUN_4001fcaf` | `drv_cmd_exec_1fcaf` | 394 B | Executor interno de comandos do drive (`drv_cmd_exec_1fcaf`). |
| `/DRV_decomp.bin` | 0x4002025e | `FUN_4002025e` | `drv_cmd_tbl_2025e` | 1484 B | Despacho por tabela de opcodes/subcomandos do drive (`drv_cmd_tbl_2025e`). |
| `/DRV_decomp.bin` | 0x400209f3 | `FUN_400209f3` | `drv_atapi_dispatch_209f3` | 1755 B | Despachante ATAPI principal — inclui opcode `0x3c` Write Buffer / Download Microcode (`drv_atapi_dispatch_209f3`). |
| `/DRV_decomp.bin` | 0x40021476 | `FUN_40021476` | `drv_atapi_pkt_21476` | 535 B | Recepção/preparo de pacote CDB ATAPI (`drv_atapi_pkt_21476`). |
| `/DRV_decomp.bin` | 0x4002168d | `FUN_4002168d` | `drv_atapi_irq_2168d` | 646 B | Rotina de interrupção/conclusão ATAPI (`drv_atapi_irq_2168d`). |
| `/DRV_decomp.bin` | 0x400d92b2 | `FUN_400d92b2` | `drv_ref_0x400d92b2` | 82 B | Referência cruzada ao subsistema Sense / erro em `0x400d92b2` (`drv_ref_0x400d92b2`). |

---

## Parte I — `/BOOT.BIN` (Estágio 1 / NOR Flash XIP)

### 1.1 Vetores de Reset, CRT0 (`.bss` + `.data` -> `0x50480000`), Nomes BGO & Strings

Esta subseção reúne os vetores brutos de reset (`0x44000000..0x440000a0` / `0x440000cf`), a desmontagem e decompilação completas do `crt0` (`0x4400133e`), a prova de cópia da região `.data` ROM (`0x44039830`) para RAM (`0x50480000`) revelando a chave Feistel exata em `0x44039aa8`, a tabela de nomes padrão BGO/VFD e os destaques da tabela de strings de `/BOOT.BIN`.

#### Vetores Brutos de Reset (`0x44000000..0x440000A0` — `doc_boot_core.txt`)

Instruções na tabela de vetores em `0x44000000` (salto inicial para `0x440000a0` e slots de exceção/interrupção).

<!-- source: doc_boot_core.txt | RAW RESET VECTORS 0x44000000..0x440000A0 -->
```asm
  0x44000000: f0 2c fe 09 88 00 00 00 04 c8 04 ca 07 dc 05 00
  0x44000010: 00 04 00 f2 f3 fe 01 04 10 00 c0 01 fe 01 02 10
  0x44000020: 00 c0 80 fc df 2c 00 00 44 cc 87 00 fc df 35 00
  0x44000030: 00 44 cc 16 01 fc df 3e 00 00 44 cc 62 01 fc df
  0x44000040: 47 00 00 44 cc 01 0e fc dc 00 00 00 53 f2 f0 fc
  0x44000050: df 58 00 00 44 cc 86 11 fc df 61 00 00 44 cc 95
  0x44000060: 11 fc df 6a 00 00 44 cc 3e 12 fc df 73 00 00 44
  0x44000070: cc a4 12 00 04 08 0c f9 68 88 f9 68 99 f9 68 aa
  0x44000080: f9 68 bb f9 68 00 f9 68 11 f9 68 22 f9 68 33 f9
  0x44000090: 68 44 f9 68 55 f9 68 66 f9 68 77 dc d4 14 00 00
```

#### Disassembly Detalhado de Reset e Setup Inicial (`0x44000000..0x440000cf` — `verify_ghidra_doc_facts_out.txt`)

```asm
--- BOOT.BIN 0x44000000..0x440000cf DISASM ---
  0x44000000: [f0 2c                ] MOV PC,A0
  0x44000002: [fe 09 88 00 00 00 04 ] AND 0x4000000,A0
  0x44000009: [c8 04                ] BEQ 0x4400000d
  0x4400000b: [ca 07                ] BRA 0x44000012
  0x4400000d: [dc 05 00 00 04       ] JMP 0x48000012
  0x44000012: [00                   ] CLR D0
  0x44000013: [f2 f3                ] MOV D0,PSW
  0x44000015: [fe 01 04 10 00 c0 01 ] BCLR 0x1,(0xc0001004)
  0x4400001c: [fe 01 02 10 00 c0 80 ] BCLR 0x80,(0xc0001002)
  0x44000023: [fc df 2c 00 00 44    ] MOV 0x4400002c,A3
  0x44000029: [cc 87 00             ] JMP 0x440000b0
  0x4400002c: [fc df 35 00 00 44    ] MOV 0x44000035,A3
  0x44000032: [cc 16 01             ] JMP 0x44000148
  0x44000035: [fc df 3e 00 00 44    ] MOV 0x4400003e,A3
  0x4400003b: [cc 62 01             ] JMP 0x4400019d
  0x4400003e: [fc df 47 00 00 44    ] MOV 0x44000047,A3
  0x44000044: [cc 01 0e             ] JMP 0x44000e45
  0x44000047: [fc                   ] ?? FCh
  0x44000048: [dc                   ] ?? DCh
  0x44000049: [00                   ] ?? 00h
  0x4400004a: [00                   ] ?? 00h
  0x4400004b: [00                   ] ?? 00h
  0x4400004c: [53                   ] ?? 53h    S
  0x4400004d: [f2                   ] ?? F2h
  0x4400004e: [f0                   ] ?? F0h
  0x4400004f: [fc                   ] ?? FCh
  0x44000050: [df                   ] ?? DFh
  0x44000051: [58                   ] ?? 58h    X
  0x44000052: [00                   ] ?? 00h
  0x44000053: [00                   ] ?? 00h
  0x44000054: [44                   ] ?? 44h    D
  0x44000055: [cc                   ] ?? CCh
  0x44000056: [86                   ] ?? 86h
  0x44000057: [11                   ] ?? 11h
  0x44000058: [fc                   ] ?? FCh
  0x44000059: [df                   ] ?? DFh
  0x4400005a: [61                   ] ?? 61h    a
  0x4400005b: [00                   ] ?? 00h
  0x4400005c: [00                   ] ?? 00h
  0x4400005d: [44                   ] ?? 44h    D
  0x4400005e: [cc                   ] ?? CCh
  0x4400005f: [95                   ] ?? 95h
  0x44000060: [11                   ] ?? 11h
  0x44000061: [fc                   ] ?? FCh
  0x44000062: [df                   ] ?? DFh
  0x44000063: [6a                   ] ?? 6Ah    j
  0x44000064: [00                   ] ?? 00h
  0x44000065: [00                   ] ?? 00h
  0x44000066: [44                   ] ?? 44h    D
  0x44000067: [cc                   ] ?? CCh
  0x44000068: [3e                   ] ?? 3Eh    >
  0x44000069: [12                   ] ?? 12h
  0x4400006a: [fc                   ] ?? FCh
  0x4400006b: [df                   ] ?? DFh
  0x4400006c: [73                   ] ?? 73h    s
  0x4400006d: [00                   ] ?? 00h
  0x4400006e: [00                   ] ?? 00h
  0x4400006f: [44                   ] ?? 44h    D
  0x44000070: [cc                   ] ?? CCh
  0x44000071: [a4                   ] ?? A4h
  0x44000072: [12                   ] ?? 12h
  0x44000073: [00                   ] ?? 00h
  0x44000074: [04                   ] ?? 04h
  0x44000075: [08                   ] ?? 08h
  0x44000076: [0c                   ] ?? 0Ch
  0x44000077: [f9                   ] ?? F9h
  0x44000078: [68                   ] ?? 68h    h
  0x44000079: [88                   ] ?? 88h
  0x4400007a: [f9                   ] ?? F9h
  0x4400007b: [68                   ] ?? 68h    h
  0x4400007c: [99                   ] ?? 99h
  0x4400007d: [f9                   ] ?? F9h
  0x4400007e: [68                   ] ?? 68h    h
  0x4400007f: [aa                   ] ?? AAh
  0x44000080: [f9                   ] ?? F9h
  0x44000081: [68                   ] ?? 68h    h
  0x44000082: [bb                   ] ?? BBh
  0x44000083: [f9                   ] ?? F9h
  0x44000084: [68                   ] ?? 68h    h
  0x44000085: [00                   ] ?? 00h
  0x44000086: [f9                   ] ?? F9h
  0x44000087: [68                   ] ?? 68h    h
  0x44000088: [11                   ] ?? 11h
  0x44000089: [f9                   ] ?? F9h
  0x4400008a: [68                   ] ?? 68h    h
  0x4400008b: [22                   ] ?? 22h    "
  0x4400008c: [f9                   ] ?? F9h
  0x4400008d: [68                   ] ?? 68h    h
  0x4400008e: [33                   ] ?? 33h    3
  0x4400008f: [f9                   ] ?? F9h
  0x44000090: [68                   ] ?? 68h    h
  0x44000091: [44                   ] ?? 44h    D
  0x44000092: [f9                   ] ?? F9h
  0x44000093: [68                   ] ?? 68h    h
  0x44000094: [55                   ] ?? 55h    U
  0x44000095: [f9                   ] ?? F9h
  0x44000096: [68                   ] ?? 68h    h
  0x44000097: [66                   ] ?? 66h    f
  0x44000098: [f9                   ] ?? F9h
  0x44000099: [68                   ] ?? 68h    h
  0x4400009a: [77                   ] ?? 77h    w
  0x4400009b: [dc                   ] ?? DCh
  0x4400009c: [d4                   ] ?? D4h
  0x4400009d: [14                   ] ?? 14h
  0x4400009e: [00                   ] ?? 00h
  0x4400009f: [00                   ] ?? 00h
  0x440000a0: [fe                   ] ?? FEh
  0x440000a1: [01                   ] ?? 01h
  0x440000a2: [04                   ] ?? 04h
  0x440000a3: [10                   ] ?? 10h
  0x440000a4: [00                   ] ?? 00h
  0x440000a5: [c0                   ] ?? C0h
  0x440000a6: [01                   ] ?? 01h
  0x440000a7: [fe                   ] ?? FEh
  0x440000a8: [00                   ] ?? 00h
  0x440000a9: [04                   ] ?? 04h
  0x440000aa: [10                   ] ?? 10h
  0x440000ab: [00                   ] ?? 00h
  0x440000ac: [c0                   ] ?? C0h
  0x440000ad: [01                   ] ?? 01h
  0x440000ae: [ca                   ] ?? CAh
  0x440000af: [f2                   ] ?? F2h
  0x440000b0: [fc cc 20 55 04 13    ] MOV 0x13045520,D0
  0x440000b6: [fc 81 00 20 00 c0    ] MOV D0,(0xc0002000)
  0x440000bc: [fc dc 00 00 c0 d8    ] MOV 0xd8c00000,A0
  0x440000c2: [fc cc 01 fc 00 80    ] MOV 0x8000fc01,D0
  0x440000c8: [fa 10 00 01          ] MOV D0,(0x100,A0)
  0x440000cc: [fc cc 00 40 44 a7    ] MOV 0xa7444000,D0
```

#### `crt0` Completo (`0x4400133e`): Disassembly, Decompilação e Dump `.data` -> `0x50480000` (`verified_extra_facts.txt` §2)

Prova direta da cópia de `0x44039830..0x4403a3af` (`0x0b80` B) para `0x50480000`, fixando `0x50480278 == 0x44039aa8` (chave Feistel de 15 bytes: `72 eb ca 91 30 45 68 ad 99 df 56 72 3a b8 10`) e as strings VFD em `0x44039a1c..0x44039a8b`.

```text
-- BOOT DISASM 0x440012a0..0x440013b0 --
  0x440012a0: [d8                   ] ?? D8h
  0x440012a1: [18                   ] ?? 18h
  0x440012a2: [d3                   ] ?? D3h
  0x440012a3: [f0                   ] ?? F0h
  0x440012a4: [f7                   ] ?? F7h
  0x440012a5: [fc                   ] ?? FCh
  0x440012a6: [dc                   ] ?? DCh
  0x440012a7: [00                   ] ?? 00h
  0x440012a8: [00                   ] ?? 00h
  0x440012a9: [48                   ] ?? 48h    H
  0x440012aa: [50                   ] ?? 50h    P
  0x440012ab: [fc                   ] ?? FCh
  0x440012ac: [dd                   ] ?? DDh
  0x440012ad: [00                   ] ?? 00h
  0x440012ae: [00                   ] ?? 00h
  0x440012af: [48                   ] ?? 48h    H
  0x440012b0: [50                   ] ?? 50h    P
  0x440012b1: [b1                   ] ?? B1h
  0x440012b2: [c8                   ] ?? C8h
  0x440012b3: [2f                   ] ?? 2Fh    /
  0x440012b4: [fc                   ] ?? FCh
  0x440012b5: [dc                   ] ?? DCh
  0x440012b6: [20                   ] ?? 20h     
  0x440012b7: [14                   ] ?? 14h
  0x440012b8: [00                   ] ?? 00h
  0x440012b9: [44                   ] ?? 44h    D
  0x440012ba: [fc                   ] ?? FCh
  0x440012bb: [dd                   ] ?? DDh
  0x440012bc: [00                   ] ?? 00h
  0x440012bd: [00                   ] ?? 00h
  0x440012be: [48                   ] ?? 48h    H
  0x440012bf: [50                   ] ?? 50h    P
  0x440012c0: [db                   ] ?? DBh
  0x440012c1: [f9                   ] ?? F9h
  0x440012c2: [6a                   ] ?? 6Ah    j
  0x440012c3: [c8                   ] ?? C8h
  0x440012c4: [f9                   ] ?? F9h
  0x440012c5: [7a                   ] ?? 7Ah    z
  0x440012c6: [c9                   ] ?? C9h
  0x440012c7: [fc                   ] ?? FCh
  0x440012c8: [d9                   ] ?? D9h
  0x440012c9: [00                   ] ?? 00h
  0x440012ca: [00                   ] ?? 00h
  0x440012cb: [48                   ] ?? 48h    H
  0x440012cc: [50                   ] ?? 50h    P
  0x440012cd: [d0                   ] ?? D0h
  0x440012ce: [fc                   ] ?? FCh
  0x440012cf: [cc                   ] ?? CCh
  0x440012d0: [00                   ] ?? 00h
  0x440012d1: [00                   ] ?? 00h
  0x440012d2: [48                   ] ?? 48h    H
  0x440012d3: [50                   ] ?? 50h    P
  0x440012d4: [fc                   ] ?? FCh
  0x440012d5: [cd                   ] ?? CDh
  0x440012d6: [00                   ] ?? 00h
  0x440012d7: [00                   ] ?? 00h
  0x440012d8: [48                   ] ?? 48h    H
  0x440012d9: [50                   ] ?? 50h    P
  0x440012da: [f1                   ] ?? F1h
  0x440012db: [01                   ] ?? 01h
  0x440012dc: [cd                   ] ?? CDh
  0x440012dd: [cb                   ] ?? CBh
  0x440012de: [00                   ] ?? 00h
  0x440012df: [00                   ] ?? 00h
  0x440012e0: [00                   ] ?? 00h
  0x440012e1: [fc                   ] ?? FCh
  0x440012e2: [dc                   ] ?? DCh
  0x440012e3: [00                   ] ?? 00h
  0x440012e4: [00                   ] ?? 00h
  0x440012e5: [00                   ] ?? 00h
  0x440012e6: [c0                   ] ?? C0h
  0x440012e7: [fc                   ] ?? FCh
  0x440012e8: [cc                   ] ?? CCh
  0x440012e9: [00                   ] ?? 00h
  0x440012ea: [00                   ] ?? 00h
  0x440012eb: [00                   ] ?? 00h
  0x440012ec: [50                   ] ?? 50h    P
  0x440012ed: [f8                   ] ?? F8h
  0x440012ee: [10                   ] ?? 10h
  0x440012ef: [24                   ] ?? 24h    $
  0x440012f0: [80                   ] ?? 80h
  0x440012f1: [40                   ] ?? 40h    @
  0x440012f2: [f0                   ] ?? F0h
  0x440012f3: [70                   ] ?? 70h    p
  0x440012f4: [80                   ] ?? 80h
  0x440012f5: [48                   ] ?? 48h    H
  0x440012f6: [f8                   ] ?? F8h
  0x440012f7: [70                   ] ?? 70h    p
  0x440012f8: [04                   ] ?? 04h
  0x440012f9: [80                   ] ?? 80h
  0x440012fa: [50                   ] ?? 50h    P
  0x440012fb: [f8                   ] ?? F8h
  0x440012fc: [70                   ] ?? 70h    p
  0x440012fd: [08                   ] ?? 08h
  0x440012fe: [80                   ] ?? 80h
  0x440012ff: [58                   ] ?? 58h    X
  0x44001300: [f8                   ] ?? F8h
  0x44001301: [70                   ] ?? 70h    p
  0x44001302: [0c                   ] ?? 0Ch
  0x44001303: [80                   ] ?? 80h
  0x44001304: [60                   ] ?? 60h    `
  0x44001305: [f8                   ] ?? F8h
  0x44001306: [70                   ] ?? 70h    p
  0x44001307: [10                   ] ?? 10h
  0x44001308: [80                   ] ?? 80h
  0x44001309: [68                   ] ?? 68h    h
  0x4400130a: [f8                   ] ?? F8h
  0x4400130b: [70                   ] ?? 70h    p
  0x4400130c: [14                   ] ?? 14h
  0x4400130d: [80                   ] ?? 80h
  0x4400130e: [70                   ] ?? 70h    p
  0x4400130f: [f8                   ] ?? F8h
  0x44001310: [70                   ] ?? 70h    p
  0x44001311: [18                   ] ?? 18h
  0x44001312: [f0                   ] ?? F0h
  0x44001313: [f7                   ] ?? F7h
  0x44001314: [fc                   ] ?? FCh
  0x44001315: [dc                   ] ?? DCh
  0x44001316: [80                   ] ?? 80h
  0x44001317: [0b                   ] ?? 0Bh
  0x44001318: [48                   ] ?? 48h    H
  0x44001319: [50                   ] ?? 50h    P
  0x4400131a: [fc                   ] ?? FCh
  0x4400131b: [dd                   ] ?? DDh
  0x4400131c: [0c                   ] ?? 0Ch
  0x4400131d: [e0                   ] ?? E0h
  0x4400131e: [59                   ] ?? 59h    Y
  0x4400131f: [50                   ] ?? 50h    P
  0x44001320: [b1                   ] ?? B1h
  0x44001321: [c8                   ] ?? C8h
  0x44001322: [09                   ] ?? 09h
  0x44001323: [00                   ] ?? 00h
  0x44001324: [db                   ] ?? DBh
  0x44001325: [f9                   ] ?? F9h
  0x44001326: [7a                   ] ?? 7Ah    z
  0x44001327: [c8                   ] ?? C8h
  0x44001328: [b1                   ] ?? B1h
  0x44001329: [d1                   ] ?? D1h
  0x4400132a: [fc                   ] ?? FCh
  0x4400132b: [dc                   ] ?? DCh
  0x4400132c: [80                   ] ?? 80h
  0x4400132d: [e0                   ] ?? E0h
  0x4400132e: [59                   ] ?? 59h    Y
  0x4400132f: [50                   ] ?? 50h    P
  0x44001330: [fc                   ] ?? FCh
  0x44001331: [dd                   ] ?? DDh
  0x44001332: [80                   ] ?? 80h
  0x44001333: [e0                   ] ?? E0h
  0x44001334: [59                   ] ?? 59h    Y
  0x44001335: [50                   ] ?? 50h    P
  0x44001336: [b1                   ] ?? B1h
  0x44001337: [c8                   ] ?? C8h
  0x44001338: [09                   ] ?? 09h
  0x44001339: [00                   ] ?? 00h
  0x4400133a: [db                   ] ?? DBh
  0x4400133b: [f9                   ] ?? F9h
  0x4400133c: [7a                   ] ?? 7Ah    z
  0x4400133d: [c8                   ] ?? C8h
  0x4400133e: [b1                   ] CMP A0,A1
  0x4400133f: [d1                   ] LGT
  0x44001340: [fc dc 00 00 48 50    ] MOV 0x50480000,A0
  0x44001346: [fc dd 80 0b 48 50    ] MOV 0x50480b80,A1
  0x4400134c: [b1                   ] CMP A0,A1
  0x4400134d: [c8 1c                ] BEQ 0x44001369
  0x4400134f: [fc dc 30 98 03 44    ] MOV 0x44039830,A0
  0x44001355: [fc dd 00 00 48 50    ] MOV 0x50480000,A1
  0x4400135b: [db                   ] SETLB
  0x4400135c: [f9 6a c8             ] MOV (A0+),D0
  0x4400135f: [f9 7a c9             ] MOV D0,(A1+)
  0x44001362: [fc d9 80 0b 48 50    ] CMP 0x50480b80,A1
  0x44001368: [d0                   ] LLT
  0x44001369: [fc dc 00 00 48 50    ] MOV 0x50480000,A0
  0x4400136f: [fc dd 00 00 48 50    ] MOV 0x50480000,A1
  0x44001375: [b1                   ] CMP A0,A1
  0x44001376: [c8 2f                ] BEQ 0x440013a5
  0x44001378: [fc dc 20 14 00 44    ] MOV 0x44001420,A0
  0x4400137e: [fc dd 00 00 48 50    ] MOV 0x50480000,A1
  0x44001384: [db                   ] SETLB
  0x44001385: [f9 6a c8             ] MOV (A0+),D0
  0x44001388: [f9 7a c9             ] MOV D0,(A1+)
  0x4400138b: [fc d9 00 00 48 50    ] CMP 0x50480000,A1
  0x44001391: [d0                   ] LLT
  0x44001392: [fc cc 00 00 48 50    ] MOV 0x50480000,D0
  0x44001398: [fc cd 00 00 48 50    ] MOV 0x50480000,D1
  0x4400139e: [f1 01                ] SUB D0,D1
  0x440013a0: [cd 07 00 00 00       ] CALL 0x440013a7,[],0x0
  0x440013a5: [f0 f7                ] JMP (A3)
  0x440013a7: [cf c0                ] MOVM [D2,D3],(SP)
  0x440013a9: [82                   ] MOV D0,D2
  0x440013aa: [fc e0 f0 ff ff ff    ] AND 0xfffffff0,D0
  0x440013b0: [e9                   ] ADD D2,D1

-- BOOT DECOMP 0x4400133e --
/* WARNING: Removing unreachable block (ram,0x44001378) */
/* WARNING: Removing unreachable block (ram,0x44001385) */
/* WARNING: Removing unreachable block (ram,0x44001392) */
/* WARNING: Removing unreachable block (ram,0x44001391) */

void FUN_4400133e(void)

{
  int in_A0;
  undefined4 *puVar1;
  int in_A1;
  undefined4 *puVar2;
  code *UNRECOVERED_JUMPTABLE;
  undefined4 uVar3;
  int in_LAR;
  
                    /* WARNING: Could not recover jumptable at 0x4400133f. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  if (in_A1 <= in_A0) {
    puVar1 = (undefined4 *)0x44039830;
    puVar2 = (undefined4 *)&DAT_50480000;
    do {
      uVar3 = *puVar1;
      puVar1 = puVar1 + 1;
      *puVar2 = uVar3;
      puVar2 = puVar2 + 1;
    } while ((int)puVar2 < 0x50480b80);
                    /* WARNING: Could not recover jumptable at 0x440013a5. Too many branches */
                    /* WARNING: Treating indirect jump as call */
    (*UNRECOVERED_JUMPTABLE)();
    return;
  }
  (*(code *)(in_LAR + -4))();
  return;
}

-- BOOT .DATA / RODATA 0x44039800..0x44039adf --
  0x44039800: 69 74 79 00 49 53 4f 2d 38 38 35 39 2d 31 00 48 |ity.ISO-8859-1.H|
  0x44039810: 65 61 70 20 61 6e 64 20 73 74 61 63 6b 20 63 6f |eap and stack co|
  0x44039820: 6c 6c 69 73 69 6f 6e 0a 00 00 00 00 00 00 00 00 |llision.........|
  0x44039830: ff ff ff ff 69 66 6f 64 65 76 31 00 00 6d 69 6e |....ifodev1..min|
  0x44039840: 69 62 6f 6f 74 00 6d 61 69 6e 73 6f 66 74 00 69 |iboot.mainsoft.i|
  0x44039850: 66 6f 64 65 76 32 00 00 6e 65 74 66 69 72 6d 00 |fodev2..netfirm.|
  0x44039860: 00 6e 65 74 69 66 6f 31 00 00 73 79 73 66 61 63 |.netifo1..sysfac|
  0x44039870: 74 00 00 66 73 31 2e 64 61 74 00 00 66 73 32 2e |t..fs1.dat..fs2.|
  0x44039880: 64 61 74 00 00 66 73 33 2e 64 61 74 00 00 66 73 |dat..fs3.dat..fs|
  0x44039890: 34 2e 64 61 74 00 00 63 6f 6e 66 69 67 31 00 00 |4.dat..config1..|
  0x440398a0: 72 74 73 64 31 00 00 00 00 72 74 73 64 32 00 00 |rtsd1....rtsd2..|
  0x440398b0: 00 00 63 6f 6e 66 69 67 32 00 00 67 75 69 64 65 |..config2..guide|
  0x440398c0: 00 00 00 00 64 65 62 75 67 00 00 00 00 75 70 64 |....debug....upd|
  0x440398d0: 61 74 65 00 00 00 64 72 76 66 69 72 6d 00 00 66 |ate...drvfirm..f|
  0x440398e0: 76 75 2e 65 78 65 00 00 63 69 6d 5f 64 76 64 00 |vu.exe..cim_dvd.|
  0x440398f0: 00 63 69 6d 5f 68 64 64 00 00 73 79 73 5f 68 64 |.cim_hdd..sys_hd|
  0x44039900: 64 00 00 6e 6d 69 63 6e 74 00 00 00 66 73 35 2e |d..nmicnt...fs5.|
  0x44039910: 64 61 74 00 00 66 73 36 2e 64 61 74 00 00 72 74 |dat..fs6.dat..rt|
  0x44039920: 73 64 33 00 00 00 00 72 74 73 64 34 00 00 00 00 |sd3....rtsd4....|
  0x44039930: 69 74 63 2e 64 61 74 00 00 65 70 67 64 61 74 61 |itc.dat..epgdata|
  0x44039940: 00 00 65 70 67 63 6f 6e 66 00 00 66 73 5f 6d 70 |..epgconf..fs_mp|
  0x44039950: 67 34 00 00 66 74 72 65 63 00 00 00 00 74 76 67 |g4..ftrec....tvg|
  0x44039960: 6f 73 00 00 00 00 66 69 6f 5f 72 65 63 76 00 66 |os....fio_recv.f|
  0x44039970: 69 6f 5f 62 75 70 00 00 66 69 6f 5f 73 70 72 30 |io_bup..fio_spr0|
  0x44039980: 00 66 69 6f 5f 73 70 72 31 00 66 69 6f 5f 73 70 |.fio_spr1.fio_sp|
  0x44039990: 72 32 00 66 69 6f 5f 73 70 72 33 00 66 73 37 2e |r2.fio_spr3.fs7.|
  0x440399a0: 64 61 74 00 00 74 76 67 6f 73 5f 63 31 00 74 76 |dat..tvgos_c1.tv|
  0x440399b0: 67 6f 73 5f 63 32 00 72 74 73 64 35 00 00 00 00 |gos_c2.rtsd5....|
  0x440399c0: 65 78 74 65 6e 64 00 00 00 72 74 73 64 36 00 00 |extend...rtsd6..|
  0x440399d0: 00 00 65 70 67 64 61 74 61 31 00 65 70 67 6d 6e |..epgdata1.epgmn|
  0x440399e0: 67 00 00 00 68 64 63 70 73 72 6d 00 00 73 65 72 |g...hdcpsrm..ser|
  0x440399f0: 65 69 73 69 64 00 63 67 69 64 00 00 00 00 00 72 |eisid.cgid.....r|
  0x44039a00: 73 76 5f 6c 69 6e 6b 00 72 73 76 5f 63 6f 6d 70 |sv_link.rsv_comp|
  0x44039a10: 00 00 00 00 ff ff 00 00 01 00 00 00 4c 44 20 46 |............LD F|
  0x44039a20: 56 55 00 49 4e 53 45 52 54 00 20 20 44 49 53 4b |VU.INSERT.  DISK|
  0x44039a30: 00 4e 47 44 49 53 4b 00 4e 4f 20 46 56 55 00 44 |.NGDISK.NO FVU.D|
  0x44039a40: 4c 20 45 52 52 00 53 55 4d 20 4e 47 00 44 52 56 |L ERR.SUM NG.DRV|
  0x44039a50: 20 4e 47 00 53 54 46 41 49 4c 00 54 49 4d 4f 55 | NG.STFAIL.TIMOU|
  0x44039a60: 54 00 54 52 4f 55 42 4c 00 44 52 56 20 4e 47 00 |T.TROUBL.DRV NG.|
  0x44039a70: 55 39 39 20 20 20 00 46 4d 4c 49 53 54 00 44 4c |U99   .FMLIST.DL|
  0x44039a80: 20 45 52 52 00 48 44 44 45 52 52 00 01 00 00 00 | ERR.HDDERR.....|
  0x44039a90: 00 00 00 00 01 00 00 00 02 00 00 00 03 00 00 00 |................|
  0x44039aa0: 04 00 00 00 05 00 00 00 72 eb ca 91 30 45 68 ad |........r...0Eh.|
  0x44039ab0: 99 df 56 72 3a b8 10 00 02 00 00 00 04 00 00 00 |..Vr:...........|
  0x44039ac0: 00 00 00 00 05 00 00 00 00 00 00 00 06 00 00 00 |................|
  0x44039ad0: 00 00 00 00 07 00 00 00 00 00 00 00 08 00 00 00 |................|
```

#### Nomes Padrão de Partições BGO (`0x44039820..` — `.data` copiada para `0x50480000`)

Tabela de nomes de partições BGO carregada em `0x50480000` (`ifodev1`, `miniboot`, `mainsoft`, etc.).

<!-- source: doc_boot_core.txt | BOOT.BIN BGO PARTITION DEFAULT NAMES (.data copy -> 0x50480000) -->
```text
  0x44039820: 6c 6c 69 73 69 6f 6e 0a 00 00 00 00 00 00 00 00  |llision.........|
  0x44039830: ff ff ff ff 69 66 6f 64 65 76 31 00 00 6d 69 6e  |....ifodev1..min|
  0x44039840: 69 62 6f 6f 74 00 6d 61 69 6e 73 6f 66 74 00 69  |iboot.mainsoft.i|
  0x44039850: 66 6f 64 65 76 32 00 00 6e 65 74 66 69 72 6d 00  |fodev2..netfirm.|
  0x44039860: 00 6e 65 74 69 66 6f 31 00 00 73 79 73 66 61 63  |.netifo1..sysfac|
  0x44039870: 74 00 00 66 73 31 2e 64 61 74 00 00 66 73 32 2e  |t..fs1.dat..fs2.|
  0x44039880: 64 61 74 00 00 66 73 33 2e 64 61 74 00 00 66 73  |dat..fs3.dat..fs|
  0x44039890: 34 2e 64 61 74 00 00 63 6f 6e 66 69 67 31 00 00  |4.dat..config1..|
  0x440398a0: 72 74 73 64 31 00 00 00 00 72 74 73 64 32 00 00  |rtsd1....rtsd2..|
  0x440398b0: 00 00 63 6f 6e 66 69 67 32 00 00 67 75 69 64 65  |..config2..guide|
  0x440398c0: 00 00 00 00 64 65 62 75 67 00 00 00 00 75 70 64  |....debug....upd|
  0x440398d0: 61 74 65 00 00 00 64 72 76 66 69 72 6d 00 00 66  |ate...drvfirm..f|
  0x440398e0: 76 75 2e 65 78 65 00 00 63 69 6d 5f 64 76 64 00  |vu.exe..cim_dvd.|
  0x440398f0: 00 63 69 6d 5f 68 64 64 00 00 73 79 73 5f 68 64  |.cim_hdd..sys_hd|
  0x44039900: 64 00 00 6e 6d 69 63 6e 74 00 00 00 66 73 35 2e  |d..nmicnt...fs5.|
  0x44039910: 64 61 74 00 00 66 73 36 2e 64 61 74 00 00 72 74  |dat..fs6.dat..rt|
  0x44039920: 73 64 33 00 00 00 00 72 74 73 64 34 00 00 00 00  |sd3....rtsd4....|
  0x44039930: 69 74 63 2e 64 61 74 00 00 65 70 67 64 61 74 61  |itc.dat..epgdata|
  0x44039940: 00 00 65 70 67 63 6f 6e 66 00 00 66 73 5f 6d 70  |..epgconf..fs_mp|
  0x44039950: 67 34 00 00 66 74 72 65 63 00 00 00 00 74 76 67  |g4..ftrec....tvg|
  0x44039960: 6f 73 00 00 00 00 66 69 6f 5f 72 65 63 76 00 66  |os....fio_recv.f|
  0x44039970: 69 6f 5f 62 75 70 00 00 66 69 6f 5f 73 70 72 30  |io_bup..fio_spr0|
  0x44039980: 00 66 69 6f 5f 73 70 72 31 00 66 69 6f 5f 73 70  |.fio_spr1.fio_sp|
  0x44039990: 72 32 00 66 69 6f 5f 73 70 72 33 00 66 73 37 2e  |r2.fio_spr3.fs7.|
  0x440399a0: 64 61 74 00 00 74 76 67 6f 73 5f 63 31 00 74 76  |dat..tvgos_c1.tv|
  0x440399b0: 67 6f 73 5f 63 32 00 72 74 73 64 35 00 00 00 00  |gos_c2.rtsd5....|
  0x440399c0: 65 78 74 65 6e 64 00 00 00 72 74 73 64 36 00 00  |extend...rtsd6..|
  0x440399d0: 00 00 65 70 67 64 61 74 61 31 00 65 70 67 6d 6e  |..epgdata1.epgmn|
  0x440399e0: 67 00 00 00 68 64 63 70 73 72 6d 00 00 73 65 72  |g...hdcpsrm..ser|
  0x440399f0: 65 69 73 69 64 00 63 67 69 64 00 00 00 00 00 72  |eisid.cgid.....r|
  0x44039a00: 73 76 5f 6c 69 6e 6b 00 72 73 76 5f 63 6f 6d 70  |sv_link.rsv_comp|
  0x44039a10: 00 00 00 00 ff ff 00 00 01 00 00 00 4c 44 20 46  |............LD F|
  0x44039a20: 56 55 00 49 4e 53 45 52 54 00 20 20 44 49 53 4b  |VU.INSERT.  DISK|
  0x44039a30: 00 4e 47 44 49 53 4b 00 4e 4f 20 46 56 55 00 44  |.NGDISK.NO FVU.D|
  0x44039a40: 4c 20 45 52 52 00 53 55 4d 20 4e 47 00 44 52 56  |L ERR.SUM NG.DRV|
  0x44039a50: 20 4e 47 00 53 54 46 41 49 4c 00 54 49 4d 4f 55  | NG.STFAIL.TIMOU|
  0x44039a60: 54 00 54 52 4f 55 42 4c 00 44 52 56 20 4e 47 00  |T.TROUBL.DRV NG.|
  0x44039a70: 55 39 39 20 20 20 00 46 4d 4c 49 53 54 00 44 4c  |U99   .FMLIST.DL|
  0x44039a80: 20 45 52 52 00 48 44 44 45 52 52 00 01 00 00 00  | ERR.HDDERR.....|
  0x44039a90: 00 00 00 00 01 00 00 00 02 00 00 00 03 00 00 00  |................|
  0x44039aa0: 04 00 00 00 05 00 00 00 72 eb ca 91 30 45 68 ad  |........r...0Eh.|
  0x44039ab0: 99 df 56 72 3a b8 10 00 02 00 00 00 04 00 00 00  |..Vr:...........|
```

#### Destaques da Tabela de Strings de `/BOOT.BIN` (`0x440332E0..0x44035680`)

Strings literais extraídas da região `0x440332E0..0x44035680` em `/BOOT.BIN`.

<!-- source: doc_boot_core.txt | BOOT.BIN STRING TABLE HIGHLIGHTS 0x440332E0..0x44035680 -->
```text
  0x440332e3: 'ò\x93\x88\x8dÊó'
  0x440332f0: '[nfrom] Format error[%d].\n'
  0x4403330b: '[nfrom] File map init error[%d].\n'
  0x4403332d: '[nfrom] Data omap init error[%d].\n'
  0x44033350: '[nfrom] Data bmap init error[%d].\n'
  0x44033373: '[nfrom] Main data init error[%d].\n'
  0x44033396: '[nfrom] NOR STATE : Backup -> Original\n'
  0x440333be: '[nfrom] NOR STATE : Original -> Backup\n'
  0x440333e6: '[nfrom] Map load error[%u].\n'
  0x44033403: '[nfrom] Map write error[%u].\n'
  0x44033421: '[nfrom] File map load error[%u].\n'
  0x44033443: '[nfrom] File map write error[%u].\n'
  0x44033466: '[nfrom] Status change error[%d].\n'
  0x44033488: '[nfrom] Status flag change error[%u].\n'
  0x440334af: '[nfrom] bgo_read_D(ID=0x%X) return error(%u).\n'
  0x440334de: '[nfrom] BGO command[%d] error.\n'
  0x44033523: 'D[nfrom] bgo_writeD(ID=0x%X) return error(%u).\n'
  0x44033553: '[nfrom] No free ID.\n'
  0x44033568: '[nfrom] BGO write error[%u].\n'
  0x44033586: '[nfrom] Free ID find error[%d].\n'
  0x440335a7: '[nfrom] Data omap update error[%d].\n'
  0x440335cc: '[nfrom] Ctrl info error(id=0x%X,po=0x%X).\n'
  0x440335f7: '[nfrom] File omap update error[%u].\n'
  0x4403361c: '[nfrom] Data bmap update error[%d].\n'
  0x44033641: '[nfrom] File bmap update error[%u].\n'
  0x44033666: '[nfrom] New file write start error[%d].\n'
  0x4403368f: '[nfrom] New data write error[%u].\n'
  0x440336b2: '[nfrom] New data write[%u].\n'
  0x440336cf: '[nfrom] New file write end error[%d].\n'
  0x440336f6: '[nfrom] Data read error[%u].\n'
  0x44033714: '[nfrom] Data write error[%d].\n'
  0x44033733: '[nfrom] Data write error[%u].\n'
  0x44033752: '[nfrom] file_no[%d],skip(%d),non_skip(%d),erase(%d),new_write(%d).\n'
  0x44033796: '[nfrom] File rewrite start error[%d].\n'
  0x440337bd: '[nfrom] File erase error[%d].\n'
  0x440337dc: '[nfrom] File rewrite error[%d].\n'
  0x440337fd: '[nfrom] File rewrite end error[%d].\n'
  0x44033822: '[nfrom] bgo_writeD_FF(ID=0x%X) return error(%u).\n'
  0x44033854: '[nfrom] BGO read error[%u].\n'
  0x44033871: '[nfrom] 0xFF data write error[%u].\n'
  0x44033895: '[nfrom] 0xFF file write end error[%d].\n'
  0x440338bd: '[nfrom] Main control data error[%u].\n'
  0x440338e3: '[nfrom] Power failer recovery error[%d].\n'
  0x4403390d: '********** Main Control Data **********\n'
  0x44033936: 'STATUS      : 0x%X\n'
  0x4403394a: 'CTRL ID NUM : %d\n'
  0x4403395c: 'FILE NUM    : %d\n'
  0x4403396e: 'DATA ID NUM : %d\n'
  0x44033980: 'FREE ID NUM : %d\n'
  0x44033992: 'DATA SIZE   : %d\n'
  0x440339a4: 'MAKER ID    : 0x%04X\n'
  0x440339ba: 'DEVICE ID   : 0x%04X\n'
  0x440339d0: '********** Original File Map **********\n'
  0x440339f9: 'NO , NAME     , SIZE       , START ID , COMP\n'
  0x44033a27: '%2d , %8s , 0x%-8X , 0x%04X   , 0x%02X\n'
  0x44033a4f: '********** BackUp File Map **********\n'
  0x44033a76: '********** Data ID Map **********\n'
  0x44033a99: '----- %2d data -----\n'
  0x44033aaf: '%04X '
  0x44033ab8: '\n***** Loader ver%s [%s](%s) *****\n'
  0x44033adc: '0.63'
  0x44033ae1: '07/10/24'
  0x44033aea: '4MB/TOP BOOT'
  0x44033af7: '***** FEonly mode *****\n'
  0x44033b10: '***** support card mode *****\n'
  0x44033b2f: 'load_address = 0x%08x , load_size = 0x%x\n\n'
  0x44033b5a: 'error! main soft is not loaded to 0x40000400.\n'
  0x44033b89: '***** normal mode *****\n'
  0x44033ba2: '***** serial boot mode *****\n'
  0x44033bc0: 'LAST_ERR_LOG load OK(0x%08X).\n'
  0x44033bdf: 'LAST_ERR_LOG load NG(%d).\n'
  0x44033bfa: 'LAST_ERR_LOG load NG.\n'
  0x44033c11: 'Copy to update program(size=0x%X) to 0x%08X from 0x%08X.\n'
  0x44033c4b: '*** jump to update program ***\n'
  0x44033c6b: 'F-CPU reset finish. 0x%X-0x%X data polling.\n'
  0x44033c98: 'Drive firmware update start.\n'
  0x44033cb6: 'I am BootLoader...\n'
  0x44033cca: 'Drive adjustment value update firmware load start.\n'
  0x44033cfe: 'LAST_ERR_LOG save NG.\n'
  0x44033d15: 'LAST_ERR_LOG save OK.\n'
  0x44033d2c: 'LAST_ERR_LOG save NG(%d).\n'
  0x44033d47: '***** F-CPU BAND change start *****\n'
  0x44033d6c: 'FE DMD(tmin) changed to 0x%04X.\n'
  0x44033d8d: 'FE DMD(tper) changed to 0x%04X.\n'
  0x44033dae: 'FE DMD(tlim) changed to 0x%04X.\n'
  0x44033dcf: 'FE DMD not aotenjyo.\n'
  0x44033de5: 'FE DMD aotenjyo.\n'
  0x44033df7: 'FE C13(tmin) changed to 0x%04X.\n'
  0x44033e18: 'FE C13(tper) changed to 0x%04X.\n'
  0x44033e39: 'FE C13(tlim) changed to 0x%04X.\n'
  0x44033e5a: 'FE C13 not aotenjyo.\n'
  0x44033e70: 'FE C13 aotenjyo.\n'
  0x44033e82: 'FE C4A(tmin) changed to 0x%04X.\n'
  0x44033ea3: 'FE C4A(tper) changed to 0x%04X.\n'
  0x44033ec4: 'FE C4A(tlim) changed to 0x%04X.\n'
  0x44033ee5: 'FE C4A not aotenjyo.\n'
  0x44033efb: 'FE C4A aotenjyo.\n'
  0x44033f0d: 'FE FPU(tmin) changed to 0x%04X.\n'
  0x44033f2e: 'FE FPU(tper) changed to 0x%04X.\n'
  0x44033f4f: 'FE FPU(tlim) changed to 0x%04X.\n'
  0x44033f70: 'FE FPU not aotenjyo.\n'
  0x44033f86: 'FE FPU aotenjyo.\n'
  0x44033f98: '***** F-CPU BAND change end *****\n'
  0x44033fbb: 'error! timeout:timer or VHS_write\n'
  0x44033fde: 'error! timeout:timer_or VHS write\n'
  0x44034001: 'error! timeout:timer or VHS_read\n'
  0x44034023: 'waiting reply.( %02d/ 70 )\n'
  0x4403403f: 'error! timeout 1min:VHS\n'
  0x44034058: 'Main hung-up time changed %u[s].\n'
  0x4403407a: 'Send timer-ucon judge command[%02X].\n'
  0x440340a0: 'Timer replay %02X %02X.\n'
  0x440340b9: 'Now model is PAL.\n'
  0x440340cc: 'Now model is NTSC.\n'
  0x440340e0: 'Error!! Timer replay %02X %02X.\n'
  0x44034101: 'Now model is unknown.\n'
  0x44034118: 'miniboot : system assert!!\n'
  0x44034134: 'Chip select = 1\n'
  0x44034145: 'Chip select = 0\n'
  0x44034156: 'first data = 0x%02X 0x%02X\n'
  0x44034172: 'check address = 0x%X, data = 0x%X\n'
  0x44034195: 'Conveyer firm boot mode...\n'
  0x440341b1: 'Clear update data.\n'
  0x440341c5: 'HDD_MAX_LBA get failed[0x%X].\n'
  0x440341e4: 'mainsoft data is invalid, so set update flag.\n'
  0x44034213: 'mainsoft-check: error! check address = 0x%x, out of range.\n'
  0x4403424f: 'mainsoft-check: error! check address = 0x%x, invalid.\n'
  0x44034286: 'Main address = 0x%08X\nMainsoft version = %s\n'
  0x440342b3: 'mainsoft-check: error! check data invalid.\n'
  0x440342df: 'Error! Failed[0x%X] to read main from HDD.\n'
  0x4403430b: 'Error! Not main compress header.\n'
  0x4403432d: 'sector = 0x%X , address = 0x%X , size = 0x%X\n'
  0x4403435b: 'Error! Read sector num[%u] over flow.\n'
  0x44034382: 'Error! Failed[0x%X] read main from HDD.\n'
  0x440343ab: 'Mainsoft data uncompress failed.\n'
  0x440343cd: 'Error! Not main compress.\n'
  0x440343e8: 'HDD INNER SECTOR MAIN RECOVERY START.\n'
  0x4403440f: 'Error! Failed[0x%X] write to HDD.\n'
  0x44034432: 'Error! Failed[0x%X] read to HDD.\n'
  0x44034454: 'Verify error.\n'
  0x44034463: 'Recovery history = (total:%u , ok:%u , ng:%u).\n'
  0x44034493: 'HDD INNER SECTOR MAIN RECOVERY SUCCESS.\n'
  0x440344bc: 'HDD INNER SECTOR MAIN RECOVERY FAILED.\n'
  0x440344e4: '\n!!! start fmu to read & exec fvu. !!!\n'
  0x4403450c: '\n*** start diag ***\n'
  0x44034521: 'error! startup reason is invalid.\n'
  0x44034544: 'error! drive-firm maybe destroyed, so power off.\n'
  0x44034576: 'Read fvu program from Update-Disk to 0x%x.\n'
  0x440345a2: 'success to read fvu program from Update-Disc.\n'
  0x440345d1: 'fvu size over 0x%X.\n'
  0x440345e6: 'fvu uncompress start...\n'
  0x440345ff: 'fvu uncompress failed...\n'
  0x44034619: 'fvu is preserved with fvu.\n'
  0x44034635: '\n*** start fvu ***\n'
  0x44034649: 'error! cannot read DeviceInfo-file from Nand-FROM.\n'
  0x4403467d: 'SD card check start\n'
  0x44034692: '[SD] Power failer recovery mode. But, SD read failed[0x%X].\n'
  0x440346cf: 'SD info get failed[%d]. But, DVD-RAM/CD verup check start.\n'
  0x4403470b: 'Failed to read fvu.\n'
  0x44034720: '!!! Insert Update-Disc !!!\n'
  0x4403473c: 'Timout for waiting for insert Update-Disc.\n'
  0x44034768: 'closed tray.\n'
  0x44034776: 'startup drive.\n'
  0x44034786: 'treat as SD.\n'
  0x44034794: 'FAT Close failed[%d].\n'
  0x440347ab: 'MATSHITA DVD-RAM RECORDER'
  0x440347c5: '300h = %s\n'
  0x440347d0: 'treat as DVD-RAM.\n'
  0x440347e3: 'treat as CD-R.\n'
  0x440347f3: 'Searching for FVU.\n'
  0x44034807: 'error! not found fvu program in Update-Disk.\n'
  0x44034835: 'PROG'
  0x4403483a: 'error! failed to read fvu program.\n'
  0x4403485e: 'copying fvu program from RTSC-buf to SDRAM.\n'
  0x4403488b: 'error! fvu program check-sum invalid.\n'
  0x440348b2: 'error! failed to read fvu program. Read_VCDDATA: %8x\n'
  0x440348e8: 'cheking for fvu program check-sum.\n'
  0x4403490c: 'error! Failed to Startup Drive.\n'
  0x4403492d: 'Timeout Power Off\n'
  0x44034940: 'pushed power-button\n'
  0x44034955: 'Firm list read failed[%d].\n'
  0x44034971: 'Not found fvu program in SD.\n'
  0x4403498f: 'FAT Seek(%d) failed[%d].\n'
  0x440349a9: 'File ID[0x%X] seek to 0x%X.\n'
  0x440349c6: 'Fvu read failed[%d].\n'
  0x440349dc: 'File ID[0x%X] read to 0x%X(size=0x%X).\n'
  0x44034a04: 'cheking for fvu program check-sum[0x%X].\n'
  0x44034a2e: 'Fvu check-sum invalid.\n'
  0x44034a46: 'SDDIAG'
  0x44034a4d: 'SD diag load failed[%d].\n'
  0x44034a67: 'SD init failed[%d].\n'
  0x44034a7c: 'diag loader uncompress start...\n'
  0x44034a9d: 'diag loader uncompress failed...\n'
  0x44034abf: 'diag loader is preserved with diag loader.\n'
  0x44034aeb: '*** CD diag boot mode ***\n'
  0x44034b06: 'CDDIAG'
  0x44034b0d: 'Drive start failed[%d].\n'
  0x44034b26: 'DIGADIAG.FRM'
  0x44034b33: 'DIGADIAG.FRM read failed.\n'
  0x44034b57: 'DÃ\x80'
  0x44034b5b: 'DØ\x80'
  0x44034b63: 'Dí\x80'
  0x44034b67: 'D,\x81'
  0x44034b6b: 'DA\x81'
  0x44034b6f: 'DV\x81'
  0x44034b73: 'Dk\x81'
  0x44034b77: 'D\x80\x81'
  0x44034b83: 'D¢\x81'
  0x44034b87: 'D³\x81'
  0x44034b8b: 'D\x91\x81'
  0x44034b8f: 'DÄ\x81'
  0x44034b93: 'DÕ\x81'
  0x44034b97: 'Dæ\x81'
  0x44034b9b: 'D÷\x81'
  0x44034b9f: 'D\x95\x82'
  0x44034ba3: 'DÑ\x82'
  0x44034ba7: 'D¡\x82'
  0x44034bab: 'D\xad\x82'
  0x44034baf: 'DÅ\x82'
  0x44034bb3: 'D¹\x82'
  0x44034bb7: 'DÝ\x82'
  0x44034bbb: 'Dè\x82'
  0x44034bbf: 'Dó\x82'
  0x44034bc3: 'Dþ\x82'
  0x44034bc7: 'D\t\x83'
  0x44034bcb: 'D_\x83'
  0x44034bcf: 'D_\x83'
  0x44034bd7: 'D*\x83'
  0x44034bdf: 'D5\x83'
  0x44034be3: 'D@\x83'
  0x44034be7: 'DK\x83'
  0x44034beb: 'DV\x83'
  0x44034bef: 'DBGO driver init failed[%u].\n'
  0x44034c0d: 'Control information init failed[%d].\n'
  0x44034c33: 'BGO driver no init.\n'
  0x44034c48: 'Filename pointer is NULL.\n'
  0x44034c63: 'Data read buffer pointer is NULL.\n'
  0x44034c86: 'ID invalid.\n'
  0x44034c93: 'ID[%d] read error[%u].\n'
  0x44034cab: 'Last data ID[%d] not equal %d.\n'
  0x44034ccb: 'error!: uncompress() = %d\n'
  0x44034ce6: '%s not found.\n'
  0x44034cf5: 'Not compress data write start.\n'
  0x44034d15: '%s rewrite error[%d].\n'
  0x44034d2c: 'status recovery error[%d].\n'
  0x44034d48: '%s write error[%d].\n'
  0x44034d5d: 'File size different. Ctrl[0x%X] != Check[0x%X].\n'
  0x44034d8e: 'ID[%d] verify error(0x%08X-0x%08X).\n'
  0x44034db3: '%s[0x%X] write error[%d].\n'
  0x44034dce: '%s[0x%X] verify error[%d].\n'
  0x44034dea: '%s[0x%X] erase error[%d].\n'
  0x44034e05: 'BGO read error[%u].\n'
  0x44034e1a: 'Now DeviceInfo2 size = 0x%X.\n'
  0x44034e38: 'ReWrite 0x%08X-0x%08X(size=0x%X) start.\n'
  0x44034e61: 'retry[%d]...\n'
  0x44034e6f: 'Erase(0x%08X-0x%08X) error.\n'
  0x44034e8c: 'Write(0x%08X-0x%08X) error from 0x%08X.\n'
  0x44034eb7: '\nReWrite end(SKIP=%u , NOT_SKIP=%u)\n'
  0x44034edc: '*** RTSC Initialize ***\n'
  0x44034ef5: '*** Drive Initialize ***\n'
  0x44034f0f: '*** HDD Initialize ***\n'
  0x44034f2b: '(%08x)'
  0x44034f32: '\tdrive_read: unit not ready [%x]\n'
  0x44034f54: '\tdrive_read: retry TUR %d times\n'
  0x44034f75: '\tdrive_read: read failed [%x]\n'
  0x44034f94: '** command retry **\n'
  0x44034fa9: '** open tray **\n'
  0x44034fba: '** can not open tray **\n'
  0x44034fd3: '** close tray **\n'
  0x44034fe5: '** can not close tray **\n'
  0x44034fff: '** startup drive **\n'
  0x44035014: 'drive start error(%08x)\n'
  0x4403502d: '** success to startup **\n'
  0x44035047: '** no disc **\n'
  0x44035058: '%d,'
  0x4403505c: 'HDD MAX LBA = 0x%X.\n'
  0x44035071: 'SD init start...\n'
  0x44035083: 'SD = 0x%X[LBA]\n'
  0x44035093: '... SD init end\n'
  0x440350a4: 'Drive connect failed[0x%X].\n'
  0x440350c1: 'Drive disconnect failed[0x%X].\n'
  0x440350e1: 'Drive(sleep mode) change failed[0x%X].\n'
  0x44035109: 'Media rotation stop failed[%d].\n'
  0x4403512a: 'Drive disconnect failed[%d].\n'
  0x44035148: '[fmu] error! list_sum is invalid.\n'
  0x4403516b: 'DRV '
  0x44035170: '[fmu] error! Read_VCDDATA(): %8x\n'
  0x44035192: 'FAT Mount failed[%d].\n'
  0x440351a9: 'FAT Mount ID = 0x%X\n'
  0x440351be: 'PANA__SD.FRM'
  0x440351cb: 'FAT Open(%s) failed[%d].\n'
  0x440351e5: 'FAT Open Old File ID = 0x%X\n'
  0x44035202: 'DIGADIAG.FRM'
  0x4403520f: 'CD001'
  0x44035215: 'PANA_DVD.FRM'
  0x44035222: 'wait 1 second.\n'
  0x44035232: '*** timer correspond initialize ***\n'
  0x44035257: 'startup reason = [%x]\n'
  0x4403526e: 'EXTRHEAD'
  0x44035277: 'EXTRFOOT'
  0x44035280: 'HEADER:%08X, DECOMP:%08X\n\n'
  0x4403529b: 'no header!\n'
  0x440352a7: 'no footer!\n'
  0x440352b3: 'Destination address(=NULL) error.\n'
  0x440352d6: 'Source address(=NULL) error.\n'
  0x440352f4: 'Source length(=0) error.\n'
  0x4403530e: 'Destination length(=0) error.\n'
  0x4403532d: 'uncompress dst=%x len=%x src=%x len=%x\n'
  0x44035355: 'error!: uncompress() = %d\n'
  0x44035370: 'error!: destLen = 0x%x, header->original_soft_size = 0x%x\n'
  0x440353ab: 'error!: check sum mismatch\n'
  0x440353c7: 'error!: lzma_uncompress() = %d\n'
  0x440353e7: 'error! unsupport argo_no = %d\n'
  0x44035406: 'Compress data output buffer size error(0x%X).\n'
  0x44035435: 'Compress start address error(NULL).\n'
  0x4403545a: 'Compress data output buffer address error(NULL).\n'
  0x4403548c: 'Compress start address align error(0x%08X).\n'
  0x440354b9: 'Compress data output buffer address align error(0x%08X).\n'
  0x440354f3: 'Error! compress() return %d.\n'
  0x44035511: 'Compress OK! Result:0x%X->0x%X\n'
  0x44035531: 'Error! Drive firmware not compress header.\n'
  0x4403555d: 'F/E Address = 0x%X , Drive(compress) Size = 0x%X\n'
  0x4403558f: 'Error! Drive firmware uncompress failed[0x%X].\n'
  0x440355bf: 'Error! Drive firmware not compress.\n'
  0x440355e4: 'F/E reset release finish ...\n'
  0x44035602: 'Error! Fvu not compress header.\n'
  0x44035623: 'Fvu Address = 0x%X , Size = 0x%X\n'
  0x44035645: 'Error! Fvu uncompress failed[0x%X].\n'
  0x4403566a: 'Error! Fvu not compress.\n'
```

#### Placeholder de Marcação `NO_FUNC reset_or_early @ 0x44000000` (`doc_boot_core.txt`)

Marcador `NO_FUNC` presente no início de `doc_boot_core.txt` (sem corpo na exportação original; `0x44000000` e `crt0` em `0x4400133e` detalhados acima).

<!-- source: doc_boot_core.txt | NO_FUNC reset_or_early @ 0x44000000 -->
*(Sem corpo na exportação original; referência cruzada detalhada na subseção correspondente.)*

#### Placeholder de Marcação `NO_FUNC early_sc0_probe_1 @ 0x440007d4` (`doc_boot_core.txt`)

Marcador `NO_FUNC` presente no início de `doc_boot_core.txt` (sem corpo na exportação original; `0x44000000` e `crt0` em `0x4400133e` detalhados acima).

<!-- source: doc_boot_core.txt | NO_FUNC early_sc0_probe_1 @ 0x440007d4 -->
*(Sem corpo na exportação original; referência cruzada detalhada na subseção correspondente.)*

#### Placeholder de Marcação `NO_FUNC early_sc0_probe_2 @ 0x440011a8` (`doc_boot_core.txt`)

Marcador `NO_FUNC` presente no início de `doc_boot_core.txt` (sem corpo na exportação original; `0x44000000` e `crt0` em `0x4400133e` detalhados acima).

<!-- source: doc_boot_core.txt | NO_FUNC early_sc0_probe_2 @ 0x440011a8 -->
*(Sem corpo na exportação original; referência cruzada detalhada na subseção correspondente.)*

#### Placeholder de Marcação `NO_FUNC crt0_bss_and_data_copy @ 0x440012a0` (`doc_boot_core.txt`)

Marcador `NO_FUNC` presente no início de `doc_boot_core.txt` (sem corpo na exportação original; `0x44000000` e `crt0` em `0x4400133e` detalhados acima).

<!-- source: doc_boot_core.txt | NO_FUNC crt0_bss_and_data_copy @ 0x440012a0 -->
*(Sem corpo na exportação original; referência cruzada detalhada na subseção correspondente.)*

### 1.2 Fluxo Principal de Boot, GPIOs, UART0 e Handshake Timer MCU / VFD

Funções centrais de controle do fluxo de inicialização em `/BOOT.BIN`, leitura de chaves de hardware/modelo, depuração serial UART0 (em grande parte código morto no fluxo padrão), comunicação com o Timer MCU / display VFD e rotinas de parada (`U99`, abertura de bandeja, espera de 1 segundo).

#### `boot_main` (`boot_main`) — `0x4400492e` (13 B)

Função principal de boot (`boot_main`); inicializa BGO, verifica estado NFROM e decide rota de inicialização.

<!-- source: doc_boot_core.txt | FUNC boot_main (boot_main) @ 0x4400492e (size=13 B, req=0x4400492e) -->
```c
// Callers (0): 
// Referenced strings/literals:
//   0x44033ab8: '\n***** Loader ver%s [%s](%s) *****\n'
//   0x44033adc: '0.63'
//   0x44033ae1: '07/10/24'
//   0x44033aea: '4MB/TOP BOOT'
//   0x44033af7: '***** FEonly mode *****\n'
//   0x44033ba2: '***** serial boot mode *****\n'
//   0x44033b89: '***** normal mode *****\n'
//   0x44033b10: '***** support card mode *****\n'
//   0x44033b2f: 'load_address = 0x%08x , load_size = 0x%x\n\n'
//   0x44033b5a: 'error! main soft is not loaded to 0x40000400.\n'
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void boot_main(void)

{
  byte bVar1;
  undefined4 *puVar2;
  undefined1 extraout_D0;
  undefined4 uVar3;
  undefined4 uStack_4;
  
  puVar2 = &uStack_4;
  _DAT_5056b5f8 = 0;
  uart0_init();
  uart0_printf(s_______Loader_ver_s___s___s_______44033ab8,&DAT_44033adc,s_07_10_24_44033ae1,
               s_4MB_TOP_BOOT_44033aea);
  bVar1 = P5OUT;
  P5OUT = bVar1 | 8;
  detect_boot_mode();
  *(undefined1 *)((int)puVar2 + -9) = extraout_D0;
  nvram_read_id(2,puVar2 + -7,0xffffffff);
  nvram_read_id(0xb,puVar2 + -6,0xffffffff);
  FUN_4402c348(&DAT_901f75e0,0xff,0x3c0);
  if (((puVar2[-6] != 0) && (*(char *)((int)puVar2 + -9) != '\x02')) || (puVar2[-7] == 0)) {
    sub_4400bb14(0,1);
  }
  if (puVar2[-7] == 0) {
    uart0_printf(s_______FEonly_mode_______44033af7);
    feonly_mode();
  }
  puVar2[-8] = (uint)*(byte *)((int)puVar2 + -9);
  if (puVar2[-8] == 2) {
    uart0_printf(s_______serial_boot_mode_______44033ba2);
    fmu_sd_dispatcher(2);
  }
  else {
    if ((int)puVar2[-8] < 3) {
      if (puVar2[-8] == 1) {
        uart0_printf(s_______normal_mode_______44033b89);
        uVar3 = bgo_driver_init();
        puVar2[-4] = uVar3;
        if (puVar2[-4] != 0) {
          boot_panic_or_halt_7812();
          return;
        }
        fmu_sd_dispatcher(1);
        return;
      }
    }
    else if (puVar2[-8] == 3) {
      uart0_printf(s_______support_card_mode_______44033b10);
      uVar3 = bgo_driver_init();
      puVar2[-4] = uVar3;
      if (puVar2[-4] != 0) {
        boot_panic_or_halt_7812();
        return;
      }
      uVar3 = parse_support_card_hdr(0x40000400);
      puVar2[-1] = uVar3;
      uart0_printf(s_load_address___0x_08x___load_siz_44033b2f,puVar2[-2],puVar2[-1]);
      if (puVar2[-1] == 0) {
        uart0_printf(s_error__main_soft_is_not_loaded_t_44033b5a);
        boot_panic_or_halt_7812();
      }
      FUN_4402c2f7(puVar2[-2],0x40000400,puVar2[-1]);
      puVar2[-5] = puVar2[-2];
      (*(code *)puVar2[-5])();
      return;
    }
    boot_panic_or_halt_7812();
  }
  return;
}
```

#### `detect_boot_mode` (`detect_boot_mode`) — `0x44006454` (116 B)

Leitura de modo de boot via flags NVRAM/hardware e cartão de suporte.

<!-- source: doc_boot_core.txt | FUNC detect_boot_mode (detect_boot_mode) @ 0x44006454 (size=116 B, req=0x44006454) -->
```c
// Callers (1): 0x44004977
// Referenced strings/literals:
//   0x44034134: 'Chip select = 1\n'
//   0x44034145: 'Chip select = 0\n'
undefined4 detect_boot_mode(void)

{
  undefined4 *puVar1;
  undefined1 auStack_8 [4];
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  nvram_read_id(1,auStack_8,0xffffffff);
  nvram_read_id(0xc,puVar1 + -2,0xffffffff);
  if (puVar1[-1] == 0) {
    uart0_printf(s_Chip_select___1_44034134);
  }
  else {
    uart0_printf(s_Chip_select___0_44034145);
  }
  if (puVar1[-1] == 0) {
    if (puVar1[-2] == 0) {
      puVar1[-3] = 2;
    }
    else {
      puVar1[-3] = 1;
    }
  }
  else {
    puVar1[-3] = 3;
  }
  return puVar1[-3];
}
```

#### `parse_support_card_hdr` (`parse_support_card_hdr`) — `0x440064c8` (197 B)

Valida cabeçalho de cartão de suporte SD para modo especial de boot.

<!-- source: doc_boot_core.txt | FUNC parse_support_card_hdr (parse_support_card_hdr) @ 0x440064c8 (size=197 B, req=0x440064c8) -->
```c
// Callers (1): 0x44004a50
// Referenced strings/literals:
//   0x44034172: 'check address = 0x%X, data = 0x%X\n'
//   0x44034156: 'first data = 0x%02X 0x%02X\n'
undefined4 parse_support_card_hdr(byte *param_1,undefined4 *param_2)

{
  undefined1 *puVar1;
  uint *puVar2;
  undefined1 auStack_4 [4];
  
  puVar1 = auStack_4;
  if ((*param_1 == 0xdc) && (param_1[1] == 0x44)) {
    *param_2 = *(undefined4 *)(param_1 + 0xc);
    puVar2 = (uint *)(param_1 + 8);
    if ((*puVar2 < 0x50000000) || (0x57ffffff < *puVar2)) {
      uart0_printf(s_check_address___0x_X__data___0x__44034172,puVar2,*puVar2);
      *(undefined4 *)(puVar1 + -0x14) = 0;
    }
  }
  else {
    uart0_printf(s_first_data___0x_02X_0x_02X_44034156,(uint)*param_1,(uint)param_1[1]);
    *(undefined4 *)(puVar1 + -0x14) = 0;
  }
  return *(undefined4 *)(puVar1 + -0x14);
}
```

#### `FUN_4400658d` (`support_card_jump_658d`) — `0x4400658d` (47 B)

Salto para endereço de execução do cartão de suporte (`support_card_jump_658d`).

<!-- source: doc_boot_core.txt | FUNC FUN_4400658d (support_card_jump_658d) @ 0x4400658d (size=47 B, req=0x4400658d) -->
```c
// Callers (1): 0x4400688a
int FUN_4400658d(int param_1)

{
  return *(int *)(param_1 + 8) + -8;
}
```

#### `FUN_44006cd9` (`boot_route_cd_sd_6cd9`) — `0x44006cd9` (777 B)

Roteamento principal entre boot via CD e via cartão SD (`boot_route_cd_sd_6cd9`).

<!-- source: doc_boot_core.txt | FUNC FUN_44006cd9 (boot_route_cd_sd_6cd9) @ 0x44006cd9 (size=777 B, req=0x44006cd9) -->
```c
// Callers (4): 0x440065ec, 0x4400662d, 0x4400668b, 0x44006816
// Referenced strings/literals:
//   0x440344e4: '\n!!! start fmu to read & exec fvu. !!!\n'
//   0x4403450c: '\n*** start diag ***\n'
//   0x44034521: 'error! startup reason is invalid.\n'
//   0x44034544: 'error! drive-firm maybe destroyed, so power off.\n'
//   0x44034576: 'Read fvu program from Update-Disk to 0x%x.\n'
//   0x440345a2: 'success to read fvu program from Update-Disc.\n'
//   0x440345d1: 'fvu size over 0x%X.\n'
//   0x440345e6: 'fvu uncompress start...\n'
//   0x440345ff: 'fvu uncompress failed...\n'
//   0x44034619: 'fvu is preserved with fvu.\n'
//   0x44034635: '\n*** start fvu ***\n'
/* WARNING: Control flow encountered bad instruction data */
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_44006cd9(void)

{
  undefined4 *puVar1;
  int iVar2;
  undefined4 uVar3;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  uart0_printf(s_____start_fmu_to_read___exec_fvu_440344e4);
  _DAT_50503760 = 0;
  _DAT_5050375c = 0;
  _DAT_50501514 = 0;
  _DAT_5048025c = 1;
  FUN_44005ed3();
  FUN_44006216();
  FUN_4400629a();
  FUN_4400531a();
  FUN_4400b344();
  if (puVar1[2] == 1) {
    FUN_440054aa(300,0x50480255);
  }
  if (*(char *)(puVar1 + 3) == '\x02') {
    FUN_44009741(0,0);
  }
  else {
    FUN_44009741(1,0);
  }
  _DAT_5056b5f8 = 1;
  if (*(char *)(puVar1 + 3) == '\x02') {
    FUN_44007ba5();
    nvram_read_id(0xb,puVar1 + -6,0xffffffff);
    if (puVar1[-6] != 0) {
      FUN_4400a4c5();
    }
    FUN_440062c4();
    FUN_4400f2d5();
    uart0_printf(s_____start_diag_____4403450c);
  }
  else {
    iVar2 = FUN_44008075(0xd);
    if (((((iVar2 != 1) && (iVar2 = FUN_44008075(0xe), iVar2 != 1)) &&
         (iVar2 = FUN_44008075(3), iVar2 != 1)) &&
        ((iVar2 = FUN_44008075(0x10), iVar2 != 1 && (iVar2 = FUN_44008075(0x13), iVar2 != 1)))) &&
       ((puVar1[2] == 0 && (*(char *)((int)puVar1 + -0xf) != '\x04')))) {
      uart0_printf(s_error__startup_reason_is_invalid_44034521);
      FUN_44007f1c();
      FUN_44007ea9();
      FUN_440079a4();
    }
    if (((*(char *)((int)puVar1 + -0xf) != '\x04') && (puVar1[2] == 0)) &&
       (iVar2 = FUN_44008075(0x11), iVar2 == 1)) {
      puVar1[-3] = 1;
    }
    puVar1[-2] = 0;
    uVar3 = FUN_440084d9(0x504800af);
    puVar1[-1] = uVar3;
    if ((puVar1[-1] == 0) && (iVar2 = uncompress(0x84050000), iVar2 == 1)) {
      puVar1[-2] = 1;
    }
    if (puVar1[-2] == 0) {
      if ((puVar1[-3] == 0) && (iVar2 = FUN_44008075(0xd), iVar2 == 1)) {
        uart0_printf(s_error__drive_firm_maybe_destroye_44034544);
        FUN_44007f1c();
        FUN_44007ea9();
        FUN_44007836(0x50480239,0x50480232);
      }
      uart0_printf(s_Read_fvu_program_from_Update_Dis_44034576,&DAT_90400000);
      if (puVar1[2] == 0) {
        FUN_440054aa(300,0x504801ec);
      }
      _DAT_5050375c = 0;
      _DAT_50503760 = 0;
      _DAT_50501514 = 0;
      uVar3 = FUN_44006fe7(puVar1[2],puVar1[-3]);
      puVar1[-5] = uVar3;
      uart0_printf(s_success_to_read_fvu_program_from_440345a2);
      if (0x20000 < (uint)puVar1[-5]) {
        uart0_printf(s_fvu_size_over_0x_X__440345d1,0x20000);
        FUN_44007f1c();
        FUN_44007ea9();
        boot_panic_or_halt_7812();
      }
      uart0_printf(s_fvu_uncompress_start____440345e6);
      iVar2 = uncompress(&DAT_93fc0000);
      if (iVar2 != 1) {
        uart0_printf(s_fvu_uncompress_failed____440345ff);
        FUN_44007f1c();
        FUN_44007ea9();
        boot_panic_or_halt_7812();
      }
      uart0_printf(s_fvu_is_preserved_with_fvu__44034619);
    }
    nvram_read_id(0xb,puVar1 + -6,0xffffffff);
    if (puVar1[-6] != 0) {
      FUN_4400a4c5();
    }
    FUN_440062c4();
    FUN_4400f2d5();
    uart0_printf(s_____start_fvu_____44034635);
  }
                    /* WARNING: Bad instruction - Truncating control flow here */
  halt_baddata();
}
```

#### `FUN_44006fe7` (`media_update_gate_6fe7`) — `0x44006fe7` (1093 B)

Portão de decisão para atualização de firmware por mídia (`media_update_gate_6fe7`).

<!-- source: doc_boot_core.txt | FUNC FUN_44006fe7 (media_update_gate_6fe7) @ 0x44006fe7 (size=1093 B, req=0x44006fe7) -->
```c
// Callers (1): 0x44006f11
// Referenced strings/literals:
//   0x44034649: 'error! cannot read DeviceInfo-file from Nand-FROM.\n'
//   0x4403467d: 'SD card check start\n'
//   0x44034692: '[SD] Power failer recovery mode. But, SD read failed[0x%X].\n'
//   0x440346cf: 'SD info get failed[%d]. But, DVD-RAM/CD verup check start.\n'
//   0x440347ab: 'MATSHITA DVD-RAM RECORDER'
//   0x440347c5: '300h = %s\n'
//   0x440347d0: 'treat as DVD-RAM.\n'
//   0x440347e3: 'treat as CD-R.\n'
//   0x44034786: 'treat as SD.\n'
//   0x44034794: 'FAT Close failed[%d].\n'
//   0x4403470b: 'Failed to read fvu.\n'
//   0x44034720: '!!! Insert Update-Disc !!!\n'
//   0x4403473c: 'Timout for waiting for insert Update-Disc.\n'
//   0x44034768: 'closed tray.\n'
//   0x44034776: 'startup drive.\n'
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

undefined4 FUN_44006fe7(void)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  int iVar3;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  uVar2 = FUN_4400934f();
  puVar1[-2] = uVar2;
  if (puVar1[-2] != 0) {
    uart0_printf(s_error__cannot_read_DeviceInfo_fi_44034649);
    FUN_44007f1c();
    FUN_44007ea9();
    boot_panic_or_halt_7812();
  }
  FUN_4402c348(puVar1 + -0xc,0,8);
  FUN_440093d4(4,8,puVar1 + -0xc);
  FUN_440093d4(0xc,1,(int)puVar1 + -10);
  uVar2 = FUN_44008075(0x11);
  puVar1[-2] = uVar2;
  if ((puVar1[-2] == 1) || (puVar1[2] == 1)) {
    uart0_printf(s_SD_card_check_start_4403467d);
    uVar2 = FUN_44007e48((int)puVar1 + -9,puVar1[2],puVar1[3]);
    puVar1[-2] = uVar2;
    if (puVar1[-2] != 0) {
      if (puVar1[3] == 1) {
        uart0_printf(s__SD__Power_failer_recovery_mode__44034692,puVar1[-2]);
        FUN_440079a4();
      }
      else {
        uart0_printf(s_SD_info_get_failed__d___But__DVD_440346cf,puVar1[-2]);
      }
    }
  }
  if (*(char *)((int)puVar1 + -9) == '\0') goto LAB_440070e0;
  do {
    if (puVar1[2] == 1) {
      FUN_440054aa(300,0x504801ec);
    }
    _DAT_50501518 = 0;
    if (*(char *)((int)puVar1 + -9) != '\x01') {
      FUN_44010760(0,puVar1 + -10,0x1a,0);
      *(undefined1 *)((int)puVar1 + -0xf) = 0;
      iVar3 = FUN_4402c420(puVar1 + -10,s_MATSHITA_DVD_RAM_RECORDER_440347ab,0x19);
      if (iVar3 == 0) {
        uart0_printf(s_300h____s_440347c5);
        uart0_printf(s_treat_as_DVD_RAM__440347d0);
        uVar2 = FUN_4400742c(puVar1 + -0xc,*(undefined1 *)((int)puVar1 + -10));
        puVar1[-0xd] = uVar2;
        iVar3 = puVar1[-0xd];
      }
      else {
        uart0_printf(s_treat_as_CD_R__440347e3);
        uVar2 = FUN_440075aa(puVar1 + -0xc,*(undefined1 *)((int)puVar1 + -10));
        puVar1[-0xd] = uVar2;
        iVar3 = puVar1[-0xd];
      }
      if (iVar3 == 0) goto LAB_44007126;
      goto LAB_44007424;
    }
    uVar2 = FUN_4400a8c7();
    puVar1[-2] = uVar2;
    if (puVar1[-2] == 1) {
LAB_44007319:
      uart0_printf(s_treat_as_SD__44034786);
      uVar2 = FUN_44007a12(puVar1 + -0xc,*(undefined1 *)((int)puVar1 + -10));
      puVar1[-0xd] = uVar2;
      if (puVar1[-0xd] != 0) goto LAB_44007388;
      uVar2 = FUN_4400c984(_DAT_50501730);
      puVar1[-2] = uVar2;
      if (puVar1[-2] != 0) {
        uart0_printf(s_FAT_Close_failed__d___44034794,puVar1[-2]);
      }
      if (puVar1[3] == 1) {
        uart0_printf(s__SD__Power_failer_recovery_mode__44034692,puVar1[-2]);
        FUN_440079a4();
LAB_44007388:
        FUN_44008027(0x11);
LAB_44007424:
        return puVar1[-0xd];
      }
      *(undefined1 *)((int)puVar1 + -9) = 0;
    }
    else {
      if (puVar1[3] == 1) {
        uart0_printf(s__SD__Power_failer_recovery_mode__44034692,puVar1[-2]);
        FUN_440079a4();
        goto LAB_44007319;
      }
      *(undefined1 *)((int)puVar1 + -9) = 0;
    }
LAB_440070e0:
    iVar3 = startup_drive(1);
    if (iVar3 != 0) {
      FUN_44007722();
    }
    _DAT_50501518 = 0;
    FUN_44009a3e(0,0x300,1);
    while (iVar3 = FUN_44009a3e(0,0x300,1), iVar3 != 0) {
      _DAT_50501518 = 1;
LAB_44007126:
      uart0_printf(s_Failed_to_read_fvu__4403470b);
      if (_DAT_50503760 == 1) {
        puVar1[-1] = 1;
      }
      FUN_44007789(puVar1[-1]);
      uart0_printf(s_____Insert_Update_Disc_____44034720);
      _DAT_50501514 = 0;
      _DAT_50501518 = 1;
      _DAT_50503760 = 0;
      puVar1[-0xe] = 0x3c;
      while (puVar1[-0xe] != 0) {
        if (puVar1[2] == 0) {
          puVar1[-0xf] = puVar1[-0xe] & 3;
          if (puVar1[-0xf] == 0) {
            FUN_440054aa(6,0x504801f3);
          }
          else if (puVar1[-0xf] == 2) {
            FUN_440054aa(6,0x504801fa);
          }
        }
        if (_DAT_50501514 == 1) {
          iVar3 = FUN_44009c85();
          if (iVar3 == 0) {
            _DAT_50501518 = 0;
          }
          _DAT_50501514 = 0;
        }
        uVar2 = FUN_44009835();
        puVar1[-2] = uVar2;
        if (puVar1[-2] == 0) break;
        if (((undefined *)puVar1[-2] == &DAT_80023a00) && (_DAT_50501518 == 0)) {
          _DAT_50501518 = 1;
          FUN_44009bd7();
        }
        if (_DAT_50503760 == 1) {
          FUN_44007f1c();
          FUN_44007ea9();
          FUN_440079a4();
        }
        delay_ms(1000);
        puVar1[-0xe] = puVar1[-0xe] + -1;
      }
      if (puVar1[-0xe] == 0) {
        uart0_printf(s_Timout_for_waiting_for_insert_Up_4403473c);
        puVar1[-1] = 1;
        FUN_44007f1c();
        FUN_44007ea9();
        FUN_440079a4();
      }
      _DAT_50503760 = 0;
      _DAT_50501518 = 0;
      _DAT_5048025c = 1;
      uart0_printf(s_closed_tray__44034768);
      uart0_printf(s_startup_drive__44034776);
      iVar3 = startup_drive(0);
      if (iVar3 != 0) {
        FUN_44007722();
      }
    }
  } while( true );
}
```

#### `FUN_44006822` (`hdd_hdr_check_6822`) — `0x44006822` (211 B)

Verifica cabeçalho de partição/imagem em HDD (`hdd_hdr_check_6822`).

<!-- source: doc_boot_core.txt | FUNC FUN_44006822 (hdd_hdr_check_6822) @ 0x44006822 (size=211 B, req=0x44006822) -->
```c
// Callers (2): 0x4400671c, 0x44006786
// Referenced strings/literals:
//   0x44034213: 'mainsoft-check: error! check address = 0x%x, out of range.\n'
//   0x44034286: 'Main address = 0x%08X\nMainsoft version = %s\n'
//   0x440342b3: 'mainsoft-check: error! check data invalid.\n'
//   0x4403424f: 'mainsoft-check: error! check address = 0x%x, invalid.\n'
undefined4 FUN_44006822(int param_1)

{
  undefined4 uVar1;
  undefined4 *puVar2;
  uint uVar3;
  undefined4 uStack_4;
  
  puVar2 = &uStack_4;
  uVar3 = *(uint *)(param_1 + 8);
  if ((uVar3 < 0x50000000) || (0x57ffffff < uVar3)) {
    uart0_printf(s_mainsoft_check__error__check_add_44034213,uVar3);
    puVar2[-4] = 1;
  }
  else if ((uVar3 & 3) == 0) {
    uVar1 = FUN_4400658d(param_1);
    uart0_printf(s_Main_address___0x_08X_Mainsoft_v_44034286,puVar2[2],uVar1);
    puVar2[-3] = 1;
    puVar2[-1] = 0;
    while ((int)puVar2[-1] < 0x20) {
      if (*(int *)puVar2[-2] != puVar2[-3]) {
        uart0_printf(s_mainsoft_check__error__check_dat_440342b3,*(int *)puVar2[-2],uVar1);
        puVar2[-4] = 3;
        goto LAB_440068ed;
      }
      puVar2[-2] = puVar2[-2] + 4;
      puVar2[-3] = puVar2[-3] * 2;
      puVar2[-1] = puVar2[-1] + 1;
    }
    puVar2[-4] = 0;
  }
  else {
    uart0_printf(s_mainsoft_check__error__check_add_4403424f,uVar3);
    puVar2[-4] = 2;
  }
LAB_440068ed:
  return puVar2[-4];
}
```

#### `FUN_440068f5` (`hdd_load_mainsoft_68f5`) — `0x440068f5` (495 B)

Carrega `mainsoft` do HDD, descompacta e salta para execução (`hdd_load_mainsoft_68f5`).

<!-- source: doc_boot_core.txt | FUNC FUN_440068f5 (hdd_load_mainsoft_68f5) @ 0x440068f5 (size=495 B, req=0x440068f5) -->
```c
// Callers (2): 0x44006704, 0x4400676e
// Referenced strings/literals:
//   0x4403430b: 'Error! Not main compress header.\n'
//   0x4403432d: 'sector = 0x%X , address = 0x%X , size = 0x%X\n'
//   0x440343ab: 'Mainsoft data uncompress failed.\n'
//   0x440343cd: 'Error! Not main compress.\n'
//   0x44034382: 'Error! Failed[0x%X] read main from HDD.\n'
//   0x4403435b: 'Error! Read sector num[%u] over flow.\n'
//   0x440342df: 'Error! Failed[0x%X] to read main from HDD.\n'
undefined4 FUN_440068f5(undefined4 param_1)

{
  undefined4 *puVar1;
  undefined4 *puVar2;
  undefined4 uVar3;
  int iVar4;
  undefined4 uStack_8;
  undefined4 uStack_4;
  
  puVar2 = &uStack_8;
  uVar3 = FUN_4400a00a(0,param_1,200);
  puVar2[-7] = uVar3;
  if (puVar2[-7] == 0) {
    FUN_4400a666(&DAT_90c00000,0x19000);
    uVar3 = FUN_4400b52e(&DAT_90c00000);
    puVar2[-1] = uVar3;
    if (puVar2[-1] == 0) {
      uart0_printf(s_Error__Not_main_compress_header__4403430b);
      puVar2[-9] = 0;
    }
    else {
      puVar1 = (undefined4 *)puVar2[4];
      uVar3 = FUN_4400b61a(&DAT_90c00000);
      *puVar1 = uVar3;
      uVar3 = FUN_4400b5e1(&DAT_90c00000);
      puVar2[-3] = uVar3;
      uVar3 = FUN_4400b4ab(&DAT_90c00000,0);
      puVar2[-5] = uVar3;
      uVar3 = FUN_4400b5b5(&DAT_90c00000);
      puVar2[-2] = uVar3;
      uart0_printf(s_sector___0x_X___address___0x_X___4403432d,puVar2[3],*(undefined4 *)puVar2[4],
                   puVar2[-5]);
      puVar2[-6] = puVar2[-5] & 7;
      if (puVar2[-6] != 0) {
        puVar2[-6] = 8 - puVar2[-6];
      }
      puVar2[-6] = puVar2[-5] + puVar2[-6];
      *(uint *)puVar2[5] = puVar2[-6] + 0x1ff >> 9;
      puVar2[-8] = (puVar2[-6] + 0x1ff >> 9) - 200;
      puVar2[-4] = puVar2[-4] + 0x19000;
      puVar2[3] = puVar2[3] + 200;
      if ((uint)puVar2[-8] < 0x10001) {
        uVar3 = FUN_4400a00a(puVar2[-4],puVar2[3],puVar2[-8]);
        puVar2[-7] = uVar3;
        if (puVar2[-7] == 0) {
          FUN_4400a666(puVar2[-6] + -0x6f419000,0x19000);
          if (puVar2[-2] == 1) {
            iVar4 = FUN_4400b6c6(&DAT_90c00000);
            if (iVar4 == 1) {
              puVar2[-9] = 1;
            }
            else {
              uart0_printf(s_Mainsoft_data_uncompress_failed__440343ab);
              puVar2[-9] = 0;
            }
          }
          else {
            uart0_printf(s_Error__Not_main_compress__440343cd);
            puVar2[-9] = 0;
          }
        }
        else {
          uart0_printf(s_Error__Failed_0x_X__read_main_fr_44034382,puVar2[-7]);
          puVar2[-9] = 0;
        }
      }
      else {
        uart0_printf(s_Error__Read_sector_num__u__over_f_4403435b,puVar2[-8]);
        puVar2[-9] = 0;
      }
    }
  }
  else {
    uart0_printf(s_Error__Failed_0x_X__to_read_main_440342df,puVar2[-7]);
    puVar2[-9] = 0;
  }
  return puVar2[-9];
}
```

#### `FUN_44006ae4` (`hdd_inner_recovery_6ae4`) — `0x44006ae4` (501 B)

Rotina interna de recuperação/fallback de carga do HDD (`hdd_inner_recovery_6ae4`).

<!-- source: doc_boot_core.txt | FUNC FUN_44006ae4 (hdd_inner_recovery_6ae4) @ 0x44006ae4 (size=501 B, req=0x44006ae4) -->
```c
// Callers (1): 0x440067b2
// Referenced strings/literals:
//   0x440343e8: 'HDD INNER SECTOR MAIN RECOVERY START.\n'
//   0x4403440f: 'Error! Failed[0x%X] write to HDD.\n'
//   0x44034432: 'Error! Failed[0x%X] read to HDD.\n'
//   0x44034454: 'Verify error.\n'
//   0x440342df: 'Error! Failed[0x%X] to read main from HDD.\n'
//   0x44034463: 'Recovery history = (total:%u , ok:%u , ng:%u).\n'
//   0x44034493: 'HDD INNER SECTOR MAIN RECOVERY SUCCESS.\n'
//   0x440344bc: 'HDD INNER SECTOR MAIN RECOVERY FAILED.\n'
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_44006ae4(void)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  uart0_printf(s_HDD_INNER_SECTOR_MAIN_RECOVERY_S_440343e8);
  uVar2 = FUN_4400a00a(0,puVar1[2],puVar1[4]);
  puVar1[-2] = uVar2;
  if (puVar1[-2] == 0) {
    puVar1[-3] = 0;
    while ((uint)puVar1[-3] < 3) {
      uVar2 = FUN_4400a0b0(0,puVar1[3],puVar1[4]);
      puVar1[-2] = uVar2;
      if (puVar1[-2] != 0) {
        uart0_printf(s_Error__Failed_0x_X__write_to_HDD_4403440f,puVar1[-2]);
        break;
      }
      uVar2 = FUN_4400a00a(0x1000000,puVar1[3],puVar1[4]);
      puVar1[-2] = uVar2;
      if (puVar1[-2] != 0) {
        uart0_printf(s_Error__Failed_0x_X__read_to_HDD__44034432,puVar1[-2]);
        break;
      }
      uVar2 = FUN_4402c2b7(&DAT_90c00000,&DAT_91c00000,puVar1[4] << 9);
      puVar1[-2] = uVar2;
      if (puVar1[-2] == 0) break;
      uart0_printf(s_Verify_error__44034454);
      puVar1[-3] = puVar1[-3] + 1;
    }
  }
  else {
    uart0_printf(s_Error__Failed_0x_X__to_read_main_440342df,puVar1[-2]);
  }
  uVar2 = FUN_4400920b(0x50480016);
  puVar1[-1] = uVar2;
  if (puVar1[-1] == 0) {
    if (puVar1[-3] == 0xc) {
      uVar2 = nvram_read_8634(0x50480016,&DAT_505180ac);
      puVar1[-1] = uVar2;
      if (puVar1[-1] != 0) {
        FUN_4402c348(&DAT_505180ac,0,0xc);
      }
    }
    else {
      FUN_4402c348(&DAT_505180ac,0,0xc);
    }
    if (puVar1[-2] == 0) {
      _DAT_505180ac = _DAT_505180ac + 1;
    }
    else {
      _DAT_505180b0 = _DAT_505180b0 + 1;
    }
    _DAT_505180b4 = _DAT_505180b4 + 1;
    uart0_printf(s_Recovery_history____total__u___o_44034463,_DAT_505180b4,_DAT_505180ac,
                 _DAT_505180b0);
    uVar2 = FUN_440090a6(0x50480016,&DAT_505180ac,0xc);
    puVar1[-1] = uVar2;
    if (puVar1[-2] == 0) {
      uart0_printf(s_HDD_INNER_SECTOR_MAIN_RECOVERY_S_44034493);
    }
    else {
      uart0_printf(s_HDD_INNER_SECTOR_MAIN_RECOVERY_F_440344bc);
    }
  }
  return;
}
```

#### `FUN_440046e4` (`gpio_model_flag_46e4`) — `0x440046e4` (101 B)

Leitura de GPIOs e flag de modelo/hardware (`gpio_model_flag_46e4`).

<!-- source: doc_boot_core.txt | FUNC FUN_440046e4 (gpio_model_flag_46e4) @ 0x440046e4 (size=101 B, req=0x440046e4) -->
```c
// Callers (7): 0x4400327d, 0x440032e5, 0x44003328, 0x4400335e, 0x44003377, 0x4400340d, 0x4400347b
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_440046e4(char param_1)

{
  if (_DAT_50480000 == 0x1c) {
    nvram_read_id(0xe,0,0);
  }
  else if (((_DAT_50480000 & 0xff) == 1) || (_DAT_50480000 == 0x20)) {
    if (param_1 == '\x01') {
      nvram_read_id(0xe,0,0);
    }
    else {
      nvram_read_id(0xe,0,1);
    }
  }
  return;
}
```

#### `uart0_init` (`uart0_init`) — `0x44004749` (86 B)

Inicialização da serial UART0 / `SC0`.

<!-- source: doc_boot_core.txt | FUNC uart0_init (uart0_init) @ 0x44004749 (size=86 B, req=0x44004749) -->
```c
// Callers (0): 
char uart0_init(void)

{
  byte bVar1;
  
  if (DAT_50480b88 == '\0') {
    nvram_read_id(0x17,0,0xffffffff);
    SC0CTR = 0xc085;
    TM2MD = 0;
    TM2BR = 0x2b;
    bVar1 = TM2MD;
    TM2MD = bVar1 | 0x80;
    DAT_50480b88 = '\x01';
  }
  return DAT_50480b88;
}
```

#### `uart0_getc` (`uart0_getc_dead`) — `0x4400479f` (40 B)

Leitura de caractere da UART0 (inalcançável no fluxo padrão de varejo).

<!-- source: doc_boot_core.txt | FUNC uart0_getc (uart0_getc_dead) @ 0x4400479f (size=40 B, req=0x4400479f) -->
```c
// Callers (2): 0x4400489a, 0x44004904
undefined1 uart0_getc(void)

{
  undefined1 uVar1;
  ushort uVar2;
  
  do {
    uVar2 = SC0STR;
  } while ((uVar2 >> 4 & 1) == 0);
  uVar1 = SC0RXB;
  return uVar1;
}
```

#### `uart0_putc` (`uart0_putc_poll`) — `0x440047c7` (65 B)

Envio de caractere com polling na UART0.

<!-- source: doc_boot_core.txt | FUNC uart0_putc (uart0_putc_poll) @ 0x440047c7 (size=65 B, req=0x440047dc) -->
```c
// Callers (9): 0x4400485c, 0x44004863, 0x44004877, 0x440048c4, 0x440048cb, 0x440048d3, 0x440048ff, 0x44004910, 0x44004917
undefined1 uart0_putc(undefined1 param_1)

{
  ushort uVar1;
  int iStack_8;
  
  iStack_8 = 0x1400000;
  while ((-1 < iStack_8 && (uVar1 = SC0STR, (uVar1 & 0xa0) != 0))) {
    iStack_8 = iStack_8 + -1;
  }
  SC0TXB = param_1;
  return param_1;
}
```

#### `console_getline` (`console_getline_dead`) — `0x4400488a` (164 B)

Leitura de linha no console serial (`console_getline_dead`).

<!-- source: doc_boot_core.txt | FUNC console_getline (console_getline_dead) @ 0x4400488a (size=164 B, req=0x4400488a) -->
```c
// Callers (0): 
undefined4 console_getline(undefined4 param_1,undefined4 param_2)

{
  undefined4 *puVar1;
  undefined1 extraout_D0;
  undefined1 extraout_D0_00;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  uart0_getc(0,param_2,0);
  *(undefined1 *)((int)puVar1 + -1) = extraout_D0;
  while (*(char *)((int)puVar1 + -1) != '\r') {
    if (*(char *)((int)puVar1 + -1) == '\b') {
      if (0 < (int)puVar1[-2]) {
        puVar1[-2] = puVar1[-2] + -1;
        uart0_putc(*(undefined1 *)((int)puVar1 + -1));
        uart0_putc(0x20);
        uart0_putc(*(undefined1 *)((int)puVar1 + -1));
      }
    }
    else {
      *(undefined1 *)(puVar1[-2] + puVar1[2]) = *(undefined1 *)((int)puVar1 + -1);
      if ((int)puVar1[-2] < puVar1[3] + -1) {
        puVar1[-2] = puVar1[-2] + 1;
      }
      uart0_putc(*(undefined1 *)((int)puVar1 + -1));
    }
    uart0_getc();
    *(undefined1 *)((int)puVar1 + -1) = extraout_D0_00;
  }
  uart0_putc(0xd);
  uart0_putc(10);
  *(undefined1 *)(puVar1[-2] + puVar1[2]) = 0;
  return 0;
}
```

#### `FUN_44005086` (`timer_mcu_5086`) — `0x44005086` (93 B)

Comunicação inicial com o Timer MCU (`timer_mcu_5086`).

<!-- source: doc_boot_core.txt | FUNC FUN_44005086 (timer_mcu_5086) @ 0x44005086 (size=93 B, req=0x44005086) -->
```c
// Callers (1): 0x4400596c
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

undefined4 FUN_44005086(uint param_1,int param_2)

{
  undefined4 uStack_8;
  
  if (0xf < param_1) {
    uStack_8 = 0xf;
    param_1 = uStack_8;
  }
  while (param_1 != 0) {
    param_1 = param_1 - 1;
    *(undefined1 *)(param_1 + param_2) =
         *(undefined1 *)(param_1 + _DAT_50480c8c * 0x10 + 0x505184bc);
  }
  return 0;
}
```

#### `FUN_44005114` (`timer_rx_handler_5114`) — `0x44005114` (492 B)

Tratador de recepção serial do Timer MCU (`timer_rx_handler_5114`).

<!-- source: doc_boot_core.txt | FUNC FUN_44005114 (timer_rx_handler_5114) @ 0x44005114 (size=492 B, req=0x44005114) -->
```c
// Callers (1): 0x440061e7
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

byte FUN_44005114(void)

{
  byte bVar1;
  char cVar2;
  undefined4 *puVar3;
  uint uStack_c;
  int iStack_8;
  undefined4 uStack_4;
  
  puVar3 = &uStack_4;
  uStack_c = (uint)(_DAT_50480c8c == 0);
  for (iStack_8 = 0; iStack_8 < 0x10; iStack_8 = iStack_8 + 1) {
    *(undefined1 *)(iStack_8 + uStack_c * 0x10 + 0x505184bc) = (&DAT_dc010020)[iStack_8];
  }
  _DAT_50480c8c = uStack_c;
  bVar1 = DAT_dc010020;
  if (((bVar1 & 0xfe) == 0x40) || (bVar1 = DAT_dc010020, (bVar1 & 0xfe) == 0x41)) {
    cVar2 = DAT_dc010023;
    if (((cVar2 == '\x01') || ((cVar2 = DAT_dc010023, cVar2 == -8 && (_DAT_50501518 == 0)))) ||
       ((cVar2 = DAT_dc010023, cVar2 == -7 && (_DAT_50501518 != 0)))) {
      FUN_44009e55();
    }
    if ((*(byte *)(puVar3[-3] + 3) & 0xfd) == 0x3d) {
      FUN_44007759();
    }
  }
  if (*(char *)puVar3[-3] == 'H') {
    if (_DAT_5051b5d8 == 0) {
      if (_DAT_5051b5d4 == 0) {
        _DAT_5051b5d4 = 1;
      }
      else {
        _DAT_5051b5d8 = 1;
      }
    }
    else {
      _DAT_5051b5d4 = 0;
    }
    if (_DAT_50480c9c != 0) {
      FUN_4400561f();
    }
  }
  if ((*(char *)puVar3[-3] == 'c') || (*(char *)puVar3[-3] == '\x7f')) {
    DAT_50480cae = *(undefined1 *)puVar3[-3];
    DAT_50480caf = *(undefined1 *)(puVar3[-3] + 1);
    _DAT_50480c98 = 1;
  }
  if (*(char *)puVar3[-3] == '\v') {
    if (*(char *)(puVar3[-3] + 1) == '\0') {
      _DAT_5051b5c0 = 1;
    }
    else {
      _DAT_5051b5c0 = 0xffffffff;
    }
  }
  if (*(char *)puVar3[-3] == '\t') {
    _DAT_5051b5dc = (uint)*(byte *)(puVar3[-3] + 1);
  }
  if ((_DAT_50480c90 != 0) && (*(byte *)puVar3[-3] == _DAT_504801e4)) {
    _DAT_50480c90 = 0;
  }
  _DAT_5051b5e0 = 1;
  bVar1 = DAT_dc010000;
  DAT_dc010000 = bVar1 | 0x10;
  bVar1 = DAT_dc010000;
  DAT_dc010000 = bVar1 & 0xef;
  return bVar1 & 0xef;
}
```

#### `FUN_4400599a` (`vfd_msg_599a`) — `0x4400599a` (125 B)

Envia mensagem para exibição no display VFD (`vfd_msg_599a`).

<!-- source: doc_boot_core.txt | FUNC FUN_4400599a (vfd_msg_599a) @ 0x4400599a (size=125 B, req=0x4400599a) -->
```c
// Callers (2): 0x440054c2, 0x4400564a
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

int FUN_4400599a(void)

{
  undefined1 *puVar1;
  int iVar2;
  undefined1 auStack_1c [20];
  int iStack_8;
  undefined1 auStack_4 [4];
  
  puVar1 = auStack_4;
  iStack_8 = 0x1400000;
  while ((-1 < iStack_8 && (_DAT_50480c94 == 1))) {
    iStack_8 = iStack_8 + -1;
  }
  FUN_4402c348(auStack_1c,0,0xf);
  puVar1[-0x18] = 0xa3;
  FUN_4400be54(puVar1 + -0x15,*(undefined4 *)(puVar1 + 8));
  FUN_4400534a(puVar1 + -0x18,*(undefined4 *)(puVar1 + -8));
  iVar2 = *(int *)(puVar1 + -8);
  if (iVar2 == 1) {
    iVar2 = 0;
    _DAT_50480c94 = 0;
  }
  return iVar2;
}
```

#### `FUN_44005cd0` (`vfd_wait_5cd0`) — `0x44005cd0` (70 B)

Aguarda conclusão/ACK do Timer MCU / VFD (`vfd_wait_5cd0`).

<!-- source: doc_boot_core.txt | FUNC FUN_44005cd0 (vfd_wait_5cd0) @ 0x44005cd0 (size=70 B, req=0x44005cd0) -->
```c
// Callers (3): 0x440078bc, 0x440079ef, 0x4400b3b3
// Referenced strings/literals:
//   0x44034058: 'Main hung-up time changed %u[s].\n'
void FUN_44005cd0(void)

{
  undefined4 *puVar1;
  undefined1 auStack_14 [16];
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  FUN_4402c348(auStack_14,0,0xf);
  *(undefined1 *)(puVar1 + -4) = 0x8a;
  *(undefined1 *)((int)puVar1 + -0xf) = 2;
  *(undefined1 *)((int)puVar1 + -0xd) = 0xff;
  FUN_44005747(0xf,puVar1 + -4);
  uart0_printf(s_Main_hung_up_time_changed__u_s___44034058,0xff);
  return;
}
```

#### `boot_panic_or_halt_7812` (`boot_panic_or_halt_7812`) — `0x44007812` (36 B)

Halt / pânico de boot.

<!-- source: doc_boot_core.txt | FUNC boot_panic_or_halt_7812 (boot_panic_or_halt_7812) @ 0x44007812 (size=36 B, req=0x44007812) -->
```c
// Callers (12): 0x44004a3c, 0x44004a86, 0x44004ace, 0x44004afa, 0x44006f52, 0x44006f91, 0x4400702b, 0x44007edc, 0x44007f10, 0x44007f51, 0x44007f87, 0x44007fd7
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void boot_panic_or_halt_7812(void)

{
  _DAT_50480cc4 = 1;
  FUN_44007836(0x50480240,0x50480240);
  return;
}
```

#### `FUN_44007836` (`vfd_u99_open_tray_7836`) — `0x44007836` (366 B)

Exibe `U99` no VFD e aciona abertura de bandeja em falha crítica (`vfd_u99_open_tray_7836`).

<!-- source: doc_boot_core.txt | FUNC FUN_44007836 (vfd_u99_open_tray_7836) @ 0x44007836 (size=366 B, req=0x44007836) -->
```c
// Callers (4): 0x44006ec7, 0x4400774f, 0x44007808, 0x4400782c
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_44007836(void)

{
  undefined1 *puVar1;
  undefined *puVar2;
  undefined1 auStack_4 [4];
  
  puVar1 = auStack_4;
  if (_DAT_5056b5f8 == 0) {
    _DAT_50503760 = 0;
    _DAT_5050375c = 0;
    _DAT_50501514 = 0;
    _DAT_5048025c = 1;
    FUN_44005ed3();
    FUN_4400531a();
    FUN_44006216();
    FUN_4400629a();
    FUN_4400597e(0);
    FUN_440056b1(0x80,0);
    FUN_4400592c(2);
    _DAT_5056b5f8 = 1;
    FUN_440056b1(0x81,1);
    FUN_44005cd0();
    FUN_440057f3();
    FUN_44009741(1,1);
  }
  _DAT_50503760 = 0;
  _DAT_5050375c = 0;
  _DAT_50501514 = 0;
  if (_DAT_50480cc4 == 1) {
    FUN_440058d0();
    FUN_440054aa(0,*(undefined4 *)(puVar1 + 8));
  }
  else {
    FUN_440054cc(*(undefined4 *)(puVar1 + 8),*(undefined4 *)(puVar1 + 0xc));
    FUN_44005662(300);
  }
  FUN_44009c85();
  startup_drive(0);
  puVar2 = (undefined *)FUN_44009835();
  if (puVar2 != &DAT_80023a00) {
    _DAT_50501518 = 1;
    FUN_44009bd7();
  }
  while (_DAT_50503760 == 0) {
    delay_ms(0x32);
  }
  if (_DAT_50501518 == 1) {
    FUN_44009c85();
    startup_drive(0);
    puVar2 = (undefined *)FUN_44009835();
    if (puVar2 != &DAT_80023a00) {
      _DAT_50501518 = 1;
      FUN_44009bd7();
    }
  }
  FUN_440079a4();
  return;
}
```

#### `FUN_440079a4` (`power_off_or_halt_79a4`) — `0x440079a4` (110 B)

Desliga alimentação ou entra em parada definitiva (`power_off_or_halt_79a4`).

<!-- source: doc_boot_core.txt | FUNC FUN_440079a4 (power_off_or_halt_79a4) @ 0x440079a4 (size=110 B, req=0x440079a4) -->
```c
// Callers (15): 0x44006e2b, 0x440070bf, 0x4400722e, 0x4400726f, 0x4400730b, 0x4400737a, 0x4400777f, 0x4400799a, 0x44007bf3, 0x44007c42, 0x44007cc0, 0x44007d32, 0x44007db7, 0x44007e36, 0x44007e9c
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_440079a4(void)

{
  if (_DAT_5056b5f8 == 0) {
    FUN_44005ed3();
    FUN_4400531a();
    FUN_44006216();
    FUN_4400629a();
    FUN_4400597e(0);
    FUN_440056b1(0x80,0);
    FUN_440056b1(0x81,1);
    FUN_44005cd0();
    FUN_44009741(1,1);
    _DAT_5056b5f8 = 1;
  }
  FUN_44005896();
  do {
                    /* WARNING: Do nothing block with infinite loop */
  } while( true );
}
```

#### `wait_1_sec_timer_sync` (`wait_1_sec_timer_sync_b34d`) — `0x4400b34d` (140 B)

Aguarda 1 segundo sincronizado com o Timer MCU (`wait_1_sec_timer_sync_b34d`).

<!-- source: doc_boot_media_crypto.txt | FUNC wait_1_sec_timer_sync (wait_1_sec_timer_sync_b34d) @ 0x4400b34d (size=140 B, req=0x4400b34d) -->
```c
// Callers (0): 
// Referenced strings/literals:
//   0x44035222: 'wait 1 second.\n'
//   0x44035232: '*** timer correspond initialize ***\n'
//   0x44035257: 'startup reason = [%x]\n'
void wait_1_sec_timer_sync(void)

{
  int in_A3;
  
  uart0_printf(s_wait_1_second__44035222);
  delay_ms(1000);
  uart0_printf(s_____timer_correspond_initialize___44035232);
  FUN_4400597e(0);
  FUN_440056b1(0x80,0);
  FUN_4400592c(2,*(undefined4 *)(in_A3 + 8));
  uart0_printf(s_startup_reason_____x__44035257,(uint)*(byte *)(*(int *)(in_A3 + 8) + 1));
  FUN_440056b1(0x81,1);
  FUN_44005cd0();
  FUN_440057f3();
  if (*(char *)(*(int *)(in_A3 + 8) + 1) != '\x04') {
    FUN_440058d0();
  }
  return;
}
```

### 1.3 Subsistema BGO / NFROM / NOR Flash & NVRAM

Conjunto de funções que administram as partições BGO em memória NOR Flash (`Original` vs `Backup`), flags de estado NFROM, recuperação pós-queda de energia (`[nfrom] NOR STATE`), leitura de NVRAM/EEPROM (`nvram_read_8634`, `nvram_read_id`) e comandos CFI de apagamento/gravação NOR (`FUN_440236ed` / `FUN_440237ba`). Inclui também `FUN_440041dc` verificado em `verify_ghidra_doc_facts_out.txt`.

#### `FUN_440041dc` (`verify_ghidra_doc_facts_out.txt` — Recuperação NFROM / Chamadora de `FUN_44003fd0`)

Função verificada adicionalmente via PyGhidra contendo as chamadas reais para recuperação NFROM (`0x440041dc..`, inclui disassembly completo).

```text
--- BOOT.BIN FUNC CONTAINING 0x440041dc: FUN_440041dc @ 440041dc (size=273) ---
undefined4 FUN_440041dc(void)

{
  undefined1 *puVar1;
  undefined4 uVar2;
  undefined1 auStack_4 [4];
  
  puVar1 = auStack_4;
  nvram_read_id(0xe,0,0);
  *(undefined1 **)(puVar1 + -4) = &DAT_84380000;
  **(undefined2 **)(puVar1 + -4) = 0x90;
  *(undefined1 **)(puVar1 + -4) = &DAT_84380aaa;
  **(undefined2 **)(puVar1 + -4) = 0xaa;
  *(undefined1 **)(puVar1 + -4) = &DAT_84380554;
  **(undefined2 **)(puVar1 + -4) = 0x55;
  *(undefined1 **)(puVar1 + -4) = &DAT_84380aaa;
  **(undefined2 **)(puVar1 + -4) = 0x90;
  *(undefined1 **)(puVar1 + -4) = &DAT_84380000;
  **(undefined2 **)(puVar1 + 8) = **(undefined2 **)(puVar1 + -4);
  if (**(short **)(puVar1 + 8) == 0x1c) {
    *(int *)(puVar1 + -4) = *(int *)(puVar1 + -4) + 2;
    **(undefined2 **)(puVar1 + 0xc) = **(undefined2 **)(puVar1 + -4);
    uVar2 = 0xffffffff;
    **(undefined2 **)(puVar1 + -4) = 0xffff;
  }
  else if ((**(ushort **)(puVar1 + 8) & 0xff) == 1) {
    *(int *)(puVar1 + -4) = *(int *)(puVar1 + -4) + 2;
    **(undefined2 **)(puVar1 + 0xc) = **(undefined2 **)(puVar1 + -4);
    if (**(short **)(puVar1 + 0xc) == 0x227e) {
      *(undefined1 **)(puVar1 + -4) = &DAT_8438001c;
      **(undefined2 **)(puVar1 + 0xc) = **(undefined2 **)(puVar1 + -4);
    }
    else {
      **(undefined2 **)(puVar1 + 0xc) = 0;
    }
    uVar2 = 0xf0;
    **(undefined2 **)(puVar1 + -4) = 0xf0;
  }
  else if (**(short **)(puVar1 + 8) == 0x20) {
    *(int *)(puVar1 + -4) = *(int *)(puVar1 + -4) + 2;
    **(undefined2 **)(puVar1 + 0xc) = **(undefined2 **)(puVar1 + -4);
    uVar2 = 0xf0;
    **(undefined2 **)(puVar1 + -4) = 0xf0;
  }
  else {
    uVar2 = 0;
    **(undefined2 **)(puVar1 + 0xc) = 0;
  }
  return uVar2;
}
--- DISASM ---
  0x440041dc: [cf 10                ] MOVM [A3],(SP)
  0x440041de: [3f                   ] MOV SP,A3
  0x440041df: [f8 fe ec             ] ADD -0x14,SP
  0x440041e2: [f8 13 08             ] MOV D0,(0x8 ,A3)
  0x440041e5: [f8 17 0c             ] MOV D1,(0xc ,A3)
  0x440041e8: [00                   ] CLR D0
  0x440041e9: [42 0c                ] MOV D0,(0xc,SP)
  0x440041eb: [80 0e                ] MOV 0xe,D0
  0x440041ed: [04                   ] CLR D1
  0x440041ee: [dd dd b3 00 00 00 00 ] CALL 0x4400f5cb,[],0x0
  0x440041f5: [fc cc 00 00 38 84    ] MOV 0x84380000,D0
  0x440041fb: [f8 13 fc             ] MOV D0,(-0x4 ,A3)
  0x440041fe: [f8 23 fc             ] MOV (-0x4,A3),A0
  0x44004201: [2c 90 00             ] MOV 0x90,D0
  0x44004204: [f0 70                ] MOVHU D0,(A0)
  0x44004206: [fc cc aa 0a 38 84    ] MOV 0x84380aaa,D0
  0x4400420c: [f8 13 fc             ] MOV D0,(-0x4 ,A3)
  0x4400420f: [f8 23 fc             ] MOV (-0x4,A3),A0
  0x44004212: [2c aa 00             ] MOV 0xaa,D0
  0x44004215: [f0 70                ] MOVHU D0,(A0)
  0x44004217: [fc cc 54 05 38 84    ] MOV 0x84380554,D0
  0x4400421d: [f8 13 fc             ] MOV D0,(-0x4 ,A3)
  0x44004220: [f8 23 fc             ] MOV (-0x4,A3),A0
  0x44004223: [80 55                ] MOV 0x55,D0
  0x44004225: [f0 70                ] MOVHU D0,(A0)
  0x44004227: [fc cc aa 0a 38 84    ] MOV 0x84380aaa,D0
  0x4400422d: [f8 13 fc             ] MOV D0,(-0x4 ,A3)
  0x44004230: [f8 23 fc             ] MOV (-0x4,A3),A0
  0x44004233: [2c 90 00             ] MOV 0x90,D0
  0x44004236: [f0 70                ] MOVHU D0,(A0)
  0x44004238: [fc cc 00 00 38 84    ] MOV 0x84380000,D0
  0x4400423e: [f8 13 fc             ] MOV D0,(-0x4 ,A3)
  0x44004241: [f8 27 08             ] MOV (0x8,A3),A1
  0x44004244: [f8 23 fc             ] MOV (-0x4,A3),A0
  0x44004247: [f0 60                ] MOVHU (A0),D0
  0x44004249: [f0 71                ] MOVHU D0,(A1)
  0x4400424b: [f8 23 08             ] MOV (0x8,A3),A0
  0x4400424e: [f0 60                ] MOVHU (A0),D0
  0x44004250: [a0 1c                ] CMP 0x1c,D0
  0x44004252: [c9 1d                ] BNE 0x4400426f
  0x44004254: [f8 03 fc             ] MOV (-0x4 ,A3),D0
  0x44004257: [28 02                ] ADD 0x2,D0
  0x44004259: [f8 13 fc             ] MOV D0,(-0x4 ,A3)
  0x4400425c: [f8 27 0c             ] MOV (0xc,A3),A1
  0x4400425f: [f8 23 fc             ] MOV (-0x4,A3),A0
  0x44004262: [f0 60                ] MOVHU (A0),D0
  0x44004264: [f0 71                ] MOVHU D0,(A1)
  0x44004266: [f8 23 fc             ] MOV (-0x4,A3),A0
  0x44004269: [80 ff                ] MOV 0xff,D0
  0x4400426b: [f0 70                ] MOVHU D0,(A0)
  0x4400426d: [ca 7b                ] BRA 0x440042e8
  0x4400426f: [f8 23 08             ] MOV (0x8,A3),A0
  0x44004272: [f0 60                ] MOVHU (A0),D0
  0x44004274: [f8 e0 ff             ] AND 0xff,D0
  0x44004277: [a0 01                ] CMP 0x1,D0
  0x44004279: [c9 44                ] BNE 0x440042bd
  0x4400427b: [f8 03 fc             ] MOV (-0x4 ,A3),D0
  0x4400427e: [28 02                ] ADD 0x2,D0
  0x44004280: [f8 13 fc             ] MOV D0,(-0x4 ,A3)
  0x44004283: [f8 27 0c             ] MOV (0xc,A3),A1
  0x44004286: [f8 23 fc             ] MOV (-0x4,A3),A0
  0x44004289: [f0 60                ] MOVHU (A0),D0
  0x4400428b: [f0 71                ] MOVHU D0,(A1)
  0x4400428d: [f8 23 0c             ] MOV (0xc,A3),A0
  0x44004290: [f0 60                ] MOVHU (A0),D0
  0x44004292: [fa c8 7e 22          ] CMP 0x227e,D0
  0x44004296: [c9 17                ] BNE 0x440042ad
  0x44004298: [fc cc 1c 00 38 84    ] MOV 0x8438001c,D0
  0x4400429e: [f8 13 fc             ] MOV D0,(-0x4 ,A3)
  0x440042a1: [f8 27 0c             ] MOV (0xc,A3),A1
  0x440042a4: [f8 23 fc             ] MOV (-0x4,A3),A0
  0x440042a7: [f0 60                ] MOVHU (A0),D0
  0x440042a9: [f0 71                ] MOVHU D0,(A1)
  0x440042ab: [ca 08                ] BRA 0x440042b3
  0x440042ad: [f8 23 0c             ] MOV (0xc,A3),A0
  0x440042b0: [00                   ] CLR D0
  0x440042b1: [f0 70                ] MOVHU D0,(A0)
  0x440042b3: [f8 23 fc             ] MOV (-0x4,A3),A0
  0x440042b6: [2c f0 00             ] MOV 0xf0,D0
  0x440042b9: [f0 70                ] MOVHU D0,(A0)
  0x440042bb: [ca 2d                ] BRA 0x440042e8
  0x440042bd: [f8 23 08             ] MOV (0x8,A3),A0
  0x440042c0: [f0 60                ] MOVHU (A0),D0
  0x440042c2: [a0 20                ] CMP 0x20,D0
  0x440042c4: [c9 1e                ] BNE 0x440042e2
  0x440042c6: [f8 03 fc             ] MOV (-0x4 ,A3),D0
  0x440042c9: [28 02                ] ADD 0x2,D0
  0x440042cb: [f8 13 fc             ] MOV D0,(-0x4 ,A3)
  0x440042ce: [f8 27 0c             ] MOV (0xc,A3),A1
  0x440042d1: [f8 23 fc             ] MOV (-0x4,A3),A0
  0x440042d4: [f0 60                ] MOVHU (A0),D0
  0x440042d6: [f0 71                ] MOVHU D0,(A1)
  0x440042d8: [f8 23 fc             ] MOV (-0x4,A3),A0
  0x440042db: [2c f0 00             ] MOV 0xf0,D0
  0x440042de: [f0 70                ] MOVHU D0,(A0)
  0x440042e0: [ca 08                ] BRA 0x440042e8
  0x440042e2: [f8 23 0c             ] MOV (0xc,A3),A0
  0x440042e5: [00                   ] CLR D0
  0x440042e6: [f0 70                ] MOVHU D0,(A0)
  0x440042e8: [f2 fc                ] MOV A3,SP
  0x440042ea: [df 10 04             ] RET [A3],0x4
```

#### `FUN_44003fd0` (`nfrom_power_fail_recovery`) — `0x44003fd0` (513 B)

Recuperação de estado/power-fail em NOR (`nfrom_power_fail_recovery`).

<!-- source: doc_boot_core.txt | FUNC FUN_44003fd0 (nfrom_power_fail_recovery) @ 0x44003fd0 (size=513 B, req=0x44003fd0) -->
```c
// Callers (3): 0x440083bf, 0x44008c1e, 0x44008c8e
// Referenced strings/literals:
//   0x440338e3: '[nfrom] Power failer recovery error[%d].\n'
//   0x440338bd: '[nfrom] Main control data error[%u].\n'
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

undefined4 FUN_44003fd0(void)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  FUN_4402c348(&DAT_505163d4,0xff,&DAT_00001ad8);
  uVar2 = FUN_44001c49(3,0x801,&DAT_505163d4);
  puVar1[-1] = uVar2;
  if (puVar1[-1] == 0) {
    uVar2 = FUN_440018bb();
    puVar1[-2] = uVar2;
    if (puVar1[-2] == 0) {
      _DAT_50517ea8 = 0;
      _DAT_50517eaa = 0;
      *(undefined2 *)((int)puVar1 + -10) = 0;
      while (*(ushort *)((int)puVar1 + -10) < 0x35) {
        *(undefined2 *)((int)puVar1 + -0xe) = 0;
        while (*(ushort *)((int)puVar1 + -0xe) < _DAT_505163d8) {
          *(short *)(puVar1 + -4) = *(short *)((int)puVar1 + -0xe) * 0x10 + 0x10;
          if (_DAT_505163da < *(ushort *)(puVar1 + -4)) {
            *(ushort *)(puVar1 + -4) = 0x10 - (_DAT_505163da - *(short *)(puVar1 + -4));
          }
          else {
            *(undefined2 *)(puVar1 + -4) = 0x10;
          }
          *(undefined2 *)(puVar1 + -3) = 0;
          while (*(ushort *)(puVar1 + -3) < *(ushort *)(puVar1 + -4)) {
            uVar2 = FUN_4402c2b7((uint)*(ushort *)((int)puVar1 + -10) * 9 + 0x50480004,
                                 ((uint)*(ushort *)(puVar1 + -3) +
                                 (uint)*(ushort *)((int)puVar1 + -0xe) * 0x20) * 0x10 + 0x505165d4,8
                                );
            puVar1[-2] = uVar2;
            if (puVar1[-2] == 0) {
              *(undefined2 *)((uint)*(ushort *)((int)puVar1 + -10) * 4 + 0x50517dd4) =
                   *(undefined2 *)((int)puVar1 + -0xe);
              *(undefined2 *)((uint)*(ushort *)((int)puVar1 + -10) * 4 + 0x50517dd6) =
                   *(undefined2 *)(puVar1 + -3);
              if ((_DAT_50517ea8 < *(ushort *)((int)puVar1 + -0xe)) ||
                 ((_DAT_50517ea8 == *(ushort *)((int)puVar1 + -0xe) &&
                  (_DAT_50517eaa <= *(ushort *)(puVar1 + -3))))) {
                _DAT_50517ea8 = *(ushort *)((int)puVar1 + -0xe);
                _DAT_50517eaa = *(short *)(puVar1 + -3) + 1;
                if (0xf < _DAT_50517eaa) {
                  _DAT_50517ea8 = _DAT_50517ea8 + 1;
                  _DAT_50517eaa = 0;
                }
              }
              break;
            }
            *(short *)(puVar1 + -3) = *(short *)(puVar1 + -3) + 1;
          }
          if (puVar1[-2] == 0) break;
          *(short *)((int)puVar1 + -0xe) = *(short *)((int)puVar1 + -0xe) + 1;
        }
        *(short *)((int)puVar1 + -10) = *(short *)((int)puVar1 + -10) + 1;
      }
      puVar1[-5] = 0;
    }
    else {
      FUN_4400bd82(1,s__nfrom__Power_failer_recovery_er_440338e3,puVar1[-2]);
      puVar1[-5] = puVar1[-2];
    }
  }
  else {
    FUN_4400bd82(1,s__nfrom__Main_control_data_error__440338bd,puVar1[-1]);
    puVar1[-5] = 0xfffffffb;
  }
  return puVar1[-5];
}
```

#### `FUN_440018bb` (`nfrom_recover_map`) — `0x440018bb` (810 B)

Recuperação e remapeamento de tabela NFROM (`nfrom_recover_map`).

<!-- source: doc_boot_core.txt | FUNC FUN_440018bb (nfrom_recover_map) @ 0x440018bb (size=810 B, req=0x440018bb) -->
```c
// Callers (1): 0x4400403c
// Referenced strings/literals:
//   0x440333be: '[nfrom] NOR STATE : Original -> Backup\n'
//   0x44033396: '[nfrom] NOR STATE : Backup -> Original\n'
//   0x440333e6: '[nfrom] Map load error[%u].\n'
//   0x44033403: '[nfrom] Map write error[%u].\n'
//   0x44033421: '[nfrom] File map load error[%u].\n'
//   0x44033443: '[nfrom] File map write error[%u].\n'
//   0x44033466: '[nfrom] Status change error[%d].\n'
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

undefined4 FUN_440018bb(void)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  if (_DAT_505163d4 == 0xffff) {
    FUN_4400bd82(2,s__nfrom__NOR_STATE___Original__>_B_440333be);
  }
  else if (_DAT_505163d4 < 0x10000) {
    if (_DAT_505163d4 != 0) goto LAB_44001bdd;
  }
  else {
    if (_DAT_505163d4 != 0xffffff) goto LAB_44001bdd;
    FUN_4400bd82(2,s__nfrom__NOR_STATE___Backup__>_Or_44033396);
  }
  *(undefined2 *)((int)puVar1 + -10) = 0;
  while (*(ushort *)((int)puVar1 + -10) < 8) {
    uVar2 = FUN_44001c49(3,*(short *)((int)puVar1 + -10) + *(short *)(puVar1 + -3),
                         (uint)*(ushort *)((int)puVar1 + -10) * 0x200 + 0x50516dd4);
    puVar1[-1] = uVar2;
    if (puVar1[-1] != 0) {
      FUN_4400bd82(1,s__nfrom__Map_load_error__u___440333e6,puVar1[-1]);
      puVar1[-7] = 0xfffffffb;
      goto LAB_44001bdd;
    }
    *(short *)((int)puVar1 + -10) = *(short *)((int)puVar1 + -10) + 1;
  }
  if (_DAT_505163d4 != 0) {
    *(undefined2 *)((int)puVar1 + -10) = 0;
    while (*(ushort *)((int)puVar1 + -10) < 8) {
      uVar2 = FUN_44001c49(3,*(short *)((int)puVar1 + -10) + *(short *)((int)puVar1 + -0xe),
                           &DAT_505180b8);
      puVar1[-1] = uVar2;
      if (puVar1[-1] != 0) {
        FUN_4400bd82(1,s__nfrom__Map_load_error__u___440333e6,puVar1[-1]);
        puVar1[-7] = 0xfffffffb;
        goto LAB_44001bdd;
      }
      uVar2 = FUN_4402c2b7(&DAT_505180b8,(uint)*(ushort *)((int)puVar1 + -10) * 0x200 + 0x50516dd4,
                           0x200);
      puVar1[-2] = uVar2;
      if (puVar1[-2] != 0) {
        uVar2 = FUN_44001d4d(*(short *)((int)puVar1 + -10) + *(short *)((int)puVar1 + -0xe));
        puVar1[-1] = uVar2;
        if (puVar1[-1] != 2) {
          FUN_4400bd82(1,s__nfrom__Map_write_error__u___44033403,puVar1[-1]);
          puVar1[-7] = 0xfffffffe;
          goto LAB_44001bdd;
        }
      }
      *(short *)((int)puVar1 + -10) = *(short *)((int)puVar1 + -10) + 1;
    }
  }
  *(undefined2 *)((int)puVar1 + -10) = 0;
  while (*(ushort *)((int)puVar1 + -10) < 4) {
    uVar2 = FUN_44001c49(3,*(short *)((int)puVar1 + -10) + 0x802,
                         (uint)*(ushort *)((int)puVar1 + -10) * 0x200 + 0x505165d4);
    puVar1[-1] = uVar2;
    if (puVar1[-1] != 0) {
      FUN_4400bd82(1,s__nfrom__File_map_load_error__u___44033421,puVar1[-1]);
      puVar1[-7] = 0xfffffffb;
      goto LAB_44001bdd;
    }
    *(short *)((int)puVar1 + -10) = *(short *)((int)puVar1 + -10) + 1;
  }
  if (_DAT_505163d4 != 0) {
    *(undefined2 *)((int)puVar1 + -10) = 0;
    while (*(ushort *)((int)puVar1 + -10) < 4) {
      FUN_4402c2f7(puVar1[-6],puVar1[-5],0x100);
      puVar1[-6] = puVar1[-6] + 0x200;
      puVar1[-5] = puVar1[-5] + 0x200;
      *(short *)((int)puVar1 + -10) = *(short *)((int)puVar1 + -10) + 1;
    }
    *(undefined2 *)((int)puVar1 + -10) = 0;
    while (*(ushort *)((int)puVar1 + -10) < 4) {
      uVar2 = FUN_44001d4d(*(short *)((int)puVar1 + -10) + 0x802);
      puVar1[-1] = uVar2;
      if (puVar1[-1] != 2) {
        FUN_4400bd82(1,s__nfrom__File_map_write_error__u__44033443,puVar1[-1]);
        puVar1[-7] = 0xfffffffe;
        goto LAB_44001bdd;
      }
      *(short *)((int)puVar1 + -10) = *(short *)((int)puVar1 + -10) + 1;
    }
    uVar2 = FUN_44001be5(0);
    puVar1[-2] = uVar2;
    if (puVar1[-2] != 0) {
      FUN_4400bd82(1,s__nfrom__Status_change_error__d___44033466,puVar1[-2]);
      puVar1[-7] = 0xfffffffc;
      goto LAB_44001bdd;
    }
  }
  puVar1[-7] = 0;
LAB_44001bdd:
  return puVar1[-7];
}
```

#### `FUN_44001c49` (`bgo_ioctl_dispatch`) — `0x44001c49` (260 B)

Despacho `ioctl` do driver BGO (`bgo_ioctl_dispatch`).

<!-- source: doc_boot_core.txt | FUNC FUN_44001c49 (bgo_ioctl_dispatch) @ 0x44001c49 (size=260 B, req=0x44001c49) -->
```c
// Callers (12): 0x440019a9, 0x44001a0b, 0x44001ad7, 0x44001d67, 0x4400292d, 0x44004011, 0x44008397, 0x440087b2, 0x44008837, 0x44008e79, 0x44008fa9, 0x44009311
// Referenced strings/literals:
//   0x440334af: '[nfrom] bgo_read_D(ID=0x%X) return error(%u).\n'
//   0x440334de: '[nfrom] BGO command[%d] error.\n'
undefined4 FUN_44001c49(undefined1 param_1,undefined2 param_2,undefined4 param_3)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  switch(param_1) {
  case 0:
    uVar2 = FUN_4401e636();
    puVar1[-1] = uVar2;
    break;
  case 1:
    uVar2 = FUN_4401eb14(1,4);
    puVar1[-1] = uVar2;
    break;
  case 2:
    uVar2 = FUN_4401e68a(0);
    puVar1[-1] = uVar2;
    break;
  case 3:
    uVar2 = FUN_4401e6ef(1,param_2,1,param_3);
    puVar1[-1] = uVar2;
    if (puVar1[-1] == 0) {
      puVar1[-1] = 0;
    }
    else {
      FUN_4400bd82(1,s__nfrom__bgo_read_D_ID_0x_X__retu_440334af,*(undefined2 *)(puVar1 + 3),
                   puVar1[-1]);
      puVar1[-1] = 1;
    }
    break;
  case 4:
    uVar2 = FUN_4401e751(1,param_2,1,param_3);
    puVar1[-1] = uVar2;
    break;
  default:
    FUN_4400bd82(1,s__nfrom__BGO_command__d__error__440334de,param_1);
    puVar1[-1] = 0xffffffff;
    break;
  case 6:
    uVar2 = FUN_4401e7ea(1,param_2,1);
    puVar1[-1] = uVar2;
    break;
  case 8:
  }
  return puVar1[-1];
}
```

#### `FUN_44007ea9` (`bgo_flag_clear_7ea9`) — `0x44007ea9` (115 B)

Limpeza de flag BGO (`bgo_flag_clear_7ea9`).

<!-- source: doc_boot_core.txt | FUNC FUN_44007ea9 (bgo_flag_clear_7ea9) @ 0x44007ea9 (size=115 B, req=0x44007ea9) -->
```c
// Callers (12): 0x440066af, 0x44006e24, 0x44006eb4, 0x44006f4b, 0x44006f8a, 0x44007024, 0x44007227, 0x44007268, 0x4400773c, 0x44007778, 0x440077f5, 0x44007e95
void FUN_44007ea9(void)

{
  undefined4 *puVar1;
  int iVar2;
  undefined4 uVar3;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  iVar2 = FUN_440084d9(0x504800a6);
  if (iVar2 == 0) {
    uVar3 = FUN_44008b02(0x504800a6,0,0);
    puVar1[-1] = uVar3;
    if (puVar1[-1] != 0) {
      boot_panic_or_halt_7812();
    }
  }
  iVar2 = FUN_440084d9(0x504800af);
  if (iVar2 == 0) {
    uVar3 = FUN_44008b02(0x504800af,0,0);
    puVar1[-1] = uVar3;
    if (puVar1[-1] != 0) {
      boot_panic_or_halt_7812();
    }
  }
  return;
}
```

#### `FUN_44007f1c` (`bgo_flag_clear2_7f1c`) — `0x44007f1c` (65 B)

Limpeza secundária de flag BGO (`bgo_flag_clear2_7f1c`).

<!-- source: doc_boot_core.txt | FUNC FUN_44007f1c (bgo_flag_clear2_7f1c) @ 0x44007f1c (size=65 B, req=0x44007f1c) -->
```c
// Callers (13): 0x440066b6, 0x44006e1d, 0x44006ead, 0x44006f44, 0x44006f83, 0x4400701d, 0x44007220, 0x44007261, 0x44007735, 0x44007771, 0x440077ee, 0x44007e8e, 0x44007fe7
void FUN_44007f1c(void)

{
  undefined1 *puVar1;
  undefined4 uVar2;
  undefined1 auStack_4 [4];
  
  puVar1 = auStack_4;
  FUN_4402c348(&DAT_50480cdc,0,0x30);
  uVar2 = FUN_44008b02(0x5048009d,0,0);
  *(undefined4 *)(puVar1 + -4) = uVar2;
  if (*(int *)(puVar1 + -4) != 0) {
    boot_panic_or_halt_7812();
  }
  return;
}
```

#### `FUN_44007fa1` (`bgo_status_7fa1`) — `0x44007fa1` (94 B)

Consulta de status de partição BGO (`bgo_status_7fa1`).

<!-- source: doc_boot_core.txt | FUNC FUN_44007fa1 (bgo_status_7fa1) @ 0x44007fa1 (size=94 B, req=0x44007fa1) -->
```c
// Callers (1): 0x44006634
undefined4 FUN_44007fa1(void)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  FUN_4402c348(&DAT_50480cdc,0,0x30);
  uVar2 = nvram_read_8634(0x5048009d,&DAT_50480cdc);
  puVar1[-1] = uVar2;
  if (puVar1[-1] == -1) {
    boot_panic_or_halt_7812();
  }
  else if (puVar1[-1] == -2) {
    FUN_44007f1c();
    puVar1[-2] = 0;
    goto LAB_44007ff7;
  }
  puVar1[-2] = 1;
LAB_44007ff7:
  return puVar1[-2];
}
```

#### `FUN_44008075` (`bgo_update_loop_8075`) — `0x44008075` (409 B)

Loop de atualização BGO entre partições (`bgo_update_loop_8075`).

<!-- source: doc_boot_core.txt | FUNC FUN_44008075 (bgo_update_loop_8075) @ 0x44008075 (size=409 B, req=0x44008075) -->
```c
// Callers (9): 0x44006653, 0x44006dc3, 0x44006dd0, 0x44006ddd, 0x44006dea, 0x44006df7, 0x44006e40, 0x44006e95, 0x44007065
undefined4 FUN_44008075(undefined4 param_1)

{
  undefined4 uStack_8;
  
  uStack_8 = 0;
  switch(param_1) {
  case 0:
    if (DAT_50480ce4 != '\0') {
      uStack_8 = 1;
    }
    break;
  case 1:
    if (DAT_50480ce9 != '\0') {
      uStack_8 = 1;
    }
    break;
  case 2:
    if (DAT_50480ce5 != '\0') {
      uStack_8 = 1;
    }
    break;
  case 3:
    if (DAT_50480ce6 != '\0') {
      uStack_8 = 1;
    }
    break;
  case 4:
    if (DAT_50480ce8 != '\0') {
      uStack_8 = 1;
    }
    break;
  case 5:
    if (DAT_50480ce7 != '\0') {
      uStack_8 = 1;
    }
    break;
  case 6:
    if (DAT_50480cea != '\0') {
      uStack_8 = 1;
    }
    break;
  case 7:
    if (DAT_50480ceb != '\0') {
      uStack_8 = 1;
    }
    break;
  case 8:
    if (DAT_50480cec != '\0') {
      uStack_8 = 1;
    }
    break;
  case 9:
    if (DAT_50480ced != '\0') {
      uStack_8 = 1;
    }
    break;
  case 10:
    if (DAT_50480cf5 != '\0') {
      uStack_8 = 1;
    }
    break;
  case 0xd:
    if (DAT_50480cef != '\0') {
      uStack_8 = 1;
    }
    break;
  case 0xe:
    if (DAT_50480cf0 != '\0') {
      uStack_8 = 1;
    }
    break;
  case 0xf:
    if (DAT_50480cee != '\0') {
      uStack_8 = 1;
    }
    break;
  case 0x10:
    if (DAT_50480cf1 != '\0') {
      uStack_8 = 1;
    }
    break;
  case 0x11:
    if (DAT_50480cf2 != '\0') {
      uStack_8 = 1;
    }
    break;
  case 0x12:
    if (DAT_50480cf3 != '\0') {
      uStack_8 = 1;
    }
    break;
  case 0x13:
    if (DAT_50480cf4 != '\0') {
      uStack_8 = 1;
    }
  }
  return uStack_8;
}
```

#### `FUN_4400825c` (`bgo_flag_update_825c`) — `0x4400825c` (275 B)

Atualização de flags/estado BGO (`bgo_flag_update_825c`).

<!-- source: doc_boot_core.txt | FUNC FUN_4400825c (bgo_flag_update_825c) @ 0x4400825c (size=275 B, req=0x4400825c) -->
```c
// Callers (1): 0x44008036
undefined4 FUN_4400825c(undefined4 param_1,undefined1 param_2)

{
  undefined1 *puVar1;
  undefined4 uVar2;
  undefined1 auStack_4 [4];
  
  puVar1 = auStack_4;
  switch(param_1) {
  case 0:
    DAT_50480ce4 = param_2;
    break;
  case 1:
    DAT_50480ce9 = param_2;
    break;
  case 2:
    DAT_50480ce5 = param_2;
    break;
  case 3:
    DAT_50480ce6 = param_2;
    break;
  case 4:
    DAT_50480ce8 = param_2;
    break;
  case 5:
    DAT_50480ce7 = param_2;
    break;
  case 6:
    DAT_50480cea = param_2;
    break;
  case 7:
    DAT_50480ceb = param_2;
    break;
  case 8:
    DAT_50480cec = param_2;
    break;
  case 9:
    DAT_50480ced = param_2;
    break;
  case 10:
    DAT_50480cf5 = param_2;
    break;
  case 0xd:
    DAT_50480cef = param_2;
    break;
  case 0xe:
    DAT_50480cf0 = param_2;
    break;
  case 0xf:
    DAT_50480cee = param_2;
    break;
  case 0x10:
    DAT_50480cf1 = param_2;
    break;
  case 0x11:
    DAT_50480cf2 = param_2;
    break;
  case 0x12:
    DAT_50480cf3 = param_2;
    break;
  case 0x13:
    DAT_50480cf4 = param_2;
  }
  uVar2 = FUN_44007f5d();
  *(undefined4 *)(puVar1 + -4) = uVar2;
  return *(undefined4 *)(puVar1 + -4);
}
```

#### `bgo_driver_init` (`bgo_driver_init`) — `0x4400836f` (140 B)

Inicialização principal do driver BGO (`bgo_driver_init`).

<!-- source: doc_boot_core.txt | FUNC bgo_driver_init (bgo_driver_init) @ 0x4400836f (size=140 B, req=0x4400836f) -->
```c
// Callers (3): 0x44004a2b, 0x44004abd, 0x44004b14
// Referenced strings/literals:
//   0x44034bf0: 'BGO driver init failed[%u].\n'
//   0x44034c0d: 'Control information init failed[%d].\n'
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

undefined4 bgo_driver_init(void)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  if (_DAT_50480d0c != 1) {
    FUN_440041dc(&DAT_50480000,&DAT_50480002);
    uVar2 = FUN_44001c49(0,0,0);
    puVar1[-1] = uVar2;
    if (puVar1[-1] != 0) {
      uart0_printf(s_BGO_driver_init_failed__u___44034bf0,puVar1[-1]);
      puVar1[-3] = 0xffffffff;
      goto LAB_440083f3;
    }
    uVar2 = FUN_44003fd0();
    puVar1[-2] = uVar2;
    if (puVar1[-2] != 0) {
      uart0_printf(s_Control_information_init_failed__44034c0d,puVar1[-2]);
      puVar1[-3] = 0xffffffff;
      goto LAB_440083f3;
    }
    _DAT_50480d0c = 1;
  }
  puVar1[-3] = 0;
LAB_440083f3:
  return puVar1[-3];
}
```

#### `nvram_read_8634` (`nvram_read_8634`) — `0x44008634` (864 B)

Leitura de parâmetros e estrutura da NVRAM (`nvram_read_8634`).

<!-- source: doc_boot_core.txt | FUNC nvram_read_8634 (nvram_read_8634) @ 0x44008634 (size=864 B, req=0x44008634) -->
```c
// Callers (4): 0x44004b36, 0x44006c05, 0x44007fc6, 0x440093aa
// Referenced strings/literals:
//   0x44034c63: 'Data read buffer pointer is NULL.\n'
//   0x44034c86: 'ID invalid.\n'
//   0x44034c93: 'ID[%d] read error[%u].\n'
//   0x44034ccb: 'error!: uncompress() = %d\n'
//   0x44034cab: 'Last data ID[%d] not equal %d.\n'
//   0x44034c33: 'BGO driver no init.\n'
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

undefined4 nvram_read_8634(undefined4 param_1,int param_2)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  if (_DAT_50480d0c == 1) {
    if (param_2 == 0) {
      uart0_printf(s_Data_read_buffer_pointer_is_NULL_44034c63);
      puVar1[-9] = 0xffffffff;
    }
    else {
      uVar2 = FUN_440084d9(param_1);
      puVar1[-1] = uVar2;
      if (puVar1[-1] == 0) {
        uVar2 = FUN_440083fb(puVar1[2],(int)puVar1 + -6,puVar1 + -2);
        puVar1[-1] = uVar2;
        if (puVar1[-1] == 0) {
          *(undefined2 *)((int)puVar1 + -10) =
               *(undefined2 *)
                (((uint)*(ushort *)((int)puVar1 + -6) * 0x20 + (uint)*(ushort *)(puVar1 + -2)) *
                 0x10 + 0x505165e0);
          puVar1[-5] = *(undefined4 *)
                        (((uint)*(ushort *)((int)puVar1 + -6) * 0x20 +
                         (uint)*(ushort *)(puVar1 + -2)) * 0x10 + 0x505165dc);
          if (*(char *)(((uint)*(ushort *)((int)puVar1 + -6) * 0x20 + (uint)*(ushort *)(puVar1 + -2)
                        ) * 0x10 + 0x505165e2) != -1) {
            puVar1[3] = &DAT_50481514;
          }
          puVar1[-6] = puVar1[-5] & 0x1ff;
          if (puVar1[-6] == 0) {
            puVar1[-6] = 0x200;
          }
          *(short *)(puVar1 + -3) = (short)(puVar1[-5] + 0x1ff >> 9) + -1;
          *(undefined2 *)((int)puVar1 + -0xe) = 0;
          while (*(ushort *)((int)puVar1 + -0xe) < *(ushort *)(puVar1 + -3)) {
            if (*(short *)((int)puVar1 + -10) == -1) {
              uart0_printf(s_ID_invalid__44034c86);
              puVar1[-9] = 0xffffffff;
              goto LAB_4400898c;
            }
            uVar2 = FUN_44001c49(3,*(undefined2 *)((int)puVar1 + -10),
                                 puVar1[3] + (uint)*(ushort *)((int)puVar1 + -0xe) * 0x200);
            puVar1[-7] = uVar2;
            if (puVar1[-7] != 0) {
              uart0_printf(s_ID__d__read_error__u___44034c93,(uint)*(ushort *)((int)puVar1 + -10),
                           puVar1[-7]);
              puVar1[-9] = 0xffffffff;
              goto LAB_4400898c;
            }
            *(short *)((int)puVar1 + -6) = (short)(*(ushort *)((int)puVar1 + -10) + 0xff >> 8);
            *(short *)((int)puVar1 + -6) = *(short *)((int)puVar1 + -6) + -1;
            *(ushort *)(puVar1 + -2) = *(short *)((int)puVar1 + -10) - 1U & 0xff;
            *(undefined2 *)((int)puVar1 + -10) =
                 *(undefined2 *)
                  (((uint)*(ushort *)(puVar1 + -2) + (uint)*(ushort *)((int)puVar1 + -6) * 0x100) *
                   2 + 0x50516dd4);
            *(short *)((int)puVar1 + -0xe) = *(short *)((int)puVar1 + -0xe) + 1;
          }
          uVar2 = FUN_44001c49(3,*(undefined2 *)((int)puVar1 + -10),&DAT_505180b8);
          puVar1[-7] = uVar2;
          if (puVar1[-7] == 0) {
            FUN_4402c2f7((uint)*(ushort *)((int)puVar1 + -0xe) * 0x200 + puVar1[3],&DAT_505180b8,
                         puVar1[-6]);
            *(short *)((int)puVar1 + -6) = (short)(*(ushort *)((int)puVar1 + -10) + 0xff >> 8);
            *(short *)((int)puVar1 + -6) = *(short *)((int)puVar1 + -6) + -1;
            *(ushort *)(puVar1 + -2) = *(short *)((int)puVar1 + -10) - 1U & 0xff;
            *(undefined2 *)((int)puVar1 + -10) =
                 *(undefined2 *)
                  (((uint)*(ushort *)(puVar1 + -2) + (uint)*(ushort *)((int)puVar1 + -6) * 0x100) *
                   2 + 0x50516dd4);
            if (*(short *)((int)puVar1 + -10) == -0x13) {
              uVar2 = FUN_440083fb(puVar1[2],(int)puVar1 + -6,puVar1 + -2);
              puVar1[-1] = uVar2;
              if (puVar1[-1] == 0) {
                if (*(char *)(((uint)*(ushort *)((int)puVar1 + -6) * 0x20 +
                              (uint)*(ushort *)(puVar1 + -2)) * 0x10 + 0x505165e2) != -1) {
                  puVar1[-6] = *(undefined4 *)puVar1[3];
                  puVar1[-5] = puVar1[-5] + -4;
                  uVar2 = FUN_44028393(puVar1[-8],puVar1 + -6,puVar1[3] + 4,puVar1[-5] + 0x100);
                  puVar1[-1] = uVar2;
                  if (puVar1[-1] != 0) {
                    uart0_printf(s_error___uncompress______d_44034ccb,puVar1[-1]);
                    puVar1[-9] = 0xffffffff;
                    goto LAB_4400898c;
                  }
                }
                puVar1[-9] = 0;
              }
              else {
                puVar1[-9] = 0xffffffff;
              }
            }
            else {
              uart0_printf(s_Last_data_ID__d__not_equal__d__44034cab,
                           (uint)*(ushort *)((int)puVar1 + -10),0xffed);
              puVar1[-9] = 0xffffffff;
            }
          }
          else {
            uart0_printf(s_ID__d__read_error__u___44034c93,(uint)*(ushort *)((int)puVar1 + -10),
                         puVar1[-7]);
            puVar1[-9] = 0xffffffff;
          }
        }
        else {
          puVar1[-9] = 0;
        }
      }
      else {
        puVar1[-9] = 0;
      }
    }
  }
  else {
    uart0_printf(s_BGO_driver_no_init__44034c33);
    puVar1[-9] = 0xffffffff;
  }
LAB_4400898c:
  return puVar1[-9];
}
```

#### `FUN_44008b02` (`bgo_ctrl_init_8b02`) — `0x44008b02` (455 B)

Inicialização do controlador BGO (`bgo_ctrl_init_8b02`).

<!-- source: doc_boot_core.txt | FUNC FUN_44008b02 (bgo_ctrl_init_8b02) @ 0x44008b02 (size=455 B, req=0x44008b02) -->
```c
// Callers (4): 0x44007ecb, 0x44007eff, 0x44007f40, 0x440090bd
// Referenced strings/literals:
//   0x44034cf5: 'Not compress data write start.\n'
//   0x44034d15: '%s rewrite error[%d].\n'
//   0x44034d2c: 'status recovery error[%d].\n'
//   0x44034d48: '%s write error[%d].\n'
//   0x44034ce6: '%s not found.\n'
//   0x44034c33: 'BGO driver no init.\n'
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

undefined4 FUN_44008b02(undefined4 param_1)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  if (_DAT_50480d0c == 1) {
    uVar2 = FUN_44008a99(param_1);
    puVar1[-2] = uVar2;
    if (puVar1[-2] == 0) {
      if (0x200 < (uint)puVar1[4]) {
        uVar2 = FUN_4400ba02(0x50481518,puVar1 + -1,puVar1[3],puVar1[4]);
        puVar1[-2] = uVar2;
        if (puVar1[-2] == 1) {
          puVar1[3] = &DAT_50481514;
          *(undefined4 *)puVar1[3] = puVar1[4];
          puVar1[4] = puVar1[-1] + 4;
          *(undefined1 *)((int)puVar1 + -0xf) = 0;
        }
        else {
          uart0_printf(s_Not_compress_data_write_start__44034cf5);
        }
      }
      uVar2 = FUN_440084d9(puVar1[2]);
      puVar1[-2] = uVar2;
      if (puVar1[-2] == 0) {
        uVar2 = FUN_44002f71(*(undefined2 *)((int)puVar1 + -0xe),puVar1[3],puVar1[4],
                             *(undefined1 *)((int)puVar1 + -0xf));
        puVar1[-2] = uVar2;
        if (puVar1[-2] != 0) {
          uart0_printf(s__s_rewrite_error__d___44034d15,puVar1[2],puVar1[-2]);
          uVar2 = FUN_44003fd0();
          puVar1[-3] = uVar2;
          if (puVar1[-3] == 0) {
            puVar1[-5] = 0xfffffffd;
          }
          else {
            uart0_printf(s_status_recovery_error__d___44034d2c,puVar1[-3]);
            puVar1[-5] = 0xffffffff;
          }
          goto LAB_44008cc1;
        }
      }
      else if (puVar1[4] != 0) {
        uVar2 = FUN_4400225d(*(undefined2 *)((int)puVar1 + -0xe),puVar1[3],puVar1[4],
                             *(undefined1 *)((int)puVar1 + -0xf));
        puVar1[-2] = uVar2;
        if (puVar1[-2] != 0) {
          uart0_printf(s__s_write_error__d___44034d48,puVar1[2],puVar1[-2]);
          uVar2 = FUN_44003fd0();
          puVar1[-3] = uVar2;
          if (puVar1[-3] == 0) {
            puVar1[-5] = 0xfffffffd;
          }
          else {
            uart0_printf(s_status_recovery_error__d___44034d2c,puVar1[-3]);
            puVar1[-5] = 0xffffffff;
          }
          goto LAB_44008cc1;
        }
      }
      puVar1[-5] = 0;
    }
    else {
      uart0_printf(s__s_not_found__44034ce6,puVar1[2]);
      puVar1[-5] = 0xffffffff;
    }
  }
  else {
    uart0_printf(s_BGO_driver_no_init__44034c33);
    puVar1[-5] = 0xffffffff;
  }
LAB_44008cc1:
  return puVar1[-5];
}
```

#### `FUN_44008cc9` (`bgo_device_info_check_8cc9`) — `0x44008cc9` (989 B)

Validação de Device Info / cabeçalho de controle BGO (`bgo_device_info_check_8cc9`).

<!-- source: doc_boot_core.txt | FUNC FUN_44008cc9 (bgo_device_info_check_8cc9) @ 0x44008cc9 (size=989 B, req=0x44008cc9) -->
```c
// Callers (1): 0x44009101
// Referenced strings/literals:
//   0x44034d5d: 'File size different. Ctrl[0x%X] != Check[0x%X].\n'
//   0x44034c86: 'ID invalid.\n'
//   0x44034c93: 'ID[%d] read error[%u].\n'
//   0x44034d8e: 'ID[%d] verify error(0x%08X-0x%08X).\n'
//   0x44034cab: 'Last data ID[%d] not equal %d.\n'
//   0x44034ce6: '%s not found.\n'
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

undefined4 FUN_44008cc9(undefined4 param_1)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack0000000c;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  uVar2 = FUN_44008a99(param_1);
  puVar1[-1] = uVar2;
  if (puVar1[-1] == 0) {
    *(undefined2 *)((int)puVar1 + -6) =
         *(undefined2 *)((uint)*(ushort *)(puVar1 + -3) * 4 + 0x50517dd4);
    *(undefined2 *)(puVar1 + -2) = *(undefined2 *)((uint)*(ushort *)(puVar1 + -3) * 4 + 0x50517dd6);
    *(undefined2 *)((int)puVar1 + -10) =
         *(undefined2 *)
          (((uint)*(ushort *)((int)puVar1 + -6) * 0x20 + (uint)*(ushort *)(puVar1 + -2)) * 0x10 +
          0x505165e0);
    if (*(char *)(((uint)*(ushort *)((int)puVar1 + -6) * 0x20 + (uint)*(ushort *)(puVar1 + -2)) *
                  0x10 + 0x505165e2) == -1) {
      if (*(int *)(((uint)*(ushort *)((int)puVar1 + -6) * 0x20 + (uint)*(ushort *)(puVar1 + -2)) *
                   0x10 + 0x505165dc) != puVar1[4]) {
        uart0_printf(s_File_size_different__Ctrl_0x_X____44034d5d,
                     *(undefined4 *)
                      (((uint)*(ushort *)((int)puVar1 + -6) * 0x20 + (uint)*(ushort *)(puVar1 + -2))
                       * 0x10 + 0x505165dc),puVar1[4]);
        puVar1[-8] = 0xffffffff;
        goto LAB_4400909e;
      }
    }
    else {
      puVar1[3] = &DAT_50481514;
      puVar1[4] = *(undefined4 *)
                   (((uint)*(ushort *)((int)puVar1 + -6) * 0x20 + (uint)*(ushort *)(puVar1 + -2)) *
                    0x10 + 0x505165dc);
    }
    puVar1[-6] = puVar1[4] & 0x1ff;
    if (puVar1[-6] == 0) {
      puVar1[-6] = 0x200;
    }
    *(short *)((int)puVar1 + -0xe) = (short)(puVar1[4] + 0x1ff >> 9) + -1;
    *(undefined2 *)(puVar1 + -4) = 0;
    while (*(ushort *)(puVar1 + -4) < *(ushort *)((int)puVar1 + -0xe)) {
      if (*(short *)((int)puVar1 + -10) == -1) {
        uart0_printf(s_ID_invalid__44034c86);
        puVar1[-8] = 0xffffffff;
        goto LAB_4400909e;
      }
      uVar2 = FUN_44001c49(3,*(undefined2 *)((int)puVar1 + -10),&DAT_505180b8);
      puVar1[-7] = uVar2;
      if (puVar1[-7] != 0) {
        uart0_printf(s_ID__d__read_error__u___44034c93,(uint)*(ushort *)((int)puVar1 + -10),
                     puVar1[-7]);
        puVar1[-8] = 0xffffffff;
        goto LAB_4400909e;
      }
      uVar2 = FUN_4402c2b7((uint)*(ushort *)(puVar1 + -4) * 0x200 + puVar1[3],&DAT_505180b8,0x200);
      puVar1[-1] = uVar2;
      if (puVar1[-1] != 0) {
        uart0_printf(s_ID__d__verify_error_0x_08X_0x_08_44034d8e,
                     (uint)*(ushort *)((int)puVar1 + -10),puVar1[3],puVar1[3] + 0x1ff);
        puVar1[-8] = 0xffffffff;
        goto LAB_4400909e;
      }
      if (((*(short *)(puVar1 + -4) == 0) &&
          (*(char *)(((uint)*(ushort *)((int)puVar1 + -6) * 0x20 + (uint)*(ushort *)(puVar1 + -2)) *
                     0x10 + 0x505165e2) != -1)) && (_DAT_505180b8 != puVar1[-5])) {
        uart0_printf(s_File_size_different__Ctrl_0x_X____44034d5d,_DAT_505180b8,puVar1[-5]);
        puVar1[-8] = 0xffffffff;
        goto LAB_4400909e;
      }
      *(short *)((int)puVar1 + -6) = (short)(*(ushort *)((int)puVar1 + -10) + 0xff >> 8);
      *(short *)((int)puVar1 + -6) = *(short *)((int)puVar1 + -6) + -1;
      *(ushort *)(puVar1 + -2) = *(short *)((int)puVar1 + -10) - 1U & 0xff;
      *(undefined2 *)((int)puVar1 + -10) =
           *(undefined2 *)
            (((uint)*(ushort *)(puVar1 + -2) + (uint)*(ushort *)((int)puVar1 + -6) * 0x100) * 2 +
            0x50516dd4);
      *(short *)(puVar1 + -4) = *(short *)(puVar1 + -4) + 1;
    }
    uVar2 = FUN_44001c49(3,*(undefined2 *)((int)puVar1 + -10),&DAT_505180b8);
    puVar1[-7] = uVar2;
    if (puVar1[-7] == 0) {
      uVar2 = FUN_4402c2b7((uint)*(ushort *)(puVar1 + -4) * 0x200 + puVar1[3],&DAT_505180b8,
                           puVar1[-6]);
      puVar1[-1] = uVar2;
      if (puVar1[-1] == 0) {
        *(short *)((int)puVar1 + -6) = (short)(*(ushort *)((int)puVar1 + -10) + 0xff >> 8);
        *(short *)((int)puVar1 + -6) = *(short *)((int)puVar1 + -6) + -1;
        *(ushort *)(puVar1 + -2) = *(short *)((int)puVar1 + -10) - 1U & 0xff;
        *(undefined2 *)((int)puVar1 + -10) =
             *(undefined2 *)
              (((uint)*(ushort *)(puVar1 + -2) + (uint)*(ushort *)((int)puVar1 + -6) * 0x100) * 2 +
              0x50516dd4);
        if (*(short *)((int)puVar1 + -10) == -0x13) {
          puVar1[-8] = 0;
        }
        else {
          uart0_printf(s_Last_data_ID__d__not_equal__d__44034cab,
                       (uint)*(ushort *)((int)puVar1 + -10),0xffed);
          puVar1[-8] = 0xffffffff;
        }
      }
      else {
        uart0_printf(s_ID__d__verify_error_0x_08X_0x_08_44034d8e,
                     (uint)*(ushort *)((int)puVar1 + -10),puVar1[3],puVar1[-6] + puVar1[3] + -1);
        puVar1[-8] = 0xffffffff;
      }
    }
    else {
      uart0_printf(s_ID__d__read_error__u___44034c93,(uint)*(ushort *)((int)puVar1 + -10),puVar1[-7]
                  );
      puVar1[-8] = 0xffffffff;
    }
  }
  else {
    uart0_printf(s__s_not_found__44034ce6,puVar1[2]);
    puVar1[-8] = 0xffffffff;
  }
LAB_4400909e:
  return puVar1[-8];
}
```

#### `FUN_440090a6` (`bgo_write_part_90a6`) — `0x440090a6` (153 B)

Gravação de partição BGO (`bgo_write_part_90a6`).

<!-- source: doc_boot_core.txt | FUNC FUN_440090a6 (bgo_write_part_90a6) @ 0x440090a6 (size=153 B, req=0x440090a6) -->
```c
// Callers (3): 0x44004cc2, 0x44006ca7, 0x44007f76
// Referenced strings/literals:
//   0x44034dce: '%s[0x%X] verify error[%d].\n'
//   0x44034db3: '%s[0x%X] write error[%d].\n'
undefined4 FUN_440090a6(undefined4 param_1,undefined4 param_2,undefined4 param_3)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  uVar2 = FUN_44008b02(param_1,param_2,param_3);
  puVar1[-1] = uVar2;
  if (puVar1[-1] == 0) {
    if (puVar1[4] != 0) {
      uVar2 = FUN_44008cc9(puVar1[2],puVar1[3],puVar1[4]);
      puVar1[-1] = uVar2;
      if (puVar1[-1] != 0) {
        uart0_printf(s__s_0x_X__verify_error__d___44034dce,puVar1[2],puVar1[4],puVar1[-1]);
        puVar1[-2] = puVar1[-1];
        goto LAB_44009137;
      }
    }
    puVar1[-2] = 0;
  }
  else {
    uart0_printf(s__s_0x_X__write_error__d___44034db3,puVar1[2],puVar1[4],puVar1[-1]);
    puVar1[-2] = puVar1[-1];
  }
LAB_44009137:
  return puVar1[-2];
}
```

#### `FUN_4400920b` (`bgo_read_part_920b`) — `0x4400920b` (324 B)

Leitura de partição BGO (`bgo_read_part_920b`).

<!-- source: doc_boot_core.txt | FUNC FUN_4400920b (bgo_read_part_920b) @ 0x4400920b (size=324 B, req=0x4400920b) -->
```c
// Callers (2): 0x44006bde, 0x4400936d
// Referenced strings/literals:
//   0x44034e05: 'BGO read error[%u].\n'
//   0x44034c33: 'BGO driver no init.\n'
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

undefined4 FUN_4400920b(undefined4 param_1,undefined4 *param_2)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  *param_2 = 0;
  if (_DAT_50480d0c == 1) {
    uVar2 = FUN_440084d9(param_1);
    puVar1[-1] = uVar2;
    if (puVar1[-1] == 0) {
      uVar2 = FUN_440083fb(puVar1[2],(int)puVar1 + -6,puVar1 + -2);
      puVar1[-1] = uVar2;
      if (puVar1[-1] == 0) {
        if (*(char *)(((uint)*(ushort *)((int)puVar1 + -6) * 0x20 + (uint)*(ushort *)(puVar1 + -2))
                      * 0x10 + 0x505165e2) == -1) {
          *(undefined4 *)puVar1[3] =
               *(undefined4 *)
                (((uint)*(ushort *)((int)puVar1 + -6) * 0x20 + (uint)*(ushort *)(puVar1 + -2)) *
                 0x10 + 0x505165dc);
        }
        else if (*(short *)(((uint)*(ushort *)((int)puVar1 + -6) * 0x20 +
                            (uint)*(ushort *)(puVar1 + -2)) * 0x10 + 0x505165e0) != -1) {
          uVar2 = FUN_44001c49(3,*(undefined2 *)
                                  (((uint)*(ushort *)((int)puVar1 + -6) * 0x20 +
                                   (uint)*(ushort *)(puVar1 + -2)) * 0x10 + 0x505165e0),
                               &DAT_505180b8);
          puVar1[-3] = uVar2;
          if (puVar1[-3] != 0) {
            uart0_printf(s_BGO_read_error__u___44034e05,puVar1[-3]);
            puVar1[-4] = 0xffffffff;
            goto LAB_44009347;
          }
          *(undefined4 *)puVar1[3] = _DAT_505180b8;
        }
        puVar1[-4] = 0;
      }
      else {
        puVar1[-4] = 0;
      }
    }
    else {
      puVar1[-4] = 0;
    }
  }
  else {
    uart0_printf(s_BGO_driver_no_init__44034c33);
    puVar1[-4] = 0xffffffff;
  }
LAB_44009347:
  return puVar1[-4];
}
```

#### `FUN_4400934f` (`conveyer_check_934f`) — `0x4400934f` (133 B)

Verificação de consistência / conveyer check BGO (`conveyer_check_934f`).

<!-- source: doc_boot_core.txt | FUNC FUN_4400934f (conveyer_check_934f) @ 0x4400934f (size=133 B, req=0x4400934f) -->
```c
// Callers (2): 0x440065f3, 0x44006fff
// Referenced strings/literals:
//   0x44034e1a: 'Now DeviceInfo2 size = 0x%X.\n'
undefined4 FUN_4400934f(void)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  if (DAT_50480d10 == '\0') {
    uVar2 = FUN_4400920b(0x5048001f);
    puVar1[-1] = uVar2;
    if ((puVar1[-1] != 0) || (0x800 < (uint)puVar1[-2])) {
      uart0_printf(s_Now_DeviceInfo2_size___0x_X__44034e1a,puVar1[-2]);
      puVar1[-3] = 0xffffffff;
      goto LAB_440093cc;
    }
    uVar2 = nvram_read_8634(0x5048001f,0x50480d14);
    puVar1[-1] = uVar2;
    if (puVar1[-1] != 0) {
      puVar1[-3] = 0xffffffff;
      goto LAB_440093cc;
    }
    DAT_50480d10 = '\x01';
  }
  puVar1[-3] = 0;
LAB_440093cc:
  return puVar1[-3];
}
```

#### `nvram_read_id` (`nvram_read_id`) — `0x4400f5cb` (196 B)

Leitura de identificador / assinatura da NVRAM (`nvram_read_id`).

<!-- source: doc_boot_core.txt | FUNC nvram_read_id (nvram_read_id) @ 0x4400f5cb (size=196 B, req=0x4400f5cb) -->
```c
// Callers (14): 0x440041ee, 0x440046fd, 0x4400472d, 0x4400473d, 0x44004761, 0x4400498b, 0x4400499c, 0x44006464, 0x44006475, 0x44006d8c, 0x44006fad, 0x44007c8a, 0x4400bc5a, 0x44010e2e
// Referenced strings/literals:
//   0x44035834: 'ðÿ\n'
undefined4 nvram_read_id(uint param_1,uint *param_2,int param_3)

{
  ushort uVar1;
  ushort *puVar2;
  byte *pbVar3;
  ushort *puVar4;
  byte bVar5;
  int in_R0;
  int iVar6;
  
  if (0x19 < param_1) {
    return 1;
  }
  if (param_1 < 0x14) {
    iVar6 = in_R0 * 0x10;
    if ((&DAT_440356fd)[iVar6] == 0) {
      if (param_2 != (uint *)0x0) {
        *(ushort *)(&PTR_P0MD_440356f0)[in_R0 * 4] =
             *(ushort *)(&DAT_440356f4 + iVar6) & *(ushort *)(&PTR_P0MD_440356f0)[in_R0 * 4];
        *param_2 = (uint)(((&DAT_440356fc)[iVar6] & *(&PTR_P0OUT_440356f8)[in_R0 * 4]) != 0);
        return 0;
      }
      return 1;
    }
    if (param_3 == 0) {
      pbVar3 = (&PTR_P0OUT_440356f8)[in_R0 * 4];
      bVar5 = (&DAT_440356fc)[iVar6] & *pbVar3;
    }
    else {
      pbVar3 = (&PTR_P0OUT_440356f8)[in_R0 * 4];
      bVar5 = (&DAT_440356fd)[iVar6] | *pbVar3;
    }
    *pbVar3 = bVar5;
    puVar4 = (ushort *)(&PTR_P0MD_440356f0)[in_R0 * 4];
    uVar1 = *puVar4;
    puVar2 = (ushort *)(&DAT_440356f4 + iVar6);
  }
  else {
    puVar4 = (ushort *)(&PTR_P1MD_44035830)[(param_1 - 0x14) * 2];
    uVar1 = *puVar4;
    puVar2 = (ushort *)(&DAT_44035834 + (param_1 - 0x14) * 8);
  }
  *puVar4 = puVar2[1] | uVar1 & *puVar2;
  return 0;
}
```

#### `FUN_4401f044` (`bgo_nor_part_check_1f044`) — `0x4401f044` (61 B)

Checagem de partição BGO em NOR Flash (`bgo_nor_part_check_1f044`).

<!-- source: doc_boot_core.txt | FUNC FUN_4401f044 (bgo_nor_part_check_1f044) @ 0x4401f044 (size=61 B, req=0x4401f044) -->
```c
// Callers (12): 0x4401e669, 0x4401e6ce, 0x4401e721, 0x4401e79d, 0x4401e832, 0x4401eb3a, 0x4401f790, 0x4401f8c3, 0x44021e24, 0x44022f38, 0x440232f7, 0x44023378
undefined4 FUN_4401f044(int param_1,uint param_2)

{
  if (param_2 == 3) {
    return *(undefined4 *)(param_1 * 0xc + 0x505743b8);
  }
  if (param_2 < 4) {
    if (param_2 != 0) {
      return *(undefined4 *)(param_1 * 0xc + 0x505743b4);
    }
  }
  else if (param_2 == 4) {
    return *(undefined4 *)(param_1 * 0xc + 0x505743bc);
  }
  return 4;
}
```

#### `FUN_4401f081` (`bgo_err_log_1f081`) — `0x4401f081` (59 B)

Registro de erro BGO em NOR Flash (`bgo_err_log_1f081`).

<!-- source: doc_boot_core.txt | FUNC FUN_4401f081 (bgo_err_log_1f081) @ 0x4401f081 (size=59 B, req=0x4401f081) -->
```c
// Callers (35): 0x4401ef92, 0x4401f0d4, 0x4401f4d9, 0x4401f577, 0x4401f63b, 0x4401f91a, 0x4402027b, 0x44020963, 0x44020c18, 0x44020d4b, 0x44020e60, 0x44020f84, 0x4402189e, 0x440219e8, 0x44021a1e, 0x44021a87
undefined4 FUN_4401f081(int param_1,uint param_2,undefined4 param_3)

{
  if (param_2 == 3) {
    *(undefined4 *)(param_1 * 0xc + 0x505743b8) = param_3;
    return param_3;
  }
  if (param_2 < 4) {
    if (param_2 != 0) {
      *(undefined4 *)(param_1 * 0xc + 0x505743b4) = param_3;
      return param_3;
    }
  }
  else {
    if (param_2 != 4) {
      return param_3;
    }
    *(undefined4 *)(param_1 * 0xc + 0x505743bc) = param_3;
  }
  return param_3;
}
```

#### `FUN_4401f804` (`bgo_nor_sum_1f804`) — `0x4401f804` (292 B)

Cálculo de checksum BGO em NOR Flash (`bgo_nor_sum_1f804`).

<!-- source: doc_boot_core.txt | FUNC FUN_4401f804 (bgo_nor_sum_1f804) @ 0x4401f804 (size=292 B, req=0x4401f804) -->
```c
// Callers (1): 0x4401f785
void FUN_4401f804(undefined4 param_1)

{
  undefined2 uVar1;
  int iVar2;
  undefined2 *puVar3;
  undefined2 *puVar4;
  int iVar5;
  int *piVar6;
  int *piVar7;
  int iVar8;
  uint uVar9;
  undefined4 uVar10;
  undefined4 uVar11;
  uint uVar12;
  int *piVar13;
  int iVar14;
  undefined4 extraout_var;
  undefined4 extraout_var_00;
  undefined4 extraout_var_01;
  undefined4 extraout_var_02;
  int extraout_var_03;
  int extraout_var_04;
  int iVar15;
  int iStack_34;
  int local_30;
  int local_2c;
  undefined4 uStack_20;
  undefined4 uStack_1c;
  undefined4 uStack_18;
  undefined4 uStack_14;
  
  uVar12 = 1;
  piVar6 = &iStack_34;
  iVar14 = 0x5057452a;
  piVar7 = &DAT_44036590;
  iVar5 = 4;
  piVar13 = &local_30;
  do {
    iVar8 = FUN_4401f169(param_1,uVar12);
    param_1 = extraout_var;
    if (iVar8 == 0) {
      *piVar13 = 4;
    }
    else {
      iVar2 = (*piVar7 + iVar8) * 2;
      iVar15 = extraout_var_03;
      if (*(short *)(iVar14 + iVar2) != -1) {
        uVar9 = FUN_4401f214(0,*(undefined2 *)(iVar14 + iVar2 + 2));
        puVar4 = (undefined2 *)((*piVar7 + iVar8) * 2 + iVar14);
        uVar1 = *puVar4;
        puVar3 = (undefined2 *)((*piVar7 + (uVar9 & 0xff)) * 2 + iStack_34);
        *puVar4 = *puVar3;
        *puVar3 = uVar1;
        param_1 = extraout_var_00;
        iVar15 = extraout_var_04;
      }
      if (*(uint *)(iVar15 + iVar5 + -4) == (uint)((iVar8 + 1) * iVar8) >> 1) {
        uVar11 = 2;
      }
      else {
        uVar11 = 1;
      }
      *(undefined4 *)((int)piVar6 + iVar5) = uVar11;
    }
    piVar7 = piVar7 + 1;
    iVar5 = iVar5 + 4;
    piVar13 = piVar13 + 1;
    uVar12 = uVar12 + 1;
  } while (uVar12 < 5);
  uVar12 = 2;
  uVar11 = 0x145;
  piVar6 = piVar6 + 2;
  do {
    iVar5 = FUN_4401f044(param_1,uVar12);
    param_1 = extraout_var_01;
    if (iVar5 == 6) {
      uVar9 = uVar12;
      if (uVar12 == 2) {
        if ((local_30 == 2) && (local_2c == 2)) {
          uVar10 = 0x708;
          iVar5 = 2;
        }
        else if ((local_30 == 4) && (local_2c == 4)) {
          uVar10 = 0x70c;
          iVar5 = 4;
          uVar9 = 2;
        }
        else {
          iVar5 = 1;
          uVar10 = 0x710;
          uVar9 = 2;
        }
      }
      else {
        iVar5 = *piVar6;
        uVar10 = 0x715;
      }
      FUN_4401f081(extraout_var_01,uVar9,iVar5,uVar10,uVar11);
      param_1 = extraout_var_02;
    }
    piVar6 = piVar6 + 1;
    uVar12 = uVar12 + 1;
  } while (uVar12 < 5);
  return;
}
```

#### `FUN_440237ba` (`bgo_nor_erase_write_237ba`) — `0x440237ba` (395 B)

Apagamento/gravação de setor em NOR Flash (`bgo_nor_erase_write_237ba`).

<!-- source: doc_boot_core.txt | FUNC FUN_440237ba (bgo_nor_erase_write_237ba) @ 0x440237ba (size=395 B, req=0x440237ba) -->
```c
// Callers (2): 0x4401ecfa, 0x44022e69
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_440237ba(undefined4 param_1)

{
  int iVar1;
  
  FUN_44025b14();
  iVar1 = FUN_440236ed(param_1);
  if (iVar1 == 0) {
    _DAT_50574770 = FUN_4402426b;
    _DAT_5057476c = &LAB_44024367;
    _DAT_50574764 = &LAB_440243dd;
    _DAT_50574778 = &LAB_440244ba;
    _DAT_50574760 = FUN_4402451a;
    _DAT_5057477c = FUN_44024673;
    _DAT_50574774 = &LAB_440247b4;
    _DAT_50574768 = &LAB_44024970;
    _DAT_50574780 = &LAB_440249f9;
    _DAT_5057475c = &LAB_44024a23;
  }
  else if (iVar1 == 1) {
    _DAT_50574770 = FUN_44025c40;
    _DAT_5057476c = &LAB_44025ca4;
    _DAT_50574764 = &LAB_44025d14;
    _DAT_50574778 = &LAB_44025db0;
    _DAT_50574760 = FUN_44025e03;
    _DAT_5057477c = FUN_44025ecc;
    _DAT_50574774 = &LAB_44025f2e;
    _DAT_50574768 = &LAB_44026081;
    _DAT_50574780 = &LAB_440260e3;
    _DAT_5057475c = &LAB_4402610d;
  }
  else {
    _DAT_50574770 = FUN_44024bf2;
    _DAT_5057476c = &LAB_44024c56;
    _DAT_50574764 = &LAB_44024cc6;
    _DAT_50574778 = &LAB_44024d62;
    _DAT_50574760 = FUN_44024db5;
    _DAT_5057477c = FUN_44024e7e;
    _DAT_50574774 = &LAB_44024ee0;
    _DAT_50574768 = &LAB_44025033;
    _DAT_50574780 = &LAB_44025095;
    _DAT_5057475c = &LAB_440250bf;
  }
  (*_DAT_50574770)(param_1);
  return;
}
```

#### `FUN_440236ed` (`bgo_nor_cfi_cmd_236ed`) — `0x440236ed` (205 B)

Sequência CFI `0xAA`/`0x55` para NOR Flash — exemplo clássico de pruning MMIO pelo decompilador (`bgo_nor_cfi_cmd_236ed`).

<!-- source: doc_boot_core.txt | FUNC FUN_440236ed (bgo_nor_cfi_cmd_236ed) @ 0x440236ed (size=205 B, req=0x440236ed) -->
```c
// Callers (1): 0x440237c8
/* WARNING: Removing unreachable block (ram,0x44023774) */
/* WARNING: Removing unreachable block (ram,0x440237a4) */
/* WARNING: Removing unreachable block (ram,0x44023782) */
/* WARNING: Removing unreachable block (ram,0x44023791) */
/* WARNING: Removing unreachable block (ram,0x44023797) */
/* WARNING: Removing unreachable block (ram,0x4402379f) */
/* WARNING: Removing unreachable block (ram,0x44023721) */
/* WARNING: Removing unreachable block (ram,0x44023738) */
/* WARNING: Removing unreachable block (ram,0x44023759) */
/* WARNING: Removing unreachable block (ram,0x4402373e) */
/* WARNING: Removing unreachable block (ram,0x44023742) */
/* WARNING: Removing unreachable block (ram,0x44023746) */
/* WARNING: Removing unreachable block (ram,0x44023730) */
/* WARNING: Removing unreachable block (ram,0x4402375d) */
/* WARNING: Removing unreachable block (ram,0x44023748) */
/* WARNING: Removing unreachable block (ram,0x44023736) */
/* WARNING: Removing unreachable block (ram,0x44023762) */
/* WARNING: Removing unreachable block (ram,0x44023768) */
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

undefined4 FUN_440236ed(void)

{
  _DAT_84200554 = 0x55;
  _DAT_84200aaa = 0x90;
  _DAT_84200000 = 0xf0;
  return 2;
}
```

### 1.4 Subsistema SD/MMC (`FMU`, `SetBusWidth`, erro `-91006`)

Funções responsáveis pela montagem de cartão SD/MMC, inicialização de clock/estado do controlador SD, OCR/RCA (`CMD8`/`ACMD41`/`CMD2`/`CMD3`), `SetBusWidth` (onde ocorre a falha `-91006` em hardware/barramento SD defeituoso), leitura de `.FVU`/`.FMU` e teste de leitura SD em `BOOT.BIN`.

#### `fmu_sd_dispatcher` (`fmu_sd_dispatcher`) — `0x440065bc` (614 B)

Despachante principal de carga `.FMU`/`.FVU` via SD (`fmu_sd_dispatcher`).

<!-- source: doc_boot_core.txt | FUNC fmu_sd_dispatcher (fmu_sd_dispatcher) @ 0x440065bc (size=614 B, req=0x440065bc) -->
```c
// Callers (2): 0x44004ad9, 0x44004af1
// Referenced strings/literals:
//   0x44034195: 'Conveyer firm boot mode...\n'
//   0x440341b1: 'Clear update data.\n'
//   0x440341c5: 'HDD_MAX_LBA get failed[0x%X].\n'
//   0x440341e4: 'mainsoft data is invalid, so set update flag.\n'
void fmu_sd_dispatcher(char param_1)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  if (param_1 == '\x02') {
    FUN_44006cd9(0,2);
  }
  uVar2 = FUN_4400934f();
  puVar1[-1] = uVar2;
  if ((puVar1[-1] == 0) &&
     (FUN_440093d4(0x402,1,(int)puVar1 + -0x11), *(char *)((int)puVar1 + -0x11) == 'U')) {
    uart0_printf(s_Conveyer_firm_boot_mode____44034195);
    FUN_44006cd9(0,2);
  }
  uVar2 = FUN_44007fa1();
  puVar1[-1] = uVar2;
  puVar1[-9] = 0;
  while ((uint)puVar1[-9] < 0xc) {
    if (puVar1[-9] != 0xb) {
      uVar2 = FUN_44008075(puVar1[-9]);
      puVar1[-2] = uVar2;
      if (puVar1[-2] == 1) break;
    }
    puVar1[-9] = puVar1[-9] + 1;
  }
  uVar2 = FUN_44008040();
  puVar1[-3] = uVar2;
  if ((puVar1[-3] == 1) && (puVar1[-2] == 1)) {
    FUN_44006cd9(0,*(undefined1 *)(puVar1 + 2));
  }
  if ((puVar1[-3] == 1) || (puVar1[-2] == 1)) {
    uart0_printf(s_Clear_update_data__440341b1);
    FUN_44007ea9();
    FUN_44007f1c();
  }
  FUN_44005ed3();
  FUN_44006216();
  FUN_4400629a();
  FUN_44009741(0,1);
  uVar2 = FUN_44009ea7();
  puVar1[-1] = uVar2;
  if (puVar1[-1] == 0) {
    uVar2 = FUN_440068f5(0x10000,puVar1 + -10,puVar1 + -8);
    puVar1[-1] = uVar2;
    if (puVar1[-1] == 1) {
      puVar1[-4] = puVar1[-10];
      uVar2 = FUN_44006822(puVar1[-4]);
      puVar1[-1] = uVar2;
      if (puVar1[-1] == 0) {
        puVar1[-3] = 1;
      }
      else {
        puVar1[-3] = 0;
      }
    }
    else {
      puVar1[-3] = 0;
    }
    if (puVar1[-3] != 1) {
      if ((uint)puVar1[-7] < 0x20000) {
        puVar1[-1] = 3;
      }
      else {
        uVar2 = FUN_440068f5(puVar1[-7] + -0x20000,puVar1 + -10,puVar1 + -8);
        puVar1[-1] = uVar2;
        if (puVar1[-1] == 1) {
          puVar1[-4] = puVar1[-10];
          uVar2 = FUN_44006822(puVar1[-4]);
          puVar1[-1] = uVar2;
        }
        else {
          puVar1[-1] = 3;
        }
        if (puVar1[-1] == 0) {
          FUN_44006ae4(puVar1[-7] + -0x20000,0x10000,puVar1[-8]);
        }
      }
    }
  }
  else {
    puVar1[-7] = 0;
    uart0_printf(s_HDD_MAX_LBA_get_failed_0x_X___440341c5,puVar1[-1]);
    puVar1[-1] = 3;
  }
  if (puVar1[-1] == 0) {
    FUN_440062c4();
    FUN_4400f2d5();
    puVar1[-6] = puVar1[-4];
    (*(code *)puVar1[-6])();
  }
  else {
    uart0_printf(s_mainsoft_data_is_invalid__so_set_440341e4);
    FUN_44007fff();
    FUN_44008027(2);
  }
  FUN_44006cd9(1,*(undefined1 *)(puVar1 + 2));
  return;
}
```

#### `FUN_44007a12` (`sd_fvu_load_7a12`) — `0x44007a12` (403 B)

Carregamento de arquivo `.FVU` / `PROG061` via SD (`sd_fvu_load_7a12`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_44007a12 (sd_fvu_load_7a12) @ 0x44007a12 (size=403 B, req=0x44007a12) -->
```c
// Callers (1): 0x4400732d
// Referenced strings/literals:
//   0x44034835: 'PROG'
//   0x440349a9: 'File ID[0x%X] seek to 0x%X.\n'
//   0x440349dc: 'File ID[0x%X] read to 0x%X(size=0x%X).\n'
//   0x44034a04: 'cheking for fvu program check-sum[0x%X].\n'
//   0x44034a2e: 'Fvu check-sum invalid.\n'
//   0x440349c6: 'Fvu read failed[%d].\n'
//   0x4403498f: 'FAT Seek(%d) failed[%d].\n'
//   0x44034971: 'Not found fvu program in SD.\n'
//   0x44034955: 'Firm list read failed[%d].\n'
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

undefined4 FUN_44007a12(void)

{
  int iVar1;
  undefined4 *puVar2;
  undefined4 uVar3;
  int iVar4;
  undefined4 uStack_8;
  undefined4 uStack_4;
  
  puVar2 = &uStack_8;
  uVar3 = FUN_4400aa5f();
  puVar2[-2] = uVar3;
  if (puVar2[-2] == 1) {
    uVar3 = FUN_4400a733(puVar2[3],*(undefined1 *)(puVar2 + 4),&DAT_44034835,puVar2 + -3,0);
    puVar2[-2] = uVar3;
    if (puVar2[-2] == 1) {
      uVar3 = FUN_4400c94d(_DAT_50501730,*(undefined4 *)(puVar2[-3] + 0xc),1);
      puVar2[-1] = uVar3;
      if (puVar2[-1] == *(int *)(puVar2[-3] + 0xc)) {
        uart0_printf(s_File_ID_0x_X__seek_to_0x_X__440349a9,_DAT_50501730,puVar2[-1]);
        uVar3 = FUN_4400c922(_DAT_50501730,&DAT_93fc0000,*(undefined4 *)(puVar2[-3] + 0x20));
        puVar2[-1] = uVar3;
        if (puVar2[-1] == *(int *)(puVar2[-3] + 0x20)) {
          uart0_printf(s_File_ID_0x_X__read_to_0x_X_size__440349dc,_DAT_50501730,&DAT_93fc0000,
                       puVar2[-1]);
          uart0_printf(s_cheking_for_fvu_program_check_su_44034a04,
                       *(undefined4 *)(puVar2[-3] + 0x24));
          iVar1 = puVar2[-3];
          iVar4 = FUN_4400a831(&DAT_93fc0000,*(undefined4 *)(puVar2[-3] + 0x20));
          if (*(int *)(iVar1 + 0x24) == iVar4) {
            puVar2[-4] = *(undefined4 *)(puVar2[-3] + 0x20);
          }
          else {
            uart0_printf(s_Fvu_check_sum_invalid__44034a2e);
            puVar2[-4] = 0;
          }
        }
        else {
          uart0_printf(s_Fvu_read_failed__d___440349c6,puVar2[-1]);
          puVar2[-4] = 0;
        }
      }
      else {
        uart0_printf(s_FAT_Seek__d__failed__d___4403498f,*(undefined4 *)(puVar2[-3] + 0xc),
                     puVar2[-1]);
        puVar2[-4] = 0;
      }
    }
    else {
      uart0_printf(s_Not_found_fvu_program_in_SD__44034971);
      puVar2[-4] = 0;
    }
  }
  else {
    uart0_printf(s_Firm_list_read_failed__d___44034955,puVar2[-2]);
    puVar2[-4] = 0;
  }
  return puVar2[-4];
}
```

#### `FUN_44007ba5` (`sd_diag_boot_7ba5`) — `0x44007ba5` (180 B)

Boot de diagnóstico via SD (`sd_diag_boot_7ba5`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_44007ba5 (sd_diag_boot_7ba5) @ 0x44007ba5 (size=180 B, req=0x44007ba5) -->
```c
// Callers (1): 0x44006d7d
// Referenced strings/literals:
//   0x44034a46: 'SDDIAG'
//   0x44034a4d: 'SD diag load failed[%d].\n'
//   0x44034a67: 'SD init failed[%d].\n'
//   0x44034a7c: 'diag loader uncompress start...\n'
//   0x44034a9d: 'diag loader uncompress failed...\n'
//   0x44034abf: 'diag loader is preserved with diag loader.\n'
void FUN_44007ba5(void)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  int iVar3;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  FUN_440054aa(300,s_SDDIAG_44034a46);
  uVar2 = FUN_4400a247(0x50480cc8);
  puVar1[-1] = uVar2;
  if (puVar1[-1] == 0) {
    uVar2 = FUN_4400a97c();
    puVar1[-1] = uVar2;
    if (puVar1[-1] != 1) {
      uart0_printf(s_SD_diag_load_failed__d___44034a4d,puVar1[-1]);
      FUN_440079a4();
    }
  }
  else {
    uart0_printf(s_SD_init_failed__d___44034a67,puVar1[-1]);
    iVar3 = cd_diag_boot_mode();
    if (iVar3 == 0) {
      return;
    }
  }
  uart0_printf(s_diag_loader_uncompress_start____44034a7c);
  iVar3 = uncompress(&DAT_93fc0000);
  if (iVar3 != 1) {
    uart0_printf(s_diag_loader_uncompress_failed____44034a9d);
    FUN_440079a4();
  }
  uart0_printf(s_diag_loader_is_preserved_with_di_44034abf);
  return;
}
```

#### `FUN_44007e48` (`sd_card_read_test_7e48`) — `0x44007e48` (97 B)

Teste de leitura do cartão SD (`sd_card_read_test_7e48`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_44007e48 (sd_card_read_test_7e48) @ 0x44007e48 (size=97 B, req=0x44007e48) -->
```c
// Callers (1): 0x44007099
undefined4 FUN_44007e48(undefined4 param_1,undefined4 param_2)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  uVar2 = FUN_4400a247(0x50480cc8,param_2,0xffffffff);
  puVar1[-1] = uVar2;
  if (puVar1[-1] == 0) {
    *(undefined1 *)puVar1[2] = 1;
  }
  if (((*(char *)puVar1[2] != '\x01') && (puVar1[3] == 0)) && (puVar1[4] != 1)) {
    FUN_44007f1c();
    FUN_44007ea9();
    FUN_440079a4();
  }
  return puVar1[-1];
}
```

#### `FUN_4400a247` (`sd_mount_and_read_a247`) — `0x4400a247` (163 B)

Monta o cartão SD e abre fluxo de leitura (`sd_mount_and_read_a247`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4400a247 (sd_mount_and_read_a247) @ 0x4400a247 (size=163 B, req=0x4400a247) -->
```c
// Callers (2): 0x44007bc1, 0x44007e5f
// Referenced strings/literals:
//   0x44035071: 'SD init start...\n'
//   0x44035083: 'SD = 0x%X[LBA]\n'
//   0x44035093: '... SD init end\n'
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

undefined4 FUN_4400a247(undefined4 param_1,undefined4 param_2)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  if (_DAT_5050152c == 0) {
    uart0_printf(s_SD_init_start____44035071,param_2,0);
    FUN_44009e77();
    uVar2 = FUN_44026278();
    puVar1[-1] = uVar2;
    if (puVar1[-1] == 0) {
      FUN_44028348();
      uVar2 = FUN_4402618b(0,puVar1[2]);
      puVar1[-1] = uVar2;
      if (puVar1[-1] == 0) {
        if (*(int *)(puVar1[2] + 8) == 0) {
          puVar1[-1] = 0xfffea06e;
        }
        else {
          uart0_printf(s_SD___0x_X_LBA__44035083,*(undefined4 *)(puVar1[2] + 8));
          _DAT_5050152c = 1;
        }
      }
    }
    FUN_44009e8f();
    uart0_printf(s_____SD_init_end_44035093);
  }
  return puVar1[-1];
}
```

#### `FUN_4400a32d` (`sd_read_file_stream_a32d`) — `0x4400a32d` (216 B)

Leitura em stream de arquivo SD (`sd_read_file_stream_a32d`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4400a32d (sd_read_file_stream_a32d) @ 0x4400a32d (size=216 B, req=0x4400a32d) -->
```c
// Callers (1): 0x4400a4f9
// Referenced strings/literals:
//   0x440350a4: 'Drive connect failed[0x%X].\n'
//   0x440350c1: 'Drive disconnect failed[0x%X].\n'
//   0x440350e1: 'Drive(sleep mode) change failed[0x%X].\n'
undefined4 FUN_4400a32d(int param_1)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  if (param_1 == 1) {
    FUN_44009e77();
    uVar2 = FUN_44018d6d(0);
    puVar1[-1] = uVar2;
    if (puVar1[-1] != 0) {
      uart0_printf(s_Drive_connect_failed_0x_X___440350a4,puVar1[-1]);
    }
    FUN_4400f715(0,1,0);
    FUN_44009e8f();
    FUN_44009835();
    FUN_44009835();
  }
  else {
    FUN_44009835();
    FUN_44009835();
    FUN_4402c348(puVar1 + -4,0,0xc);
    *(undefined1 *)(puVar1 + -4) = 0x1b;
    *(undefined1 *)(puVar1 + -3) = 0x50;
    uVar2 = atapi_issue_pkt(puVar1 + -4,0,0,1);
    puVar1[-5] = uVar2;
    if (puVar1[-5] == 0) {
      uVar2 = FUN_440182bc(0);
      puVar1[-1] = uVar2;
      if (puVar1[-1] != 0) {
        uart0_printf(s_Drive_disconnect_failed_0x_X___440350c1,puVar1[-1]);
      }
    }
    else {
      puVar1[-1] = puVar1[-5];
      uart0_printf(s_Drive_sleep_mode__change_failed__440350e1,puVar1[-1]);
    }
  }
  return puVar1[-1];
}
```

#### `FUN_4400a8c7` (`sd_load_file_a8c7`) — `0x4400a8c7` (181 B)

Carregamento de arquivo nomeado do SD para buffer/RAM (`sd_load_file_a8c7`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4400a8c7 (sd_load_file_a8c7) @ 0x4400a8c7 (size=181 B, req=0x4400a8c7) -->
```c
// Callers (1): 0x440072e3
// Referenced strings/literals:
//   0x44035192: 'FAT Mount failed[%d].\n'
//   0x440351a9: 'FAT Mount ID = 0x%X\n'
//   0x440351be: 'PANA__SD.FRM'
//   0x440351cb: 'FAT Open(%s) failed[%d].\n'
//   0x440351e5: 'FAT Open Old File ID = 0x%X\n'
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

undefined4 FUN_4400a8c7(void)

{
  undefined4 *puVar1;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  _DAT_50501734 = FUN_4400c696();
  if (_DAT_50501734 < 0) {
    FUN_4400b3d9(1,s_FAT_Mount_failed__d___44035192,_DAT_50501734);
    puVar1[-1] = 0;
  }
  else {
    uart0_printf(s_FAT_Mount_ID___0x_X_440351a9,_DAT_50501734);
    _DAT_50501730 = FUN_4400c879(_DAT_50501734,s_PANA__SD_FRM_440351be);
    if (_DAT_50501730 < 0) {
      FUN_4400b3d9(1,s_FAT_Open__s__failed__d___440351cb,s_PANA__SD_FRM_440351be,_DAT_50501730);
      puVar1[-1] = 0;
    }
    else {
      uart0_printf(s_FAT_Open_Old_File_ID___0x_X_440351e5,_DAT_50501730);
      puVar1[-1] = 1;
    }
  }
  return puVar1[-1];
}
```

#### `FUN_4400a97c` (`sd_diag_load_a97c`) — `0x4400a97c` (227 B)

Carga de binário de diagnóstico a partir do SD (`sd_diag_load_a97c`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4400a97c (sd_diag_load_a97c) @ 0x4400a97c (size=227 B, req=0x4400a97c) -->
```c
// Callers (1): 0x44007bd2
// Referenced strings/literals:
//   0x44035192: 'FAT Mount failed[%d].\n'
//   0x44035202: 'DIGADIAG.FRM'
//   0x440351cb: 'FAT Open(%s) failed[%d].\n'
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

undefined4 FUN_4400a97c(void)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  _DAT_50501734 = FUN_4400c696();
  if (_DAT_50501734 < 0) {
    FUN_4400b3d9(1,s_FAT_Mount_failed__d___44035192,_DAT_50501734);
    puVar1[-2] = 0;
  }
  else {
    _DAT_50501730 = FUN_4400c879(_DAT_50501734,s_DIGADIAG_FRM_44035202);
    if (_DAT_50501730 < 0) {
      FUN_4400b3d9(1,s_FAT_Open__s__failed__d___440351cb,s_DIGADIAG_FRM_44035202,_DAT_50501730);
      puVar1[-2] = 0;
    }
    else {
      uVar2 = FUN_4400c94d(_DAT_50501730,0,1);
      puVar1[-1] = uVar2;
      if (puVar1[-1] == 0) {
        uVar2 = FUN_4400c922(_DAT_50501730,&DAT_93fc0000,0x20000);
        puVar1[-1] = uVar2;
        if (puVar1[-1] == 0x20000) {
          puVar1[-2] = 1;
        }
        else {
          puVar1[-2] = 0;
        }
      }
      else {
        puVar1[-2] = 0;
      }
    }
  }
  return puVar1[-2];
}
```

#### `FUN_4400aa5f` (`sd_read_firm_list_aa5f`) — `0x4400aa5f` (99 B)

Leitura de arquivo lista `.FMU` / firmware list no SD (`sd_read_firm_list_aa5f`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4400aa5f (sd_read_firm_list_aa5f) @ 0x4400aa5f (size=99 B, req=0x4400aa5f) -->
```c
// Callers (1): 0x44007a23
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

undefined4 FUN_4400aa5f(void)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  uVar2 = FUN_4400c94d(_DAT_50501730,0,1);
  puVar1[-1] = uVar2;
  if (puVar1[-1] == 0) {
    uVar2 = FUN_4400c922(_DAT_50501730,0x5050175c,0x2000);
    puVar1[-1] = uVar2;
    if (puVar1[-1] == 0x2000) {
      puVar1[-2] = 1;
    }
    else {
      puVar1[-2] = 0;
    }
  }
  else {
    puVar1[-2] = 0;
  }
  return puVar1[-2];
}
```

#### `FUN_4402618b` (`sd_get_info_2618b`) — `0x4402618b` (88 B)

Consulta informações e estado atual do controlador SD (`sd_get_info_2618b`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4402618b (sd_get_info_2618b) @ 0x4402618b (size=88 B, req=0x4402618b) -->
```c
// Callers (2): 0x4400a28f, 0x440280dc
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

int FUN_4402618b(ushort param_1,undefined4 *param_2)

{
  int iVar1;
  
  iVar1 = -0x15f97;
  param_1 = param_1 & 0xff;
  if (param_1 == 0) {
    iVar1 = FUN_44026626();
    if (iVar1 == -0x15f92) {
      iVar1 = 0;
    }
    else if ((iVar1 != 0) && (iVar1 != -0x15f91)) {
      return iVar1;
    }
    *param_2 = _DAT_50574914;
    param_2[1] = _DAT_50574918;
    param_2[2] = _DAT_5057491c;
    *(ushort *)(param_2 + 3) = param_1;
    *(ushort *)((int)param_2 + 0xe) = param_1;
    *(ushort *)(param_2 + 4) = param_1;
  }
  return iVar1;
}
```

#### `FUN_44026278` (`sd_slot_init_26278`) — `0x44026278` (83 B)

Inicialização básica de slot SD (`sd_slot_init_26278`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_44026278 (sd_slot_init_26278) @ 0x44026278 (size=83 B, req=0x44026278) -->
```c
// Callers (2): 0x4400a273, 0x44010e49
undefined4 FUN_44026278(void)

{
  uint uVar1;
  
  uVar1 = (uint)DAT_505748e0;
  if (uVar1 == 1) {
    FUN_440264a6(0,4);
    FUN_44026549(0);
    FUN_4401d3b8(0x1000,0x180000,uVar1);
  }
  else {
    FUN_440264a6(0,4);
    FUN_44026549(0);
    DAT_505748e0 = 1;
  }
  return 0;
}
```

#### `FUN_440264a6` (`sd_set_state_264a6`) — `0x440264a6` (163 B)

Configuração de máquina de estados SD (`sd_set_state_264a6`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_440264a6 (sd_set_state_264a6) @ 0x440264a6 (size=163 B, req=0x440264a6) -->
```c
// Callers (7): 0x4402628a, 0x440262b0, 0x44026c16, 0x44026c2d, 0x44026ccf, 0x44028054, 0x44028076
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

char FUN_440264a6(undefined4 param_1,undefined4 param_2)

{
  char cVar1;
  
  _DAT_505748e4 = param_1;
  if (DAT_505748e0 == '\0') {
    FUN_44026496();
  }
  _DAT_505748e8 = 9;
  _DAT_505748ec = 9;
  _DAT_505748f0 = 0;
  DAT_505748f2 = 1;
  DAT_505748f3 = 1;
  DAT_505748fc = 1;
  DAT_505748fd = 1;
  DAT_50574900 = 0;
  _DAT_50574904 = 0;
  cVar1 = DAT_50574910;
  if (DAT_50574910 == '\0') {
    cVar1 = '\x01';
    _DAT_50574918 = 1;
  }
  _DAT_50574914 = param_2;
  _DAT_5057491c = 0;
  _DAT_50574920 = 0;
  _DAT_505748f8 = 0;
  return cVar1;
}
```

#### `FUN_44026549` (`sd_clock_cfg_26549`) — `0x44026549` (221 B)

Configuração de divisor/clock SD (`sd_clock_cfg_26549`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_44026549 (sd_clock_cfg_26549) @ 0x44026549 (size=221 B, req=0x44026549) -->
```c
// Callers (8): 0x44026292, 0x440262b8, 0x44026c1d, 0x44026c33, 0x44026cd6, 0x44028092, 0x44028102, 0x44028243
void FUN_44026549(uint param_1)

{
  ushort uVar1;
  undefined4 uVar2;
  
  param_1 = param_1 & 0xff;
  if (param_1 < 2) {
    FUN_4401d0a9(0x8000);
    uVar1 = FUN_4401c122(0x890019c0);
    FUN_4401c11c(0x890019c0,uVar1 | 7);
  }
  FUN_4401c122(0x89001810);
  FUN_4401c11c(0x89001810);
  FUN_4401c122(0x89001848);
  FUN_4401c11c(0x89001848);
  FUN_4401c11c(0x89001850,0x80b8);
  FUN_4401c11c(0x89001a10,0);
  FUN_4401c11c(0x89001a18,1);
  if (param_1 == 0) {
    FUN_4401c11c(0x8900184c,0x200);
    uVar2 = 0x12;
    FUN_44015cb7(7,3,0x12);
    FUN_44015cb7(3,7,uVar2);
  }
  return;
}
```

#### `FUN_4402724f` (`sd_send_ocr_rca_2724f`) — `0x4402724f` (776 B)

Negociação de OCR e obtenção de RCA do cartão SD (`sd_send_ocr_rca_2724f`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4402724f (sd_send_ocr_rca_2724f) @ 0x4402724f (size=776 B, req=0x4402724f) -->
```c
// Callers (1): 0x4402803b
// Referenced strings/literals:
//   0x44039015: '[SDCD]SendOCR:RANG OUT \n'
//   0x4403904a: '[SDCD]SendOCR:TIMEOUT\n'
//   0x4403902e: '[SDCD]SendOCR:Card removed\n'
//   0x44039061: "[SDCD]CMD2 command(ALL_SEND_CID) can't be sent \n"
//   0x440390ee: "[SDCD]CMD3 command(SEND_RCA) can't be sent \n"
//   0x44039092: '[SDCD]CardSetRCA:RCA data=[%04x]\n'
//   0x440390cf: '[SDCD]Set RCA failed ERROR:%d\n'
//   0x440390b4: '[SDCD]CMD3 Get RCA Retry \n'
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

int FUN_4402724f(void)

{
  uint uVar1;
  int iVar2;
  int iVar3;
  uint uVar4;
  
  uVar1 = 0;
LAB_44027256:
  FUN_4401c11c(0x89001848,0x140);
  delay_ms(10);
  FUN_440271a1(0,0);
  _DAT_505748ec = 0;
  _DAT_505748e8 = 0;
  delay_ms(10);
  iVar2 = FUN_440271fc();
  if (iVar2 == -0x1637c) {
    return -0x1637c;
  }
  if (_DAT_505748f8 == 0) {
    uVar4 = _DAT_505748f8;
    delay_ms(10);
    FUN_440271a1(0,0);
    _DAT_505748e8 = uVar4;
    _DAT_505748ec = uVar4;
    delay_ms(10);
    do {
      if (_DAT_50574914 == 2) {
        iVar2 = FUN_440271a1(0x37,0);
        if (iVar2 == 0) {
          iVar2 = FUN_440271a1(0x69,0x100000);
          if (iVar2 == -0x1637c) {
            iVar2 = -0x1637c;
            goto LAB_440273e4;
          }
        }
        else {
          if (iVar2 == -0x1637c) {
            iVar2 = -0x1637c;
            goto LAB_440273e4;
          }
          if ((iVar2 == -0x1637d) || (uVar4 = uVar4 + 99, iVar2 == -0x1637b)) {
            uVar4 = 0;
            _DAT_50574914 = 3;
            _DAT_505748f8 = 3;
            _DAT_505748f0 = 1;
            FUN_440271a1(0,0);
          }
        }
      }
      else {
        iVar2 = FUN_440271a1(1,0xff0000);
        if (iVar2 == -0x1637c) {
          iVar2 = -0x1637c;
          goto LAB_44027456;
        }
        if (iVar2 != 0) {
          uVar4 = uVar4 + 99;
        }
      }
      if ((iVar2 == 0) && ((_DAT_50574926 & 0x8000) != 0)) {
        if ((_DAT_50574926 & 0xff) == 0xff) {
          _DAT_505748e8 = 1;
          iVar2 = 0;
        }
        else {
          uart0_printf(s__SDCD_SendOCR_RANG_OUT_44039015);
          iVar2 = -0x1637e;
        }
        goto LAB_44027456;
      }
      delay_ms(10);
      uVar4 = uVar4 + 1;
    } while ((uVar4 & 0xffff) < 300);
LAB_44027443:
    uart0_printf(s__SDCD_SendOCR_TIMEOUT_4403904a);
    iVar2 = -0x1637e;
    goto LAB_44027456;
  }
  iVar2 = -0x1637e;
  if (_DAT_505748f8 == 2) {
    uVar4 = 0;
    do {
      iVar2 = FUN_440271a1(0x37,0);
      if (iVar2 == 0) {
        iVar2 = FUN_440271a1(0x69,0x40100000);
        if (iVar2 == -0x1637c) goto LAB_440273e4;
      }
      else {
        if (iVar2 == -0x1637c) goto LAB_440273e4;
        if (iVar2 == -0x1637b) {
          _DAT_50574914 = 4;
          goto LAB_44027456;
        }
      }
      if ((iVar2 == 0) && ((_DAT_50574926 & 0x8000) != 0)) {
        if ((_DAT_50574926 & 0x4000) == 0) {
          _DAT_505748f8 = 1;
        }
        else {
          _DAT_505748f8 = 2;
        }
        _DAT_505748e8 = 1;
        iVar2 = 0;
        goto LAB_44027456;
      }
      delay_ms(10);
      uVar4 = uVar4 + 1;
    } while ((uVar4 & 0xffff) < 300);
    goto LAB_44027443;
  }
  goto LAB_4402749a;
LAB_440273e4:
  uart0_printf(s__SDCD_SendOCR_Card_removed_4403902e);
LAB_44027456:
  if (iVar2 == -0x1637c) {
    return -0x1637c;
  }
  if (iVar2 == 0) {
    iVar2 = FUN_44026ce4(1);
  }
  if (iVar2 == 0) {
    iVar2 = FUN_440271a1(2,0);
    _DAT_505748e8 = 2;
    if (iVar2 == -0x1637c) {
      _DAT_505748e8 = 2;
      return -0x1637c;
    }
    if (iVar2 == 0) {
      iVar2 = FUN_44026ce4();
    }
  }
  else {
LAB_4402749a:
    uart0_printf(s__SDCD_CMD2_command_ALL_SEND_CID__44039061);
  }
  if (iVar2 != 0) {
    uart0_printf(s__SDCD_CMD3_command_SEND_RCA__can_440390ee);
    return iVar2;
  }
  iVar2 = 0;
  do {
    iVar3 = FUN_440271a1(3,0x10000);
    if (iVar3 == 0) {
      _DAT_505748e8 = 3;
      if (_DAT_505748f0 != 0) break;
    }
    else if (iVar3 == -0x1637c) goto LAB_44027507;
    delay_ms(10);
    iVar2 = iVar2 + 1;
  } while (iVar2 < 10);
  if ((_DAT_505748f0 == 0) && (iVar3 == 0)) {
    uart0_printf(s__SDCD_CardSetRCA_RCA_data___04x__44039092,0);
  }
LAB_44027507:
  if (iVar3 == -0x1637c) {
    return -0x1637c;
  }
  if ((iVar3 == 0) || (2 < uVar1)) {
    if (iVar3 == 0) {
      iVar2 = FUN_44026ce4(3);
      return iVar2;
    }
    uart0_printf(s__SDCD_Set_RCA_failed_ERROR__d_440390cf);
    return iVar3;
  }
  uVar1 = uVar1 + 1;
  uart0_printf(s__SDCD_CMD3_Get_RCA_Retry_440390b4);
  goto LAB_44027256;
}
```

#### `GetSD_Status` (`GetSD_Status_27826`) — `0x44027826` (244 B)

Obtém registrador de status SD (`ACMD13` / `GetSD_Status_27826`).

<!-- source: doc_boot_media_crypto.txt | FUNC GetSD_Status (GetSD_Status_27826) @ 0x44027826 (size=244 B, req=0x44027826) -->
```c
// Callers (2): 0x44027d87, 0x44027f93
// Referenced strings/literals:
//   0x440391ea: '[SDCD]GetSD_Status:CMD55 send ERROR %d\n'
//   0x44039212: '[SDCD]GetSD_Status:ACMD13 send ERROR %d\n'
//   0x4403923b: "[SDCD]This SD Card is't memory card \n"
//   0x44039261: '[SDCD]GetSD_Status:Data read ERROR:%d\n'
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

int GetSD_Status(void)

{
  int iVar1;
  char *__format;
  byte abStack_88 [2];
  char cStack_86;
  undefined1 uStack_85;
  byte bStack_84;
  byte bStack_83;
  byte bStack_82;
  byte bStack_81;
  int iStack_10;
  undefined4 uStack_c;
  undefined4 uStack_8;
  undefined4 uStack_4;
  
  iVar1 = FUN_440275e0(0x40);
  if (iVar1 != 0) {
    return iVar1;
  }
  iVar1 = FUN_440271a1(0x37,(uint)_DAT_505748f0 << 0x10);
  __format = s__SDCD_GetSD_Status_CMD55_send_ER_440391ea;
  if (iVar1 == 0) {
    iVar1 = FUN_440271a1(0x4d,0);
    __format = s__SDCD_GetSD_Status_ACMD13_send_E_44039212;
    if (iVar1 == 0) goto LAB_44027871;
  }
  uart0_printf(__format,iVar1);
LAB_44027871:
  if (iVar1 == 0) {
    iVar1 = FUN_4402772c(abStack_88,0x40);
    uStack_c._2_2_ =
         (ushort)(((abStack_88[0] & 0x20) << 0x18) >> 0x10) |
         (ushort)(((uint)(abStack_88[0] >> 6) << 0x1e) >> 0x10) | uStack_c._2_2_ & 0x1fff;
    uStack_c = CONCAT22(uStack_c._2_2_,CONCAT11(cStack_86,uStack_85));
    iStack_10 = (uint)bStack_84 * 0x1000000 + (uint)bStack_83 * 0x10000 + (uint)bStack_82 * 0x100 +
                (uint)bStack_81;
    if (iVar1 == 0) {
      if (cStack_86 == '\0') {
        _DAT_50574914 = 2;
      }
      else {
        _DAT_50574914 = 4;
        iVar1 = -0x1637e;
        uart0_printf(s__SDCD_This_SD_Card_is_t_memory_c_4403923b);
      }
      DAT_505748f2 = (byte)((uint)uStack_c >> 0x1e);
    }
    else {
      uart0_printf(s__SDCD_GetSD_Status_Data_read_ERR_44039261,iVar1);
    }
  }
  return iVar1;
}
```

#### `FUN_4402791a` (`SetBusWidth_2791a`) — `0x4402791a` (1774 B)

Comutação de largura de barramento 1-bit/4-bit (`SetBusWidth_2791a`, origem das strings `SetBusWidth` e código `-91006`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4402791a (SetBusWidth_2791a) @ 0x4402791a (size=1774 B, req=0x4402791a) -->
```c
// Callers (1): 0x4402805d
// Referenced strings/literals:
//   0x440392c1: '[SDCD] CMD9_GETCSD err= %d\n'
//   0x440392dd: '[SDCD] CMD7 SELECT err = %d\n'
//   0x440392fa: '[SDCD] ACMD51 SEND SCR Eject\n'
//   0x44039318: '[SDCD] ACMD13 SD Status err=%d\n'
//   0x44039338: '[SDCD]GetSCR:CMD55 send ERROR %d\n'
//   0x4403935a: '[SDCD]GetSCR:ACMD13 send ERROR %d\n'
//   0x4403937d: '[SDCD]GetSCR:Data Read ERROR %d\n'
//   0x4403939e: '[SDCD] Get SCR fail ERROR:%d\n'
//   0x440393bc: '[SDCD]SetBusWidth:ACMD6 send ERROR %d\n'
//   0x440393e3: '[SDCD]SetBusWidth:CMD55 send ERROR %d\n'
//   0x4403940a: '[SDCD]SetBusWidth:Setting ERROR %d\n'
//   0x4403942e: '[SDCD]SetBusWidth:SD Status ERROR %d\n'
//   0x44039454: '[SDCD] ACMD6_SET_BUS_WIDTH Eject\n'
//   0x44039476: '[SDCD] Set SD Bus Width fail ERROR:%d\n'
//   0x4403949d: '[SDCD] CMD16 SET BLOCKLEN err=%d\n'
//   0x44039288: '[SDCD] CMD10 GETCID Eject \n'
//   0x440392a4: '[SDCD] CMD10_GETCID err= %d\n'
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

int FUN_4402791a(void)

{
  undefined4 uVar1;
  uint uVar2;
  uint uVar3;
  ushort uVar4;
  int iVar5;
  char *pcVar6;
  uint uVar7;
  uint uVar8;
  int iVar9;
  uint uVar10;
  uint uVar11;
  uint uVar12;
  uint uVar13;
  uint uVar14;
  uint uVar15;
  undefined1 uStack_3c;
  byte bStack_3b;
  uint uStack_34;
  uint uStack_30;
  uint uStack_2c;
  uint uStack_28;
  undefined4 uStack_24;
  undefined4 uStack_20;
  undefined4 uStack_1c;
  undefined4 uStack_18;
  undefined4 uStack_14;
  
  iVar5 = FUN_440271a1(10,(uint)_DAT_505748f0 << 0x10);
  if (iVar5 == 0) {
LAB_44027953:
    iVar5 = FUN_440271a1(9,(uint)_DAT_505748f0 << 0x10);
    if (iVar5 == 0) {
      uVar10 = (uint)_DAT_50574924;
      uVar3 = uVar10 & 0xc;
      uVar8 = uVar10 & 0x10;
      uVar13 = uVar10 & 0x20;
      uVar12 = uVar10 & 0x40;
      uVar15 = uVar10 & 0x80;
      uVar10 = uVar10 & 0x2000;
      uVar14 = (uint)_DAT_50574926;
      uVar11 = ((uint)(_DAT_50574924 >> 0xe) + (uVar14 & 3) * 4) * 0x4000;
      uVar7 = (uVar14 & 0x1c) << 0x10;
      if (_DAT_50574914 == 2) {
        uStack_30 = (_DAT_50574928 & 0x3f) * 2 + (uint)(_DAT_50574926 >> 0xf) |
                    uStack_30 & 0xffffff00 | (uint)_DAT_50574928 * 2 & 0x80;
        uVar2 = (uVar14 & 0x7f00) << 0x10 |
                (uVar14 & 0x80) << 0x10 |
                uVar10 | uVar12 | uVar13 | uVar8 | uStack_34 & 0x80601f03 | uVar3 | uVar15 | uVar11
                | uVar7;
      }
      else {
        uVar2 = uVar10 | uVar12 | uVar13 | uVar8 | uStack_34 & 0xffe01f03 | uVar3 | uVar15 | uVar11
                | uVar7;
        if (_DAT_50574914 == 3) {
          uStack_30 = _DAT_50574928 >> 2 & 0x1f | uStack_30 & 0xffffff00;
          uVar2 = uVar14 << 0x10 & 0x1f000000 |
                  uVar14 << 0x10 & 0x800000 |
                  (uVar14 & 0x60) << 0x10 |
                  uVar10 | uVar12 | uVar13 | uVar8 | uStack_34 & 0x80001f03 | uVar3 | uVar15 |
                  uVar11 | uVar7;
        }
      }
      uStack_34 = uVar2;
      uStack_24._2_2_ = _DAT_50574932 & 0xc0 | (ushort)((uint)uStack_24 >> 0x10) & 0xff3f;
      if ((_DAT_50574932 & 0xc0) == 0) {
        uVar3 = (uint)_DAT_5057492c;
        uStack_2c = (uint)(_DAT_5057492c >> 6) << 0x1f |
                    (uVar3 & 0x20) << 0x19 |
                    (uVar3 & 0x10) << 0x19 |
                    ((uint)(_DAT_5057492a >> 6) + (uVar3 & 3) * 0x400) * 0x8000 |
                    (_DAT_5057492a & 3) << 9 |
                    (uint)(_DAT_50574928 >> 0xd) << 6 |
                    _DAT_50574928 >> 7 & 0x38 | _DAT_50574928 >> 7 & 7 | uStack_2c & 0x18000000 |
                    (_DAT_5057492a & 0x38) << 9;
      }
      else {
        uStack_2c = (uint)(_DAT_5057492c >> 6) << 0x1f |
                    (_DAT_5057492c & 0x20) << 0x19 |
                    (_DAT_5057492c & 0x10) << 0x19 |
                    ((uint)(_DAT_50574928 >> 8) + (_DAT_5057492a & 0x3fff) * 0x100) * 2 |
                    uStack_2c & 0x1f800001;
      }
      uVar3 = _DAT_5057492c >> 7 & 0x1e;
      uStack_28 = (uint)(_DAT_5057492e >> 8) << 0x11 |
                  ((uint)(_DAT_5057492c >> 0xc) + (_DAT_5057492e & 0xff) * 0x10) * 0x20 |
                  uVar3 | _DAT_5057492c >> 7 & 1 | uStack_28 & 0xfe000000;
      uStack_24 = CONCAT22(uStack_24._2_2_,_DAT_50574930);
      uVar1 = uStack_24;
      if (_DAT_505748f8 < 2) {
        if ((uStack_24._2_1_ >> 6 != 0) ||
           (iVar9 = ((uStack_2c >> 0xf & 0xfff) + 1 << (uStack_2c & 7) + 2) << (uVar3 >> 1),
           _DAT_5057491c =
                (uint)((((ulonglong)(uint)(iVar9 >> 0x1f) << 0x20) + (longlong)iVar9) / 0x200),
           0x3f6004 < _DAT_5057491c)) {
LAB_44027c76:
          iVar5 = -0x16380;
          _DAT_50574914 = 4;
        }
      }
      else if (_DAT_505748f8 == 2) {
        if (((uStack_24._2_1_ >> 6 != 1) ||
            (_DAT_5057491c = (uStack_2c >> 1) * 0x400 + 0x400, _DAT_5057491c < 0x3f6004)) ||
           (0x3fd8000 < _DAT_5057491c)) goto LAB_44027c76;
      }
      else {
        iVar9 = ((uStack_2c >> 0xf & 0xfff) + 1 << (uStack_2c & 7) + 2) << (uVar3 >> 1);
        _DAT_5057491c =
             (uint)((((ulonglong)(uint)(iVar9 >> 0x1f) << 0x20) + (longlong)iVar9) / 0x200);
      }
      DAT_505748fe = (undefined1)(uStack_28 >> 0x11);
      iVar9 = 1 << (uVar3 >> 1);
      DAT_505748fc = (undefined1)
                     ((((ulonglong)(uint)(iVar9 >> 0x1f) << 0x20) + (longlong)iVar9) / 0x200);
      iVar9 = 1 << (uStack_34 >> 0xe & 0xf);
      DAT_505748fd = (undefined1)
                     ((((ulonglong)(uint)(iVar9 >> 0x1f) << 0x20) + (longlong)iVar9) / 0x200);
      uStack_24 = uVar1;
    }
    pcVar6 = s__SDCD__CMD9_GETCSD_err___d_440392c1;
    if (iVar5 == 0) {
      uVar15 = 1;
      uVar3 = 0;
      uVar8 = FUN_440276dc();
      uVar12 = 0;
      do {
        if (20000 >> (uVar12 & 0x1f) <= (int)(uVar8 & 0xffff)) {
          uVar15 = 0;
          if ((uVar12 & 0xffff) != 0) {
            uVar15 = 1 << ((uVar12 & 0xffff) - 1 & 0x1f);
          }
          uVar3 = 1;
          break;
        }
        uVar12 = uVar12 + 1;
      } while ((uVar12 & 0xffff) < 8);
      if ((uVar3 & 0xffff) == 1) {
        uVar3 = uVar15 & 0xffff | 0x100;
      }
      else {
        uVar3 = 0x140;
      }
      FUN_4401c11c(0x89001848,uVar3);
      delay_ms(10);
      _DAT_505748ec = 4;
      iVar5 = FUN_44027600(0x32);
      pcVar6 = s__SDCD__CMD7_SELECT_err____d_440392dd;
      if (iVar5 == 0) {
        if (_DAT_50574914 == 2) {
          iVar5 = GetSD_Status();
          if (iVar5 != 0) {
            if (iVar5 == -0x1637c) {
              uart0_printf(s__SDCD__ACMD51_SEND_SCR_Eject_440392fa);
            }
            pcVar6 = s__SDCD__ACMD13_SD_Status_err__d_44039318;
            goto LAB_44027ffc;
          }
          iVar5 = FUN_440275e0(8);
          if (iVar5 == 0) {
            iVar5 = FUN_440271a1(0x37,(uint)_DAT_505748f0 << 0x10);
            pcVar6 = s__SDCD_GetSCR_CMD55_send_ERROR__d_44039338;
            if (iVar5 == 0) {
              iVar5 = FUN_440271a1(0x73,0);
              pcVar6 = s__SDCD_GetSCR_ACMD13_send_ERROR___4403935a;
              if (iVar5 != 0) goto LAB_44027dec;
            }
            else {
LAB_44027dec:
              uart0_printf(pcVar6,iVar5);
            }
            if (iVar5 != 0) goto LAB_44027e96;
            iVar5 = FUN_4402772c(&uStack_3c,8);
            if (iVar5 != 0) {
              uart0_printf(s__SDCD_GetSCR_Data_Read_ERROR__d_4403937d,iVar5);
              goto LAB_44027e96;
            }
            _DAT_505748f3 = CONCAT12((char)(_DAT_505748f3 >> 0x10),(ushort)bStack_3b) & 0xff000f;
          }
          else {
LAB_44027e96:
            if (iVar5 != 0) {
              _DAT_505748f3 = CONCAT21(_DAT_505748f4,1);
            }
          }
          if (_DAT_505748f8 == 0) {
            if (1 < _DAT_505748f4) goto LAB_44027ec7;
          }
          else if ((_DAT_505748f8 < 3) && (DAT_505748f4 < 2)) {
LAB_44027ec7:
            iVar5 = -0x16380;
            _DAT_50574914 = 4;
          }
          if (iVar5 == -0x1637c) {
            pcVar6 = s__SDCD__ACMD51_SEND_SCR_Eject_440392fa;
            iVar5 = -0x1637c;
            goto LAB_44027ee3;
          }
          if ((iVar5 != 0) &&
             (uart0_printf(s__SDCD__Get_SCR_fail_ERROR__d_4403939e,iVar5), iVar5 == -0x16380)) {
            return -0x16380;
          }
          uVar3 = (uint)DAT_505748f3;
          iVar5 = FUN_440271a1(0x37,(uint)_DAT_505748f0 << 0x10);
          if (iVar5 == 0) {
            uVar3 = 0;
            iVar5 = FUN_440271a1(0x46);
            pcVar6 = s__SDCD_SetBusWidth_ACMD6_send_ERR_440393bc;
            if (iVar5 != 0) goto LAB_44027f54;
          }
          else {
            pcVar6 = s__SDCD_SetBusWidth_CMD55_send_ERR_440393e3;
LAB_44027f54:
            uart0_printf(pcVar6,iVar5);
          }
          if (iVar5 == 0) {
            uVar4 = FUN_4401c122(0x89001850);
            uVar3 = uVar3 & 0xff;
            if (uVar3 == 2) {
              uVar4 = uVar4 & 0x7fff;
            }
            else {
              uVar4 = uVar4 | 0x8000;
            }
            FUN_4401c11c(0x89001850,uVar4);
            iVar5 = GetSD_Status();
            if (iVar5 == 0) {
              if (uVar3 == DAT_505748f2) goto LAB_44027fc2;
              iVar5 = -0x1637e;
              pcVar6 = s__SDCD_SetBusWidth_Setting_ERROR___4403940a;
            }
            else {
              pcVar6 = s__SDCD_SetBusWidth_SD_Status_ERRO_4403942e;
            }
            uart0_printf(pcVar6,iVar5);
          }
LAB_44027fc2:
          pcVar6 = s__SDCD__ACMD6_SET_BUS_WIDTH_Eject_44039454;
          if (iVar5 == -0x1637c) goto LAB_44027ee3;
          if (iVar5 != 0) {
            uart0_printf(s__SDCD__Set_SD_Bus_Width_fail_ERR_44039476,iVar5);
          }
          if (iVar5 != 0) {
            return iVar5;
          }
        }
        iVar5 = FUN_440275e0(0x200);
        if (iVar5 == 0) {
          return 0;
        }
        pcVar6 = s__SDCD__CMD16_SET_BLOCKLEN_err__d_4403949d;
      }
    }
LAB_44027ffc:
    uart0_printf(pcVar6,iVar5);
  }
  else {
    pcVar6 = s__SDCD__CMD10_GETCID_Eject_44039288;
    if (iVar5 != -0x1637c) {
      uart0_printf(s__SDCD__CMD10_GETCID_err___d_440392a4,iVar5);
      goto LAB_44027953;
    }
    iVar5 = -0x1637c;
LAB_44027ee3:
    uart0_printf(pcVar6);
  }
  return iVar5;
}
```

#### `FUN_44028008` (`sd_init_entry_28008`) — `0x44028008` (492 B)

Ponto de entrada da inicialização do controlador SD (`sd_init_entry_28008`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_44028008 (sd_init_entry_28008) @ 0x44028008 (size=492 B, req=0x44028008) -->
```c
// Callers (2): 0x44028248, 0x4402838b
// Referenced strings/literals:
//   0x440394bf: '[SDCD] === This Card removed ==== \n\n'
//   0x440394e4: "[SDCD] === This Card can't be controled ==== \n\n"
//   0x44039514: '[SDCD] === This Card Spec ==== \n'
//   0x44039535: '\tCard type:%d(condition:%d) wp:%d\n'
//   0x44039558: '\tcapacity:%x[LBA](%dByte) \t transfer speed:%d[Kbps]\n'
//   0x4403958d: '\tRCA:%x \t Bus Width:%d(current) %d(support)\n'
//   0x440395ba: '\tMulti Max Block Len Read:%d[SECT] / Write:%d[SECT]\n'
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_44028008(void)

{
  uint uVar1;
  int iVar2;
  char *__format;
  uint uVar3;
  undefined1 *puVar4;
  uint uVar5;
  undefined1 auStack_20 [20];
  undefined4 uStack_c;
  undefined4 uStack_8;
  undefined4 uStack_4;
  
  iVar2 = FUN_44026b43();
  if (iVar2 == 1) {
    if (DAT_50574910 == '\0') {
      FUN_44026ae3();
    }
    if (_DAT_505748f5 < 4) {
      do {
        _DAT_505748e4 = 2;
        iVar2 = FUN_4402724f();
        if (iVar2 != 0) {
          if (iVar2 == -0x1637c) {
            return;
          }
          FUN_440264a6(3,4);
        }
        if ((iVar2 == 0) && (iVar2 = FUN_4402791a(), iVar2 != 0)) {
          if (iVar2 == -0x1637c) {
            return;
          }
          FUN_440264a6(3,4);
        }
        if (iVar2 != -0x1637f) break;
        _DAT_505748f5 = CONCAT11(DAT_505748f6,DAT_505748f5 + '\x01');
        FUN_44026549(1);
      } while (DAT_505748f5 < 4);
    }
    FUN_4401c122(0x89001848);
    FUN_4401c11c(0x89001848);
    _DAT_505748e4 = 3;
    if (_DAT_50574914 == 2 || _DAT_50574914 == 3) {
      puVar4 = auStack_20;
      iVar2 = FUN_4402618b(0,puVar4);
      if (((iVar2 == 0) && (_DAT_505748d8 != (code *)0x0)) && (DAT_50574910 == '\0')) {
        (*_DAT_505748d8)(puVar4);
      }
    }
    else {
      FUN_44026549(1);
      FUN_4401d3b8(0x1000,0x180000,1);
    }
  }
  if (_DAT_50574914 == 4) {
    if (_DAT_505748e4 == 0) {
      __format = s__SDCD______This_Card_removed_____440394bf;
    }
    else {
      __format = s__SDCD______This_Card_can_t_be_co_440394e4;
    }
    uart0_printf(__format);
  }
  else {
    uVar3 = FUN_440276dc();
    uVar1 = 1;
    if (_DAT_505748f2 == 2) {
      uVar1 = 4;
    }
    uVar5 = 1;
    if ((_DAT_505748f2 & 0x400) != 0) {
      uVar5 = 4;
    }
    uart0_printf(s__SDCD______This_Card_Spec______44039514);
    uart0_printf(s_Card_type__d_condition__d__wp__d_44039535,_DAT_50574914,_DAT_505748e4,
                 _DAT_50574918);
    uart0_printf(s_capacity__x_LBA___dByte__transfe_44039558,_DAT_5057491c,_DAT_5057491c << 9,
                 uVar3 & 0xffff);
    uart0_printf(s_RCA__x_Bus_Width__d_current___d__4403958d,(uint)_DAT_505748f0,uVar1 & 0xff,
                 uVar5 & 0xff);
    uart0_printf(s_Multi_Max_Block_Len_Read__d_SECT_440395ba,(uint)DAT_505748fc,(uint)DAT_505748fd);
  }
  return;
}
```

#### `FUN_440281f4` (`sd_card_mount_281f4`) — `0x440281f4` (99 B)

Montagem de volume/sistema de arquivos no SD (`sd_card_mount_281f4`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_440281f4 (sd_card_mount_281f4) @ 0x440281f4 (size=99 B, req=0x440281f4) -->
```c
// Callers (4): 0x440282b3, 0x440282c3, 0x4402832a, 0x4402833a
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_440281f4(int param_1)

{
  undefined4 uVar1;
  
  FUN_44027557();
  uVar1 = 0x18;
  if ((1 < (param_1 - 0x11U & 0xffff)) && ((param_1 - 0x18U & 0xffff) < 2)) {
    uVar1 = 0x19;
  }
  FUN_440145d8(uVar1);
  if (DAT_505748ff < 3) {
    DAT_505748ff = DAT_505748ff + 1;
    FUN_440271a1(0xf,0);
    _DAT_505748e4 = 2;
    FUN_44026549(1);
    FUN_44028008();
  }
  return;
}
```

#### `FUN_44028348` (`sd_card_check_28348`) — `0x44028348` (75 B)

Verificação de presença/prontidão do cartão SD (`sd_card_check_28348`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_44028348 (sd_card_check_28348) @ 0x44028348 (size=75 B, req=0x44028348) -->
```c
// Callers (1): 0x4400a284
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_44028348(void)

{
  _DAT_505748dc = _DAT_505748dc | 0x180000;
  FUN_4401c8b4(0x1000,0x180000);
  FUN_4401d3b8(0x1000,0x180000,1);
  FUN_44028008();
  return;
}
```

#### Strings `SDCD` / `SetBusWidth` / `-91006` (`0x440392E0..0x44039500`)

Strings de diagnóstico do driver SD em `/BOOT.BIN` associadas à falha em barramento/cartão SD.

<!-- source: doc_boot_media_crypto.txt | SDCD / SetBusWidth / -91006 STRINGS 0x440392E0..0x44039500 -->
```text
  0x440392e0: 'CD] CMD7 SELECT err = %d\n'
  0x440392fa: '[SDCD] ACMD51 SEND SCR Eject\n'
  0x44039318: '[SDCD] ACMD13 SD Status err=%d\n'
  0x44039338: '[SDCD]GetSCR:CMD55 send ERROR %d\n'
  0x4403935a: '[SDCD]GetSCR:ACMD13 send ERROR %d\n'
  0x4403937d: '[SDCD]GetSCR:Data Read ERROR %d\n'
  0x4403939e: '[SDCD] Get SCR fail ERROR:%d\n'
  0x440393bc: '[SDCD]SetBusWidth:ACMD6 send ERROR %d\n'
  0x440393e3: '[SDCD]SetBusWidth:CMD55 send ERROR %d\n'
  0x4403940a: '[SDCD]SetBusWidth:Setting ERROR %d\n'
  0x4403942e: '[SDCD]SetBusWidth:SD Status ERROR %d\n'
  0x44039454: '[SDCD] ACMD6_SET_BUS_WIDTH Eject\n'
  0x44039476: '[SDCD] Set SD Bus Width fail ERROR:%d\n'
  0x4403949d: '[SDCD] CMD16 SET BLOCKLEN err=%d\n'
  0x440394bf: '[SDCD] === This Card removed ==== \n\n'
  0x440394e4: "[SDCD] === This Card can't be controled ==== \n\n"
```

### 1.5 Optical Drive (`CD`/`DVD-RAM`), Joliet, `feonly_mode` & HDD RTSD/DAVIS IDE

Funções de envio de pacotes ATAPI (`atapi_issue_pkt`), polling de teclas do painel/controle para abortar ou entrar em diagnóstico (`CD`/`DVD-RAM`), leitura de diretórios ISO9660/Joliet (`joliet_utf16_to_ascii_b20a`, `PANA_DVD.FRM`, `DIGADIAG`), `feonly_mode` e barramento ATA/IDE RTSD/DAVIS para descoberta de Master/Slave e leitura de HDD (`startup_drive`, `hdd_max_lba_9ea7`, `davis_bus_init_aa1f`).

#### `feonly_mode` (`feonly_mode`) — `0x44004b06` (214 B)

Modo Front-End Only de depuração/gravação direta com o drive.

<!-- source: doc_boot_core.txt | FUNC feonly_mode (feonly_mode) @ 0x44004b06 (size=214 B, req=0x44004b06) -->
```c
// Callers (1): 0x440049eb
// Referenced strings/literals:
//   0x44033bc0: 'LAST_ERR_LOG load OK(0x%08X).\n'
//   0x44033bdf: 'LAST_ERR_LOG load NG(%d).\n'
//   0x44033bfa: 'LAST_ERR_LOG load NG.\n'
//   0x44033c11: 'Copy to update program(size=0x%X) to 0x%08X from 0x%08X.\n'
//   0x44033c4b: '*** jump to update program ***\n'
/* WARNING: Control flow encountered bad instruction data */

void feonly_mode(void)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  uVar2 = bgo_driver_init();
  puVar1[-1] = uVar2;
  if (puVar1[-1] == 0) {
    puVar1[-2] = 0xffffffff;
    uVar2 = nvram_read_8634(0x504800ee,&DAT_901f75e0);
    puVar1[-1] = uVar2;
    if (puVar1[-1] == 0) {
      uart0_printf(s_LAST_ERR_LOG_load_OK_0x_08X___44033bc0,&DAT_901f75e0);
    }
    else {
      uart0_printf(s_LAST_ERR_LOG_load_NG__d___44033bdf,puVar1[-1]);
    }
  }
  else {
    uart0_printf(s_LAST_ERR_LOG_load_NG__44033bfa);
  }
  DAT_901f3bce = 0xff;
  fe_drive_mailbox_write(puVar1[-2]);
  uart0_printf(s_Copy_to_update_program_size_0x_X_44033c11,0x40000,&DAT_90400000,&DAT_90002000);
  FUN_4402c2f7(&DAT_90400000,&DAT_90002000,0x40000);
  uart0_printf(s_____jump_to_update_program_____44033c4b);
                    /* WARNING: Bad instruction - Truncating control flow here */
  halt_baddata();
}
```

#### `fe_drive_mailbox_write` (`fe_drive_mailbox_write`) — `0x44004bde` (300 B)

Escrita via mailbox/registradores de comunicação com o drive FE.

<!-- source: doc_boot_core.txt | FUNC fe_drive_mailbox_write (fe_drive_mailbox_write) @ 0x44004bde (size=300 B, req=0x44004bde) -->
```c
// Callers (1): 0x44004b86
// Referenced strings/literals:
//   0x44033c6b: 'F-CPU reset finish. 0x%X-0x%X data polling.\n'
//   0x44033c98: 'Drive firmware update start.\n'
//   0x44033cb6: 'I am BootLoader...\n'
//   0x44033cfe: 'LAST_ERR_LOG save NG.\n'
//   0x44033d15: 'LAST_ERR_LOG save OK.\n'
//   0x44033d2c: 'LAST_ERR_LOG save NG(%d).\n'
//   0x44033cca: 'Drive adjustment value update firmware load start.\n'
void fe_drive_mailbox_write(void)

{
  undefined4 in_A3;
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  uStack_4 = in_A3;
  uart0_printf(s_F_CPU_reset_finish__0x_X_0x_X_da_44033c6b,&DAT_901f3bda,&DAT_901f3bdf,0);
  while( true ) {
    while( true ) {
      while( true ) {
        puVar1[-1] = DAT_901f3be0;
        if (DAT_901f3bdf == -1) {
          uart0_printf(s_Drive_firmware_update_start__44033c98);
          return;
        }
        if (DAT_901f3bde != -1) break;
        fe_drive_band_change();
      }
      if (DAT_901f3bdd != -1) break;
      uart0_printf(s_I_am_BootLoader____44033cb6);
      DAT_901f3bdd = -0x10;
    }
    if (DAT_901f3bdc == -1) break;
    if (DAT_901f3bda == -1) {
      if (puVar1[2] == 0) {
        uart0_printf(s_LAST_ERR_LOG_save_NG__44033cfe);
        DAT_901f3bda = -0x10;
      }
      else {
        uVar2 = FUN_440090a6(0x504800ee,&DAT_901f75e0,0x3c0);
        puVar1[-1] = uVar2;
        if (puVar1[-1] == 0) {
          uart0_printf(s_LAST_ERR_LOG_save_OK__44033d15);
          DAT_901f3bda = '\0';
        }
        else {
          uart0_printf(s_LAST_ERR_LOG_save_NG__d___44033d2c,puVar1[-1]);
          DAT_901f3bda = -0x10;
        }
      }
    }
  }
  uart0_printf(s_Drive_adjustment_value_update_fi_44033cca);
  return;
}
```

#### `FUN_4400742c` (`dvdram_diag_boot_742c`) — `0x4400742c` (382 B)

Fluxo de boot de diagnóstico via DVD-RAM (`dvdram_diag_boot_742c`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4400742c (dvdram_diag_boot_742c) @ 0x4400742c (size=382 B, req=0x4400742c) -->
```c
// Callers (1): 0x440073ec
// Referenced strings/literals:
//   0x440347f3: 'Searching for FVU.\n'
//   0x44034835: 'PROG'
//   0x4403485e: 'copying fvu program from RTSC-buf to SDRAM.\n'
//   0x4403488b: 'error! fvu program check-sum invalid.\n'
//   0x4403483a: 'error! failed to read fvu program.\n'
//   0x44034807: 'error! not found fvu program in Update-Disk.\n'
undefined4 FUN_4400742c(void)

{
  int iVar1;
  undefined4 *puVar2;
  undefined4 uVar3;
  int iVar4;
  undefined4 uStack_8;
  undefined4 uStack_4;
  
  puVar2 = &uStack_8;
  uart0_printf(s_Searching_for_FVU__440347f3);
  uVar3 = FUN_4400aac2();
  puVar2[-1] = uVar3;
  if (puVar2[-1] == 1) {
    uVar3 = FUN_4400a733(puVar2[3],*(undefined1 *)(puVar2 + 4),&DAT_44034835,puVar2 + -3,0);
    puVar2[-1] = uVar3;
    if (puVar2[-1] == 1) {
      puVar2[-2] = *(int *)(puVar2[-3] + 0x20) + 0x7ffU >> 0xb;
      uVar3 = FUN_44009a3e(0,*(undefined4 *)(puVar2[-3] + 0xc),puVar2[-2]);
      puVar2[-1] = uVar3;
      if (puVar2[-1] == 0) {
        uart0_printf(s_copying_fvu_program_from_RTSC_bu_4403485e);
        FUN_44010760(0,&DAT_93fc0000,*(undefined4 *)(puVar2[-3] + 0x20),0);
        iVar1 = puVar2[-3];
        iVar4 = FUN_4400a831(&DAT_93fc0000,*(undefined4 *)(puVar2[-3] + 0x20));
        if (*(int *)(iVar1 + 0x24) == iVar4) {
          puVar2[-4] = *(undefined4 *)(puVar2[-3] + 0x20);
        }
        else {
          uart0_printf(s_error__fvu_program_check_sum_inv_4403488b);
          FUN_44005676(6,0x50480201,0x50480216);
          puVar2[-4] = 0;
        }
      }
      else {
        uart0_printf(s_error__failed_to_read_fvu_progra_4403483a);
        FUN_44005676(6,0x50480201,0x5048020f);
        puVar2[-4] = 0;
      }
    }
    else {
      uart0_printf(s_error__not_found_fvu_program_in_U_44034807);
      FUN_44005676(6,0x50480201,0x50480208);
      puVar2[-4] = 0;
    }
  }
  else {
    uart0_printf(s_error__not_found_fvu_program_in_U_44034807);
    FUN_44005676(6,0x50480201,0x50480208);
    puVar2[-4] = 0;
  }
  return puVar2[-4];
}
```

#### `FUN_440075aa` (`cdr_diag_boot_75aa`) — `0x440075aa` (376 B)

Fluxo de boot de diagnóstico via CD-R (`cdr_diag_boot_75aa`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_440075aa (cdr_diag_boot_75aa) @ 0x440075aa (size=376 B, req=0x440075aa) -->
```c
// Callers (1): 0x44007412
// Referenced strings/literals:
//   0x440347f3: 'Searching for FVU.\n'
//   0x44034835: 'PROG'
//   0x440348e8: 'cheking for fvu program check-sum.\n'
//   0x4403488b: 'error! fvu program check-sum invalid.\n'
//   0x440348b2: 'error! failed to read fvu program. Read_VCDDATA: %8x\n'
//   0x44034807: 'error! not found fvu program in Update-Disk.\n'
undefined4 FUN_440075aa(void)

{
  int iVar1;
  undefined4 *puVar2;
  undefined4 uVar3;
  int iVar4;
  undefined4 uStack_8;
  undefined4 uStack_4;
  
  puVar2 = &uStack_8;
  uart0_printf(s_Searching_for_FVU__440347f3);
  uVar3 = FUN_4400a86d();
  puVar2[-1] = uVar3;
  if (puVar2[-1] == 1) {
    uVar3 = FUN_4400a733(puVar2[3],*(undefined1 *)(puVar2 + 4),&DAT_44034835,puVar2 + -3,0);
    puVar2[-1] = uVar3;
    if (puVar2[-1] == 1) {
      uVar3 = FUN_4400b124(&DAT_93fc0000,*(undefined4 *)(puVar2[-3] + 0xc),
                           *(undefined4 *)(puVar2[-3] + 0x20));
      puVar2[-2] = uVar3;
      if (*(int *)(puVar2[-3] + 0x20) == puVar2[-2]) {
        FUN_4400a666(&DAT_93fc0000,*(undefined4 *)(puVar2[-3] + 0x20));
        uart0_printf(s_cheking_for_fvu_program_check_su_440348e8);
        iVar1 = puVar2[-3];
        iVar4 = FUN_4400a831(&DAT_93fc0000,*(undefined4 *)(puVar2[-3] + 0x20));
        if (*(int *)(iVar1 + 0x24) == iVar4) {
          puVar2[-4] = *(undefined4 *)(puVar2[-3] + 0x20);
        }
        else {
          uart0_printf(s_error__fvu_program_check_sum_inv_4403488b);
          FUN_44005676(6,0x50480201,0x50480216);
          puVar2[-4] = 0;
        }
      }
      else {
        uart0_printf(s_error__failed_to_read_fvu_progra_440348b2,puVar2[-2]);
        FUN_44005676(6,0x50480201,0x5048020f);
        puVar2[-4] = 0;
      }
    }
    else {
      uart0_printf(s_error__not_found_fvu_program_in_U_44034807);
      FUN_44005676(6,0x50480201,0x50480208);
      puVar2[-4] = 0;
    }
  }
  else {
    uart0_printf(s_error__not_found_fvu_program_in_U_44034807);
    FUN_44005676(6,0x50480201,0x50480208);
    puVar2[-4] = 0;
  }
  return puVar2[-4];
}
```

#### `FUN_44007722` (`drive_key_poll_7722`) — `0x44007722` (55 B)

Polling de teclas do painel durante checagem do drive (`drive_key_poll_7722`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_44007722 (drive_key_poll_7722) @ 0x44007722 (size=55 B, req=0x44007722) -->
```c
// Callers (2): 0x440070ed, 0x440072b3
// Referenced strings/literals:
//   0x4403490c: 'error! Failed to Startup Drive.\n'
void FUN_44007722(void)

{
  uart0_printf(s_error__Failed_to_Startup_Drive__4403490c);
  FUN_44007f1c();
  FUN_44007ea9();
  FUN_44007836(0x5048021d,0x50480224);
  return;
}
```

#### `FUN_44007789` (`drive_key_abort_7789`) — `0x44007789` (137 B)

Aborta espera do drive por combinação de teclas (`drive_key_abort_7789`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_44007789 (drive_key_abort_7789) @ 0x44007789 (size=137 B, req=0x44007789) -->
```c
// Callers (1): 0x44007145
// Referenced strings/literals:
//   0x4403492d: 'Timeout Power Off\n'
//   0x44034940: 'pushed power-button\n'
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_44007789(int param_1)

{
  undefined4 in_A3;
  undefined4 *puVar1;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  if ((_DAT_50501518 == 0) || (uStack_4 = in_A3, param_1 == 0)) {
    _DAT_50501518 = 1;
    FUN_44009bd7();
  }
  if (puVar1[2] != 0) {
    if (_DAT_50503760 == 0) {
      FUN_440054aa(0x1e,0x5048022b);
      uart0_printf(s_Timeout_Power_Off_4403492d);
    }
    else {
      uart0_printf(s_pushed_power_button_44034940);
    }
    FUN_44007f1c();
    FUN_44007ea9();
    FUN_44007836(0x50480247,0x5048024e);
  }
  return;
}
```

#### `cd_diag_boot_mode` (`cd_diag_boot_mode_7c59`) — `0x44007c59` (495 B)

Modo completo de boot/diagnóstico via disco óptico (`cd_diag_boot_mode_7c59`).

<!-- source: doc_boot_media_crypto.txt | FUNC cd_diag_boot_mode (cd_diag_boot_mode_7c59) @ 0x44007c59 (size=495 B, req=0x44007c59) -->
```c
// Callers (1): 0x44007c0a
// Referenced strings/literals:
//   0x44034aeb: '*** CD diag boot mode ***\n'
//   0x44034b06: 'CDDIAG'
//   0x44034b0d: 'Drive start failed[%d].\n'
//   0x44034b26: 'DIGADIAG.FRM'
//   0x44034b33: 'DIGADIAG.FRM read failed.\n'
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

undefined4 cd_diag_boot_mode(void)

{
  undefined4 *puVar1;
  int iVar2;
  undefined4 uVar3;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  uart0_printf(s_____CD_diag_boot_mode_____44034aeb);
  FUN_440054aa(300,s_CDDIAG_44034b06);
  nvram_read_id(0xb,puVar1 + -2,0xffffffff);
  if (puVar1[-2] != 0) {
    iVar2 = sub_4400bb14(0,1);
    if (iVar2 != 1) {
      iVar2 = uncompress(0x84050000);
      if (iVar2 == 1) {
        puVar1[-3] = 0;
        goto LAB_44007e40;
      }
      FUN_440079a4();
    }
    delay_ms(3000);
  }
  iVar2 = FUN_44009741(1,0);
  if ((iVar2 == 0) || (iVar2 = uncompress(0x84050000), iVar2 != 1)) {
    uVar3 = startup_drive(1);
    puVar1[-1] = uVar3;
    if (puVar1[-1] != 0) {
      uart0_printf(s_Drive_start_failed__d___44034b0d,puVar1[-1]);
      iVar2 = uncompress(0x84050000);
      if (iVar2 == 1) {
        puVar1[-3] = 0;
        goto LAB_44007e40;
      }
      FUN_440079a4();
    }
    FUN_44009bd7();
    _DAT_50501518 = 1;
    _DAT_50503760 = 0;
    while( true ) {
      if (_DAT_50501514 == 1) {
        uVar3 = FUN_44009c85();
        puVar1[-1] = uVar3;
        if (puVar1[-1] == 0) {
          _DAT_50501518 = 0;
        }
        _DAT_50501514 = 0;
      }
      uVar3 = FUN_44009835();
      puVar1[-1] = uVar3;
      if (puVar1[-1] == 0) break;
      if (((undefined *)puVar1[-1] == &DAT_80023a00) && (_DAT_50501518 == 0)) {
        FUN_44009bd7();
        _DAT_50501518 = 1;
      }
      if (_DAT_50503760 == 1) {
        FUN_440079a4();
      }
      delay_ms(1000);
    }
    _DAT_50503760 = 0;
    _DAT_50501518 = 0;
    _DAT_5048025c = 1;
    uVar3 = FUN_44009a3e(0,0,1);
    puVar1[-1] = uVar3;
    uVar3 = FUN_4400b31e(&DAT_93fc0000,0,0x20000,s_DIGADIAG_FRM_44034b26);
    puVar1[-1] = uVar3;
    if (puVar1[-1] != 0x20000) {
      uart0_printf(s_DIGADIAG_FRM_read_failed__44034b33);
      FUN_44009bd7();
      _DAT_50501518 = 1;
      FUN_440079a4();
    }
    puVar1[-3] = 1;
  }
  else {
    puVar1[-3] = 0;
  }
LAB_44007e40:
  return puVar1[-3];
}
```

#### `FUN_44009835` (`drive_get_status_9835`) — `0x44009835` (199 B)

Consulta status ATAPI/ATA do drive (`drive_get_status_9835`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_44009835 (drive_get_status_9835) @ 0x44009835 (size=199 B, req=0x44009835) -->
```c
// Callers (17): 0x440071df, 0x4400792e, 0x4400797c, 0x44007d76, 0x44009a4f, 0x44009a57, 0x44009a6a, 0x44009bc7, 0x44009cfa, 0x44009cff, 0x44009d7c, 0x4400a2f0, 0x4400a2f5, 0x4400a376, 0x4400a37b, 0x4400a382
// Referenced strings/literals:
//   0x44034f2b: '(%08x)'
undefined4 FUN_44009835(void)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined1 auStack_10 [12];
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  FUN_4402c348(auStack_10,0,0xc);
  do {
    FUN_44009e77();
    uVar2 = FUN_44010725(puVar1 + -3,0,0,1,&DAT_00005dc0);
    puVar1[-4] = uVar2;
    FUN_44009e8f();
    if ((puVar1[-4] & 0x7f000000) != 0) {
      FUN_44009e77();
      FUN_4400f715(0,1,0);
      uart0_printf(&DAT_44034f27);
    }
    uart0_printf(&DAT_44034f29);
    puVar1[-5] = puVar1[-5] + -1;
  } while ((-1 < (int)puVar1[-5]) && ((puVar1[-4] & 0x7f000000) != 0));
  if ((undefined *)puVar1[-4] == &DAT_80023a00) {
    puVar1[-4] = &DAT_80023a00;
  }
  uart0_printf(s___08x__44034f2b,puVar1[-4]);
  return puVar1[-4];
}
```

#### `FUN_44009a3e` (`drive_read_tur_9a3e`) — `0x44009a3e` (211 B)

Envia `TEST UNIT READY` / leitura de prontidão do drive (`drive_read_tur_9a3e`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_44009a3e (drive_read_tur_9a3e) @ 0x44009a3e (size=211 B, req=0x44009a3e) -->
```c
// Callers (5): 0x44007101, 0x44007110, 0x440074f1, 0x44007de4, 0x4400aad0
// Referenced strings/literals:
//   0x44034f54: '\tdrive_read: retry TUR %d times\n'
//   0x44034f32: '\tdrive_read: unit not ready [%x]\n'
//   0x44034f75: '\tdrive_read: read failed [%x]\n'
undefined4 FUN_44009a3e(void)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  FUN_44009e77();
  uVar2 = FUN_44009835();
  puVar1[-2] = uVar2;
  uVar2 = FUN_44009835();
  puVar1[-2] = uVar2;
  *(undefined1 *)((int)puVar1 + -1) = 0;
  while (*(byte *)((int)puVar1 + -1) < 3) {
    uVar2 = FUN_44009835();
    puVar1[-2] = uVar2;
    if (puVar1[-2] == 0) {
      if (*(char *)((int)puVar1 + -1) != '\0') {
        uart0_printf(s_drive_read__retry_TUR__d_times_44034f54,(uint)*(byte *)((int)puVar1 + -1));
      }
      *(undefined1 *)((int)puVar1 + -1) = 0;
      goto LAB_44009ac8;
    }
    if ((undefined *)puVar1[-2] == &DAT_80023a00) break;
    delay_ms(1000);
    *(char *)((int)puVar1 + -1) = *(char *)((int)puVar1 + -1) + '\x01';
  }
  uart0_printf(s_drive_read__unit_not_ready___x__44034f32,puVar1[-2]);
LAB_44009b04:
  FUN_44009e8f();
  return puVar1[-2];
LAB_44009ac8:
  if (2 < *(byte *)((int)puVar1 + -1)) goto LAB_44009af4;
  uVar2 = FUN_44009980(puVar1[2],puVar1[3],puVar1[4]);
  puVar1[-2] = uVar2;
  if (puVar1[-2] == 0) goto LAB_44009b04;
  *(char *)((int)puVar1 + -1) = *(char *)((int)puVar1 + -1) + '\x01';
  goto LAB_44009ac8;
LAB_44009af4:
  uart0_printf(s_drive_read__read_failed___x__44034f75,puVar1[-2]);
  goto LAB_44009b04;
}
```

#### `atapi_issue_pkt` (`atapi_issue_pkt_9b11`) — `0x44009b11` (142 B)

Envio de CDB ATAPI de pacote (`atapi_issue_pkt_9b11`).

<!-- source: doc_boot_media_crypto.txt | FUNC atapi_issue_pkt (atapi_issue_pkt_9b11) @ 0x44009b11 (size=142 B, req=0x44009b11) -->
```c
// Callers (6): 0x44009c04, 0x44009c52, 0x44009cb7, 0x44009d4e, 0x4400a31d, 0x4400a3b4
undefined4 atapi_issue_pkt(void)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  FUN_44009e77();
  uVar2 = FUN_44010725(puVar1[2],puVar1[3],puVar1[4],puVar1[5] & 1,puVar1[-2]);
  puVar1[-3] = uVar2;
  FUN_44009e8f();
  if ((puVar1[-3] & 0x7f000000) != 0) {
    FUN_44009b9f();
  }
  return puVar1[-3];
}
```

#### `FUN_44009bd7` (`open_tray_9bd7`) — `0x44009bd7` (174 B)

Comando ATAPI para abrir bandeja do drive (`open_tray_9bd7`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_44009bd7 (open_tray_9bd7) @ 0x44009bd7 (size=174 B, req=0x44009bd7) -->
```c
// Callers (8): 0x4400720f, 0x440077ab, 0x44007945, 0x44007993, 0x44007d37, 0x44007d9e, 0x44007e27, 0x44009e39
// Referenced strings/literals:
//   0x44034fa9: '** open tray **\n'
//   0x44034fba: '** can not open tray **\n'
undefined4 FUN_44009bd7(void)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined1 auStack_14 [16];
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  FUN_4402c348(auStack_14,0,0xc);
  *(undefined1 *)(puVar1 + -4) = 0x1e;
  *(undefined1 *)(puVar1 + -3) = 0;
  uVar2 = atapi_issue_pkt(puVar1 + -4,0,0,1);
  puVar1[-5] = uVar2;
  if (puVar1[-5] == 0) {
    puVar1[-1] = 0;
    while ((int)puVar1[-1] < 6) {
      *(undefined1 *)((int)puVar1 + puVar1[-1] + -0x10) = 0;
      puVar1[-1] = puVar1[-1] + 1;
    }
    *(undefined1 *)(puVar1 + -4) = 0x1b;
    *(undefined1 *)((int)puVar1 + -0xf) = 0;
    *(undefined1 *)(puVar1 + -3) = 2;
    uVar2 = atapi_issue_pkt(puVar1 + -4,0,0,1);
    puVar1[-5] = uVar2;
  }
  if (puVar1[-5] == 0) {
    uart0_printf(s____open_tray____44034fa9);
  }
  else {
    uart0_printf(s____can_not_open_tray____44034fba);
  }
  return puVar1[-5];
}
```

#### `FUN_44009c85` (`close_tray_9c85`) — `0x44009c85` (101 B)

Comando ATAPI para fechar bandeja do drive (`close_tray_9c85`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_44009c85 (close_tray_9c85) @ 0x44009c85 (size=101 B, req=0x44009c85) -->
```c
// Callers (5): 0x440071c6, 0x4400791f, 0x4400796d, 0x44007d57, 0x44009e21
// Referenced strings/literals:
//   0x44034fd3: '** close tray **\n'
//   0x44034fe5: '** can not close tray **\n'
undefined4 FUN_44009c85(void)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined1 auStack_10 [12];
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  FUN_4402c348(auStack_10,0,0xc);
  *(undefined1 *)(puVar1 + -3) = 0x1b;
  *(undefined1 *)((int)puVar1 + -0xb) = 0;
  *(undefined1 *)(puVar1 + -2) = 3;
  uVar2 = atapi_issue_pkt(puVar1 + -3,0,0,1);
  puVar1[-4] = uVar2;
  if (puVar1[-4] == 0) {
    uart0_printf(s____close_tray____44034fd3);
  }
  else {
    uart0_printf(s____can_not_close_tray____44034fe5);
  }
  return puVar1[-4];
}
```

#### `startup_drive` (`startup_drive_9cea`) — `0x44009cea` (295 B)

Inicialização sequencial dos drives IDE/ATAPI (HDD e ODD, `startup_drive_9cea`).

<!-- source: doc_boot_media_crypto.txt | FUNC startup_drive (startup_drive_9cea) @ 0x44009cea (size=295 B, req=0x44009cea) -->
```c
// Callers (5): 0x440070e2, 0x440072a5, 0x44007927, 0x44007975, 0x44007cf8
// Referenced strings/literals:
//   0x44034fff: '** startup drive **\n'
//   0x44035014: 'drive start error(%08x)\n'
//   0x4403502d: '** success to startup **\n'
//   0x44035047: '** no disc **\n'
//   0x44035058: '%d,'
undefined4 startup_drive(int param_1)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  if (param_1 != 0) {
    FUN_44009835();
    FUN_44009835();
  }
  FUN_4402c348(puVar1 + -3,0,0xc);
  *(undefined1 *)(puVar1 + -3) = 0x1b;
  *(undefined1 *)((int)puVar1 + -0xb) = 0;
  *(undefined1 *)((int)puVar1 + -10) = 0;
  *(undefined1 *)((int)puVar1 + -9) = 0;
  *(undefined1 *)(puVar1 + -2) = 3;
  *(undefined1 *)((int)puVar1 + -7) = 0;
  *(undefined1 *)((int)puVar1 + -6) = 1;
  uart0_printf(s____startup_drive____44034fff);
  uVar2 = atapi_issue_pkt(puVar1 + -3,0,0,1);
  puVar1[-5] = uVar2;
  if (puVar1[-5] != 0) {
    uart0_printf(s_drive_start_error__08x__44035014,puVar1[-5]);
  }
  puVar1[-4] = 0x3c;
  do {
    if ((int)puVar1[-4] < 1) {
LAB_44009e09:
      return puVar1[-5];
    }
    uVar2 = FUN_44009835();
    puVar1[-5] = uVar2;
    if (puVar1[-5] == 0) {
      uart0_printf(s____success_to_startup____4403502d);
      goto LAB_44009e09;
    }
    if ((undefined *)puVar1[-5] == &DAT_80023a00) {
      uart0_printf(s____no_disc____44035047);
      puVar1[-5] = 0;
      goto LAB_44009e09;
    }
    if ((puVar1[-5] & 0x7f000000) != 0) {
      FUN_44009e77();
      uart0_printf(&DAT_44035056);
      FUN_4400f715(0,1,0);
    }
    puVar1[-4] = puVar1[-4] + -1;
    uart0_printf(&DAT_44035058,puVar1[-4]);
    delay_ms(1000);
  } while( true );
}
```

#### `FUN_44009ea7` (`hdd_max_lba_9ea7`) — `0x44009ea7` (355 B)

Obtém `MAX LBA` / capacidade do HDD (`hdd_max_lba_9ea7`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_44009ea7 (hdd_max_lba_9ea7) @ 0x44009ea7 (size=355 B, req=0x44009ea7) -->
```c
// Callers (1): 0x440066e0
// Referenced strings/literals:
//   0x4403505c: 'HDD MAX LBA = 0x%X.\n'
undefined4 FUN_44009ea7(void)

{
  undefined1 *puVar1;
  undefined4 uVar2;
  undefined1 auStack_4 [4];
  
  puVar1 = auStack_4;
  FUN_4402c2f7(0x50501530,&DAT_90c00000,0x200);
  FUN_4402c348(puVar1 + -0xc,0,0xc);
  puVar1[-0xd] = 0;
  while ((byte)puVar1[-0xd] < 3) {
    FUN_44009e77();
    FUN_44010f91(puVar1 + -0xc,0xec,0,0,0);
    uVar2 = FUN_44010e54(puVar1 + -0xc,0,0x200,0,1000);
    *(undefined4 *)(puVar1 + -0x14) = uVar2;
    FUN_44011537(0,puVar1 + -0x218,0x200);
    FUN_44009e8f();
    if ((-1 < *(int *)(puVar1 + -0x14)) && ((((byte)puVar1[-0x218] >> 2 ^ 1) & 1) != 0)) break;
    FUN_44009e77();
    FUN_4400f715(1,0,0);
    puVar1[-0xd] = puVar1[-0xd] + '\x01';
  }
  FUN_44009e8f();
  FUN_4402c2f7(&DAT_90c00000,0x50501530,0x200);
  if (*(int *)(puVar1 + -0x14) < 0) {
    *(undefined4 *)(puVar1 + -0x21c) = *(undefined4 *)(puVar1 + -0x14);
  }
  else if (((byte)puVar1[-0x218] >> 2 & 1) == 0) {
    *(undefined2 *)(puVar1 + -0x16) = *(undefined2 *)(puVar1 + -0x172);
    if ((*(ushort *)(puVar1 + -0x16) >> 10 & 1) == 0) {
      **(undefined4 **)(puVar1 + 8) = *(undefined4 *)(puVar1 + -0x1a0);
    }
    else {
      **(undefined4 **)(puVar1 + 8) = *(undefined4 *)(puVar1 + -0x150);
    }
    **(uint **)(puVar1 + 8) = **(uint **)(puVar1 + 8) & 0xfffffffc;
    uart0_printf(s_HDD_MAX_LBA___0x_X__4403505c,**(undefined4 **)(puVar1 + 8));
    *(undefined4 *)(puVar1 + -0x21c) = 0;
  }
  else {
    *(undefined **)(puVar1 + -0x21c) = &DAT_8004f000;
  }
  return *(undefined4 *)(puVar1 + -0x21c);
}
```

#### `FUN_4400a86d` (`cdr_search_fvu_a86d`) — `0x4400a86d` (90 B)

Procura arquivo `.FVU` em mídia CD-R (`cdr_search_fvu_a86d`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4400a86d (cdr_search_fvu_a86d) @ 0x4400a86d (size=90 B, req=0x4400a86d) -->
```c
// Callers (1): 0x440075c4
// Referenced strings/literals:
//   0x44035170: '[fmu] error! Read_VCDDATA(): %8x\n'
undefined4 FUN_4400a86d(void)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  uVar2 = FUN_4400b124(0x5050175c,0,0x2000);
  puVar1[-1] = uVar2;
  if (puVar1[-1] == 0x2000) {
    FUN_4400a666(0x5050175c,0x2000);
    puVar1[-2] = 1;
  }
  else {
    FUN_4400b3d9(1,s__fmu__error__Read_VCDDATA_____8x_44035170,puVar1[-1]);
    puVar1[-2] = 0;
  }
  return puVar1[-2];
}
```

#### `FUN_4400aac2` (`dvdram_read_list_aac2`) — `0x4400aac2` (72 B)

Leitura de lista de arquivos/firmware em DVD-RAM (`dvdram_read_list_aac2`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4400aac2 (dvdram_read_list_aac2) @ 0x4400aac2 (size=72 B, req=0x4400aac2) -->
```c
// Callers (1): 0x44007446
undefined4 FUN_4400aac2(void)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  uVar2 = FUN_44009a3e(0,0x301,4);
  puVar1[-1] = uVar2;
  if (puVar1[-1] == 0) {
    FUN_44010760(0,0x5050175c,0x2000,0);
    puVar1[-2] = 1;
  }
  else {
    puVar1[-2] = 0;
  }
  return puVar1[-2];
}
```

#### `FUN_4400ab0a` (`cd_load_file_ab0a`) — `0x4400ab0a` (1562 B)

Parser ISO9660/Joliet e carregamento de arquivo por setor em CD (`cd_load_file_ab0a`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4400ab0a (cd_load_file_ab0a) @ 0x4400ab0a (size=1562 B, req=0x4400ab0a) -->
```c
// Callers (2): 0x4400b143, 0x4400b33a
// Referenced strings/literals:
//   0x4403520f: 'CD001'
undefined4 FUN_4400ab0a(void)

{
  undefined1 *puVar1;
  int iVar2;
  undefined4 uVar3;
  undefined1 auStack_8 [8];
  
  puVar1 = auStack_8;
  do {
    iVar2 = FUN_4400b14d(puVar1 + -0x8a3c,1,*(undefined4 *)(puVar1 + -0x21c));
    if ((iVar2 != 0) || (iVar2 = FUN_4402c2b7(puVar1 + -0x8a3b,s_CD001_4403520f,5), iVar2 != 0))
    break;
    puVar1[-0x21d] = puVar1[-0x8a3c];
    if ((puVar1[-0x21d] == '\x01') || (puVar1[-0x21d] == '\x02')) {
      *(undefined1 **)(puVar1 + -8) = puVar1 + -0x89a0;
      *(uint *)(puVar1 + -0xc) =
           (uint)*(byte *)(*(int *)(puVar1 + -8) + 8) << 8 |
           (uint)*(byte *)(*(int *)(puVar1 + -8) + 7) << 0x10 |
           (uint)*(byte *)(*(int *)(puVar1 + -8) + 6) << 0x18 |
           (uint)*(byte *)(*(int *)(puVar1 + -8) + 9);
      *(uint *)(puVar1 + -0x10) =
           (uint)*(byte *)(*(int *)(puVar1 + -8) + 0x10) << 8 |
           (uint)*(byte *)(*(int *)(puVar1 + -8) + 0xf) << 0x10 |
           (uint)*(byte *)(*(int *)(puVar1 + -8) + 0xe) << 0x18 |
           (uint)*(byte *)(*(int *)(puVar1 + -8) + 0x11);
      uVar3 = FUN_4400b14d(puVar1 + -0x8a3c,1,*(undefined4 *)(puVar1 + -0xc));
      *(undefined4 *)(puVar1 + -4) = uVar3;
      if (*(int *)(puVar1 + -4) != 0) break;
      *(undefined4 *)(puVar1 + -0x218) = 0;
      while ((*(int *)(puVar1 + -0x10) != 0 || (*(uint *)(puVar1 + -0x218) < 0x800))) {
        *(undefined4 *)(puVar1 + -0x23c) = 0;
        while (*(int *)(puVar1 + -0x23c) < 0x100) {
          if ((int)(uint)(byte)puVar1[*(int *)(puVar1 + -0x218) + -0x8a1c] <=
              *(int *)(puVar1 + -0x23c)) {
            puVar1[*(int *)(puVar1 + -0x23c) + -0x110] = 0;
            break;
          }
          puVar1[*(int *)(puVar1 + -0x23c) + -0x110] =
               puVar1[*(int *)(puVar1 + -0x23c) + *(int *)(puVar1 + -0x218) + -0x8a1b];
          if ((0x60 < (byte)puVar1[*(int *)(puVar1 + -0x23c) + -0x110]) &&
             ((byte)puVar1[*(int *)(puVar1 + -0x23c) + -0x110] < 0x7b)) {
            puVar1[*(int *)(puVar1 + -0x23c) + -0x110] =
                 puVar1[*(int *)(puVar1 + -0x23c) + -0x110] + -0x20;
          }
          if (puVar1[*(int *)(puVar1 + -0x23c) + -0x110] == ';') {
            puVar1[*(int *)(puVar1 + -0x23c) + -0x110] = 0;
            if ((0 < *(int *)(puVar1 + -0x23c)) &&
               (puVar1[*(int *)(puVar1 + -0x23c) + -0x111] == '.')) {
              puVar1[*(int *)(puVar1 + -0x23c) + -0x111] = 0;
            }
            break;
          }
          *(int *)(puVar1 + -0x23c) = *(int *)(puVar1 + -0x23c) + 1;
        }
        if (puVar1[-0x21d] == '\x02') {
          puVar1[-0x214] = 0x10;
          FUN_4402c2f7(puVar1 + -0x213,puVar1 + -0x110,*(int *)(puVar1 + -0x23c) + 1);
          FUN_4402c348(puVar1 + -0x110,0,0x100);
          FUN_4400b20a(puVar1 + -0x214,puVar1 + -0x110,*(int *)(puVar1 + -0x23c) + 1);
        }
        iVar2 = FUN_4402c2b7(puVar1 + -0x110,*(undefined4 *)(puVar1 + 0x18),0xd);
        if (iVar2 == 0) {
          *(undefined4 *)(puVar1 + -0x224) = 0;
          *(uint *)(puVar1 + -0x234) =
               (uint)(byte)puVar1[*(int *)(puVar1 + -0x218) + -0x8a33] |
               (uint)(byte)puVar1[*(int *)(puVar1 + -0x218) + -0x8a36] << 0x18 |
               (uint)(byte)puVar1[*(int *)(puVar1 + -0x218) + -0x8a35] << 0x10 |
               (uint)(byte)puVar1[*(int *)(puVar1 + -0x218) + -0x8a34] << 8;
          *(uint *)(puVar1 + -0x238) =
               (uint)(byte)puVar1[*(int *)(puVar1 + -0x218) + -0x8a2b] |
               (uint)(byte)puVar1[*(int *)(puVar1 + -0x218) + -0x8a2e] << 0x18 |
               (uint)(byte)puVar1[*(int *)(puVar1 + -0x218) + -0x8a2d] << 0x10 |
               (uint)(byte)puVar1[*(int *)(puVar1 + -0x218) + -0x8a2c] << 8;
          *(int *)(puVar1 + -0x228) =
               (int)((((ulonglong)(uint)(*(int *)(puVar1 + 0x10) >> 0x1f) << 0x20) +
                     (longlong)*(int *)(puVar1 + 0x10)) / 0x800);
          if (*(int *)(puVar1 + 0x10) <=
              (int)((((ulonglong)(uint)(*(int *)(puVar1 + 0x10) >> 0x1f) << 0x20) +
                    (longlong)*(int *)(puVar1 + 0x10)) / 0x800) * 0x800) goto LAB_4400af9b;
          uVar3 = FUN_4400b14d(puVar1 + -0x823c,1,
                               *(int *)(puVar1 + -0x228) + *(int *)(puVar1 + -0x234));
          *(undefined4 *)(puVar1 + -4) = uVar3;
          if (*(int *)(puVar1 + -4) < 0) {
            *(undefined4 *)(puVar1 + -0x8a40) = *(undefined4 *)(puVar1 + -4);
            goto LAB_4400b119;
          }
          *(int *)(puVar1 + -0x230) =
               *(int *)(puVar1 + 0x10) +
               (int)((((ulonglong)(uint)(*(int *)(puVar1 + 0x10) >> 0x1f) << 0x20) +
                     (longlong)*(int *)(puVar1 + 0x10)) / 0x800) * -0x800;
          if (*(int *)(puVar1 + 0x14) <= 0x800 - *(int *)(puVar1 + -0x230)) {
            FUN_4402c2f7(*(undefined4 *)(puVar1 + 0xc),puVar1 + *(int *)(puVar1 + -0x230) + -0x823c,
                         *(undefined4 *)(puVar1 + 0x14));
            *(undefined4 *)(puVar1 + -0x8a40) = *(undefined4 *)(puVar1 + 0x14);
            goto LAB_4400b119;
          }
          FUN_4402c2f7(*(undefined4 *)(puVar1 + 0xc),puVar1 + *(int *)(puVar1 + -0x230) + -0x823c,
                       *(int *)(puVar1 + -0x230) + -0x800);
          *(int *)(puVar1 + -0x224) =
               (*(int *)(puVar1 + -0x230) - *(int *)(puVar1 + -0x224)) + 0x800;
          *(int *)(puVar1 + 0x10) = (*(int *)(puVar1 + -0x230) - *(int *)(puVar1 + 0x10)) + 0x800;
          *(int *)(puVar1 + 0x14) = *(int *)(puVar1 + -0x230) + *(int *)(puVar1 + 0x14) + -0x800;
          *(int *)(puVar1 + -0x228) = *(int *)(puVar1 + -0x228) + 1;
          goto LAB_4400af9b;
        }
        if (puVar1[*(int *)(puVar1 + -0x218) + -0x8a3c] == '\0') break;
        *(uint *)(puVar1 + -0x10) =
             *(int *)(puVar1 + -0x10) - (uint)(byte)puVar1[*(int *)(puVar1 + -0x218) + -0x8a3c];
        *(uint *)(puVar1 + -0x218) =
             *(int *)(puVar1 + -0x218) + (uint)(byte)puVar1[*(int *)(puVar1 + -0x218) + -0x8a3c];
      }
    }
    *(int *)(puVar1 + -0x21c) = *(int *)(puVar1 + -0x21c) + 1;
  } while( true );
  *(undefined4 *)(puVar1 + -0x8a40) = *(undefined4 *)(puVar1 + -4);
  goto LAB_4400b119;
LAB_4400af9b:
  if (*(int *)(puVar1 + 0x14) < 1) goto LAB_4400b0aa;
  iVar2 = *(int *)(puVar1 + 0x14) + *(int *)(puVar1 + 0x10) + 0x7ff;
  *(int *)(puVar1 + -0x22c) =
       *(int *)(puVar1 + -0x228) -
       (int)((((ulonglong)(uint)(iVar2 >> 0x1f) << 0x20) + (longlong)iVar2) / 0x800);
  if (0x10 < *(int *)(puVar1 + -0x22c)) {
    *(undefined4 *)(puVar1 + -0x22c) = 0x10;
  }
  uVar3 = FUN_4400b14d(puVar1 + -0x823c,*(undefined4 *)(puVar1 + -0x22c),
                       *(int *)(puVar1 + -0x228) + *(int *)(puVar1 + -0x234));
  *(undefined4 *)(puVar1 + -4) = uVar3;
  if (*(int *)(puVar1 + -4) < 0) {
    *(undefined4 *)(puVar1 + -0x8a40) = *(undefined4 *)(puVar1 + -4);
    goto LAB_4400b119;
  }
  if (*(int *)(puVar1 + 0x14) <= *(int *)(puVar1 + -0x22c) * 0x800) {
    FUN_4402c2f7(*(int *)(puVar1 + 0xc) + *(int *)(puVar1 + -0x224),puVar1 + -0x823c,
                 *(undefined4 *)(puVar1 + 0x14));
    *(int *)(puVar1 + -0x8a40) = *(int *)(puVar1 + 0x14) + *(int *)(puVar1 + -0x224);
    goto LAB_4400b119;
  }
  FUN_4402c2f7(*(int *)(puVar1 + 0xc) + *(int *)(puVar1 + -0x224),puVar1 + -0x823c,
               *(int *)(puVar1 + -0x22c) << 0xb);
  *(int *)(puVar1 + -0x224) = *(int *)(puVar1 + -0x224) + *(int *)(puVar1 + -0x22c) * 0x800;
  *(int *)(puVar1 + 0x10) = *(int *)(puVar1 + 0x10) + *(int *)(puVar1 + -0x22c) * 0x800;
  *(int *)(puVar1 + 0x14) = *(int *)(puVar1 + 0x14) + *(int *)(puVar1 + -0x22c) * -0x800;
  *(int *)(puVar1 + -0x228) = *(int *)(puVar1 + -0x22c) + *(int *)(puVar1 + -0x228);
  goto LAB_4400af9b;
LAB_4400b0aa:
  *(undefined4 *)(puVar1 + -0x8a40) = *(undefined4 *)(puVar1 + -0x224);
LAB_4400b119:
  return *(undefined4 *)(puVar1 + -0x8a40);
}
```

#### `FUN_4400b124` (`cd_load_pana_dvd_b124`) — `0x4400b124` (41 B)

Wrapper para carregar `PANA_DVD.FRM` de CD (`cd_load_pana_dvd_b124`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4400b124 (cd_load_pana_dvd_b124) @ 0x4400b124 (size=41 B, req=0x4400b124) -->
```c
// Callers (2): 0x44007667, 0x4400a87f
// Referenced strings/literals:
//   0x44035215: 'PANA_DVD.FRM'
void FUN_4400b124(undefined4 param_1,undefined4 param_2,undefined4 param_3)

{
  FUN_4400ab0a(param_1,param_2,param_3,s_PANA_DVD_FRM_44035215);
  return;
}
```

#### `FUN_4400b14d` (`cd_read_sectors_chunked_b14d`) — `0x4400b14d` (189 B)

Leitura em blocos/chunks de setores de CD (`cd_read_sectors_chunked_b14d`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4400b14d (cd_read_sectors_chunked_b14d) @ 0x4400b14d (size=189 B, req=0x4400b14d) -->
```c
// Callers (4): 0x4400ab33, 0x4400ac0e, 0x4400aec7, 0x4400afee
undefined4 FUN_4400b14d(void)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  do {
    if (puVar1[3] == 0) {
      puVar1[-5] = puVar1[3];
LAB_4400b202:
      return puVar1[-5];
    }
    if ((uint)puVar1[3] < 0x101) {
      puVar1[-1] = puVar1[3];
    }
    else {
      puVar1[-1] = 0x100;
    }
    puVar1[-3] = 0x1e38000;
    uVar2 = FUN_440098fc(0,puVar1[-3],puVar1[4],puVar1[-1]);
    puVar1[-4] = uVar2;
    if ((int)puVar1[-4] < 0) {
      puVar1[-5] = puVar1[-4];
      goto LAB_4400b202;
    }
    puVar1[-2] = puVar1[-1] << 0xb;
    uVar2 = FUN_4400a5f5(puVar1[-3],puVar1[2],puVar1[-2]);
    puVar1[-4] = uVar2;
    if ((int)puVar1[-4] < 0) {
      puVar1[-5] = puVar1[-4];
      goto LAB_4400b202;
    }
    puVar1[4] = puVar1[-1] + puVar1[4];
    puVar1[3] = puVar1[-1] - puVar1[3];
    puVar1[2] = puVar1[-2] + puVar1[2];
  } while( true );
}
```

#### `FUN_4400b20a` (`joliet_utf16_to_ascii_b20a`) — `0x4400b20a` (276 B)

Conversão de nomes UTF-16BE Joliet para ASCII (`joliet_utf16_to_ascii_b20a`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4400b20a (joliet_utf16_to_ascii_b20a) @ 0x4400b20a (size=276 B, req=0x4400b20a) -->
```c
// Callers (1): 0x4400ada0
int FUN_4400b20a(char *param_1,undefined1 *param_2,int param_3)

{
  char cVar1;
  int iVar2;
  int iStack_14;
  int iStack_10;
  int iStack_c;
  
  cVar1 = *param_1;
  if ((((cVar1 == '\b') || (cVar1 == '\x10')) || (cVar1 == -2)) || (cVar1 == -1)) {
    iStack_c = 0;
    iStack_10 = 1;
    if ((cVar1 == '\b') || (cVar1 == -2)) {
      for (; iStack_10 < param_3; iStack_10 = iStack_10 + 1) {
        param_2[iStack_c] = param_1[iStack_10];
        iStack_c = iStack_c + 1;
      }
      iStack_14 = iStack_c;
    }
    else {
      while (iVar2 = iStack_10, iStack_10 < param_3) {
        if (param_1[iStack_10] != '\0') {
          param_2[iStack_c] = param_1[iStack_10];
          iStack_c = iStack_c + 1;
        }
        iStack_10 = iStack_10 + 1;
        param_2[iStack_c] = param_1[iStack_10];
        iStack_c = iStack_c + 1;
        iStack_10 = iVar2 + 2;
      }
      iStack_14 = iStack_c;
    }
  }
  else {
    *param_2 = 0;
    iStack_14 = -1;
  }
  return iStack_14;
}
```

#### `FUN_4400b31e` (`cd_load_digadiag_b31e`) — `0x4400b31e` (38 B)

Wrapper para carregar pacote `DIGADIAG` de CD (`cd_load_digadiag_b31e`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4400b31e (cd_load_digadiag_b31e) @ 0x4400b31e (size=38 B, req=0x4400b31e) -->
```c
// Callers (1): 0x44007e05
void FUN_4400b31e(undefined4 param_1,undefined4 param_2,undefined4 param_3,undefined4 param_4)

{
  FUN_4400ab0a(param_1,param_2,param_3,param_4);
  return;
}
```

#### `FUN_4400fb67` (`rtsd_cmd_wait_fb67`) — `0x4400fb67` (216 B)

Espera conclusão de comando no controlador IDE RTSD (`rtsd_cmd_wait_fb67`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4400fb67 (rtsd_cmd_wait_fb67) @ 0x4400fb67 (size=216 B, req=0x4400fb67) -->
```c
// Callers (1): 0x440104c1
// Referenced strings/literals:
//   0x4403821f: '[RTSD] Request Sense Error: %04X \n'
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

uint FUN_4400fb67(char param_1,undefined4 param_2)

{
  int iVar1;
  undefined4 uVar2;
  uint uVar3;
  undefined4 uVar4;
  int iVar5;
  undefined1 *puVar6;
  byte bStackY_f4;
  undefined1 auStack_6c [4];
  undefined1 auStack_68 [4];
  undefined4 uStack_64;
  undefined4 uStack_60;
  undefined4 uStack_58;
  undefined2 auStack_54 [2];
  undefined4 uStack_50;
  undefined4 uStack_4c;
  undefined1 auStack_48 [40];
  undefined4 uStack_20;
  undefined4 uStack_1c;
  undefined4 uStack_18;
  undefined4 uStack_14;
  
  uVar3 = 0;
  do {
    auStack_48[uVar3] = auStack_48[uVar3];
    uVar3 = uVar3 + 1;
  } while (uVar3 < 0xc);
  puVar6 = auStack_6c;
  uVar3 = (uint)bStackY_f4;
  uVar4 = _DAT_50573400;
  iVar5 = _DAT_505733d0;
  if (param_1 == '\x01') {
    uVar4 = _DAT_50573438;
    iVar5 = _DAT_505733d0 + 0x800;
  }
  FUN_44011377(iVar5,puVar6,auStack_68);
  uVar2 = 0x12;
  iVar1 = uVar3 * 0x3c;
  uVar3 = FUN_4401b157((&DAT_505732e6)[iVar1],uVar4,auStack_54,puVar6,700);
  if (CARRY4(uVar3,uVar3)) {
    uart0_printf(s__RTSD__Request_Sense_Error___04X_4403821f,uVar3);
    (&DAT_50573306)[iVar1] = 0xf8;
  }
  else {
    FUN_44011537(iVar5,param_2,uVar2);
    (&DAT_50573306)[iVar1] = (&DAT_50573306)[iVar1];
  }
  return uVar3;
}
```

#### `FUN_4400fc3f` (`rtsd_channel_ready_fc3f`) — `0x4400fc3f` (942 B)

Prepara canal RTSD / espera `DRDY`/prontidão IDE (`rtsd_channel_ready_fc3f`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4400fc3f (rtsd_channel_ready_fc3f) @ 0x4400fc3f (size=942 B, req=0x4400fc3f) -->
```c
// Callers (1): 0x44010072
// Referenced strings/literals:
//   0x4403825f: '[RTSD] HDD spin-up error (%X)\n'
//   0x44038242: '[RTSD] HDD already spin-up \n'
//   0x4403829a: '[RTSD] HDD Auto Spin up Mode \n'
//   0x4403827e: '[RTSD] Identify Device NG \n'
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

undefined * FUN_4400fc3f(byte param_1,byte param_2)

{
  undefined1 *puVar1;
  uint uVar2;
  uint uVar3;
  undefined *puVar4;
  char *__format;
  int iVar5;
  int iVar6;
  undefined4 uVar7;
  int iVar8;
  byte *pbVar9;
  uint uVar10;
  byte bStack_76;
  byte bStack_75;
  byte local_74 [2];
  byte abStack_72 [2];
  byte abStack_70 [8];
  undefined1 auStack_68 [4];
  undefined4 uStack_64;
  undefined4 uStack_60;
  undefined4 uStack_58;
  undefined2 auStack_54 [2];
  undefined4 uStack_50;
  undefined4 uStack_4c;
  undefined2 uStack_3c;
  undefined2 uStack_3a;
  undefined2 uStack_38;
  undefined2 uStack_36;
  undefined2 uStack_34;
  undefined4 uStack_20;
  undefined4 uStack_1c;
  undefined4 uStack_18;
  undefined4 uStack_14;
  
  abStack_70[0] = 2;
  abStack_70[1] = 0;
  abStack_70[2] = 1;
  abStack_70[3] = 0;
  if (DAT_505733d4 == '\0') {
    abStack_70[1] = 1;
    abStack_70[2] = 0;
  }
  uVar2 = (uint)param_1;
  uVar3 = (uint)param_2;
  iVar8 = _DAT_505733d0;
  if (uVar2 == 1) {
    iVar8 = _DAT_505733d0 + 0x800;
  }
  uVar10 = (uint)(uVar2 == 1);
  if (local_74[uVar3 + uVar2 * 2 + 4] == 1) {
    pbVar9 = local_74 + 8;
    FUN_44011377(iVar8,pbVar9,auStack_68);
    uVar7 = _DAT_50573400;
    if (uVar2 == 1) {
      uVar7 = _DAT_50573438;
    }
    iVar6 = uVar2 * 2;
    puVar4 = (undefined *)FUN_4401b157(iVar6 + uVar3,uVar7,auStack_54,pbVar9,4000);
    if ((int)puVar4 < 0) {
      if (puVar4 != &UNK_80000409) {
        FUN_4401aedd(uVar10 & 0xffff,2);
        uart0_printf(s__RTSD__HDD_spin_up_error___X__4403825f,puVar4);
        puVar1 = (undefined1 *)((uVar3 & 1) + iVar6 + 0x505732d8);
        *puVar1 = *puVar1;
        return puVar4;
      }
      uart0_printf(s__RTSD__HDD_already_spin_up_44038242);
    }
    uVar10 = uVar10 & 0xffff;
    FUN_4401a94d(uVar10);
    FUN_44011377(iVar8,pbVar9,auStack_68);
    uVar7 = _DAT_50573400;
    if (uVar2 == 1) {
      uVar7 = _DAT_50573438;
    }
    iVar6 = uVar2 * 2;
    puVar4 = (undefined *)FUN_4401b157(iVar6 + uVar3,uVar7,auStack_54,pbVar9,0x32);
    uVar3 = uVar3 & 1;
    iVar6 = iVar6 + 0x505732d8;
    if ((int)puVar4 < 0) {
      FUN_4401aedd(uVar10,2);
      *(undefined1 *)(uVar3 + iVar6) = *(undefined1 *)(uVar3 + iVar6);
      return puVar4;
    }
    *(undefined1 *)(uVar3 + iVar6) = 1;
    uVar7 = 2;
    FUN_44011537(iVar8,local_74 + 2,2);
    FUN_44011537(iVar8 + 0xac,local_74,uVar7);
    if ((abStack_72[0] & 4) == 0) {
      if ((local_74[0] & 0x20) != 0) goto LAB_4400ff2e;
      __format = s__RTSD__HDD_Auto_Spin_up_Mode_4403829a;
    }
    else {
      __format = s__RTSD__Identify_Device_NG_4403827e;
    }
    uart0_printf(__format);
  }
  else {
    if (local_74[uVar3 + uVar2 * 2 + 4] == 2) {
      FUN_4401a94d(uVar10);
      pbVar9 = local_74 + 8;
      FUN_44011377(iVar8,pbVar9,auStack_68);
      uVar7 = _DAT_50573400;
      if (uVar2 == 1) {
        uVar7 = _DAT_50573438;
      }
      iVar6 = uVar2 * 2;
      iVar5 = FUN_4401b157(iVar6 + uVar3,uVar7,auStack_54,pbVar9,0x32);
      uVar3 = uVar3 & 1;
      iVar6 = iVar6 + 0x505732d8;
      if (-1 < iVar5) {
        *(undefined1 *)(uVar3 + iVar6) = 2;
        goto LAB_4400ff2e;
      }
      FUN_4401aedd(uVar10,2);
      puVar1 = (undefined1 *)(uVar3 + iVar6);
    }
    else {
      puVar1 = (undefined1 *)((uVar3 & 1) + uVar2 * 2 + 0x505732d8);
      bStack_76 = param_2;
      bStack_75 = param_1;
    }
    *puVar1 = *puVar1;
  }
LAB_4400ff2e:
  uVar3 = (uint)bStack_76;
  uVar2 = (uint)bStack_75;
  iVar6 = uVar2 * 2;
  puVar1 = (undefined1 *)(uVar3 + iVar6 + 0x505732e0);
  *puVar1 = 0x32;
  if (*(char *)(iVar6 + 0x505732d8 + uVar3) == '\0') {
    *puVar1 = 0xff;
  }
  else {
    FUN_4401afe7(iVar6 + uVar3,1,2);
    pbVar9 = local_74 + 8;
    FUN_44011377(iVar8,pbVar9,auStack_68);
    uVar7 = _DAT_50573400;
    if (uVar2 == 1) {
      uVar7 = _DAT_50573438;
    }
    iVar8 = uVar2 * 2;
    iVar6 = FUN_4401b157(iVar8 + uVar3,uVar7,auStack_54,pbVar9,0x32);
    if (iVar6 != 0) {
      *(undefined1 *)(uVar3 + iVar8 + 0x505732e0) = 0xff;
    }
  }
  return (undefined *)0x0;
}
```

#### `FUN_4400ffed` (`rtsd_ide_exec_ffed`) — `0x4400ffed` (1382 B)

Executor baixo nível de comandos ATA/ATAPI no controlador RTSD (`rtsd_ide_exec_ffed`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4400ffed (rtsd_ide_exec_ffed) @ 0x4400ffed (size=1382 B, req=0x4400ffed) -->
```c
// Callers (2): 0x440106cf, 0x44010f3f
// Referenced strings/literals:
//   0x440382b9: '[RTSD] DAVIS IDE ERROR: %04X \n'
//   0x440382d8: '[RTSD] SRST Command Set (Timeout Occured) \n'
undefined4 FUN_4400ffed(uint param_1,uint param_2)

{
  bool bVar1;
  byte *pbVar2;
  uint uVar3;
  undefined1 uVar4;
  uint uVar5;
  int iVar6;
  undefined *puVar7;
  byte bVar8;
  uint uVar9;
  undefined1 *puVar10;
  uint uVar11;
  undefined4 uVar12;
  undefined1 auStack_8c [4];
  uint uStack_88;
  uint uStack_84;
  uint uStack_80;
  undefined4 uStack_6c;
  undefined1 auStack_68 [4];
  undefined4 uStack_64;
  int iStack_60;
  undefined1 uStack_5c;
  undefined4 uStack_58;
  ushort auStack_54 [2];
  undefined4 uStack_50;
  undefined4 uStack_4c;
  undefined1 auStack_48 [12];
  undefined2 uStack_3c;
  undefined2 uStack_3a;
  undefined2 uStack_38;
  undefined2 uStack_36;
  undefined2 uStack_34;
  undefined4 uStack_2c;
  undefined4 uStack_28;
  undefined2 uStack_24;
  undefined2 uStack_22;
  undefined4 uStack_20;
  undefined4 uStack_1c;
  undefined4 uStack_18;
  undefined4 uStack_14;
  
  uVar3 = param_2 & 0xff;
  iVar6 = uVar3 * 0x3c;
  bVar8 = (&DAT_505732e6)[iVar6];
  uVar5 = bVar8 & 1;
  (&DAT_50573306)[iVar6] = (&DAT_50573306)[iVar6];
  *(undefined1 *)(iVar6 + 0x50573307) = *(undefined1 *)(iVar6 + 0x50573307);
  if ((bVar8 & 1) == 0) {
    if (*(char *)((param_1 & 0xff) + 0x505732b8) < '\0') goto LAB_4401006d;
LAB_44010051:
    if ((((&DAT_505732e4)[iVar6] & 0x10) == 0) &&
       (*(byte *)((param_1 & 0xff) * 2 + 0x505732e0 + uVar5) == 0xffffffff)) goto LAB_4401006d;
  }
  else {
    if ((uVar5 != 1) || ((*(byte *)((param_1 & 0xff) + 0x505732b8) & 0x40) == 0)) goto LAB_44010051;
LAB_4401006d:
    uVar9 = param_1 & 0xff;
    uVar11 = uVar5;
    iVar6 = FUN_4400fc3f(uVar9,uVar5);
    if ((iVar6 != 0) || (*(char *)(uVar9 * 2 + 0x505732d8 + uVar11) == '\0')) {
      *(uint *)(&DAT_505732e4 + uVar3 * 0x3c) =
           *(uint *)(&DAT_505732e4 + uVar3 * 0x3c) & 0xfffffffc | 0x80;
      (&DAT_50573306)[uVar3 * 0x3c] = 0xe8;
      goto LAB_44010547;
    }
  }
  if (uVar5 == 0) {
    pbVar2 = (byte *)((param_1 & 0xff) + 0x505732b8);
    *pbVar2 = *pbVar2 | 0x7f;
  }
  else {
    pbVar2 = (byte *)((param_1 & 0xff) + 0x505732b8);
    *pbVar2 = *pbVar2 | 0xbf;
  }
  if ((byte)(&DAT_505732e8)[uVar3 * 0x3c] == 0xffffffff) {
    FUN_44011377(*(undefined4 *)(&DAT_50573314 + (param_2 & 0xff) * 0x3c),&uStack_6c,auStack_68);
  }
  else {
    bVar8 = 0;
    do {
      auStack_48[bVar8] = (&DAT_505732e8)[(uint)bVar8 + uVar3 * 0x3c];
      bVar8 = bVar8 + 1;
    } while (bVar8 < 0xc);
    FUN_44011377(*(undefined4 *)(&DAT_50573314 + uVar3 * 0x3c),&uStack_6c,auStack_68);
  }
  uVar3 = param_2 & 0xff;
  uVar12 = *(undefined4 *)(&DAT_5057331c + uVar3 * 0x3c);
  uVar5 = 30000;
  if (*(ushort *)(&DAT_5057330a + uVar3 * 0x3c) < 0x7531) {
    uVar5 = (uint)*(ushort *)(&DAT_5057330a + uVar3 * 0x3c);
  }
  FUN_4401083d(uStack_6c,auStack_8c,0x20);
  iVar6 = uVar3 * 0x3c;
  puVar10 = &DAT_505732e4 + iVar6;
  puVar7 = (undefined *)FUN_4401b157((&DAT_505732e6)[iVar6],uVar12,auStack_54,&uStack_6c,uVar5);
  if ((int)puVar7 < 0) {
    if (puVar7 == &UNK_80000409) {
      *(undefined1 *)(iVar6 + 0x50573307) = 2;
      (&DAT_50573306)[iVar6] = (&DAT_50573306)[iVar6];
      *(undefined4 *)(puVar10 + 0x2c) = 0;
      if (*(char *)(iVar6 + 0x50573308) != '\0') {
        if ((byte)(&DAT_505732e8)[iVar6] == 0xffffffff) {
          FUN_44011c57(uVar12);
        }
        bVar1 = (puVar10[1] & 8) != 0;
        if (bVar1) {
          uStack_84 = uStack_80;
        }
        uVar3 = uStack_84 & 0x80000000;
        uVar5 = iStack_60 + (uStack_84 & 0x7fffffff);
        if (uStack_88 <= uVar5) {
          uVar3 = uVar3 ^ 0x80000000;
          uVar5 = iStack_60 - (uStack_88 - (uStack_84 & 0x7fffffff));
        }
        FUN_4401093e(uStack_6c,uVar3 | uVar5,bVar1);
      }
    }
    else {
      if (puVar7 == &UNK_80000410) {
        *(undefined1 *)(iVar6 + 0x50573307) = 4;
        (&DAT_50573306)[iVar6] = (&DAT_50573306)[iVar6];
        iVar6 = 0;
        goto LAB_44010435;
      }
      *(undefined1 *)(iVar6 + 0x50573307) = *(undefined1 *)(iVar6 + 0x50573307);
      if ((puVar7 == &UNK_80000401) ||
         ((uVar4 = 0xc3, puVar7 != &UNK_8000040e && (uVar4 = 0xc0, puVar7 != &DAT_8000040f)))) {
        uVar4 = 0xe8;
      }
      (&DAT_50573306)[iVar6] = uVar4;
      *(undefined4 *)(&DAT_50573310 + uVar3 * 0x3c) = 0;
      uart0_printf(s__RTSD__DAVIS_IDE_ERROR___04X_440382b9);
    }
  }
  else {
    if (((uint)puVar7 & 1) != 0) {
      puVar7 = puVar7 + 1;
    }
    *(undefined1 *)(iVar6 + 0x50573307) = *(undefined1 *)(iVar6 + 0x50573307);
    (&DAT_50573306)[iVar6] = (&DAT_50573306)[iVar6];
    iVar6 = *(int *)(puVar10 + 0x28) - (int)puVar7;
LAB_44010435:
    *(int *)(puVar10 + 0x2c) = iVar6;
  }
  uVar5 = param_2 & 0xff;
  iVar6 = uVar5 * 0x3c;
  if ((((&DAT_50573306)[iVar6] == '\0') && (*(char *)(iVar6 + 0x50573307) == '\x02')) &&
     (auStack_54[0] == 0xffffffa0)) {
    if (((&DAT_505732e4)[iVar6] & 8) == 0) {
      FUN_4400fb67(param_1 & 0xff,iVar6 + 0x505732f4,uVar5);
    }
  }
  else {
    iVar6 = uVar5 * 0x3c;
    if (((&DAT_50573306)[iVar6] != '\0') || (*(char *)(iVar6 + 0x50573307) != '\0')) {
      FUN_44018238((&DAT_505732e6)[iVar6]);
    }
  }
  iVar6 = (param_2 & 0xff) * 0x3c;
  *(uint *)(&DAT_505732e4 + iVar6) = *(uint *)(&DAT_505732e4 + iVar6) & 0xfffffffc | 0x80;
  if ((&DAT_50573306)[iVar6] != '\0') {
    uart0_printf(s__RTSD__SRST_Command_Set__Timeout_440382d8);
    param_1 = param_1 & 0xff;
    FUN_4401aedd(param_1,2);
    *(byte *)(param_1 + 0x505732b8) = *(byte *)(param_1 + 0x505732b8) | 0xc0;
  }
LAB_44010547:
  FUN_4400f82a(param_2 & 0xff);
  return 0;
}
```

#### `FUN_4401a5cd` (`davis_probe_master_slave_a5cd`) — `0x4401a5cd` (896 B)

Detecção Master/Slave no barramento IDE DAVIS (`davis_probe_master_slave_a5cd`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4401a5cd (davis_probe_master_slave_a5cd) @ 0x4401a5cd (size=896 B, req=0x4401a5cd) -->
```c
// Callers (2): 0x4401aa00, 0x4401aa0e
// Referenced strings/literals:
//   0x440388ac: 'DAVIS>> Set Feature ( PIO ) Error \n'
//   0x440388d0: 'DAVIS>> MultiWord DMA MODE2 is not supported.\n'
//   0x440388ff: 'DAVIS>> Set Feature ( Enable Rel Int ) Error \n'
/* WARNING: Removing unreachable block (ram,0x4401a70e) */

undefined * FUN_4401a5cd(uint param_1,ushort param_2)

{
  short sVar1;
  int iVar2;
  int iVar3;
  char *pcVar4;
  short *psVar5;
  uint uVar6;
  undefined1 *puVar7;
  undefined2 uVar8;
  undefined *puVar9;
  uint uVar10;
  uint uVar11;
  undefined *puVar12;
  uint in_R5;
  undefined1 *puVar13;
  uint *puVar14;
  ushort uStack_7c;
  uint uStack_78;
  int iStack_74;
  undefined1 auStack_70 [52];
  undefined1 auStack_3c [24];
  short asStack_24 [2];
  undefined4 uStack_20;
  undefined4 uStack_1c;
  undefined4 uStack_18;
  undefined4 uStack_14;
  
  uVar10 = (uint)param_2;
  uVar11 = param_1 & 0xffff;
  iVar2 = uVar10 + uVar11 * 0xb4;
  if (*(char *)(iVar2 + 0x50574070) == '\x01') {
    iVar2 = (int)&uStack_20 + uVar10 * 2;
    uVar8 = 1;
LAB_4401a623:
    *(undefined2 *)(iVar2 + -4) = uVar8;
  }
  else {
    if (*(char *)(iVar2 + 0x50574072) != '\x01') {
      iVar2 = (int)&uStack_20 + uVar10 * 2;
      uVar8 = 2;
      goto LAB_4401a623;
    }
    asStack_24[uVar10] = 0;
  }
  uStack_7c = param_2;
  if (asStack_24[uVar10] != 2) {
    in_R5 = 0;
    do {
      puVar12 = (undefined *)FUN_44018a85(1,uVar11);
      if (puVar12 != (undefined *)0x0) {
        return puVar12;
      }
      psVar5 = asStack_24 + uVar10;
      puVar14 = &uStack_78;
      iVar2 = FUN_44018fb3(*psVar5,uVar11,uVar10,puVar14);
      uVar6 = uStack_78;
      if (iStack_74 == 1) {
        FUN_44018e74(uVar11,uVar10);
        uVar6 = uStack_78;
      }
      puVar12 = (undefined *)FUN_44018a85(1,uVar11);
      if (puVar12 != (undefined *)0x0) {
        return puVar12;
      }
      iVar3 = FUN_44018fb3(*psVar5,uVar11,uVar10,puVar14);
      uVar6 = uVar6 & uStack_78;
      if (iStack_74 == 1) {
        FUN_44018e74(uVar11,uVar10);
      }
      if (iVar2 == iVar3) {
        if (iVar2 != 0) break;
        if ((uVar6 & 0xffff) != 0) goto LAB_4401a6d6;
      }
      in_R5 = in_R5 + 1;
    } while ((in_R5 & 0xff) < 5);
    if (iVar2 == 0) {
LAB_4401a6d6:
      if ((in_R5 & 0xff) != 5) goto LAB_4401a6ed;
    }
    asStack_24[uVar10] = 2;
  }
LAB_4401a6ed:
  puVar12 = (undefined *)0x0;
  uVar10 = (uint)uStack_7c;
  sVar1 = asStack_24[uVar10];
  if (sVar1 == 1) {
    iVar3 = uVar10 + (param_1 & 0xffff) * 0xb4;
    iVar2 = (param_1 & 0xffff) * 0x5a;
    if (*(char *)(iVar3 + 0x50574072) != '\x01') {
      *(undefined1 *)(iVar3 + 0x50574070) = 1;
      *(undefined2 *)((uVar10 + iVar2) * 2 + 0x5057409e) = 1;
      goto LAB_4401a7e3;
    }
    *(undefined1 *)(iVar3 + 0x50574072) = *(undefined1 *)(iVar3 + 0x50574072);
  }
  else if (sVar1 == 0) {
    iVar3 = uVar10 + (param_1 & 0xffff) * 0xb4;
    pcVar4 = (char *)(iVar3 + 0x50574070);
    iVar2 = (param_1 & 0xffff) * 0x5a;
    if (*pcVar4 != '\x01') {
      *(undefined1 *)(iVar3 + 0x50574072) = 1;
      *(undefined2 *)((uVar10 + iVar2) * 2 + 0x5057409e) = 1;
      goto LAB_4401a7e3;
    }
    *pcVar4 = *pcVar4;
  }
  else {
    if (sVar1 != 2) goto LAB_4401a7e3;
    uVar11 = param_1 & 0xffff;
    iVar2 = uVar10 + uVar11 * 0xb4;
    pcVar4 = (char *)(iVar2 + 0x50574070);
    if ((*(char *)(iVar2 + 0x50574072) != '\x01') && (*pcVar4 != '\x01')) {
      *(undefined2 *)((uVar10 + uVar11 * 0x5a) * 2 + 0x5057409e) = 0;
      goto LAB_4401a7e3;
    }
    *(undefined1 *)(iVar2 + 0x50574072) = *(undefined1 *)(iVar2 + 0x50574072);
    *pcVar4 = *pcVar4;
    iVar2 = uVar11 * 0x5a;
  }
  *(undefined2 *)((uVar10 + iVar2) * 2 + 0x5057409e) = 0;
  puVar12 = &UNK_8000040d;
LAB_4401a7e3:
  param_1 = param_1 & 0xffff;
  iVar2 = uVar10 + param_1 * 0xb4;
  if ((*(char *)(iVar2 + 0x50574070) == '\x01') || (*(char *)(iVar2 + 0x50574072) == '\x01')) {
    uVar11 = in_R5 & 0xffff;
    puVar13 = auStack_70;
    puVar7 = auStack_3c;
    puVar9 = (undefined *)FUN_4401a438(uVar11,0,puVar13,puVar7,1000);
    if ((CARRY4((uint)puVar9,(uint)puVar9)) &&
       (FUN_44031747(s_DAVIS>>_Set_Feature___PIO___Erro_440388ac), -1 < (int)puVar12)) {
      puVar12 = puVar9;
    }
    puVar9 = (undefined *)FUN_4401a438(uVar11,0,puVar13,puVar7,1000);
    if ((CARRY4((uint)puVar9,(uint)puVar9)) &&
       (FUN_44031747(s_DAVIS>>_MultiWord_DMA_MODE2_is_n_440388d0), -1 < (int)puVar12)) {
      puVar12 = puVar9;
    }
  }
  if (((*(char *)(param_1 * 0xb4 + uVar10 * 0x2c + 0x505740c8) == '\x01') &&
      (puVar9 = (undefined *)
                FUN_4401a438((uint)uStack_7c + param_1 * 2 & 0xffff,0,auStack_70,auStack_3c,1000),
      CARRY4((uint)puVar9,(uint)puVar9))) &&
     (FUN_44031747(s_DAVIS>>_Set_Feature___Enable_Rel_440388ff), -1 < (int)puVar12)) {
    puVar12 = puVar9;
  }
  return puVar12;
}
```

#### `FUN_4401aa1f` (`davis_bus_init_aa1f`) — `0x4401aa1f` (1214 B)

Inicialização completa do barramento DAVIS (`davis_bus_init_aa1f`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4401aa1f (davis_bus_init_aa1f) @ 0x4401aa1f (size=1214 B, req=0x4401aa1f) -->
```c
// Callers (1): 0x4401af4f
// Referenced strings/literals:
//   0x4403169a: 'Ï\x80øþô\xa0'
//   0x440388ac: 'DAVIS>> Set Feature ( PIO ) Error \n'
//   0x44038944: 'DAVIS>> Multiword DMA MODE2 is not supported.\n'
//   0x440388ff: 'DAVIS>> Set Feature ( Enable Rel Int ) Error \n'
/* WARNING: Removing unreachable block (ram,0x4401ac8b) */

undefined * FUN_4401aa1f(void)

{
  int iVar1;
  char *pcVar2;
  ushort *puVar3;
  undefined *puVar4;
  short sVar5;
  ushort uVar6;
  undefined2 uVar7;
  int iVar8;
  uint uVar9;
  undefined *puVar10;
  uint uVar11;
  uint uVar12;
  uint in_R4;
  uint uVar13;
  byte bVar14;
  uint uVar15;
  undefined1 *puVar16;
  uint *puVar17;
  undefined1 *puVar18;
  ushort uStack_82;
  uint uStack_80;
  int iStack_7c;
  undefined1 auStack_78 [8];
  undefined1 auStack_70 [52];
  undefined1 auStack_3c [24];
  ushort auStack_24 [2];
  undefined4 uStack_20;
  undefined4 uStack_1c;
  undefined4 uStack_18;
  undefined4 uStack_14;
  
  uVar15 = 0;
  do {
    uVar11 = uVar15 & 0xff;
    iVar8 = FUN_44017fd9(auStack_78);
    if (iVar8 == 0) {
      uVar12 = 0;
      do {
        uVar13 = (uint)uStack_82;
        FUN_44016703(0xff10,uVar13,uVar11);
        FUN_4401660b(1,uVar13);
        FUN_440188fc(2,uVar13,0x81,0);
        sVar5 = FUN_440166c1(0x38,uVar13);
        if ((sVar5 == 0xff) || (uVar9 = FUN_440166c1(0x18,uVar13), (uVar9 & 0x10) == uVar11 * 0x10))
        break;
        puVar4 = (undefined *)FUN_44018a85(1,uVar13);
        if (puVar4 != (undefined *)0x0) {
          return puVar4;
        }
        uVar12 = uVar12 + 1;
      } while ((uVar12 & 0xff) < 100);
      uVar6 = FUN_440166c1(0x38,uVar13);
      if (((uVar6 == 0xff) || ((uVar6 & 1) != 0)) || ((uVar12 & 0xff) == 100)) goto LAB_4401ab69;
      uVar12 = FUN_440166c1(0x10,uVar13);
      sVar5 = FUN_440166c1(0x14,uVar13);
      if (((uVar12 & 0xffff) == 0x14) && (sVar5 == 0xeb)) {
        iVar8 = (int)&uStack_20 + uVar11 * 2;
        uVar7 = 1;
        goto LAB_4401ab76;
      }
      in_R4 = 0;
      do {
        uVar6 = FUN_440166c1(0x38,uVar13);
        uVar6 = uVar6 & 0x40;
        if (uVar6 != 0) {
          iVar8 = (int)&uStack_20 + uVar11 * 2;
          uVar7 = 0;
          goto LAB_4401ab76;
        }
        FUN_4403169a(1);
        in_R4 = in_R4 + 1;
        if ((in_R4 & 0xff) == 3) {
          uVar12 = FUN_440166c1(0x38,uVar13);
          if ((uVar12 & 0x40) == 0) goto LAB_4401ab69;
          auStack_24[uVar11] = uVar6;
          break;
        }
      } while ((in_R4 & 0xff) < 3);
    }
    else {
LAB_4401ab69:
      iVar8 = (int)&uStack_20 + uVar11 * 2;
      uVar7 = 2;
LAB_4401ab76:
      *(undefined2 *)(iVar8 + -4) = uVar7;
    }
    uVar15 = uVar15 + 1;
  } while ((uVar15 & 0xff) < 2);
  uVar15 = 0;
  do {
    uVar12 = (uint)uStack_82;
    uVar11 = uVar15 & 0xff;
    if (auStack_24[uVar11] != 2) {
      in_R4 = 0;
      do {
        puVar4 = (undefined *)FUN_44018a85(1,uVar12);
        if (puVar4 != (undefined *)0x0) {
          return puVar4;
        }
        puVar3 = auStack_24 + uVar11;
        puVar17 = &uStack_80;
        iVar8 = FUN_44018fb3(*puVar3,uVar12,uVar11,puVar17);
        uVar13 = uStack_80;
        if (iStack_7c == 1) {
          FUN_44018e74(uVar12,uVar11);
          uVar13 = uStack_80;
        }
        puVar4 = (undefined *)FUN_44018a85(1,uVar12);
        if (puVar4 != (undefined *)0x0) {
          return puVar4;
        }
        iVar1 = FUN_44018fb3(*puVar3,uVar12,uVar11,puVar17);
        uVar13 = uVar13 & uStack_80;
        if (iStack_7c == 1) {
          FUN_44018e74(uVar12,uVar11);
        }
        if (iVar8 == iVar1) {
          if (iVar8 != 0) break;
          if ((uVar13 & 0xffff) != 0) goto LAB_4401ac41;
        }
        in_R4 = in_R4 + 1;
      } while ((in_R4 & 0xff) < 5);
      if (iVar8 == 0) {
LAB_4401ac41:
        if ((in_R4 & 0xff) != 5) goto LAB_4401ac58;
      }
      auStack_24[uVar11] = 2;
    }
LAB_4401ac58:
    uVar15 = uVar15 + 1;
  } while ((uVar15 & 0xff) < 2);
  puVar4 = (undefined *)0x0;
  bVar14 = 0;
  do {
    uVar15 = (uint)bVar14;
    uVar6 = auStack_24[uVar15];
    if (uVar6 == 1) {
      iVar1 = uVar15 + (uint)uStack_82 * 0xb4;
      iVar8 = (uint)uStack_82 * 0x5a;
      if (*(char *)(iVar1 + 0x50574072) == '\x01') {
        *(undefined1 *)(iVar1 + 0x50574072) = *(undefined1 *)(iVar1 + 0x50574072);
LAB_4401aceb:
        *(undefined2 *)((uVar15 + iVar8) * 2 + 0x5057409e) = 0;
LAB_4401ad40:
        puVar4 = &UNK_8000040d;
      }
      else {
        *(undefined1 *)(iVar1 + 0x50574070) = 1;
        *(undefined2 *)((uVar15 + iVar8) * 2 + 0x5057409e) = 1;
      }
    }
    else {
      if (uVar6 == 0) {
        iVar1 = uVar15 + (uint)uStack_82 * 0xb4;
        pcVar2 = (char *)(iVar1 + 0x50574070);
        iVar8 = (uint)uStack_82 * 0x5a;
        if (*pcVar2 == '\x01') {
          *pcVar2 = *pcVar2;
          *(undefined2 *)((uVar15 + iVar8) * 2 + 0x5057409e) = 0;
          goto LAB_4401ad40;
        }
        *(undefined1 *)(iVar1 + 0x50574072) = 1;
      }
      else {
        if (uVar6 != 2) goto LAB_4401ad5e;
        uVar11 = (uint)uStack_82;
        iVar8 = uVar15 + uVar11 * 0xb4;
        pcVar2 = (char *)(iVar8 + 0x50574070);
        if ((*(char *)(iVar8 + 0x50574072) == '\x01') || (*pcVar2 == '\x01')) {
          *(undefined1 *)(iVar8 + 0x50574072) = *(undefined1 *)(iVar8 + 0x50574072);
          *pcVar2 = *pcVar2;
          iVar8 = uVar11 * 0x5a;
          goto LAB_4401aceb;
        }
        iVar8 = uVar11 * 0x5a;
      }
      *(undefined2 *)((uVar15 + iVar8) * 2 + 0x5057409e) = 1;
    }
LAB_4401ad5e:
    bVar14 = bVar14 + 1;
    if (1 < bVar14) {
      uVar15 = 0;
      do {
        uVar11 = uVar15 & 0xffff;
        uVar12 = (uint)uStack_82;
        iVar8 = uVar11 + uVar12 * 0xb4;
        if ((*(char *)(iVar8 + 0x50574070) == '\x01') || (*(char *)(iVar8 + 0x50574072) == '\x01'))
        {
          in_R4 = in_R4 & 0xffff;
          puVar18 = auStack_70;
          puVar16 = auStack_3c;
          puVar10 = (undefined *)FUN_4401a438(in_R4,0,puVar18,puVar16,1000);
          if ((CARRY4((uint)puVar10,(uint)puVar10)) &&
             (FUN_44031747(s_DAVIS>>_Set_Feature___PIO___Erro_440388ac), -1 < (int)puVar4)) {
            puVar4 = puVar10;
          }
          puVar10 = (undefined *)FUN_4401a438(in_R4,0,puVar18,puVar16,1000);
          if ((CARRY4((uint)puVar10,(uint)puVar10)) &&
             (FUN_44031747(s_DAVIS>>_Multiword_DMA_MODE2_is_n_44038944), -1 < (int)puVar4)) {
            puVar4 = puVar10;
          }
        }
        if (((*(char *)(uVar12 * 0xb4 + uVar11 * 0x2c + 0x505740c8) == '\x01') &&
            (puVar10 = (undefined *)
                       FUN_4401a438(uVar15 + uVar12 * 2 & 0xffff,0,auStack_70,auStack_3c,1000),
            CARRY4((uint)puVar10,(uint)puVar10))) &&
           (FUN_44031747(s_DAVIS>>_Set_Feature___Enable_Rel_440388ff), -1 < (int)puVar4)) {
          puVar4 = puVar10;
        }
        uVar15 = uVar15 + 1;
      } while ((uVar15 & 0xffff) < 2);
      return puVar4;
    }
  } while( true );
}
```

### 1.6 Criptografia Feistel (`0x44039aa8` + S-Box `0x44035d84`), `ExtrHead`/`ExtrFoot` & LZ (`uncompress`)

Pipeline completo de decifração e descompressão de contêineres `.FRM` / partições `ExtrHead`...`ExtrFoot`: chave de 15 bytes em `0x50480278` (ROM `0x44039aa8`), S-Box de 256 bytes em `0x44035d84`, Adler-32 com passo variável (`strided_adler32_1e5a3`) e motor LZ (`sub_4400bb14` + `uncompress`).

#### `FUN_4400a666` (`feistel_decrypt_buf_a666`) — `0x4400a666` (106 B)

Descriptografa buffer com cifra Feistel usando chaves em `0x50480278` (`feistel_decrypt_buf_a666`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4400a666 (feistel_decrypt_buf_a666) @ 0x4400a666 (size=106 B, req=0x4400a666) -->
```c
// Callers (4): 0x44006945, 0x44006a92, 0x440076b6, 0x4400a8b5
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_4400a666(void)

{
  undefined1 *puVar1;
  undefined1 auStack_4 [4];
  
  puVar1 = auStack_4;
  if (_DAT_50501738 == 0) {
    FUN_44011d42(0x50480278,0x5048027f,0x5050173c);
    _DAT_50501738 = 1;
  }
  while (0 < *(int *)(puVar1 + 0xc)) {
    FUN_44012029(0x5050173c,*(undefined4 *)(puVar1 + 8));
    *(int *)(puVar1 + 0xc) = *(int *)(puVar1 + 0xc) + -8;
    *(int *)(puVar1 + 8) = *(int *)(puVar1 + 8) + 8;
  }
  return;
}
```

#### `FUN_4400a6d0` (`extrhead_is_encrypted_a6d0`) — `0x4400a6d0` (99 B)

Verifica se cabeçalho `ExtrHead` indica payload criptografado (`extrhead_is_encrypted_a6d0`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4400a6d0 (extrhead_is_encrypted_a6d0) @ 0x4400a6d0 (size=99 B, req=0x4400a6d0) -->
```c
// Callers (1): 0x4400a7fb
// Referenced strings/literals:
//   0x44035148: '[fmu] error! list_sum is invalid.\n'
undefined4 FUN_4400a6d0(byte *param_1)

{
  undefined4 *puVar1;
  short sStack_a;
  byte *pbStack_8;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  sStack_a = 0;
  for (pbStack_8 = param_1; pbStack_8 < param_1 + 0x3e; pbStack_8 = pbStack_8 + 1) {
    sStack_a = sStack_a + (ushort)*pbStack_8;
  }
  if (*(short *)(param_1 + 0x3e) != sStack_a) {
    FUN_4400b3d9(1,s__fmu__error__list_sum_is_invalid_44035148);
    puVar1[-3] = 0;
  }
  return puVar1[-3];
}
```

#### `FUN_4400a733` (`find_firm_entry_a733`) — `0x4400a733` (254 B)

Busca entrada de firmware em tabela de diretório `.FRM` (`find_firm_entry_a733`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4400a733 (find_firm_entry_a733) @ 0x4400a733 (size=254 B, req=0x4400a733) -->
```c
// Callers (3): 0x44007499, 0x44007617, 0x44007a62
// Referenced strings/literals:
//   0x4403516b: 'DRV '
undefined4 FUN_4400a733(void)

{
  undefined4 *puVar1;
  int iVar2;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  do {
    if (0x7f < (int)puVar1[-1]) {
      puVar1[-2] = 0;
LAB_4400a829:
      return puVar1[-2];
    }
    iVar2 = FUN_4402c420(puVar1[-1] * 0x40 + 0x5050176c,puVar1[2],8);
    if ((((iVar2 == 0) &&
         (iVar2 = FUN_4402c420(puVar1[-1] * 0x40 + 0x5050175c,puVar1[4],4), iVar2 == 0)) &&
        (*(char *)(puVar1[-1] * 0x40 + 0x50501774) == *(char *)(puVar1 + 3))) &&
       (((iVar2 = FUN_4402c420(puVar1[4],&DAT_4403516b,4), iVar2 != 0 ||
         (iVar2 = FUN_4402c420(puVar1[-1] * 0x40 + 0x5050178c,puVar1[6],0xc), iVar2 == 0)) &&
        (iVar2 = FUN_4400a6d0(), iVar2 == 1)))) {
      *(int *)puVar1[5] = puVar1[-1] * 0x40 + 0x5050175c;
      puVar1[-2] = 1;
      goto LAB_4400a829;
    }
    puVar1[-1] = puVar1[-1] + 1;
  } while( true );
}
```

#### `FUN_4400a831` (`bytes_sum32_a831`) — `0x4400a831` (60 B)

Soma linear de 32 bits em bytes (`bytes_sum32_a831`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4400a831 (bytes_sum32_a831) @ 0x4400a831 (size=60 B, req=0x4400a831) -->
```c
// Callers (3): 0x44007561, 0x440076d9, 0x44007b73
int FUN_4400a831(byte *param_1,int param_2)

{
  int iStack_8;
  
  iStack_8 = 0;
  for (; param_2 != 0; param_2 = param_2 + -1) {
    iStack_8 = iStack_8 + (uint)*param_1;
    param_1 = param_1 + 1;
  }
  return iStack_8;
}
```

#### `FUN_4400b4ab` (`extrhead_hdr_check_b4ab`) — `0x4400b4ab` (81 B)

Validação de cabeçalho `ExtrHead` (`extrhead_hdr_check_b4ab`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4400b4ab (extrhead_hdr_check_b4ab) @ 0x4400b4ab (size=81 B, req=0x4400b4ab) -->
```c
// Callers (3): 0x440069a0, 0x4400bbb3, 0x4400bd0e
undefined4 FUN_4400b4ab(undefined4 param_1)

{
  undefined4 *puVar1;
  int iVar2;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  iVar2 = FUN_4400b52e(param_1);
  if (iVar2 == 0) {
    puVar1[-2] = 0;
  }
  else if ((*(char *)(puVar1 + 3) == '\0') || (iVar2 = FUN_4400b56b(puVar1[2]), iVar2 != 0)) {
    puVar1[-2] = *(int *)(puVar1[-1] + 0x20) + 8;
  }
  else {
    puVar1[-2] = 0;
  }
  return puVar1[-2];
}
```

#### `FUN_4400b52e` (`extrhead_magic_check_b52e`) — `0x4400b52e` (61 B)

Verifica magic string `"ExtrHead"` (`extrhead_magic_check_b52e`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4400b52e (extrhead_magic_check_b52e) @ 0x4400b52e (size=61 B, req=0x4400b52e) -->
```c
// Callers (5): 0x44006952, 0x4400b4c1, 0x4400b6fd, 0x4400bb79, 0x4400bcdb
// Referenced strings/literals:
//   0x4403526e: 'EXTRHEAD'
undefined4 FUN_4400b52e(undefined4 param_1)

{
  undefined1 *puVar1;
  int iVar2;
  undefined1 auStack_4 [4];
  
  puVar1 = auStack_4;
  iVar2 = FUN_4402c2b7(param_1,s_EXTRHEAD_4403526e,8);
  if (iVar2 == 0) {
    *(undefined4 *)(puVar1 + -8) = 1;
  }
  else {
    *(undefined4 *)(puVar1 + -8) = 0;
  }
  return *(undefined4 *)(puVar1 + -8);
}
```

#### `FUN_4400b56b` (`extrfoot_magic_check_b56b`) — `0x4400b56b` (74 B)

Verifica magic string de rodapé `"ExtrFoot"` (`extrfoot_magic_check_b56b`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4400b56b (extrfoot_magic_check_b56b) @ 0x4400b56b (size=74 B, req=0x4400b56b) -->
```c
// Callers (2): 0x4400b4da, 0x4400b71d
// Referenced strings/literals:
//   0x44035277: 'EXTRFOOT'
undefined4 FUN_4400b56b(int param_1)

{
  undefined1 *puVar1;
  int iVar2;
  undefined1 auStack_4 [4];
  
  puVar1 = auStack_4;
  iVar2 = FUN_4402c2b7(param_1 + *(int *)(param_1 + 0x20),s_EXTRFOOT_44035277,8);
  if (iVar2 == 0) {
    *(undefined4 *)(puVar1 + -0xc) = 1;
  }
  else {
    *(undefined4 *)(puVar1 + -0xc) = 0;
  }
  return *(undefined4 *)(puVar1 + -0xc);
}
```

#### `FUN_4400b5b5` (`extrhead_get_chk_b5b5`) — `0x4400b5b5` (44 B)

Lê campo de checksum do `ExtrHead` (`extrhead_get_chk_b5b5`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4400b5b5 (extrhead_get_chk_b5b5) @ 0x4400b5b5 (size=44 B, req=0x4400b5b5) -->
```c
// Callers (4): 0x440069b0, 0x4400b5f3, 0x4400bbc1, 0x4400bd19
bool FUN_4400b5b5(int param_1)

{
  return *(short *)(param_1 + 0xc) != 0;
}
```

#### `FUN_4400b61a` (`extrhead_get_dst_b61a`) — `0x4400b61a` (30 B)

Lê endereço de destino de carga do `ExtrHead` (`extrhead_get_dst_b61a`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4400b61a (extrhead_get_dst_b61a) @ 0x4400b61a (size=30 B, req=0x4400b61a) -->
```c
// Callers (3): 0x44006980, 0x4400bba2, 0x4400bd00
undefined4 FUN_4400b61a(int param_1)

{
  return *(undefined4 *)(param_1 + 0x24);
}
```

#### `FUN_4400b652` (`extrhead_verify_adler_b652`) — `0x4400b652` (116 B)

Verifica checksum Adler-32 strided do payload `ExtrHead` (`extrhead_verify_adler_b652`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4400b652 (extrhead_verify_adler_b652) @ 0x4400b652 (size=116 B, req=0x4400b652) -->
```c
// Callers (3): 0x4400b876, 0x4400b90a, 0x4400b9c0
// Referenced strings/literals:
//   0x44035280: 'HEADER:%08X, DECOMP:%08X\n\n'
undefined4 FUN_4400b652(int param_1)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack_8;
  undefined4 uStack_4;
  
  puVar1 = &uStack_8;
  if ((*(int *)(param_1 + 0x28) != 0) && (*(char *)(param_1 + 0x2c) != '\0')) {
    uVar2 = FUN_4401e5a3(*(undefined4 *)(param_1 + 0x14),*(undefined4 *)(param_1 + 0x10),
                         *(undefined1 *)(param_1 + 0x2c));
    puVar1[-1] = uVar2;
    uart0_printf(s_HEADER__08X__DECOMP__08X_44035280,*(undefined4 *)(puVar1[3] + 0x28),puVar1[-1]);
    puVar1[-2] = puVar1[-1] - *(int *)(puVar1[3] + 0x28);
  }
  return puVar1[-2];
}
```

#### `FUN_4400b6c6` (`extrhead_unpack_b6c6`) — `0x4400b6c6` (828 B)

Desempacota payload `ExtrHead`: decifra (se cifrado), valida Adler-32 e descomprime (`extrhead_unpack_b6c6`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4400b6c6 (extrhead_unpack_b6c6) @ 0x4400b6c6 (size=828 B, req=0x4400b6c6) -->
```c
// Callers (3): 0x44006aa6, 0x4400bbee, 0x4400bd43
// Referenced strings/literals:
//   0x4403529b: 'no header!\n'
//   0x440352a7: 'no footer!\n'
//   0x440352b3: 'Destination address(=NULL) error.\n'
//   0x440352d6: 'Source address(=NULL) error.\n'
//   0x440352f4: 'Source length(=0) error.\n'
//   0x4403530e: 'Destination length(=0) error.\n'
//   0x4403532d: 'uncompress dst=%x len=%x src=%x len=%x\n'
//   0x44035370: 'error!: destLen = 0x%x, header->original_soft_size = 0x%x\n'
//   0x440353ab: 'error!: check sum mismatch\n'
//   0x440353e7: 'error! unsupport argo_no = %d\n'
//   0x44035355: 'error!: uncompress() = %d\n'
//   0x440353c7: 'error!: lzma_uncompress() = %d\n'
undefined4 FUN_4400b6c6(undefined4 param_1)

{
  undefined4 *puVar1;
  int iVar2;
  undefined4 uVar3;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  iVar2 = FUN_4400b52e(param_1);
  if (iVar2 == 0) {
    uart0_printf(s_no_header__4403529b);
    puVar1[-7] = 0;
    goto LAB_4400b9fa;
  }
  iVar2 = FUN_4400b56b(puVar1[-4]);
  if (iVar2 == 0) {
    uart0_printf(s_no_footer__440352a7);
    puVar1[-7] = 0;
    goto LAB_4400b9fa;
  }
  if (puVar1[-3] == 0) {
    uart0_printf(s_Destination_address__NULL__error_440352b3);
    puVar1[-7] = 0;
    goto LAB_4400b9fa;
  }
  if (puVar1[-4] == 0) {
    uart0_printf(s_Source_address__NULL__error__440352d6);
    puVar1[-7] = 0;
    goto LAB_4400b9fa;
  }
  if (puVar1[-6] == 0) {
    uart0_printf(s_Source_length__0__error__440352f4);
    puVar1[-7] = 0;
    goto LAB_4400b9fa;
  }
  if (puVar1[-5] == 0) {
    uart0_printf(s_Destination_length__0__error__4403530e);
    puVar1[-7] = 0;
    goto LAB_4400b9fa;
  }
  puVar1[-5] = puVar1[-5] + 0x400;
  puVar1[-8] = (uint)*(ushort *)(puVar1[-1] + 0xe);
  if (puVar1[-8] == 2) {
    uart0_printf(s_uncompress_dst__x_len__x_src__x_l_4403532d,puVar1[-3],
                 *(undefined4 *)(puVar1[-1] + 0x10),puVar1[-4] + 0x40,puVar1[-6]);
    FUN_4402ba08(puVar1[-3],puVar1 + -5,puVar1[-4] + 0x40,puVar1[-6]);
    if (*(int *)(puVar1[-1] + 0x10) != puVar1[-5]) {
      uart0_printf(s_error___destLen___0x_x__header_>_44035370,puVar1[-5],
                   *(undefined4 *)(puVar1[-1] + 0x10));
      puVar1[-7] = 0;
      goto LAB_4400b9fa;
    }
    iVar2 = FUN_4400b652(puVar1[-1]);
    if (iVar2 != 0) {
      uart0_printf(s_error___check_sum_mismatch_440353ab);
      puVar1[-7] = 0;
      goto LAB_4400b9fa;
    }
  }
  else if ((int)puVar1[-8] < 3) {
    if (puVar1[-8] != 0) {
LAB_4400b9dc:
      uart0_printf(s_error__unsupport_argo_no____d_440353e7,(uint)*(ushort *)(puVar1[-1] + 0xe));
      puVar1[-7] = 0;
      goto LAB_4400b9fa;
    }
    uart0_printf(s_uncompress_dst__x_len__x_src__x_l_4403532d,puVar1[-3],
                 *(undefined4 *)(puVar1[-1] + 0x10),puVar1[-4] + 0x40,puVar1[-6]);
    uVar3 = FUN_44028393(puVar1[-3],puVar1 + -5,puVar1[-4] + 0x40,puVar1[-6]);
    puVar1[-2] = uVar3;
    if (puVar1[-2] != 0) {
      uart0_printf(s_error___uncompress______d_44035355,puVar1[-2]);
      puVar1[-7] = 0;
      goto LAB_4400b9fa;
    }
    if (*(int *)(puVar1[-1] + 0x10) != puVar1[-5]) {
      uart0_printf(s_error___destLen___0x_x__header_>_44035370,puVar1[-5],
                   *(undefined4 *)(puVar1[-1] + 0x10));
      puVar1[-7] = 0;
      goto LAB_4400b9fa;
    }
    iVar2 = FUN_4400b652(puVar1[-1]);
    if (iVar2 != 0) {
      uart0_printf(s_error___check_sum_mismatch_440353ab);
      puVar1[-7] = 0;
      goto LAB_4400b9fa;
    }
  }
  else {
    if (puVar1[-8] != 3) goto LAB_4400b9dc;
    uart0_printf(s_uncompress_dst__x_len__x_src__x_l_4403532d,puVar1[-3],
                 *(undefined4 *)(puVar1[-1] + 0x10),puVar1[-4] + 0x40,puVar1[-6]);
    uVar3 = FUN_4402c22c(puVar1[-4] + 0x40,puVar1[-3],puVar1 + -6,puVar1 + -5);
    puVar1[-2] = uVar3;
    if (puVar1[-2] != 0) {
      uart0_printf(s_error___lzma_uncompress______d_440353c7,puVar1[-2]);
      puVar1[-7] = 0;
      goto LAB_4400b9fa;
    }
    if (*(int *)(puVar1[-1] + 0x10) != puVar1[-5]) {
      uart0_printf(s_error___destLen___0x_x__header_>_44035370,puVar1[-5],
                   *(undefined4 *)(puVar1[-1] + 0x10));
      puVar1[-7] = 0;
      goto LAB_4400b9fa;
    }
    iVar2 = FUN_4400b652(puVar1[-1]);
    if (iVar2 != 0) {
      uart0_printf(s_error___check_sum_mismatch_440353ab);
      puVar1[-7] = 0;
      goto LAB_4400b9fa;
    }
  }
  puVar1[-7] = 1;
LAB_4400b9fa:
  return puVar1[-7];
}
```

#### `sub_4400bb14` (`sub_4400bb14`) — `0x4400bb14` (430 B)

Sub-rotina interna de decodificação de tokens/cópias LZ77/LZSS (`sub_4400bb14`).

<!-- source: doc_boot_media_crypto.txt | FUNC sub_4400bb14 (sub_4400bb14) @ 0x4400bb14 (size=430 B, req=0x4400bb14) -->
```c
// Callers (2): 0x440049d0, 0x44007c9b
// Referenced strings/literals:
//   0x44035531: 'Error! Drive firmware not compress header.\n'
//   0x4403555d: 'F/E Address = 0x%X , Drive(compress) Size = 0x%X\n'
//   0x4403558f: 'Error! Drive firmware uncompress failed[0x%X].\n'
//   0x440355bf: 'Error! Drive firmware not compress.\n'
//   0x440355e4: 'F/E reset release finish ...\n'
/* WARNING: Removing unreachable block (ram,0x4400bb54) */

undefined4 sub_4400bb14(char param_1)

{
  ushort uVar1;
  undefined4 *puVar2;
  undefined4 uVar3;
  undefined4 uStack_4;
  
  puVar2 = &uStack_4;
  if (param_1 == '\x01') {
    uVar1 = DAT_c0001008;
    DAT_c0001008 = uVar1 & 0x7fff;
  }
  else {
    FUN_4402c348(&DAT_90202000,0xff,0x100);
    uVar3 = FUN_4400b52e(0x84070000);
    puVar2[-1] = uVar3;
    if (puVar2[-1] == 0) {
      uart0_printf(s_Error__Drive_firmware_not_compre_44035531);
      *(undefined1 *)((int)puVar2 + -0x11) = 0;
    }
    else {
      uVar3 = FUN_4400b61a(0x84070000);
      puVar2[-3] = uVar3;
      uVar3 = FUN_4400b4ab(0x84070000,0xff);
      puVar2[-4] = uVar3;
      uVar3 = FUN_4400b5b5(0x84070000);
      puVar2[-2] = uVar3;
      uart0_printf(s_F_E_Address___0x_X___Drive_compr_4403555d,puVar2[-3],puVar2[-4]);
      if (puVar2[-2] == 1) {
        uVar3 = FUN_4400b6c6(0x84070000);
        puVar2[-2] = uVar3;
        if (puVar2[-2] != 1) {
          uart0_printf(s_Error__Drive_firmware_uncompress_4403558f,puVar2[-2]);
          *(undefined1 *)((int)puVar2 + -0x11) = 0;
        }
      }
      else {
        uart0_printf(s_Error__Drive_firmware_not_compre_440355bf);
        *(undefined1 *)((int)puVar2 + -0x11) = 0;
      }
    }
    if ((*(char *)(puVar2 + 3) == '\0') && (*(char *)((int)puVar2 + -0x11) != '\x01')) {
      puVar2[-7] = (uint)*(byte *)((int)puVar2 + -0x11);
      goto LAB_4400bcba;
    }
    FUN_4402c348(&DAT_901f3b60,0,0x80);
    nvram_read_id(2,puVar2 + -6,0xffffffff);
    if (puVar2[-6] == 0) {
      DAT_901f3bcf = 0xff;
    }
    FUN_4402c2f7(&DAT_90362000,0x84120000,&DAT_00001070);
    DAT_dc0f0080 = 0x202000;
    uVar1 = DAT_c0001008;
    DAT_c0001008 = uVar1 | 0x8000;
    uart0_printf(s_F_E_reset_release_finish_____440355e4);
  }
  puVar2[-7] = (uint)*(byte *)((int)puVar2 + -0x11);
LAB_4400bcba:
  return puVar2[-7];
}
```

#### `uncompress` (`uncompress_fe_bcc2`) — `0x4400bcc2` (192 B)

Descompressor principal utilizado pelo loader de firmware (`uncompress_fe_bcc2`).

<!-- source: doc_boot_media_crypto.txt | FUNC uncompress (uncompress_fe_bcc2) @ 0x4400bcc2 (size=192 B, req=0x4400bcc2) -->
```c
// Callers (6): 0x44006e71, 0x44006f6a, 0x44007c29, 0x44007cad, 0x44007ce3, 0x44007d1f
// Referenced strings/literals:
//   0x44035602: 'Error! Fvu not compress header.\n'
//   0x44035623: 'Fvu Address = 0x%X , Size = 0x%X\n'
//   0x44035645: 'Error! Fvu uncompress failed[0x%X].\n'
//   0x4403566a: 'Error! Fvu not compress.\n'
undefined1 uncompress(undefined4 param_1)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  uVar2 = FUN_4400b52e(param_1);
  puVar1[-1] = uVar2;
  if (puVar1[-1] == 0) {
    uart0_printf(s_Error__Fvu_not_compress_header__44035602);
    *(undefined1 *)((int)puVar1 + -0x11) = 0;
  }
  else {
    uVar2 = FUN_4400b61a(puVar1[2]);
    puVar1[-3] = uVar2;
    uVar2 = FUN_4400b4ab(puVar1[2],0xff);
    puVar1[-4] = uVar2;
    uVar2 = FUN_4400b5b5(puVar1[2]);
    puVar1[-2] = uVar2;
    uart0_printf(s_Fvu_Address___0x_X___Size___0x_X_44035623,puVar1[-3],puVar1[-4]);
    if (puVar1[-2] == 1) {
      uVar2 = FUN_4400b6c6(puVar1[2]);
      puVar1[-2] = uVar2;
      if (puVar1[-2] != 1) {
        uart0_printf(s_Error__Fvu_uncompress_failed_0x__44035645,puVar1[-2]);
        *(undefined1 *)((int)puVar1 + -0x11) = 0;
      }
    }
    else {
      uart0_printf(s_Error__Fvu_not_compress__4403566a);
      *(undefined1 *)((int)puVar1 + -0x11) = 0;
    }
  }
  return *(undefined1 *)((int)puVar1 + -0x11);
}
```

#### `FUN_44011d42` (`feistel_key_schedule_11d42`) — `0x44011d42` (383 B)

Key schedule da cifra Feistel a partir de `0x50480278`/`0x5048027f` (`feistel_key_schedule_11d42`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_44011d42 (feistel_key_schedule_11d42) @ 0x44011d42 (size=383 B, req=0x44011d42) -->
```c
// Callers (1): 0x4400a693
undefined4 FUN_44011d42(int param_1,int param_2,int param_3)

{
  undefined4 *puVar1;
  byte bStack_19;
  byte abStack_14 [8];
  undefined4 uStack_c;
  undefined4 uStack_8;
  undefined4 uStack_4;
  
  puVar1 = &uStack_c;
  for (bStack_19 = 0; bStack_19 < 7; bStack_19 = bStack_19 + 1) {
    abStack_14[bStack_19] =
         *(byte *)(param_2 + (uint)bStack_19) ^ *(byte *)(param_1 + (uint)bStack_19);
  }
  abStack_14[1] = *(byte *)(param_2 + 7) ^ abStack_14[1];
  abStack_14[4] = *(byte *)(param_2 + 7) ^ abStack_14[4];
  for (bStack_19 = 0; bStack_19 < 7; bStack_19 = bStack_19 + 1) {
    *(byte *)(param_3 + (uint)bStack_19) = abStack_14[bStack_19];
  }
  for (bStack_19 = 7; bStack_19 < 0x20; bStack_19 = bStack_19 + 1) {
    *(undefined1 *)(param_3 + (uint)bStack_19) = 0;
  }
  while (*(byte *)((int)puVar1 + -0xd) < 0x10) {
    *(byte *)((int)puVar1 + -0xe) = (*(byte *)((int)puVar1 + -0xd) & 7) << 2;
    FUN_44012191((uint)*(byte *)((int)puVar1 + -0xe) + puVar1[6],
                 puVar1[6] + (uint)*(byte *)((int)puVar1 + -0xe) + 1,
                 puVar1[6] + (uint)*(byte *)((int)puVar1 + -0xe) + 2,
                 puVar1[6] + (uint)*(byte *)((int)puVar1 + -0xe) + 3,puVar1 + -3);
    *(undefined1 *)(puVar1 + -3) = *(undefined1 *)(puVar1[6] + (uint)*(byte *)((int)puVar1 + -0xe));
    *(undefined1 *)((int)puVar1 + -0xb) =
         *(undefined1 *)((uint)*(byte *)((int)puVar1 + -0xe) + puVar1[6] + 1);
    *(undefined1 *)((int)puVar1 + -10) =
         *(undefined1 *)((uint)*(byte *)((int)puVar1 + -0xe) + puVar1[6] + 2);
    *(undefined1 *)((int)puVar1 + -9) =
         *(undefined1 *)((uint)*(byte *)((int)puVar1 + -0xe) + puVar1[6] + 3);
    *(char *)((int)puVar1 + -0xd) = *(char *)((int)puVar1 + -0xd) + '\x01';
  }
  return 0;
}
```

#### `FUN_44012029` (`feistel_block_dec_12029`) — `0x44012029` (360 B)

Decifração de bloco de 64 bits (8 bytes) Feistel (`feistel_block_dec_12029`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_44012029 (feistel_block_dec_12029) @ 0x44012029 (size=360 B, req=0x44012029) -->
```c
// Callers (1): 0x4400a6b2
undefined4 FUN_44012029(void)

{
  undefined4 *puVar1;
  undefined4 uStack_c;
  undefined4 uStack_8;
  undefined4 uStack_4;
  
  puVar1 = &uStack_c;
  while (-1 < (int)puVar1[-4]) {
    *(undefined1 *)((int)puVar1 + -1) = *(undefined1 *)((int)puVar1 + -5);
    *(undefined1 *)((int)puVar1 + -2) = *(undefined1 *)((int)puVar1 + -6);
    *(undefined1 *)((int)puVar1 + -3) = *(undefined1 *)((int)puVar1 + -7);
    *(undefined1 *)(puVar1 + -1) = *(undefined1 *)(puVar1 + -2);
    FUN_44012191((int)puVar1 + -5,(int)puVar1 + -6,(int)puVar1 + -7,puVar1 + -2,
                 puVar1[4] + (puVar1[-4] & 7) * 4);
    *(byte *)((int)puVar1 + -5) = *(byte *)((int)puVar1 + -9) ^ *(byte *)((int)puVar1 + -5);
    *(byte *)((int)puVar1 + -6) = *(byte *)((int)puVar1 + -10) ^ *(byte *)((int)puVar1 + -6);
    *(byte *)((int)puVar1 + -7) = *(byte *)((int)puVar1 + -0xb) ^ *(byte *)((int)puVar1 + -7);
    *(byte *)(puVar1 + -2) = *(byte *)(puVar1 + -3) ^ *(byte *)(puVar1 + -2);
    *(undefined1 *)((int)puVar1 + -9) = *(undefined1 *)((int)puVar1 + -1);
    *(undefined1 *)((int)puVar1 + -10) = *(undefined1 *)((int)puVar1 + -2);
    *(undefined1 *)((int)puVar1 + -0xb) = *(undefined1 *)((int)puVar1 + -3);
    *(undefined1 *)(puVar1 + -3) = *(undefined1 *)(puVar1 + -1);
    puVar1[-4] = puVar1[-4] + -1;
  }
  *(undefined1 *)puVar1[5] = *(undefined1 *)((int)puVar1 + -9);
  *(undefined1 *)(puVar1[5] + 1) = *(undefined1 *)((int)puVar1 + -10);
  *(undefined1 *)(puVar1[5] + 2) = *(undefined1 *)((int)puVar1 + -0xb);
  *(undefined1 *)(puVar1[5] + 3) = *(undefined1 *)(puVar1 + -3);
  *(undefined1 *)(puVar1[5] + 4) = *(undefined1 *)((int)puVar1 + -5);
  *(undefined1 *)(puVar1[5] + 5) = *(undefined1 *)((int)puVar1 + -6);
  *(undefined1 *)(puVar1[5] + 6) = *(undefined1 *)((int)puVar1 + -7);
  *(undefined1 *)(puVar1[5] + 7) = *(undefined1 *)(puVar1 + -2);
  return 0;
}
```

#### `FUN_44012191` (`feistel_round_f_12191`) — `0x44012191` (383 B)

Função de rodada Feistel usando S-Box `0x44035d84` (`feistel_round_f_12191`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_44012191 (feistel_round_f_12191) @ 0x44012191 (size=383 B, req=0x44012191) -->
```c
// Callers (2): 0x44011e6b, 0x440120da
byte * FUN_44012191(byte *param_1,byte *param_2,byte *param_3,byte *param_4,byte *param_5)

{
  byte bVar1;
  byte bVar2;
  byte bVar3;
  byte bVar4;
  byte bVar5;
  byte bVar6;
  
  bVar3 = param_5[1] ^ *param_2;
  bVar4 = *param_5 ^ *param_1;
  bVar1 = (param_5[3] ^ *param_4) >> 2;
  bVar5 = (param_5[3] ^ *param_4) << 6 | bVar1;
  bVar2 = (param_5[2] ^ *param_3) >> 5;
  bVar6 = (param_5[2] ^ *param_3) * '\b' | bVar2;
  *param_4 = bVar6 >> 1 | bVar1 << 7;
  *param_3 = bVar3 & 0x7f | bVar2 << 7;
  *param_2 = bVar4 >> 1 | (bVar3 >> 7) << 7;
  *param_1 = bVar5 >> 1 | bVar4 << 7;
  bVar1 = (&DAT_44035d84)[(byte)(bVar4 ^ (bVar3 * '\x02' | bVar3 >> 7) ^ bVar6 ^ bVar5)];
  *param_4 = *param_4 + bVar1;
  *param_3 = *param_3 + (bVar1 * '\x02' | bVar1 >> 7);
  *param_2 = *param_2 + (bVar1 << 2 | bVar1 >> 6);
  *param_1 = *param_1 + (bVar1 * '\b' | bVar1 >> 5);
  return param_1;
}
```

#### `FUN_4401e5a3` (`strided_adler32_1e5a3`) — `0x4401e5a3` (147 B)

Checksum estilo Adler-32 com step configurável (`strided_adler32_1e5a3`).

<!-- source: doc_boot_media_crypto.txt | FUNC FUN_4401e5a3 (strided_adler32_1e5a3) @ 0x4401e5a3 (size=147 B, req=0x4401e5a3) -->
```c
// Callers (1): 0x4400b683
uint FUN_4401e5a3(int param_1,uint param_2,int param_3)

{
  int iVar1;
  undefined4 uStack_10;
  undefined4 uStack_c;
  undefined4 uStack_8;
  
  uStack_8 = 1;
  uStack_c = 0;
  for (uStack_10 = 0; uStack_10 < param_2; uStack_10 = param_3 + uStack_10) {
    iVar1 = uStack_8 + *(byte *)(uStack_10 + param_1);
    uStack_8 = ((uint)(iVar1 * -0x7ff87f8f) >> 0xf) * 0xfff1 - iVar1;
    uStack_c = ((uStack_c + uStack_8) * -0x7ff87f8f >> 0xf) * 0xfff1 - (uStack_c + uStack_8);
  }
  return uStack_8 | uStack_c << 0x10;
}
```

#### S-Box Completa da Cifra Feistel (256 Bytes @ `0x44035D84`)

Tabela de substituição de 256 bytes em `/BOOT.BIN` utilizada por `FUN_44012191`.

<!-- source: doc_boot_media_crypto.txt | FEISTEL S-BOX (256B @ 0x44035D84) -->
```text
  0x44035d84: 00 01 bb 9c 3f 5e c7 3e 36 a4 92 93 38 9b 8d 1a
  0x44035d94: 3c 84 f7 2a cb 3d 70 ad 30 a2 c1 95 03 39 a5 0e
  0x44035da4: e6 fb 09 1e b9 c4 24 57 5f c3 5d fa 60 56 fd ed
  0x44035db4: 81 f3 4a ff 85 db cd a0 d0 9d 8b 26 4c 50 0c 1d
  0x44035dc4: 7d 77 de 7a 0a 0b 73 e1 54 76 e8 c2 28 05 98 99
  0x44035dd4: 29 4b ee 87 42 59 65 b8 1f df 23 58 63 69 1c 79
  0x44035de4: 64 c5 6f 13 3b ce 08 a7 b2 12 20 7f e2 89 21 2b
  0x44035df4: 41 49 7c 82 be f2 43 8e 86 02 9e d8 67 8f a3 e7
  0x44035e04: bc f8 66 22 4d e5 ba d2 da ea 61 25 b0 c0 7b a6
  0x44035e14: 48 96 dd dc 71 90 55 5b 4f 4e 6d d1 11 34 aa 1b
  0x44035e24: f4 17 80 b7 cc 8c d9 37 8a 2c 94 16 72 40 ef 9a
  0x44035e34: c8 47 f6 fc 2e b3 2f bd cf a9 15 10 18 ec 6a 6e
  0x44035e44: c9 91 53 78 a8 3a af 0d e0 97 32 51 b1 f5 27 2d
  0x44035e54: 35 88 14 e3 fe 06 d7 d6 ab a1 d5 e4 45 44 9f 0f
  0x44035e64: 5a 83 eb ca 07 04 b4 b6 52 ae d4 33 31 75 d3 6c
  0x44035e74: 62 bf 6b f9 ac 46 f0 5c 19 7e 68 b5 f1 74 c6 e9
```

---

## Parte II — `/eh55_frm_PROG061_BOOT.bin` (Estágio 2 / DIGADIAG & FVU Loader)

Binário de Estágio 2 carregado em RAM (`0x50400000..0x5043ffff`). Fornece console interativo de serviço (`DIGADIAG`), rotina de atualização `FVU` via cartão SD/`FMU` e drivers BGO/NOR específicos do carregador.

### 2.1 Inicialização, Menu DIGADIAG, `FMU_list_sum` (`0x5040ea30`) & `TimerMCU_update`

#### `FUN_50400014` (`p61_startup_0014`) — `0x50400014` (101 B)

Ponto de entrada pós-carga de `PROG061` em `0x50400014` (`p61_startup_0014`).

<!-- source: doc_prog061.txt | FUNC FUN_50400014 (p61_startup_0014) @ 0x50400014 (size=101 B, req=0x50400014) -->
```c
// Callers (1): 0x50400000
/* WARNING: This function may have set the stack pointer */
/* WARNING: Removing unreachable block (ram,0x50400475) */

void FUN_50400014(void)

{
  undefined2 uVar1;
  short sVar2;
  byte bVar3;
  ushort uVar4;
  char cVar5;
  int iVar6;
  code *UNRECOVERED_JUMPTABLE;
  undefined4 *puVar7;
  uint uVar8;
  
  Ramd4000008 = 0;
  cVar5 = TM0MD;
  if (cVar5 == '\x01') goto LAB_504000b1;
  if (cVar5 == '\0') {
code_r0x5040021b:
    bVar3 = UNK_c0001004;
    sVar2 = Ramc0000044;
    uVar4 = Ramd4000000;
    if (((((bVar3 & 2) != 0) || (sVar2 != 0)) || ((uVar4 & 7) != 0)) ||
       (((iRam50400334 != 0 || (iRam50400338 != 0)) || ((iRam5040033c != 1 || (iRam50400340 != 0))))
       )) {
      do {
                    /* WARNING: Do nothing block with infinite loop */
      } while( true );
    }
    puVar7 = (undefined4 *)0x504f8b00;
    do {
      *puVar7 = 0;
      puVar7 = puVar7 + 1;
    } while ((int)puVar7 < 0x504f9178);
    UNK_c0001002 = 0x22;
    func_0x50400311();
    uVar8 = func_0x5040031a();
    if ((uVar8 & 0x18) != 0x18) {
      func_0x50400328();
      Ramd2000008 = pcRam50400010;
      func_0x50400937();
      goto code_r0x5040008d;
    }
  }
  else {
    iVar6 = Ramd4003000;
    if (iVar6 != -1) {
      Ramc0000018 = 0xaa01;
      UNK_c0001004 = 0;
      UNK_c0001004 = 1;
      TM0MD = 0;
      goto code_r0x5040021b;
    }
    uVar8 = 0x8800000;
code_r0x5040008d:
    func_0x5040099c(uVar8);
    func_0x504002a0();
    uVar1 = Ramc0000018;
  }
  func_0x50400a87();
LAB_504000b1:
  UNRECOVERED_JUMPTABLE = (code *)Ramd2000008;
  if (UNRECOVERED_JUMPTABLE == (code *)0xffffffff) {
    UNRECOVERED_JUMPTABLE = pcRam50400010;
  }
                    /* WARNING: Could not recover jumptable at 0x504000e7. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (*UNRECOVERED_JUMPTABLE)(0,0);
  return;
}
```

#### `FUN_50400810` (`p61_data_init_0810`) — `0x50400810` (150 B)

Inicialização de dados e variáveis globais de `PROG061` (`p61_data_init_0810`).

<!-- source: doc_prog061.txt | FUNC FUN_50400810 (p61_data_init_0810) @ 0x50400810 (size=150 B, req=0x50400810) -->
```c
// Callers (0): 
// Referenced strings/literals:
//   0x50438920: 'ÿÿÿÿifodev1'
/* WARNING: Removing unreachable block (ram,0x50400839) */
/* WARNING: Removing unreachable block (ram,0x5040083a) */
/* WARNING: Removing unreachable block (ram,0x50400875) */
/* WARNING: Removing unreachable block (ram,0x50400881) */
/* WARNING: Removing unreachable block (ram,0x5040088d) */

undefined4 FUN_50400810(void)

{
  undefined4 *puVar1;
  undefined4 *puVar2;
  undefined4 uVar3;
  
  puVar1 = (undefined4 *)0x50480b80;
  do {
    *puVar1 = 0;
    puVar1 = puVar1 + 1;
  } while ((int)puVar1 < 0x504f8ab0);
  puVar1 = (undefined4 *)0x50438920;
  puVar2 = (undefined4 *)0x50480000;
  do {
    uVar3 = *puVar1;
    *puVar2 = uVar3;
    puVar1 = puVar1 + 1;
    puVar2 = puVar2 + 1;
  } while ((int)puVar2 < 0x50480b74);
  return uVar3;
}
```

#### `FUN_50401f3c` (`p61_nfrom_format_orphan_1f3c`) — `0x50401f3c` (483 B)

Rotina órfã/especial de formatação NFROM (`p61_nfrom_format_orphan_1f3c`).

<!-- source: doc_prog061.txt | FUNC FUN_50401f3c (p61_nfrom_format_orphan_1f3c) @ 0x50401f3c (size=483 B, req=0x50401f3c) -->
```c
// Callers (0): 
// Referenced strings/literals:
//   0x5043263b: '[nfrom] File map init error[%d].\n'
//   0x5043265d: '[nfrom] Data omap init error[%d].\n'
//   0x50432680: '[nfrom] Data bmap init error[%d].\n'
//   0x504326a3: '[nfrom] Main data init error[%d].\n'
//   0x50432620: '[nfrom] Format error[%d].\n'
undefined4 FUN_50401f3c(void)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  uVar2 = FUN_5040248e(2,0,0);
  puVar1[-1] = uVar2;
  if (puVar1[-1] == 0) {
    func_0x5042d323(0x50496d10,0xff,0x1cc4);
    uRam50496d14 = 0;
    uRam50496d16 = 0;
    uRam50496d18 = 0;
    uRam50496d1a = 0x200;
    uRam50496d1c = uRam50480000;
    uRam50496d1e = uRam50480002;
    uRam50496d10 = 0;
    *(undefined2 *)((int)puVar1 + -6) = 0;
    while (*(ushort *)((int)puVar1 + -6) < 3) {
      uVar2 = func_0x5040257b(*(short *)((int)puVar1 + -6) + 0xa02,
                              (uint)*(ushort *)((int)puVar1 + -6) * 0x200 + 0x50496f10);
      puVar1[-1] = uVar2;
      if (puVar1[-1] != 2) {
        func_0x50410d86(1,0x5043263b,puVar1[-1]);
        return 0xfffffffe;
      }
      *(short *)((int)puVar1 + -6) = *(short *)((int)puVar1 + -6) + 1;
    }
    *(undefined2 *)((int)puVar1 + -6) = 0;
    while (*(ushort *)((int)puVar1 + -6) < 10) {
      uVar2 = func_0x5040257b(*(short *)((int)puVar1 + -6) + 0xa07,
                              (uint)*(ushort *)((int)puVar1 + -6) * 0x200 + 0x50497510);
      puVar1[-1] = uVar2;
      if (puVar1[-1] != 2) {
        func_0x50410d86(1,0x5043265d,puVar1[-1]);
        return 0xfffffffe;
      }
      *(short *)((int)puVar1 + -6) = *(short *)((int)puVar1 + -6) + 1;
    }
    *(undefined2 *)((int)puVar1 + -6) = 0;
    while (*(ushort *)((int)puVar1 + -6) < 10) {
      uVar2 = func_0x5040257b(*(short *)((int)puVar1 + -6) + 0xa11,
                              (uint)*(ushort *)((int)puVar1 + -6) * 0x200 + 0x50497510);
      puVar1[-1] = uVar2;
      if (puVar1[-1] != 2) {
        func_0x50410d86(1,0x50432680,puVar1[-1]);
        return 0xfffffffe;
      }
      *(short *)((int)puVar1 + -6) = *(short *)((int)puVar1 + -6) + 1;
    }
    func_0x50404585(0xa01);
    uVar2 = func_0x5040257b(0xa01,0x50496d10);
    puVar1[-1] = uVar2;
    if (puVar1[-1] == 2) {
      uVar2 = 0;
    }
    else {
      func_0x50404585(0xffffffff);
      func_0x50410d86(1,0x504326a3,puVar1[-1]);
      uVar2 = 0xfffffffe;
    }
  }
  else {
    func_0x50410d86(1,0x50432620,puVar1[-1]);
    uVar2 = 0xffffffff;
  }
  return uVar2;
}
```

#### `FUN_5040211f` (`p61_cmd_dispatch_211f`) — `0x5040211f` (788 B)

Despachante de comandos DIGADIAG/serviço (`p61_cmd_dispatch_211f`).

<!-- source: doc_prog061.txt | FUNC FUN_5040211f (p61_cmd_dispatch_211f) @ 0x5040211f (size=788 B, req=0x5040211f) -->
```c
// Callers (1): 0x504043eb
// Referenced strings/literals:
//   0x504326ee: '[nfrom] NOR STATE : Original -> Backup\n'
//   0x50432716: '[nfrom] Map load error[%u].\n'
//   0x50432733: '[nfrom] Map write error[%u].\n'
//   0x50432751: '[nfrom] File map load error[%u].\n'
//   0x50432773: '[nfrom] File map write error[%u].\n'
//   0x50432796: '[nfrom] Status change error[%d].\n'
//   0x504326c6: '[nfrom] NOR STATE : Backup -> Original\n'
undefined4 FUN_5040211f(void)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 extraout_var;
  undefined4 extraout_var_00;
  undefined4 extraout_var_01;
  undefined4 uVar3;
  undefined4 extraout_var_02;
  undefined4 extraout_var_03;
  undefined4 extraout_var_04;
  undefined4 uStack_8;
  undefined4 uStack_4;
  
  puVar1 = &uStack_8;
  if (uRam50496d10 == 0xffff) {
    func_0x50410d86(2,0x504326ee);
    uVar2 = extraout_var_00;
    uVar3 = extraout_var_03;
LAB_504021c4:
    *(undefined2 *)((int)puVar1 + -10) = 0;
    while (*(ushort *)((int)puVar1 + -10) < 10) {
      uVar2 = FUN_5040248e(3,*(short *)(puVar1 + -3) + *(short *)((int)puVar1 + -10),
                           (uint)*(ushort *)((int)puVar1 + -10) * 0x200 + 0x50497510,uVar2,uVar3);
      puVar1[-1] = uVar2;
      if (puVar1[-1] != 0) {
        func_0x50410d86(1,0x50432716,puVar1[-1]);
        return 0xfffffffb;
      }
      *(short *)((int)puVar1 + -10) = *(short *)((int)puVar1 + -10) + 1;
      uVar2 = extraout_var_01;
      uVar3 = extraout_var_04;
    }
    if (uRam50496d10 != 0) {
      *(undefined2 *)((int)puVar1 + -10) = 0;
      while (*(ushort *)((int)puVar1 + -10) < 10) {
        uVar2 = FUN_5040248e(3,*(short *)((int)puVar1 + -0xe) + *(short *)((int)puVar1 + -10),
                             0x50498bf0);
        puVar1[-1] = uVar2;
        if (puVar1[-1] != 0) {
          func_0x50410d86(1,0x50432716,puVar1[-1]);
          return 0xfffffffb;
        }
        uVar2 = func_0x5042d292(0x50498bf0,(uint)*(ushort *)((int)puVar1 + -10) * 0x200 + 0x50497510
                                ,0x200);
        puVar1[-2] = uVar2;
        if (puVar1[-2] != 0) {
          uVar2 = func_0x5040257b(*(short *)((int)puVar1 + -10) + *(short *)((int)puVar1 + -0xe),
                                  (uint)*(ushort *)((int)puVar1 + -10) * 0x200 + 0x50497510);
          puVar1[-1] = uVar2;
          if (puVar1[-1] != 2) {
            func_0x50410d86(1,0x50432733,puVar1[-1]);
            return 0xfffffffe;
          }
        }
        *(short *)((int)puVar1 + -10) = *(short *)((int)puVar1 + -10) + 1;
      }
    }
    *(undefined2 *)((int)puVar1 + -10) = 0;
    while (*(ushort *)((int)puVar1 + -10) < 3) {
      uVar2 = FUN_5040248e(3,*(short *)((int)puVar1 + -10) + 0xa02,
                           (uint)*(ushort *)((int)puVar1 + -10) * 0x200 + 0x50496f10);
      puVar1[-1] = uVar2;
      if (puVar1[-1] != 0) {
        func_0x50410d86(1,0x50432751,puVar1[-1]);
        return 0xfffffffb;
      }
      *(short *)((int)puVar1 + -10) = *(short *)((int)puVar1 + -10) + 1;
    }
    if (uRam50496d10 != 0) {
      *(undefined2 *)((int)puVar1 + -10) = 0;
      while (*(ushort *)((int)puVar1 + -10) < 3) {
        func_0x5042d2d2(puVar1[-6],puVar1[-5],0x100);
        puVar1[-6] = puVar1[-6] + 0x200;
        puVar1[-5] = puVar1[-5] + 0x200;
        *(short *)((int)puVar1 + -10) = *(short *)((int)puVar1 + -10) + 1;
      }
      *(undefined2 *)((int)puVar1 + -10) = 0;
      while (*(ushort *)((int)puVar1 + -10) < 3) {
        uVar2 = func_0x5040257b(*(short *)((int)puVar1 + -10) + 0xa02,
                                (uint)*(ushort *)((int)puVar1 + -10) * 0x200 + 0x50496f10);
        puVar1[-1] = uVar2;
        if (puVar1[-1] != 2) {
          func_0x50410d86(1,0x50432773,puVar1[-1]);
          return 0xfffffffe;
        }
        *(short *)((int)puVar1 + -10) = *(short *)((int)puVar1 + -10) + 1;
      }
      uVar2 = func_0x50402435(0);
      puVar1[-2] = uVar2;
      if (puVar1[-2] != 0) {
        func_0x50410d86(1,0x50432796,puVar1[-2]);
        return 0xfffffffc;
      }
    }
    uVar2 = 0;
  }
  else {
    if (uRam50496d10 < 0x10000) {
      if (uRam50496d10 == 0) {
        uVar3 = 0;
        uVar2 = 0;
        goto LAB_504021c4;
      }
    }
    else if (uRam50496d10 == 0xffffff) {
      func_0x50410d86(2,0x504326c6);
      uVar2 = extraout_var;
      uVar3 = extraout_var_02;
      goto LAB_504021c4;
    }
    uVar2 = 0xfffffffc;
  }
  return uVar2;
}
```

#### `FUN_5040248e` (`p61_fmu_sd_diag_248e`) — `0x5040248e` (74 B)

Diagnóstico de cartão SD / FMU em `PROG061` (`p61_fmu_sd_diag_248e`).

<!-- source: doc_prog061.txt | FUNC FUN_5040248e (p61_fmu_sd_diag_248e) @ 0x5040248e (size=74 B, req=0x5040248e) -->
```c
// Callers (12): 0x50401f48, 0x504021f0, 0x50402250, 0x5040232b, 0x50402591, 0x5040308e, 0x504043c3, 0x5040a2bc, 0x5040a671, 0x5040a6f7, 0x5040ab80, 0x5040ac52
// Referenced strings/literals:
//   0x504327df: '[nfrom] bgo_read_D(ID=0x%X) return error(%u).\n'
//   0x5043280e: '[nfrom] BGO command[%d] error.\n'
undefined4 FUN_5040248e(undefined1 param_1,undefined2 param_2,undefined4 param_3)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  undefined4 uStack_8;
  undefined4 uStack_4;
  
  puVar1 = &uStack_8;
  switch(param_1) {
  case 0:
    uVar2 = func_0x50421e6a();
    puVar1[-2] = uVar2;
    break;
  case 1:
    uVar2 = FUN_50422359(1,4);
    puVar1[-2] = uVar2;
    break;
  case 2:
    uVar2 = FUN_50421ebe(0);
    puVar1[-2] = uVar2;
    break;
  case 3:
    uVar2 = func_0x50421f23(1,param_2,1,param_3);
    puVar1[-2] = uVar2;
    if (puVar1[-2] == 0) {
      puVar1[-2] = 0;
    }
    else {
      func_0x50410d86(1,0x504327df,*(undefined2 *)(puVar1 + -1),puVar1[-2]);
      puVar1[-2] = 1;
    }
    break;
  case 4:
    uVar2 = func_0x50421f85(1,param_2,1,param_3);
    puVar1[-2] = uVar2;
    break;
  default:
    func_0x50410d86(1,0x5043280e,param_1);
    puVar1[-2] = 0xffffffff;
    break;
  case 6:
    uVar2 = FUN_5042201e(1,param_2,1);
    puVar1[-2] = uVar2;
  }
  return puVar1[-2];
}
```

#### `FUN_50404dcd` (`p61_print_helper_4dcd`) — `0x50404dcd` (144 B)

Helper de impressão formatada no console `PROG061` (`p61_print_helper_4dcd`).

<!-- source: doc_prog061.txt | FUNC FUN_50404dcd (p61_print_helper_4dcd) @ 0x50404dcd (size=144 B, req=0x50404dcd) -->
```c
// Callers (372): 0x50405018, 0x50405053, 0x50405072, 0x504050a0, 0x504050f3, 0x504051d5, 0x50405214, 0x50405233, 0x5040529a, 0x504052b2, 0x5040546e, 0x504054b7, 0x504054de, 0x50405529, 0x5040555d, 0x50405591
undefined4 FUN_50404dcd(undefined4 param_1)

{
  undefined4 *puVar1;
  undefined4 uStack_4;
  
  puVar1 = &uStack_4;
  func_0x5042d51e(0x50480b8a,param_1,&stack0x00000008);
  puVar1[-2] = 0;
  while( true ) {
    if (*(char *)(puVar1[-2] + 0x50480b8a) == '\0') break;
    if (*(char *)(puVar1[-2] + 0x50480b8a) == '\n') {
      func_0x50404d15(0xd);
      func_0x50404d15(10);
    }
    else {
      func_0x50404d15(*(undefined1 *)(puVar1[-2] + 0x50480b8a));
    }
    puVar1[-2] = puVar1[-2] + 1;
  }
  return 0;
}
```

#### `FUN_50404ec0` (`p61_hit_any_key_4ec0`) — `0x50404ec0` (148 B)

Leitura de linha / espera de tecla (`Hit any key`, `p61_hit_any_key_4ec0`).

<!-- source: doc_prog061.txt | FUNC FUN_50404ec0 (p61_hit_any_key_4ec0) @ 0x50404ec0 (size=148 B, req=0x50404ec0) -->
```c
// Callers (0): 
undefined4 FUN_50404ec0(void)

{
  int in_A3;
  undefined1 extraout_D0;
  
  do {
    if (*(int *)(in_A3 + -8) < *(int *)(in_A3 + 0xc) + -1) {
      *(int *)(in_A3 + -8) = *(int *)(in_A3 + -8) + 1;
    }
    func_0x50404d15(*(undefined1 *)(in_A3 + -1));
    while( true ) {
      func_0x50404ca5();
      *(undefined1 *)(in_A3 + -1) = extraout_D0;
      if (*(char *)(in_A3 + -1) == '\r') {
        func_0x50404d15(0xd);
        func_0x50404d15(10);
        *(undefined1 *)(*(int *)(in_A3 + -8) + *(int *)(in_A3 + 8)) = 0;
        return 0;
      }
      if (*(char *)(in_A3 + -1) != '\b') break;
      if (0 < *(int *)(in_A3 + -8)) {
        *(int *)(in_A3 + -8) = *(int *)(in_A3 + -8) + -1;
        func_0x50404d15(*(undefined1 *)(in_A3 + -1));
        func_0x50404d15(0x20);
        func_0x50404d15(*(undefined1 *)(in_A3 + -1));
      }
    }
    *(undefined1 *)(*(int *)(in_A3 + -8) + *(int *)(in_A3 + 8)) = *(undefined1 *)(in_A3 + -1);
  } while( true );
}
```

#### `FUN_50404f12` (`HitAnyKey_xref` — `re_followup_decompiled_v2.txt`)

Exemplifica o uso do registrador de loop de hardware `LAR` (`in_LAR`) interpretado incorretamente como salto indireto `(*(code *)(in_LAR + -4))()` pelo decompilador MN103S.

```c
--- FUNCTION REPORT HitAnyKey_xref @ 0x50404F12 ---
  Function: FUN_50404f12 entry=0x50404F12 size=32 bytes


void FUN_50404f12(void)



{

  int in_LAR;

  ushort in_PSW;

  

                    /* WARNING: Could not recover jumptable at 0x50404f12. Too many branches */

                    /* WARNING: Treating indirect jump as call */

  if (((byte)(in_PSW >> 1) & 1 ^ (byte)(in_PSW >> 3) & 1) == 1) {

    (*(code *)(in_LAR + -4))();

    return;

  }

  func_0x50404dcd(0x5043);

  func_0x50404ca5();

  func_0x50404dcd(0x50432e1e);

  return;

}
```

#### `FUN_50404fa7` (`p61_main_menu_4fa7`) — `0x50404fa7` (1126 B)

Menu principal DIGADIAG de `PROG061` (`p61_main_menu_4fa7`).

<!-- source: doc_prog061.txt | FUNC FUN_50404fa7 (p61_main_menu_4fa7) @ 0x50404fa7 (size=1126 B, req=0x50404fa7) -->
```c
// Callers (0): 
// Referenced strings/literals:
//   0x50432e70: '***** Fvu ver%s [%s] *****\n'
//   0x50432e8c: '0.61'
//   0x50432e91: '05/11/01'
//   0x50432e9a: '***** Factory Production Mode *****\n'
//   0x50432ebf: '*** RTSC Initialize ***\n'
//   0x50432ed8: 'error! cannot read DeviceInfo-file from Nand-FROM.\n'
//   0x50432f0c: 'SD version up mode.\n'
//   0x50432fa8: 'FINISH'
//   0x50432f21: '*** Chip Reset ***\n'
//   0x50432f35: 'RSTCTR(CHIPRST)->0 retry %d.\n'
//   0x50432f53: 'RSTCTR(CHIPRST) not 0.\n'
//   0x50432f6b: 'RSTERR'
//   0x50432f90: 'RSTCTR(CHIPRST) not 1.\n'
//   0x50432f72: 'RSTCTR(CHIPRST)->1 retry %d.\n'
/* WARNING: Removing unreachable block (ram,0x50405324) */

void FUN_50404fa7(void)

{
  byte bVar1;
  ushort uVar2;
  undefined4 *puVar3;
  undefined4 uVar4;
  int iVar5;
  undefined4 uStack_4;
  
  puVar3 = &uStack_4;
  uRam504f00f0 = 0;
  iRam504f00f4 = 0;
  uRam504f00ec = 0;
  uRam504802d8 = 0;
  uRam504802e0 = 1;
  iRam504802a0 = 0;
  func_0x50404b11();
  func_0x5040c352();
  func_0x5040b61b();
  func_0x5040d516();
  func_0x5040d587();
  func_0x504069ca();
  FUN_50404dcd(0x50432e70,0x50432e8c,0x50432e91);
  uVar4 = FUN_5040a294();
  puVar3[-1] = uVar4;
  if (puVar3[-1] != 0) {
    FUN_5040689d();
  }
  uVar4 = FUN_50409ef4();
  puVar3[-1] = uVar4;
  iVar5 = FUN_50409fdb(0x11);
  if (iVar5 == 1) {
    FUN_50404dcd(0x50432e9a);
    func_0x5040c6d5();
    iRam504802a0 = 1;
  }
  else {
    FUN_50404dcd(0x50432ebf);
    func_0x5040c700();
  }
  uRam504f00f0 = 1;
  uVar4 = FUN_5040af11();
  puVar3[-1] = uVar4;
  if (puVar3[-1] != 0) {
    FUN_50404dcd(0x50432ed8);
    FUN_5040689d();
  }
  func_0x5042d323(0x50480ec8,0,9);
  func_0x5040af8c(4,8,0x50480ec8);
  uVar4 = FUN_50409fdb(0x10);
  puVar3[-1] = uVar4;
  if (puVar3[-1] == 1) {
    iRam50480284 = 5;
    FUN_50404dcd(0x50432f0c);
  }
  FUN_50405416(puVar3 + -2);
  if ((iRam50480288 == 0) || (iRam5048028c == 0)) {
    func_0x50409df2();
    func_0x50409e65();
  }
  if (((iRam50480288 != 1) || ((iRam50480284 != 3 && (iRam5048028c == 0)))) && (iRam50480284 != 5))
  {
    func_0x5040cb59();
  }
  iRam504f00f4 = 0;
  uRam504f00ec = 0;
  uRam504802d8 = 0;
  if (puVar3[-2] == 1) {
    if (iRam504802a4 == 1) {
      uVar2 = Ramdb000500;
      Ramdb000500 = uVar2 & 0xff3f;
      uVar2 = Ramdb000500;
      Ramdb000500 = uVar2 | 0x40;
      bVar1 = P5OUT;
      P5OUT = bVar1 | 8;
    }
    func_0x5040e3d6();
  }
  if (iRam504802a0 != 1) {
    do {
      while( true ) {
        if (iRam50480284 == 5) {
          func_0x5040b99b(2,0x50432fa8,0x50432fa8);
        }
        if (iRam504f00f4 != 0) {
          func_0x504067d5();
          return;
        }
        if (iRam50480288 != 1) break;
        if (iRam5048028c == 0) {
          func_0x5040b99b(2,0x504801b4,0x5048025c);
        }
        else {
          func_0x5040b99b(2,0x504801bb,0x5048025c);
        }
      }
      puVar3[-3] = 0;
      while ((uint)puVar3[-3] < 0xb) {
        if (*(int *)(puVar3[-3] * 0x34 + 0x50480c9c) != 0) {
          if ((iRam504f00f4 != 0) ||
             (func_0x5040b99b(2,*(undefined4 *)(puVar3[-3] * 0x34 + 0x50480c98),
                              *(undefined4 *)(puVar3[-3] * 0x34 + 0x50480c9c)), iRam504f00f4 != 0))
          break;
          func_0x5040b99b(2,*(undefined4 *)(puVar3[-3] * 0x34 + 0x50480c98),
                          *(undefined4 *)(puVar3[-3] * 0x34 + 0x50480c9c));
        }
        puVar3[-3] = puVar3[-3] + 1;
      }
    } while( true );
  }
  FUN_50404dcd(0x50432f21);
  puVar3[-3] = 0;
  while ((uint)puVar3[-3] < 0x10) {
    bVar1 = UNK_c0001004;
    UNK_c0001004 = bVar1 & 0xfe;
    bVar1 = UNK_c0001004;
    if ((bVar1 & 1) == 0) break;
    FUN_50404dcd(0x50432f35,puVar3[-3]);
    puVar3[-3] = puVar3[-3] + 1;
  }
  if (0xf < (uint)puVar3[-3]) {
    FUN_50404dcd(0x50432f53);
    func_0x5040b7b4(300,0x50432f6b);
    func_0x5040d59f();
    func_0x50416765();
    do {
                    /* WARNING: Do nothing block with infinite loop */
    } while( true );
  }
  puVar3[-3] = 0;
  while( true ) {
    if (0xf < (uint)puVar3[-3]) {
      FUN_50404dcd(0x50432f90);
      func_0x5040b7b4(300,0x50432f6b);
      func_0x5040d59f();
      func_0x50416765();
      do {
                    /* WARNING: Do nothing block with infinite loop */
      } while( true );
    }
    bVar1 = UNK_c0001004;
    UNK_c0001004 = bVar1 | 1;
    bVar1 = UNK_c0001004;
    if ((bVar1 & 1) != 0) break;
    FUN_50404dcd(0x50432f72,puVar3[-3]);
    puVar3[-3] = puVar3[-3] + 1;
  }
  do {
                    /* WARNING: Do nothing block with infinite loop */
  } while( true );
}
```

#### `FUN_50405416` (`p61_diag_loop_5416`) — `0x50405416` (653 B)

Loop de diagnóstico DIGADIAG (`p61_diag_loop_5416`).

<!-- source: doc_prog061.txt | FUNC FUN_50405416 (p61_diag_loop_5416) @ 0x50405416 (size=653 B, req=0x50405416) -->
```c
// Callers (1): 0x504050ff
// Referenced strings/literals:
//   0x50432faf: 'error! Failed to read Firm-list.\n'
//   0x50432fd1: 'do not need to update %s.\n'
void FUN_50405416(void)

{
  undefined1 *puVar1;
  int iVar2;
  undefined1 auStack_4 [4];
  
  puVar1 = auStack_4;
  if (((iRam50480284 == 5) || (iVar2 = FUN_50409fdb(0xc), iVar2 != 1)) || (iRam504802ac == 1)) {
    iRam504802ac = 0;
    func_0x504056a3();
    if (iRam50480284 == 0) {
      FUN_50404dcd(0x50432faf);
      func_0x5040821d(0,0,0x504801d7,0x504801e5);
    }
  }
  else {
    iRam504802ac = 1;
    func_0x5040ce91();
  }
  iVar2 = FUN_50409fdb(0);
  if (iVar2 == 1) {
    func_0x50406daa(0,*(undefined4 *)(puVar1 + 8),iRam504802ac);
  }
  else {
    FUN_50404dcd(0x50432fd1,0x50480ca1);
  }
  if ((iRam504802ac == 1) && (func_0x504056a3(), iRam50480284 == 0)) {
    FUN_50404dcd(0x50432faf);
    func_0x5040821d(0,0,0x504801d7,0x504801e5);
  }
  iVar2 = FUN_50409fdb(2);
  if (iVar2 == 1) {
    func_0x50406daa(2,*(undefined4 *)(puVar1 + 8),iRam504802ac);
  }
  else {
    FUN_50404dcd(0x50432fd1,0x50480d09);
  }
  iVar2 = FUN_50409fdb(5);
  if (iVar2 == 1) {
    func_0x50406daa(5,*(undefined4 *)(puVar1 + 8),iRam504802ac);
  }
  else {
    FUN_50404dcd(0x50432fd1,0x50480da5);
  }
  iVar2 = FUN_50409fdb(4);
  if (iVar2 == 1) {
    func_0x50406daa(4,*(undefined4 *)(puVar1 + 8),iRam504802ac);
  }
  else {
    FUN_50404dcd(0x50432fd1,0x50480d71);
  }
  iVar2 = FUN_50409fdb(7);
  if (iVar2 == 1) {
    func_0x50406daa(7,*(undefined4 *)(puVar1 + 8),iRam504802ac);
  }
  else {
    FUN_50404dcd(0x50432fd1,0x50480e0d);
  }
  iVar2 = FUN_50409fdb(8);
  if (iVar2 == 1) {
    func_0x50406daa(8,*(undefined4 *)(puVar1 + 8),iRam504802ac);
  }
  else {
    FUN_50404dcd(0x50432fd1,0x50480e41);
  }
  iVar2 = FUN_50409fdb(9);
  if (iVar2 == 1) {
    func_0x50406daa(9,*(undefined4 *)(puVar1 + 8),iRam504802ac);
  }
  else {
    FUN_50404dcd(0x50432fd1,0x50480e75);
  }
  iVar2 = FUN_50409fdb(6);
  if (iVar2 == 1) {
    func_0x50406daa(6,*(undefined4 *)(puVar1 + 8),iRam504802ac);
  }
  else {
    FUN_50404dcd(0x50432fd1,0x50480dd9);
  }
  iVar2 = FUN_50409fdb(1);
  if (iVar2 == 1) {
    func_0x50406daa(1,*(undefined4 *)(puVar1 + 8),iRam504802ac);
  }
  else {
    FUN_50404dcd(0x50432fd1,0x50480cd5);
  }
  return;
}
```

#### `FUN_5040689d` (`p61_halt_689d`) — `0x5040689d` (32 B)

Rotina de parada em `PROG061` (`p61_halt_689d`).

<!-- source: doc_prog061.txt | FUNC FUN_5040689d (p61_halt_689d) @ 0x5040689d (size=32 B, req=0x5040689d) -->
```c
// Callers (7): 0x50405030, 0x504050a7, 0x50409e25, 0x50409e59, 0x50409eae, 0x50409ee1, 0x50409f3f
void FUN_5040689d(void)

{
  func_0x504068bd(1,0x50480255,0x50480255);
  return;
}
```

#### `FUN_50406d30` (`p61_state_init_6d30`) — `0x50406d30` (122 B)

Inicialização de estados internos em `PROG061` (`p61_state_init_6d30`).

<!-- source: doc_prog061.txt | FUNC FUN_50406d30 (p61_state_init_6d30) @ 0x50406d30 (size=122 B, req=0x50406d30) -->
```c
// Callers (0): 
// Referenced strings/literals:
//   0x5043361e: 'deviceinfo'
//   0x50433629: 'STB '
//   0x5043362e: 'STB-data'
void FUN_50406d30(undefined4 param_1)

{
  undefined4 *in_A2;
  undefined4 in_D2;
  
  *in_A2 = in_D2;
  uRam00005048 = (undefined1)param_1;
  uRam50480e38 = 0x5048026a;
  uRam50480e34 = param_1;
  func_0x5042d47c(0x50480e41,0x5043361e);
  func_0x5042d47c(0x50480e64,0x50433629);
  uRam50480e68 = 0x50480278;
  uRam50480e6c = 0x50480278;
  func_0x5042d47c(0x50480e75,0x5043362e);
  return;
}
```

#### `FUN_5040df40` (`p61_TimerMCU_update_df40`) — `0x5040df40` (1058 B)

Atualização de firmware do Timer MCU via `PROG061` (`p61_TimerMCU_update_df40`).

<!-- source: doc_prog061.txt | FUNC FUN_5040df40 (p61_TimerMCU_update_df40) @ 0x5040df40 (size=1058 B, req=0x5040df40) -->
```c
// Callers (0): 
// Referenced strings/literals:
//   0x504346cf: 'Send 0x84 0x00 0x00 0xFF... Drive power down.\n'
//   0x504346fe: 'wait 3[s]...\n'
//   0x5043470c: 'send 0x8B.\n'
//   0x50434718: 'error! echo = 0x%x\n'
//   0x5043472c: 'changing BaudRate from 9600 to 31250...\n'
//   0x50434755: 'changing BaudRate from 9600 to 57600...\n'
//   0x5043477e: 'write retry[%d].\n'
//   0x50434790: 'erasing Timer-flash...\n'
//   0x504347ea: 'programing Timer-flash...(size=0x%x)\n'
//   0x50434810: 'verify checking Timer-flash...\n'
//   0x504347a8: 'error! busy now.\n'
//   0x504347d3: 'error! erase timeout.\n'
//   0x504347ba: 'error! failed to erase.\n'
//   0x50434830: 'success to update Timer.\n'
//   0x5043484a: 'failed to update Timer.\n'
undefined4 FUN_5040df40(int param_1)

{
  ushort uVar1;
  byte bVar2;
  int in_A3;
  int iVar3;
  undefined4 uVar4;
  
  if (param_1 == 1) {
    FUN_50404dcd(0x504346cf);
    func_0x5042d323(in_A3 + -0x1c,0);
    *(undefined1 *)(in_A3 + -0x1c) = 0x84;
    *(undefined1 *)(in_A3 + -0x19) = 0xff;
    func_0x5040ba7e(0xf);
    FUN_50404dcd(0x504346fe);
    func_0x5040d655(3000);
    bVar2 = P5OUT;
    P5OUT = bVar2 & 0xf7;
    uVar1 = Ramdb000500;
    Ramdb000500 = uVar1 & 0xff3f;
    uVar1 = Ramdb000500;
    Ramdb000500 = uVar1 | 0x40;
  }
  func_0x5040b9d9(0x8b,0);
  FUN_50404dcd(0x5043470c);
  uVar1 = Ramdb000100;
  Ramdb000100 = uVar1 & 0xfff0;
  uVar1 = Ramdb000100;
  Ramdb000100 = uVar1 | 10;
  if (*(int *)(in_A3 + 0x10) == 1) {
    func_0x5040d655(2000);
  }
  else {
    func_0x5040d655(1000);
  }
  if (*(int *)(in_A3 + 0x10) != 1) {
    func_0x5040bdc6(1,*(undefined4 *)(in_A3 + 0x10));
    *(undefined4 *)(in_A3 + -4) = 0;
    while (*(int *)(in_A3 + -4) < 0x10) {
      uRam50481934 = 0;
      iVar3 = func_0x5040be58(0x50481934,1);
      if (iVar3 == 0) {
        return 0;
      }
      func_0x5040d655(0x1e);
      *(int *)(in_A3 + -4) = *(int *)(in_A3 + -4) + 1;
    }
    uRam50481934 = 0xb0;
    iVar3 = func_0x5040be58(0x50481934,1);
    if (iVar3 == 0) {
      return 0;
    }
    iVar3 = func_0x5040bf33(0x50481a37,1);
    if (iVar3 == 0) {
      return 0;
    }
    if (cRam50481a37 != -0x50) {
      FUN_50404dcd(0x50434718,cRam50481a37);
      return 0;
    }
    func_0x5040d655(3000);
  }
  if (*(int *)(in_A3 + 0x10) == 1) {
    FUN_50404dcd(0x5043472c);
    func_0x5040bdc6(3,*(undefined4 *)(in_A3 + 0x10));
  }
  else {
    FUN_50404dcd(0x50434755);
  }
  uRam50481934 = 0xb3;
  iVar3 = func_0x5040be58(0x50481934,1);
  if (iVar3 == 0) {
    uVar4 = 0;
  }
  else {
    iVar3 = func_0x5040bf33(0x50481a37,1);
    if (iVar3 == 0) {
      uVar4 = 0;
    }
    else if (cRam50481a37 == -0x4d) {
      if (*(int *)(in_A3 + 0x10) != 1) {
        func_0x5040bdc6(2,*(undefined4 *)(in_A3 + 0x10));
      }
      func_0x5040d655(3000);
      *(undefined4 *)(in_A3 + -8) = 0;
      while (*(int *)(in_A3 + -8) < 2) {
        if (0 < *(int *)(in_A3 + -8)) {
          FUN_50404dcd(0x5043477e,*(undefined4 *)(in_A3 + -8));
          func_0x5040d655(1000);
        }
        FUN_50404dcd(0x50434790);
        uRam50481934 = 0xa7;
        iVar3 = func_0x5040be58(0x50481934,1);
        if (iVar3 != 0) {
          uRam50481934 = 0xd0;
          iVar3 = func_0x5040be58(0x50481934,1);
          if (iVar3 != 0) {
            func_0x5040d655(5000);
            if (*(int *)(in_A3 + 0x10) == 1) {
              func_0x5040d655(35000);
LAB_5040e2c2:
              FUN_50404dcd(0x504347ea,*(undefined4 *)(in_A3 + 0xc));
              iVar3 = func_0x5040e41b(*(undefined4 *)(in_A3 + 8),*(undefined4 *)(in_A3 + 0xc));
              if (iVar3 != 0) {
                if (*(int *)(in_A3 + 0x10) == 1) {
                  func_0x5040d655(0xfa);
                }
                FUN_50404dcd(0x50434810);
                iVar3 = func_0x5040e6dc(*(undefined4 *)(in_A3 + 8),*(undefined4 *)(in_A3 + 0xc));
                if (iVar3 != 0) break;
              }
            }
            else {
              uRam50481934 = 0x70;
              iVar3 = func_0x5040be58(0x50481934,1);
              if ((iVar3 != 0) && (iVar3 = func_0x5040bf33(0x50481a37,2), iVar3 != 0)) {
                *(char *)(in_A3 + -9) = cRam50481a37;
                *(undefined1 *)(in_A3 + -10) = uRam50481a38;
                if ((*(byte *)(in_A3 + -9) & 0x80) == 0) {
                  FUN_50404dcd(0x504347a8);
                }
                else if ((*(byte *)(in_A3 + -9) & 0x20) == 0) {
                  if ((*(byte *)(in_A3 + -10) & 2) == 0) goto LAB_5040e2c2;
                  FUN_50404dcd(0x504347d3);
                }
                else {
                  FUN_50404dcd(0x504347ba);
                }
              }
            }
          }
        }
        *(int *)(in_A3 + -8) = *(int *)(in_A3 + -8) + 1;
      }
      func_0x5040e364(*(undefined4 *)(in_A3 + 0x10));
      if (*(int *)(in_A3 + -8) < 2) {
        FUN_50404dcd(0x50434830);
        uVar4 = 1;
      }
      else {
        FUN_50404dcd(0x5043484a);
        uVar4 = 0;
      }
    }
    else {
      FUN_50404dcd(0x50434718,cRam50481a37);
      uVar4 = 0;
    }
  }
  return uVar4;
}
```

#### Placeholder `NO_FUNC p61_FMU_list_sum_ea30 @ 0x5040ea30` (`doc_prog061.txt`)

Na exportação inicial de `doc_prog061.txt`, `0x5040ea30` ainda não possuía função definida; a decompilação e o disassembly sobreposto reais seguem imediatamente abaixo.

<!-- source: doc_prog061.txt | NO_FUNC p61_FMU_list_sum_ea30 @ 0x5040ea30 -->
*(Sem corpo na exportação original; referência cruzada detalhada na subseção correspondente.)*

#### `FUN_5040ea30` (`FMU_list_sum` — Decompilação Verificada & Disassembly de Sobreposição `0x5040ea2c..0x5040ea80`)

Prova da armadilha de instrução sobreposta em `0x5040ea2f`/`0x5040ea30` (`bra 0x5040ea2f` saltando 1 byte antes de `0x5040ea30`, produzindo `WARNING: Instruction at (ram,0x5040ea32) overlaps instruction at (ram,0x5040ea30)`).

```c
--- FUNCTION REPORT FMU_list_sum @ 0x5040EA30 ---
  Function: FUN_5040ea30 entry=0x5040EA30 size=75 bytes


/* WARNING: Instruction at (ram,0x5040ea32) overlaps instruction at (ram,0x5040ea30)

    */



bool FUN_5040ea30(int param_1,int param_2)



{

  bool bVar1;

  int in_A3;

  

  while (uRam00008408 = (undefined2)param_2, *(uint *)(in_A3 + -4) < param_1 + 0x2eU) {

    *(ushort *)(in_A3 + -6) = (ushort)**(byte **)(in_A3 + -4) + *(short *)(in_A3 + -6);

    *(int *)(in_A3 + -4) = *(int *)(in_A3 + -4) + 1;

    param_1 = *(int *)(in_A3 + 8);

    param_2 = param_1;

  }

  bVar1 = *(short *)(*(int *)(in_A3 + 8) + 0x2e) != *(short *)(in_A3 + -6);

  if (bVar1) {

    func_0x5040fe60(1,0x504349f6);

  }

  return !bVar1;

}
```

```asm
  0x5040ea2c: [f8                   ] ?? F8h
  0x5040ea2d: [73                   ] ?? 73h    s
  0x5040ea2e: [fa                   ] ?? FAh
  0x5040ea2f: [f8                   ] ?? F8h
  0x5040ea30: [07 08 84             ] MOVHU D1,(0x8408)
  0x5040ea33: [28 2e                ] ADD 0x2e,D0
  0x5040ea35: [f8 07 fc             ] MOV (-0x4 ,A3),D1
  0x5040ea38: [a1                   ] CMP D0,D1
  0x5040ea39: [c4 04                ] BCS 0x5040ea3d
  0x5040ea3b: [ca 19                ] BRA 0x5040ea54
  0x5040ea3d: [f8 23 fc             ] MOV (-0x4,A3),A0
  0x5040ea40: [f0 44                ] MOVBU (A0),D1
  0x5040ea42: [84                   ] MOV D1,D0
  0x5040ea43: [f8 67 fa             ] MOVHU (-0x6 ,A3),D1
  0x5040ea46: [e4                   ] ADD D1,D0
  0x5040ea47: [f8 73 fa             ] MOVHU D0,(-0x6 ,A3)
  0x5040ea4a: [f8 03 fc             ] MOV (-0x4 ,A3),D0
  0x5040ea4d: [81                   ] MOV D0,D1
  0x5040ea4e: [44                   ] INC D1
  0x5040ea4f: [f8 17 fc             ] MOV D1,(-0x4 ,A3)
  0x5040ea52: [ca dd                ] BRA 0x5040ea2f
  0x5040ea54: [f8 23 08             ] MOV (0x8,A3),A0
  0x5040ea57: [f8 60 2e             ] MOVHU (0x2e ,A0),D0
  0x5040ea5a: [f8 67 fa             ] MOVHU (-0x6 ,A3),D1
  0x5040ea5d: [a4                   ] CMP D1,D0
  0x5040ea5e: [c9 08                ] BNE 0x5040ea66
  0x5040ea60: [80 01                ] MOV 0x1,D0
  0x5040ea62: [ca 16                ] BRA 0x5040ea78
  0x5040ea64: [ca                   ] ?? CAh
  0x5040ea65: [14                   ] ?? 14h
  0x5040ea66: [fc cd f6 49 43 50    ] MOV 0x504349f6,D1
  0x5040ea6c: [80 01                ] MOV 0x1,D0
  0x5040ea6e: [dd f2 13 00 00 00 00 ] CALL 0x5040fe60,[],0x0
  0x5040ea75: [00                   ] CLR D0
  0x5040ea76: [ca 02                ] BRA 0x5040ea78
  0x5040ea78: [f2 fc                ] MOV A3,SP
  0x5040ea7a: [df 10 04             ] RET [A3],0x4
  0x5040ea7d: [cf                   ] ?? CFh
  0x5040ea7e: [10                   ] ?? 10h
  0x5040ea7f: [3f                   ] ?? 3Fh    ?
  0x5040ea80: [f8                   ] ?? F8h
```

### 2.2 Drivers BGO, Controles NOR & Os 3 Patch Sites Exatos do PROG061

#### `FUN_5040a294` (`p61_bgo_init_hook_a294`) — `0x5040a294` (130 B)

Hook de inicialização BGO em `PROG061` (`p61_bgo_init_hook_a294`).

<!-- source: doc_prog061.txt | FUNC FUN_5040a294 (p61_bgo_init_hook_a294) @ 0x5040a294 (size=130 B, req=0x5040a294) -->
```c
// Callers (1): 0x5040501f
// Referenced strings/literals:
//   0x50433eb0: 'BGO driver init failed[%u].\n'
//   0x50433ecd: 'Control information init failed[%d].\n'
undefined4 FUN_5040a294(void)

{
  undefined1 *puVar1;
  undefined4 uVar2;
  undefined1 auStack_4 [4];
  
  puVar1 = auStack_4;
  if (iRam504802b0 != 1) {
    FUN_50404592(0x50480000,0x50480002);
    uVar2 = FUN_5040248e(0,0,0);
    *(undefined4 *)(puVar1 + -4) = uVar2;
    if (*(int *)(puVar1 + -4) != 0) {
      FUN_50404dcd(0x50433eb0,*(undefined4 *)(puVar1 + -4));
      return 0xffffffff;
    }
    uVar2 = func_0x5040439b();
    *(undefined4 *)(puVar1 + -8) = uVar2;
    if (*(int *)(puVar1 + -8) != 0) {
      FUN_50404dcd(0x50433ecd,*(undefined4 *)(puVar1 + -8));
      return 0xffffffff;
    }
    iRam504802b0 = 1;
  }
  return 0;
}
```

#### `FUN_50421ebe` (`p61_bgo_ctrl_21ebe`) — `0x50421ebe` (101 B)

Controle BGO equivalente em `PROG061` (`p61_bgo_ctrl_21ebe`).

<!-- source: doc_prog061.txt | FUNC FUN_50421ebe (p61_bgo_ctrl_21ebe) @ 0x50421ebe (size=101 B, req=0x50421ebe) -->
```c
// Callers (0): 
undefined4 FUN_50421ebe(int param_1)

{
  undefined4 uVar1;
  undefined4 uVar2;
  int iVar3;
  uint uVar4;
  uint uVar5;
  
  uVar4 = 1;
  uVar2 = 0xffffff90;
  uVar1 = 0x14d;
  do {
    func_0x504228fe(uVar4,4,uVar2,uVar1);
    uVar4 = uVar4 + 1;
  } while (uVar4 < 5);
  uVar4 = (uint)(param_1 != 0);
  uVar5 = 1;
  do {
    func_0x504266e0(uVar5,uVar4);
    func_0x50422a58(uVar5);
    uVar5 = uVar5 + 1;
  } while (uVar5 < 2);
  uVar4 = 1;
  do {
    uVar5 = 2;
    do {
      iVar3 = func_0x50422886(uVar4,uVar5);
      if ((iVar3 != 0) && (iVar3 != 4)) {
        return 1;
      }
      uVar5 = uVar5 + 1;
    } while (uVar5 < 5);
    uVar4 = uVar4 + 1;
    if (1 < uVar4) {
      return 0;
    }
  } while( true );
}
```

#### `FUN_5042201e` (`p61_bgo_read_2201e`) — `0x5042201e` (165 B)

Leitura BGO em `PROG061` (`p61_bgo_read_2201e`).

<!-- source: doc_prog061.txt | FUNC FUN_5042201e (p61_bgo_read_2201e) @ 0x5042201e (size=165 B, req=0x5042201e) -->
```c
// Callers (0): 
uint FUN_5042201e(uint param_1,uint param_2,int param_3)

{
  uint uVar1;
  int iVar2;
  uint uVar3;
  undefined1 auStack_74 [116];
  
  if (param_1 != 1) {
    return 0x13;
  }
  if (param_3 - 1U < 0x10) {
    if (param_2 != 0) {
      uVar3 = (param_2 + param_3) - 1;
      uVar1 = func_0x50422a1b(1,1);
      if (uVar3 <= uVar1) {
        if ((param_2 & 0xf) + param_3 < 0x11) {
          iVar2 = func_0x50422886(param_1,param_1);
          if (iVar2 != 0) {
            return 0x11;
          }
          uVar1 = func_0x50426a4a(param_1);
          if ((uVar1 < 0x10) && (iVar2 = func_0x504268f6(auStack_74), iVar2 == 0)) {
            uVar1 = 0;
            iVar2 = func_0x504227e1(param_1,2);
            if (iVar2 == 0) {
              uVar1 = param_1;
            }
            iVar2 = func_0x504227e1(param_1,param_1);
            if (iVar2 != 0) {
              return uVar1;
            }
            return uVar1 | 2;
          }
          return 0x10;
        }
        goto LAB_50422060;
      }
    }
    uVar1 = 0x12;
  }
  else {
LAB_50422060:
    uVar1 = 0x14;
  }
  return uVar1;
}
```

#### `FUN_50422359` (`p61_bgo_nor_22359`) — `0x50422359` (50 B)

Operação BGO em NOR Flash no `PROG061` (`p61_bgo_nor_22359`).

<!-- source: doc_prog061.txt | FUNC FUN_50422359 (p61_bgo_nor_22359) @ 0x50422359 (size=50 B, req=0x50422359) -->
```c
// Callers (0): 
undefined4 FUN_50422359(int param_1,uint param_2)

{
  undefined4 uVar1;
  
  if (param_1 != 1) goto LAB_50422360;
  if (param_2 == 2) {
    uVar1 = 4;
LAB_5042237f:
    uVar1 = func_0x50422886(1,uVar1);
  }
  else {
    if (param_2 < 3) {
      if (param_2 == 1) {
        uVar1 = 3;
        goto LAB_5042237f;
      }
    }
    else if (param_2 == 4) {
      uVar1 = 1;
      goto LAB_5042237f;
    }
LAB_50422360:
    uVar1 = 4;
  }
  return uVar1;
}
```

#### `FUN_50423058` (`p61_bgo_sum_23058`) — `0x50423058` (292 B)

Soma/checksum BGO em `PROG061` (`p61_bgo_sum_23058`).

<!-- source: doc_prog061.txt | FUNC FUN_50423058 (p61_bgo_sum_23058) @ 0x50423058 (size=292 B, req=0x50423058) -->
```c
// Callers (1): 0x50422fd9
void FUN_50423058(undefined4 param_1)

{
  undefined2 uVar1;
  int iVar2;
  undefined2 *puVar3;
  undefined2 *puVar4;
  int iVar5;
  int *piVar6;
  int *piVar7;
  int iVar8;
  uint uVar9;
  undefined4 uVar10;
  undefined4 uVar11;
  uint uVar12;
  int *piVar13;
  int iVar14;
  undefined4 extraout_var;
  undefined4 extraout_var_00;
  undefined4 extraout_var_01;
  undefined4 extraout_var_02;
  int extraout_var_03;
  int extraout_var_04;
  int iVar15;
  int iStack_34;
  int iStack_30;
  int iStack_2c;
  undefined4 uStack_20;
  undefined4 uStack_1c;
  undefined4 uStack_18;
  undefined4 uStack_14;
  
  uVar12 = 1;
  piVar6 = &iStack_34;
  iVar14 = 0x504f861a;
  piVar7 = (int *)0x50436ea0;
  iVar5 = 4;
  piVar13 = &iStack_30;
  do {
    iVar8 = func_0x504229be(param_1,uVar12);
    param_1 = extraout_var;
    if (iVar8 == 0) {
      *piVar13 = 4;
    }
    else {
      iVar2 = (*piVar7 + iVar8) * 2;
      iVar15 = extraout_var_03;
      if (*(short *)(iVar14 + iVar2) != -1) {
        uVar9 = func_0x50422a69(0,*(undefined2 *)(iVar14 + iVar2 + 2));
        puVar4 = (undefined2 *)((*piVar7 + iVar8) * 2 + iVar14);
        uVar1 = *puVar4;
        puVar3 = (undefined2 *)((*piVar7 + (uVar9 & 0xff)) * 2 + iStack_34);
        *puVar4 = *puVar3;
        *puVar3 = uVar1;
        param_1 = extraout_var_00;
        iVar15 = extraout_var_04;
      }
      if (*(uint *)(iVar15 + iVar5 + -4) == (uint)((iVar8 + 1) * iVar8) >> 1) {
        uVar11 = 2;
      }
      else {
        uVar11 = 1;
      }
      *(undefined4 *)((int)piVar6 + iVar5) = uVar11;
    }
    piVar7 = piVar7 + 1;
    iVar5 = iVar5 + 4;
    piVar13 = piVar13 + 1;
    uVar12 = uVar12 + 1;
  } while (uVar12 < 5);
  uVar12 = 2;
  uVar11 = 0x145;
  piVar6 = piVar6 + 2;
  do {
    iVar5 = func_0x50422886(param_1,uVar12);
    param_1 = extraout_var_01;
    if (iVar5 == 6) {
      uVar9 = uVar12;
      if (uVar12 == 2) {
        if ((iStack_30 == 2) && (iStack_2c == 2)) {
          uVar10 = 0x708;
          iVar5 = 2;
        }
        else if ((iStack_30 == 4) && (iStack_2c == 4)) {
          uVar10 = 0x70c;
          iVar5 = 4;
          uVar9 = 2;
        }
        else {
          iVar5 = 1;
          uVar10 = 0x710;
          uVar9 = 2;
        }
      }
      else {
        iVar5 = *piVar6;
        uVar10 = 0x715;
      }
      func_0x504228c3(extraout_var_01,uVar9,iVar5,uVar10,uVar11);
      param_1 = extraout_var_02;
    }
    piVar6 = piVar6 + 1;
    uVar12 = uVar12 + 1;
  } while (uVar12 < 5);
  return;
}
```

#### Disassembly dos 3 Sítios Exatos de Patch BGO no `PROG061`

Listagem exata em máquina MN103S dos 3 sítios de verificação/patch BGO localizados no `/eh55_frm_PROG061_BOOT.bin`.

<!-- source: doc_prog061.txt | PROG061 EXACT BGO PATCH SITES DISASSEMBLY -->
```asm
-- Site 1 (0x50423058..0x50423110, Site 0x504230F0 beq+10) --
  0x504230d0: [f1 60               ] ADD D0,A0
  0x504230d2: [f1 70               ] ADD A0,A0
  0x504230d4: [fb 8a 00 18         ] MOV (0x18,SP),R0
  0x504230d8: [f9 78 08            ] ADD R0,A0
  0x504230db: [f0 60               ] MOVHU (A0),D0
  0x504230dd: [f0 71               ] MOVHU D0,(A1)
  0x504230df: [f0 74               ] MOVHU D1,(A0)
  0x504230e1: [5c 54               ] MOV (0x54,SP),A0
  0x504230e3: [f1 78               ] ADD A2,A0
  0x504230e5: [88                  ] MOV D2,D0
  0x504230e6: [40                  ] INC D0
  0x504230e7: [f2 42               ] MUL D0,D2
  0x504230e9: [f8 c6 01            ] LSR 0x1,D2
  0x504230ec: [f8 00 fc            ] MOV (-0x4 ,A0),D0
  0x504230ef: [a8                  ] CMP D2,D0
  0x504230f0: [c8 0a               ] BEQ 0x504230fa
  0x504230f2: [f5 9c               ] MOV R7,A0
  0x504230f4: [f1 78               ] ADD A2,A0
  0x504230f6: [80 01               ] MOV 0x1,D0
  0x504230f8: [ca 08               ] BRA 0x50423100
  0x504230fa: [f5 9c               ] MOV R7,A0
  0x504230fc: [f1 78               ] ADD A2,A0
  0x504230fe: [80 02               ] MOV 0x2,D0
  0x50423100: [60                  ] MOV D0,(A0)
  0x50423101: [53                  ] INC4 A3
  0x50423102: [52                  ] INC4 A2
  0x50423103: [f9 c8 44            ] INC4 R4
  0x50423106: [4c                  ] INC D3
  0x50423107: [af 04               ] CMP 0x4,D3
  0x50423109: [c7 87               ] BLS 0x50423090
  0x5042310b: [8f 02               ] MOV 0x2,D3
  0x5042310d: [2e 45 01            ] MOV 0x145,D2
  0x50423110: [f5 9e               ] MOV R7,A2
  0x50423112: [22 08               ] ADD 0x8,A2
  0x50423114: [58 50               ] MOV (0x50,SP),D0
  0x50423116: [8d                  ] MOV D3,D1
  0x50423117: [cd 6f f7 00 00      ] CALL 0x50422886,[],0x0
-- Site 2 (0x50405070..0x504050C0, Site 0x50405097 beq+21) --
  0x50405071: [84                  ] MOV D1,D0
  0x50405072: [dd 5b fd ff ff 00 00] CALL 0x50404dcd,[],0x0
  0x50405079: [dd 87 76 00 00 00 00] CALL 0x5040c700,[],0x0
  0x50405080: [80 01               ] MOV 0x1,D0
  0x50405082: [fc 81 f0 00 4f 50   ] MOV D0,(0x504f00f0)
  0x50405088: [dd 89 5e 00 00 00 00] CALL 0x5040af11,[],0x0
  0x5040508f: [f8 13 fc            ] MOV D0,(-0x4 ,A3)
  0x50405092: [f8 03 fc            ] MOV (-0x4 ,A3),D0
  0x50405095: [a0 00               ] CMP 0x0,D0
  0x50405097: [c8 15               ] BEQ 0x504050ac
  0x50405099: [fc cd d8 2e 43 50   ] MOV 0x50432ed8,D1
  0x5040509f: [84                  ] MOV D1,D0
  0x504050a0: [dd 2d fd ff ff 00 00] CALL 0x50404dcd,[],0x0
  0x504050a7: [cd f6 17 00 00      ] CALL 0x5040689d,[],0x0
  0x504050ac: [fc cc c8 0e 48 50   ] MOV 0x50480ec8,D0
  0x504050b2: [85 09               ] MOV 0x9,D1
  0x504050b4: [46 0c               ] MOV D1,(0xc,SP)
  0x504050b6: [04                  ] CLR D1
  0x504050b7: [dd 6c 82 02 00 00 00] CALL 0x5042d323,[],0x0
  0x504050be: [fc cc c8 0e 48 50   ] MOV 0x50480ec8,D0
-- Site 3 (0x5040A294..0x5040A320, Site 0x5040A2CD BGO fail hook) --
  0x5040a294: [cf 10               ] MOVM [A3],(SP)
  0x5040a296: [3f                  ] MOV SP,A3
  0x5040a297: [f8 fe e8            ] ADD -0x18,SP
  0x5040a29a: [fc a4 b0 02 48 50   ] MOV (0x504802b0),D0
  0x5040a2a0: [a0 01               ] CMP 0x1,D0
  0x5040a2a2: [c8 6c               ] BEQ 0x5040a30e
  0x5040a2a4: [fc cc 00 00 48 50   ] MOV 0x50480000,D0
  0x5040a2aa: [fc cd 02 00 48 50   ] MOV 0x50480002,D1
  0x5040a2b0: [dd e2 a2 ff ff 00 00] CALL 0x50404592,[],0x0
  0x5040a2b7: [00                  ] CLR D0
  0x5040a2b8: [42 0c               ] MOV D0,(0xc,SP)
  0x5040a2ba: [00                  ] CLR D0
  0x5040a2bb: [04                  ] CLR D1
  0x5040a2bc: [dd d2 81 ff ff 00 00] CALL 0x5040248e,[],0x0
  0x5040a2c3: [f8 13 fc            ] MOV D0,(-0x4 ,A3)
  0x5040a2c6: [f8 03 fc            ] MOV (-0x4 ,A3),D0
  0x5040a2c9: [a0 00               ] CMP 0x0,D0
  0x5040a2cb: [c8 16               ] BEQ 0x5040a2e1
  0x5040a2cd: [fc cc b0 3e 43 50   ] MOV 0x50433eb0,D0
  0x5040a2d3: [f8 07 fc            ] MOV (-0x4 ,A3),D1
  0x5040a2d6: [dd f7 aa ff ff 00 00] CALL 0x50404dcd,[],0x0
  0x5040a2dd: [80 ff               ] MOV 0xff,D0
  0x5040a2df: [ca 32               ] BRA 0x5040a311
  0x5040a2e1: [dd ba a0 ff ff 00 00] CALL 0x5040439b,[],0x0
  0x5040a2e8: [f8 13 f8            ] MOV D0,(-0x8 ,A3)
  0x5040a2eb: [f8 03 f8            ] MOV (-0x8 ,A3),D0
  0x5040a2ee: [a0 00               ] CMP 0x0,D0
  0x5040a2f0: [c8 16               ] BEQ 0x5040a306
  0x5040a2f2: [fc cc cd 3e 43 50   ] MOV 0x50433ecd,D0
  0x5040a2f8: [f8 07 f8            ] MOV (-0x8 ,A3),D1
  0x5040a2fb: [dd d2 aa ff ff 00 00] CALL 0x50404dcd,[],0x0
  0x5040a302: [80 ff               ] MOV 0xff,D0
  0x5040a304: [ca 0d               ] BRA 0x5040a311
  0x5040a306: [80 01               ] MOV 0x1,D0
  0x5040a308: [fc 81 b0 02 48 50   ] MOV D0,(0x504802b0)
  0x5040a30e: [00                  ] CLR D0
  0x5040a30f: [ca 02               ] BRA 0x5040a311
  0x5040a311: [f2 fc               ] MOV A3,SP
  0x5040a313: [df 10 04            ] RET [A3],0x4
  0x5040a316: [cf 10               ] MOVM [A3],(SP)
  0x5040a318: [3f                  ] MOV SP,A3
  0x5040a319: [f8 fe e8            ] ADD -0x18,SP
  0x5040a31c: [f8 13 08            ] MOV D0,(0x8 ,A3)
  0x5040a31f: [f8 17 0c            ] MOV D1,(0xc ,A3)
```

---

## Parte III — `/EH55_MAIN_3410PQ.bin` (Estágio 3 / Main OS)

Imagem principal do firmware operacional (`0x57000000..0x57c6692b`, 13 MB), executada após descompressão da partição `mainsoft`. Abriga o console serial `_dev_SERIAL` / GDB RSP, controle de painel frontal em `TestMode`, interface de ODD (`ATIH`) e despachante IDE/LBA48 de disco rígido.

### 3.1 Console `_dev_SERIAL` / GDB RSP, Gates de UART0 & Loop de Menu `ENC`

#### `FUN_57000e1c` (`main_uart0_gate_000e1c`) — `0x57000e1c` (127 B)

Gate de inicialização/ativação da UART0 no `MAIN` (`main_uart0_gate_000e1c`).

<!-- source: doc_main.txt | FUNC FUN_57000e1c (main_uart0_gate_000e1c) @ 0x57000e1c (size=127 B, req=0x57000e1c) -->
```c
// Callers (0): 
void FUN_57000e1c(void)

{
  uRam934e334c = 1;
  uRam934e3358 = 0x22;
  if ((uRam934e2eac & 0x18) == 0) {
    uRam934e2eac = uRam934e2eac & 0xffffffdf;
  }
  else {
    if ((uRam934e2eac & 0x18) != 8) {
      FUN_57001fee();
    }
    if (((uRam934e2eac & 0x20) == 0x20) && (iRam934e3350 != 0xaa01)) {
      TM0MD = 1;
      do {
                    /* WARNING: Do nothing block with infinite loop */
      } while( true );
    }
  }
  return;
}
```

#### `FUN_574ef813` (`main_sc0txb_ring_4ef813`) — `0x574ef813` (94 B)

Envio por ring-buffer de transmissão serial `SC0TXB` (`main_sc0txb_ring_4ef813`).

<!-- source: doc_main.txt | FUNC FUN_574ef813 (main_sc0txb_ring_4ef813) @ 0x574ef813 (size=94 B, req=0x574ef813) -->
```c
// Callers (3): 0x574ef8bd, 0x574ef8d6, 0x574ef9bd
ushort FUN_574ef813(void)

{
  undefined1 uVar1;
  ushort uVar2;
  uint uVar3;
  
  iRam515b2e24 = iRam515b2e24 + 1;
  uVar1 = *puRam5316ae70;
  puRam5316ae70 = puRam5316ae70 + 1;
  sRam5316a254 = sRam5316a254 + 1;
  if ((undefined1 *)0x5316aa5f < puRam5316ae70) {
    puRam5316ae70 = (undefined1 *)0x5316a260;
  }
  uVar3 = 0;
  do {
    uVar2 = Ramd400200c;
    if ((uVar2 & 0x80) == 0) break;
    uVar3 = uVar3 + 1;
  } while (uVar3 < 10000);
  SC0TXB = uVar1;
  return uVar2;
}
```

#### `FUN_574f0538` (`main_dev_serial_init_4f0538`) — `0x574f0538` (73 B)

Inicialização do driver `_dev_SERIAL` (`main_dev_serial_init_4f0538`).

<!-- source: doc_main.txt | FUNC FUN_574f0538 (main_dev_serial_init_4f0538) @ 0x574f0538 (size=73 B, req=0x574f0538) -->
```c
// Callers (0): 
undefined4 FUN_574f0538(undefined4 param_1)

{
  int in_D2;
  int in_D3;
  
  func_0x5770897c(param_1,0x5177f640);
  iRam515b2e40 = in_D2;
  if (in_D2 == 0) {
    iRam515b2e40 = 0x574f12f7;
  }
  iRam515b2e44 = in_D3;
  if (in_D3 == 0) {
    iRam515b2e44 = 0x574f1323;
  }
  return 0;
}
```

#### `FUN_574f0590` (`main_dev_serial_rsp_4f0590`) — `0x574f0590` (742 B)

Parser completo de pacotes GDB Remote Serial (`+$`, `-$`, `#cs`, checksum módulo 256) em `_dev_SERIAL` (`main_dev_serial_rsp_4f0590`).

<!-- source: doc_main.txt | FUNC FUN_574f0590 (main_dev_serial_rsp_4f0590) @ 0x574f0590 (size=742 B, req=0x574f0590) -->
```c
// Callers (0): 
// Referenced strings/literals:
//   0x57a72453: '-$E00#A5'
//   0x57a72464: '-$E02#A7'
//   0x57a7245c: '+$OK#9A'
//   0x57a7244a: '-$E01#A6'
uint FUN_574f0590(void)

{
  char cVar1;
  undefined4 uVar2;
  char *pcVar3;
  undefined4 uVar4;
  char *pcVar5;
  byte bVar6;
  uint uVar7;
  int iVar8;
  undefined4 uVar9;
  uint uVar10;
  ushort *puVar11;
  int iVar12;
  int *piVar13;
  undefined1 *puVar14;
  int iStack_28;
  undefined1 auStack_24 [36];
  
  puVar11 = (ushort *)&UNK_d4000054;
  if (iRam515b270c == 0) {
    iVar12 = iRam515b270c;
    func_0x5770d625(uRam577e5dc4);
    func_0x5770d53e(uRam577e5dc4);
    bVar6 = func_0x574f08b6();
    uVar10 = (uint)bVar6;
    if (uVar10 == 0x15) {
      bVar6 = UNK_c0001002;
      UNK_c0001002 = bVar6 & 0x7f;
      iVar12 = 1;
      pcVar5 = (char *)0x5177fe42;
      piVar13 = &iStack_28;
      do {
        func_0x574f0913(pcVar5,piVar13);
        uVar10 = 0;
        if ((*pcVar5 == '$') && (iStack_28 != -1)) {
          iVar8 = 0;
          puVar14 = auStack_24;
          pcVar3 = pcVar5 + -2;
          if (0 < iStack_28 + -4) {
            do {
              iVar8 = iVar8 + 1;
              uVar10 = uVar10 + (byte)pcVar5[iVar8];
            } while (iVar8 < iStack_28 + -4);
          }
          func_0x5771184a(puVar14,pcVar3 + iStack_28,2);
          uVar7 = func_0x57711bcd(puVar14,0,0x10);
          if ((uVar7 & 0xff) != (uVar10 & 0xff)) {
            uVar9 = 0x57a72453;
            goto LAB_574f081f;
          }
          cVar1 = pcVar3[3];
          if (cVar1 != 'm') {
            if (cVar1 == 'M') {
              iVar8 = func_0x574f0c63();
              goto LAB_574f0833;
            }
            if (cVar1 == 'x') {
              iVar8 = func_0x574f0a33();
              goto LAB_574f0833;
            }
            if (cVar1 == 'X') {
              iVar8 = func_0x574f0c63();
              goto LAB_574f0833;
            }
            if (cVar1 == 'c') {
              iVar8 = func_0x574f0a33();
              goto LAB_574f0833;
            }
            if (cVar1 == 'g') {
              iVar8 = func_0x574f0da3();
              goto LAB_574f0833;
            }
            if (cVar1 != 'Z') {
LAB_574f080c:
              uVar9 = 0x57a72464;
              goto LAB_574f081f;
            }
            if (pcVar3[4] == '0') {
              iVar12 = 0;
              uVar9 = 0x57a7245c;
              uVar2 = 8;
            }
            else {
              if (pcVar3[4] != '1') goto LAB_574f080c;
              uVar9 = 0x57a7245c;
              uVar2 = 8;
            }
            goto LAB_574f0821;
          }
          iVar8 = func_0x574f0a33();
        }
        else {
          uVar9 = 0x57a7244a;
LAB_574f081f:
          uVar2 = 9;
LAB_574f0821:
          uRam51780282 = 0x15;
          uVar4 = 0x51780282;
          func_0x57710c81(0x51780283,uVar9,uVar2);
          iVar8 = func_0x5771166e(uVar4);
        }
LAB_574f0833:
        if (iVar8 != 0) {
          func_0x574f0876(0x51780282);
        }
      } while (iVar12 != 0);
      func_0x5770d53e(uRam577e5dc4);
      func_0x5770d5bc(uRam577e5dc4);
      bVar6 = UNK_c0001002;
      uVar7 = bVar6 | 0x80;
      UNK_c0001002 = (char)uVar7;
    }
    else {
      uVar7 = func_0x5770d5bc(uRam577e5dc4);
      if (uRam515b2e58 < 0x800) {
        *(byte *)(uRam515b2e5c + 0x5177f642) = bVar6;
        uRam515b2e5c = uRam515b2e5c + 1 & 0x7ff;
        uRam515b2e58 = uRam515b2e58 + 1;
        uRam515b2e3e = 1;
        *puVar11 = *puVar11 | 0x11;
        uVar7 = (uint)bRam515b2e60;
        if ((uVar7 == 0) && (uVar10 == 0xe)) {
          uVar7 = 3;
          bRam515b2e60 = 3;
        }
        else if ((uVar7 == 3) && (uVar10 == 4)) {
          uVar7 = 0xc3;
          bRam515b2e60 = 0xc3;
        }
        else {
          iRam515b2e64 = iVar12;
          if ((uVar7 == 0xc3) && (uVar10 == 2)) {
            uVar7 = 1;
            iRam515b2e64 = 1;
          }
        }
      }
    }
  }
  else {
    func_0x5770d625(uRam577e5dc4);
    func_0x5770d53e(uRam577e5dc4);
    uVar7 = func_0x5770d5bc(uRam577e5dc4);
  }
  return uVar7;
}
```

#### `FUN_574f8354` (`main_enc_menu_read_4f8354`) — `0x574f8354` (48 B)

Leitura de entrada no menu de diagnóstico do encoder `ENC` (`main_enc_menu_read_4f8354`).

<!-- source: doc_main.txt | FUNC FUN_574f8354 (main_enc_menu_read_4f8354) @ 0x574f8354 (size=48 B, req=0x574f8354) -->
```c
// Callers (1): 0x574f863a
undefined4 FUN_574f8354(undefined1 *param_1)

{
  int iVar1;
  undefined4 uVar2;
  undefined1 auStack_9 [9];
  
  uVar2 = 1;
  iVar1 = func_0x57516ecd(0xe4,auStack_9,1);
  if (iVar1 != 0) {
    auStack_9[0] = 0x4c;
    uVar2 = 0;
  }
  *param_1 = auStack_9[0];
  return uVar2;
}
```

#### `FUN_574f862b` (`main_enc_menu_loop_4f862b`) — `0x574f862b` (142 B)

Loop de seleção do menu `ENC` (`main_enc_menu_loop_4f862b`).

<!-- source: doc_main.txt | FUNC FUN_574f862b (main_enc_menu_loop_4f862b) @ 0x574f862b (size=142 B, req=0x574f862b) -->
```c
// Callers (1): 0x577d9017
void FUN_574f862b(void)

{
  undefined4 uVar1;
  uint uVar2;
  undefined1 *puVar3;
  byte bStack_1a;
  char cStack_19;
  undefined1 auStack_18 [24];
  
  puVar3 = auStack_18;
  FUN_574f8354(&cStack_19);
  if (cStack_19 == 'I') {
    func_0x575170c9(1);
    uVar1 = 1;
    func_0x57516ecd(0xd0,&bStack_1a,1);
    if (uRam577e50d0 == bStack_1a) {
      uVar2 = 0;
      do {
        func_0x57516fdd(0xd3,puVar3 + (uVar2 & 0xff),uVar1,uVar2 & 0xff);
        uVar2 = uVar2 + 1;
      } while ((uVar2 & 0xff) < 0xb);
      uVar2 = 0;
      do {
        func_0x57516fbb(0xd3,puVar3 + (uVar2 & 0xff),uVar2 & 0xff);
        uVar2 = uVar2 + 1;
      } while ((uVar2 & 0xff) < 0xb);
    }
    func_0x575170c9(0);
  }
  return;
}
```

### 3.2 Front Panel Test Mode (`FPC_TestModeControl`), `TEST+00` & ODD / `ATIH` / `NFROM BGO FAIL`

#### `FUN_574f77b0` (`main_odd_gate_4f77b0`) — `0x574f77b0` (133 B)

Gate do subsistema de drive óptico ODD (`main_odd_gate_4f77b0`).

<!-- source: doc_main.txt | FUNC FUN_574f77b0 (main_odd_gate_4f77b0) @ 0x574f77b0 (size=133 B, req=0x574f77b0) -->
```c
// Callers (0): 
// Referenced strings/literals:
//   0x57ae2e5b: '-- set_alm failed --\n'
//   0x57ae2e32: '-- already set HungUpTimer -- task = %d\n'
undefined4 FUN_574f77b0(undefined4 param_1)

{
  undefined4 uVar1;
  int iVar2;
  int iVar3;
  undefined1 auStack_1c [28];
  
  uVar1 = 1;
  if (iRam515cc564 == 0) {
    iVar3 = iRam515cc564;
    func_0x5770dcd5(0x515cc568);
    iVar2 = func_0x5770eb1a(0,param_1,auStack_1c,0x515cc564);
    if (iVar2 != 0) {
      uRam515cc568 = (undefined2)iVar3;
      iRam515cc564 = iVar3;
      func_0x574f6fb1(0x57ae2e5b);
      uVar1 = 0;
    }
  }
  else {
    func_0x574f6fb1(0x57ae2e32,0);
    uVar1 = 0;
  }
  return uVar1;
}
```

#### `FUN_574f786a` (`main_odd_state_4f786a`) — `0x574f786a` (8 B)

Leitura de estado ODD (`main_odd_state_4f786a`).

<!-- source: doc_main.txt | FUNC FUN_574f786a (main_odd_state_4f786a) @ 0x574f786a (size=8 B, req=0x574f786a) -->
```c
// Callers (0): 
// Referenced strings/literals:
//   0x57c154ab: 'DVR_SYSTEM_LIB: sys_boot.a Ver6.0.076.03(2006/03/14) : sys_boot(M6)'
undefined4 FUN_574f786a(void)

{
  return 0x57c154ab;
}
```

#### `FUN_574f7b00` (`main_odd_loop_4f7b00`) — `0x574f7b00` (106 B)

Loop de comunicação ODD (`main_odd_loop_4f7b00`).

<!-- source: doc_main.txt | FUNC FUN_574f7b00 (main_odd_loop_4f7b00) @ 0x574f7b00 (size=106 B, req=0x574f7b00) -->
```c
// Callers (0): 
// Referenced strings/literals:
//   0x57ae2f53: 'src/sys_cd_apl_main.c'
//   0x57ae2f69: 'CD APL Start!! \n'
void FUN_574f7b00(undefined4 param_1,undefined1 param_2)

{
  int in_A3;
  byte bVar1;
  short sVar2;
  int iVar3;
  byte in_PSW;
  int in_R7;
  
  uRam000003a0 = param_2;
  if ((in_PSW & 1) != 0) {
    bVar1 = func_0x574f8c99();
    if (bVar1 == 2) {
      func_0x574f8c5d();
      func_0x574fbac7(3,0x81);
    }
    else if (bVar1 < 2) {
      if (bVar1 == 1) {
        func_0x574f8c40();
        func_0x574fbac7(3,0x80);
      }
      else {
LAB_574f7b49:
        func_0x570b0377(0,0x57ae2f53);
      }
    }
    else if (bVar1 != 3) goto LAB_574f7b49;
  }
  func_0x574fb0ae();
  func_0x574fa8d6();
  func_0x574fa8c7();
  func_0x574f8242();
  func_0x574fa62b();
  func_0x574fb7d8(uRam5318455c);
  if (in_A3 < 4) {
    if (1 < in_A3) goto LAB_574f7c54;
    if (in_A3 != 1) goto LAB_574f7c6e;
    sVar2 = func_0x5750a5b5();
    if (sVar2 == 0xfb) {
      if (in_R7 == 2) goto LAB_574f7c05;
      func_0x574f9e33();
      func_0x574fb0bf();
      if (((cRam53184a18 == '\x01') || (cRam53184a18 == '\x03')) &&
         (iVar3 = func_0x574f968d(), iVar3 == 0)) {
        func_0x574fb20f();
        goto LAB_574f7c81;
      }
      func_0x574fb2cd();
LAB_574f7bf5:
      func_0x574f8495();
      goto LAB_574f7c81;
    }
    if (in_R7 == 2) {
LAB_574f7c05:
      func_0x574fb929(2);
      sVar2 = func_0x5750a5b5();
      if ((sVar2 != 0xfb) ||
         (((cRam53184a18 == '\x01' || (cRam53184a18 == '\x03')) &&
          (iVar3 = func_0x574f968d(), iVar3 != 1)))) goto LAB_574f7c81;
      goto LAB_574f7bf5;
    }
    if ((in_R7 == 0) && (sRam53184a19 == 0)) {
      func_0x574fb9d7();
      goto LAB_574f7c81;
    }
  }
  else {
    if (in_A3 != 9) {
LAB_574f7c6e:
      func_0x570b0377(0,0x57ae2f53);
      goto LAB_574f7c81;
    }
LAB_574f7c54:
    if (in_R7 != 0) {
      func_0x574fb929(in_R7);
      goto LAB_574f7c81;
    }
  }
  func_0x574fb9b0();
LAB_574f7c81:
  func_0x574fba10(in_A3,1);
  func_0x574f7dd5(0x57ae2f69);
  func_0x57088334();
  func_0x574fbabf();
  return;
}
```

#### `FUN_574f7b6a` (`main_odd_fn_4f7b6a`) — `0x574f7b6a` (318 B)

Função de controle ODD (`main_odd_fn_4f7b6a`).

<!-- source: doc_main.txt | FUNC FUN_574f7b6a (main_odd_fn_4f7b6a) @ 0x574f7b6a (size=318 B, req=0x574f7b6a) -->
```c
// Callers (0): 
// Referenced strings/literals:
//   0x57ae2f53: 'src/sys_cd_apl_main.c'
//   0x57ae2f69: 'CD APL Start!! \n'
void FUN_574f7b6a(void)

{
  int in_A3;
  short sVar1;
  int iVar2;
  int in_R7;
  
  func_0x574fa8c7();
  func_0x574f8242();
  func_0x574fa62b();
  func_0x574fb7d8(uRam5318455c);
  if (in_A3 < 4) {
    if (1 < in_A3) goto LAB_574f7c54;
    if (in_A3 != 1) goto LAB_574f7c6e;
    sVar1 = func_0x5750a5b5();
    if (sVar1 == 0xfb) {
      if (in_R7 == 2) goto LAB_574f7c05;
      func_0x574f9e33();
      func_0x574fb0bf();
      if (((cRam53184a18 == '\x01') || (cRam53184a18 == '\x03')) &&
         (iVar2 = func_0x574f968d(), iVar2 == 0)) {
        func_0x574fb20f();
        goto LAB_574f7c81;
      }
      func_0x574fb2cd();
LAB_574f7bf5:
      func_0x574f8495();
      goto LAB_574f7c81;
    }
    if (in_R7 == 2) {
LAB_574f7c05:
      func_0x574fb929(2);
      sVar1 = func_0x5750a5b5();
      if ((sVar1 != 0xfb) ||
         (((cRam53184a18 == '\x01' || (cRam53184a18 == '\x03')) &&
          (iVar2 = func_0x574f968d(), iVar2 != 1)))) goto LAB_574f7c81;
      goto LAB_574f7bf5;
    }
    if ((in_R7 == 0) && (sRam53184a19 == 0)) {
      func_0x574fb9d7();
      goto LAB_574f7c81;
    }
  }
  else {
    if (in_A3 != 9) {
LAB_574f7c6e:
      func_0x570b0377(0,0x57ae2f53);
      goto LAB_574f7c81;
    }
LAB_574f7c54:
    if (in_R7 != 0) {
      func_0x574fb929(in_R7);
      goto LAB_574f7c81;
    }
  }
  func_0x574fb9b0();
LAB_574f7c81:
  func_0x574fba10(in_A3,1);
  func_0x574f7dd5(0x57ae2f69);
  func_0x57088334();
  func_0x574fbabf();
  return;
}
```

#### `FUN_5771abfb` (`main_fpc_status_ctrl_71abfb`) — `0x5771abfb` (174 B)

Controle de status/painel frontal `FPC_TestModeControl` — contém `0x5771ac87: BCLR 0xff,(0x0,A2)` (`main_fpc_status_ctrl_71abfb`).

<!-- source: doc_main.txt | FUNC FUN_5771abfb (main_fpc_status_ctrl_71abfb) @ 0x5771abfb (size=174 B, req=0x5771abfb) -->
```c
// Callers (0): 
// Referenced strings/literals:
//   0x57b988f4: 'FPC_TestModeControl()\n'
int FUN_5771abfb(undefined4 param_1,uint param_2,undefined1 *param_3)

{
  int iVar1;
  undefined1 uStack_2c;
  char cStack_2b;
  undefined1 uStack_2a;
  undefined1 uStack_29;
  undefined1 uStack_28;
  undefined1 auStack_1c [28];
  
  func_0x5706b3f4(1,0x57b988f4);
  param_2 = param_2 & 0xff;
  iVar1 = func_0x5706ac58(0x15,auStack_1c,&uStack_2c);
  if (iVar1 != 0) {
    return iVar1;
  }
  if (param_2 != 0x3f) {
    if ((int)param_2 < 0x3f) {
      if (param_2 != 5) {
        if (param_2 != 0x3e) {
          return 0;
        }
        *param_3 = uStack_2a;
        param_3[1] = uStack_29;
        param_3[2] = uStack_28;
        return 0;
      }
    }
    else {
      if (0x94 < (int)param_2) {
        return 0;
      }
      if ((int)param_2 < 0x91) {
        return 0;
      }
      if ((cStack_2b != -0x6f) && (cStack_2b != -0x6d)) {
        *param_3 = *param_3;
        return 0;
      }
      uStack_2a = 1;
    }
  }
  *param_3 = uStack_2a;
  return 0;
}
```

#### `FUN_5749efd3` (`main_test_mode_49efd3`) — `0x5749efd3` (179 B)

Tratamento de entrada em modo de teste e `TEST+00` (`main_test_mode_49efd3`).

<!-- source: doc_main.txt | FUNC FUN_5749efd3 (main_test_mode_49efd3) @ 0x5749efd3 (size=179 B, req=0x5749efd3) -->
```c
// Callers (9): 0x57082529, 0x5749f185, 0x5749f41a, 0x574a007e, 0x57557fa8, 0x5755811c, 0x5756975b, 0x576bff44, 0x576c008f
// Referenced strings/literals:
//   0x57a50342: '[nfrom] Error. NFD_Init() not call.\n'
//   0x57a50295: 'src/nfd_api.c'
//   0x57a50367: '[nfrom] File number[%d] error.\n'
undefined4 FUN_5749efd3(uint param_1)

{
  undefined4 uVar1;
  uint uVar2;
  uint uVar3;
  undefined4 uVar4;
  
  if (iRam515b252c == 0) {
    func_0x574a2cf6(1,0x57a50342);
    func_0x570b0377(0,0x57a50295,0x14c);
    uVar1 = 0xfffffff5;
  }
  else if (param_1 < 0x30) {
    uVar1 = func_0x574a02b3();
    uVar3 = (uint)*(ushort *)(param_1 * 4 + 0x530fcd80);
    if (((uVar3 < 3) && (uVar2 = (uint)*(ushort *)(param_1 * 4 + 0x530fcd82), uVar2 < 0x10)) &&
       (*(int *)(uVar3 * 0x200 + uVar2 * 0x10 + 0x530fb388) != 0)) {
      uVar4 = 0;
    }
    else {
      uVar4 = 0xfffffffc;
    }
    uVar1 = func_0x5749ee17(uVar1,uVar4,0);
  }
  else {
    func_0x574a2cf6(1,0x57a50367,param_1);
    func_0x570b0377(0,0x57a50295,0x153);
    uVar1 = 0xfffffffb;
  }
  return uVar1;
}
```

#### `FUN_574a00cc` (`main_nfrom_bgo_fail_4a00cc`) — `0x574a00cc` (270 B)

Manipulador de falha BGO/NFROM no `MAIN` (`main_nfrom_bgo_fail_4a00cc`).

<!-- source: doc_main.txt | FUNC FUN_574a00cc (main_nfrom_bgo_fail_4a00cc) @ 0x574a00cc (size=270 B, req=0x574a00cc) -->
```c
// Callers (3): 0x573e3c2e, 0x573e3c54, 0x573e3c76
// Referenced strings/literals:
//   0x57a50260: 'NFD_NMIWRITE_END[nfrom] BGO driver init failed[%u].\n'
int FUN_574a00cc(uint param_1,undefined4 param_2,uint param_3)

{
  int iVar1;
  int iVar2;
  undefined4 uVar3;
  
  if (iRam515b252c == 0) {
    return -0xb;
  }
  if (0x2f < param_1) {
    return -5;
  }
  if (iRam515b2538 != 1) {
    iVar2 = func_0x574a0d02(param_2);
    if (iVar2 != 0) {
      return iVar2;
    }
    iRam531844ac = 0;
    if ((param_3 & 0xfffffff0) + iRam515b2544 + 0x30 < 0x84080000) {
      iVar1 = 0x530fd060;
      iRam531844ac = iVar2;
      func_0x57710c81(0x530fd060,param_1 * 9 + 0x515b2554,8);
      *(uint *)(iVar1 + 8) = param_3;
      *(undefined1 *)(iVar1 + 0xc) = *(undefined1 *)(iVar1 + 0xc);
      *(undefined1 *)(iVar1 + 0xd) = *(undefined1 *)(iVar1 + 0xd);
      *(undefined1 *)(iVar1 + 0xe) = *(undefined1 *)(iVar1 + 0xe);
      *(undefined1 *)(iVar1 + 0xf) = *(undefined1 *)(iVar1 + 0xf);
      uVar3 = 0x10;
      iVar2 = func_0x574a207f(iRam515b2544,iVar1,0x10);
      if (iVar2 == 0) {
        iRam515b2544 = iRam515b2544 + 0x10;
        iVar2 = func_0x574a207f(iRam515b2544,param_2,param_3);
        if (iVar2 == 0) {
          iRam515b2544 = iRam515b2544 + param_3;
          if ((param_3 & 0xf) != 0) {
            iRam515b2544 = (iRam515b2544 + 0x10) - (param_3 & 0xf);
          }
          iVar2 = func_0x574a207f(iRam515b2544,0x57a50260,uVar3);
          if (iVar2 == 0) {
            iRam515b2544 = iRam515b2544 + 0x10;
            return 0;
          }
        }
      }
      return -2;
    }
  }
  return 0;
}
```

#### `FUN_5749859a` (`main_atih_caller_49859a`) — `0x5749859a` (805 B)

Chamador/handshake de autenticação/comando `'ATIH'` (`main_atih_caller_49859a`).

<!-- source: doc_main.txt | FUNC FUN_5749859a (main_atih_caller_49859a) @ 0x5749859a (size=805 B, req=0x5749859a) -->
```c
// Callers (1): 0x574bc1c3
// Referenced strings/literals:
//   0x57a4e1e0: '[fmu] *** PRE UPDATE START *** \n'
//   0x57a4e201: '[fmu] *** media = DVD-RAM *** \n'
//   0x57a4e221: '[fmu] <deviceinfo> model = %s, oem = %c\n'
//   0x57a4e24a: '[fmu] <now> Drive Ver = %s\n'
//   0x57a4e266: '[fmu] <now> Main Ver  = %s\n'
//   0x57a4e282: '[fmu] <now> Timer Ver = %s\n'
//   0x57a4e29e: '[fmu] Drive Firm Read error \n'
//   0x57a4e2bc: 'src/fmupre/pre_update_firm.c'
//   0x57a4e2d9: 'PROG'
//   0x57a4e2de: 'DRV '
//   0x57a4e2e3: 'MAIN'
//   0x57a4e2e8: 'TIM '
//   0x57a4e2ed: 'LDER'
//   0x57a4e2f2: 'USER'
//   0x57a4e2f7: 'INFO'
//   0x57a4e2fc: '[fmu] Timer = Flash.\n'
//   0x57a4e312: '[fmu] Timer = Mask.\n'
//   0x57a4e327: '[fmu] no need to update firm.\n'
//   0x57a4e346: '[fmu] *** PRE UPDATE END ***\n'
void FUN_5749859a(void)

{
  int iVar1;
  undefined4 uVar2;
  int iVar3;
  int iVar4;
  undefined4 uVar5;
  int iVar6;
  undefined1 *puVar7;
  undefined4 uVar8;
  undefined4 in_R5;
  undefined4 in_R7;
  int iStack_78;
  undefined4 uStack_74;
  undefined4 uStack_70;
  undefined4 uStack_6c;
  undefined4 uStack_68;
  char acStack_61 [16];
  char cStack_51;
  undefined1 auStack_4c [12];
  undefined1 auStack_40 [24];
  char cStack_28;
  
  iVar1 = 1;
  uRam531844b8 = 1;
  func_0x574990ac();
  uVar8 = 0x20;
  func_0x57710d14(auStack_40,0,0x20);
  func_0x57710d14(0x51779958,0,0xb);
  puVar7 = auStack_4c;
  func_0x574a2e60(4,8,puVar7);
  func_0x574a2e60(0xc,iVar1,acStack_61);
  func_0x574bab20(iVar1);
  func_0x576ef874();
  func_0x574f74f1();
  func_0x574992d8(iVar1,0x57a4e1e0);
  func_0x574992d8(iVar1,0x57a4e201);
  func_0x574992d8(iVar1,0x57a4e221,puVar7,acStack_61[0]);
  func_0x574992d8(iVar1,0x57a4e24a,uStack_68);
  func_0x574992d8(iVar1,0x57a4e266,in_R7);
  func_0x574992d8(iVar1,0x57a4e282,in_R5);
  iVar3 = func_0x574df186(uVar8,0x301,iVar1);
  iVar6 = iStack_78;
  iVar4 = iStack_78;
  iStack_78 = iVar1;
  if (iVar3 < 0) {
    func_0x574992d8(iVar1,0x57a4e29e);
    func_0x570b0377(0,0x57a4e2bc,0xbd);
  }
  else {
    do {
      func_0x574bb954(iStack_78 << 5,iVar4,uVar8);
      iVar1 = iStack_78 + -1;
      func_0x57499488(iVar4,iVar1);
      iStack_78 = iVar1 + 2;
    } while (iVar1 + 2 < 0x40);
    func_0x574994c7(0x3f);
    iStack_78 = 1;
    uVar8 = 8;
    uVar2 = 4;
    do {
      func_0x574bb954(iStack_78 << 5,iVar6,0x20);
      iVar4 = func_0x577117bf(uStack_6c,uStack_74,uVar8);
      if ((iVar4 == 0) && (cStack_28 == acStack_61[0])) {
        iVar4 = func_0x577117bf(iVar6,0x57a4e2d9,uVar2);
        if (iVar4 == 0) {
          uRam51779962 = 1;
        }
        else {
          iVar4 = func_0x577117bf(iVar6,0x57a4e2de,uVar2);
          if (iVar4 == 0) {
            uVar5 = 0;
          }
          else {
            iVar4 = func_0x577117bf(iVar6,0x57a4e2e3,uVar2);
            if (iVar4 == 0) {
              uVar5 = 2;
              uStack_68 = in_R7;
            }
            else {
              iVar4 = func_0x577117bf(iVar6,0x57a4e2e8,uVar2);
              if (iVar4 == 0) {
                uVar5 = 1;
                uStack_68 = in_R5;
              }
              else {
                iVar4 = func_0x577117bf(iVar6,0x57a4e2ed,uVar2);
                if (iVar4 == 0) {
                  uVar5 = 5;
                  uStack_68 = 0;
                }
                else {
                  iVar4 = func_0x577117bf(iVar6,0x57a4e2f2,uVar2);
                  if (iVar4 == 0) {
                    uVar5 = 7;
                    uStack_68 = 0;
                  }
                  else {
                    uVar5 = uVar8;
                    iVar4 = func_0x577117bf(iVar6,0x57a4e2f7,uVar2);
                    uVar8 = uVar5;
                    if (iVar4 != 0) goto LAB_5749881e;
                    uStack_68 = 0;
                  }
                }
              }
            }
          }
          func_0x57498eaf(uVar5,iVar6,uStack_68,0);
        }
      }
LAB_5749881e:
      iStack_78 = iStack_78 + 1;
    } while (iStack_78 < 0x40);
    func_0x57068b46(uStack_70);
    if (cStack_51 == '\0') {
      uVar8 = 0x57a4e2fc;
    }
    else {
      uVar8 = 0x57a4e312;
    }
    func_0x574992d8(1,uVar8);
    func_0x57498fdc();
    iStack_78 = 0;
    do {
      if ((iStack_78 != 10) && (iVar6 = func_0x5749914a(iStack_78), iVar6 == 1)) {
        func_0x574990f7();
        iRam515b24fc = iVar6;
        func_0x57509c3d(0);
        func_0x57498592();
        break;
      }
      iStack_78 = iStack_78 + 1;
    } while (iStack_78 < 0xb);
    if (10 < iStack_78) {
      func_0x574992d8(1,0x57a4e327);
    }
    func_0x574992d8(1,0x57a4e346);
  }
  return;
}
```

### 3.3 Subsistema HDD LBA48 (`FUN_574dd010..0x574dd273`), `400GB` vs `200GB` & Gate `MAIN`

#### Placeholder `NO_FUNC main_hdd_cmd_010 @ 0x574dd010` (`doc_main.txt`)

Na exportação inicial de `doc_main.txt`, `0x574dd010` ainda estava sem função criada; a decompilação verificada segue abaixo.

<!-- source: doc_main.txt | NO_FUNC main_hdd_cmd_010 @ 0x574dd010 -->
*(Sem corpo na exportação original; referência cruzada detalhada na subseção correspondente.)*

#### `FUN_574dd010` (`main_hdd_cmd_010` — Decompilação Verificada via `verify_ghidra_doc_facts_out.txt`)

```c
--- MAIN DECOMP @ 0x574dd010 (FUN_574dd010) ---
undefined4 FUN_574dd010(void)

{
  uint uVar1;
  undefined4 in_D3;
  undefined1 *puStack00000064;
  
  FUN_574efdda();
  uVar1 = 0;
  puStack00000064 = (undefined1 *)0x5311ba80;
  do {
    puStack00000064 = (undefined1 *)FUN_574efdda(0x57a5e8a6,*puStack00000064);
    uVar1 = uVar1 + 1;
  } while (uVar1 < 0x10);
  func_0x570b0377(0,0x57a5e8fc);
  return in_D3;
}
```

#### `FUN_574dd0fd` (`main_hdd_cmd_0fd`) — `0x574dd0fd` (74 B)

Sub-rotina LBA48 do subsistema HDD (`main_hdd_cmd_0fd`).

<!-- source: doc_main.txt | FUNC FUN_574dd0fd (main_hdd_cmd_0fd) @ 0x574dd0fd (size=74 B, req=0x574dd0fd) -->
```c
// Callers (0): 
// Referenced strings/literals:
//   0x57a5e8a6: '%02X %02X %02X %02X  %02X %02X %02X %02X    %02X %02X %02X %02X  %02X %02X %02X %02X\n'
//   0x57a5e8fc: 'src/hdd_cmd.c'
undefined4 FUN_574dd0fd(undefined4 param_1,uint param_2)

{
  uint in_D2;
  undefined4 in_D3;
  byte *pbStack00000064;
  
  while( true ) {
    pbStack00000064 = (byte *)FUN_574efdda(0x57a5e8a6,param_2);
    in_D2 = in_D2 + 1;
    if (0xf < in_D2) break;
    param_2 = (uint)*pbStack00000064;
  }
  func_0x570b0377(0,0x57a5e8fc);
  return in_D3;
}
```

#### `thunk_FUN_574dd15b` (`main_hdd_cmd_156`) — `0x574dd156` (2 B)

Thunk para `FUN_574dd15b` (`main_hdd_cmd_156`).

<!-- source: doc_main.txt | FUNC thunk_FUN_574dd15b (main_hdd_cmd_156) @ 0x574dd156 (size=2 B, req=0x574dd156) -->
```c
// Callers (0): 
// Referenced strings/literals:
//   0x57a5e930: '[RTSD] Read DevInfo(option) Err=%08x \n'
//   0x57a5e971: '[RTSD] M7920(HDD 200GB) \n'
//   0x57a5e90a: '[RTSD] Read DevInfo(model) Err=%08x \n'
//   0x57a5e8fc: 'src/hdd_cmd.c'
/* WARNING: Removing unreachable block (ram,0x574dd208) */

uint thunk_FUN_574dd15b(uint param_1)

{
  char cVar1;
  char cVar2;
  bool bVar3;
  char *pcVar4;
  uint uVar5;
  char *pcStack0000007c;
  
  uRam5311bc80 = (param_1 & 0xfffffffc) - 0x50000 >> 2;
  pcVar4 = &stack0x00000090;
  uVar5 = func_0x574a2e60(4,8);
  if (uVar5 == 0) {
    bVar3 = true;
    uVar5 = 0;
    do {
      cVar1 = *pcStack0000007c;
      pcStack0000007c = pcStack0000007c + 1;
      cVar2 = *pcVar4;
      pcVar4 = pcVar4 + 1;
      if (cVar1 != cVar2) {
        bVar3 = false;
        break;
      }
      uVar5 = uVar5 + 1;
    } while (uVar5 < 8);
    if (bVar3) {
      uVar5 = func_0x574a2e60(0xd,6);
      if (uVar5 != 0) {
        FUN_574efdda(0x57a5e930,uVar5);
        goto LAB_574dd1ef;
      }
      mn_todo();
      uRam5311b9ac = 0x5d21dba;
      FUN_574efdda(0x57a5e971);
    }
    else {
      uRam5311b9ac = 0x4a6d7c8;
    }
    if (uRam5311b9ac < uRam5311bc80) {
      uRam5311bc80 = uRam5311b9ac;
    }
    uRam5311bc80 = uRam5311bc80 - 1;
    uVar5 = uRam5311bc80;
  }
  else {
    FUN_574efdda(0x57a5e90a,uVar5);
LAB_574dd1ef:
    func_0x570b0377(0,0x57a5e8fc);
  }
  return uVar5;
}
```

#### `FUN_574dd15b` (`main_hdd_cmd_1fc_containing`) — `0x574dd15b` (281 B)

Função LBA48 contendo `0x574dd1fc` e o ramo `0x574dd208` ocultado pelo decompilador (`[RTSD] M7920(HDD 400GB)` vs `200GB`, `main_hdd_cmd_1fc_containing`).

<!-- source: doc_main.txt | FUNC FUN_574dd15b (main_hdd_cmd_1fc_containing) @ 0x574dd15b (size=281 B, req=0x574dd1fc) -->
```c
// Callers (1): 0x574dd156
// Referenced strings/literals:
//   0x57a5e930: '[RTSD] Read DevInfo(option) Err=%08x \n'
//   0x57a5e971: '[RTSD] M7920(HDD 200GB) \n'
//   0x57a5e90a: '[RTSD] Read DevInfo(model) Err=%08x \n'
//   0x57a5e8fc: 'src/hdd_cmd.c'
/* WARNING: Removing unreachable block (ram,0x574dd208) */

uint FUN_574dd15b(uint param_1)

{
  char cVar1;
  char cVar2;
  bool bVar3;
  char *pcVar4;
  uint uVar5;
  char *pcStack0000007c;
  
  uRam5311bc80 = (param_1 & 0xfffffffc) - 0x50000 >> 2;
  pcVar4 = &stack0x00000090;
  uVar5 = func_0x574a2e60(4,8);
  if (uVar5 == 0) {
    bVar3 = true;
    uVar5 = 0;
    do {
      cVar1 = *pcStack0000007c;
      pcStack0000007c = pcStack0000007c + 1;
      cVar2 = *pcVar4;
      pcVar4 = pcVar4 + 1;
      if (cVar1 != cVar2) {
        bVar3 = false;
        break;
      }
      uVar5 = uVar5 + 1;
    } while (uVar5 < 8);
    if (bVar3) {
      uVar5 = func_0x574a2e60(0xd,6);
      if (uVar5 != 0) {
        FUN_574efdda(0x57a5e930,uVar5);
        goto LAB_574dd1ef;
      }
      mn_todo();
      uRam5311b9ac = 0x5d21dba;
      FUN_574efdda(0x57a5e971);
    }
    else {
      uRam5311b9ac = 0x4a6d7c8;
    }
    if (uRam5311b9ac < uRam5311bc80) {
      uRam5311bc80 = uRam5311b9ac;
    }
    uRam5311bc80 = uRam5311bc80 - 1;
    uVar5 = uRam5311bc80;
  }
  else {
    FUN_574efdda(0x57a5e90a,uVar5);
LAB_574dd1ef:
    func_0x570b0377(0,0x57a5e8fc);
  }
  return uVar5;
}
```

#### Disassembly Exato dos Sítios de Patch / Gate no `/EH55_MAIN_3410PQ.bin`

Listagem exata de disassembly dos gates `UART0`/`ODD`/`FPC` no `MAIN`.

<!-- source: doc_main.txt | MAIN EXACT PATCH / GATE DISASSEMBLY -->
```asm
-- TBC Site 1 (0x5771ABFB) --
  0x5771abfb: [cf e0               ] MOVM [D2,D3,A2],(SP)
  0x5771abfd: [f8 fe d0            ] ADD -0x30,SP
  0x5771ac00: [82                  ] MOV D0,D2
  0x5771ac01: [5e 48               ] MOV (0x48,SP),A2
  0x5771ac03: [87                  ] MOV D1,D3
  0x5771ac04: [fc cd f4 88 b9 57   ] MOV 0x57b988f4,D1
  0x5771ac0a: [80 01               ] MOV 0x1,D0
  0x5771ac0c: [dd e8 07 95 ff 00 00] CALL 0x5706b3f4,[],0x0
  0x5771ac13: [00                  ] CLR D0
  0x5771ac14: [42 20               ] MOV D0,(0x20,SP)
  0x5771ac16: [3c                  ] MOV SP,A0
  0x5771ac17: [20 24               ] ADD 0x24,A0
  0x5771ac19: [f9 7a c8            ] MOV D0,(A0+)
  0x5771ac1c: [f9 7a c8            ] MOV D0,(A0+)
  0x5771ac1f: [60                  ] MOV D0,(A0)
  0x5771ac20: [16                  ] EXTBU D2
  0x5771ac21: [3d                  ] MOV SP,A1
  0x5771ac22: [21 20               ] ADD 0x20,A1
  0x5771ac24: [f1 d5               ] MOV A1,D1
  0x5771ac26: [aa 01               ] CMP 0x1,D2
  0x5771ac28: [c9 07               ] BNE 0x5771ac2f
  0x5771ac2a: [2c c2 00            ] MOV 0xc2,D0
  0x5771ac2d: [ca 05               ] BRA 0x5771ac32
  0x5771ac2f: [2c b8 00            ] MOV 0xb8,D0
-- TBC Site 2 & 3 (0x574F7B6A..0x574F7CA0) --
  0x574f7b6a: [dd 5d 2d 00 00 00 00] CALL 0x574fa8c7,[],0x0
  0x574f7b71: [dd d1 06 00 00 00 00] CALL 0x574f8242,[],0x0
  0x574f7b78: [dd b3 2a 00 00 00 00] CALL 0x574fa62b,[],0x0
  0x574f7b7f: [fc a4 5c 45 18 53   ] MOV (0x5318455c),D0
  0x574f7b85: [dd 53 3c 00 00 00 00] CALL 0x574fb7d8,[],0x0
  0x574f7b8c: [bf 03               ] CMP 0x3,A3
  0x574f7b8e: [c1 10               ] BGT 0x574f7b9e
  0x574f7b90: [bf 02               ] CMP 0x2,A3
  0x574f7b92: [c0 05               ] BLT 0x574f7b97
  0x574f7b94: [cc c0 00            ] JMP 0x574f7c54
  0x574f7b97: [bf 01               ] CMP 0x1,A3
  0x574f7b99: [c8 0f               ] BEQ 0x574f7ba8
  0x574f7b9b: [cc d3 00            ] JMP 0x574f7c6e
  0x574f7b9e: [bf 09               ] CMP 0x9,A3
  0x574f7ba0: [c9 05               ] BNE 0x574f7ba5
  0x574f7ba2: [cc b2 00            ] JMP 0x574f7c54
  0x574f7ba5: [cc c9 00            ] JMP 0x574f7c6e
  0x574f7ba8: [dd 0d 2a 01 00 00 00] CALL 0x5750a5b5,[],0x0
  0x574f7baf: [1c                  ] EXTHU D0
  0x574f7bb0: [fa c8 fb 00         ] CMP 0xfb,D0
  0x574f7bb4: [c9 4b               ] BNE 0x574f7bff
  0x574f7bb6: [fb d8 77 02         ] CMP 0x2,R7
  0x574f7bba: [c8 4b               ] BEQ 0x574f7c05
  0x574f7bbc: [dd 77 22 00 00 00 00] CALL 0x574f9e33,[],0x0
  0x574f7bc3: [dd fc 34 00 00 00 00] CALL 0x574fb0bf,[],0x0
  0x574f7bca: [fc a8 18 4a 18 53   ] MOVBU (0x53184a18),D0
  0x574f7bd0: [a0 01               ] CMP 0x1,D0
  0x574f7bd2: [c8 06               ] BEQ 0x574f7bd8
  0x574f7bd4: [a0 03               ] CMP 0x3,D0
  0x574f7bd6: [c9 18               ] BNE 0x574f7bee
  0x574f7bd8: [dd b5 1a 00 00 00 00] CALL 0x574f968d,[],0x0
  0x574f7bdf: [10                  ] EXTB D0
  0x574f7be0: [a0 00               ] CMP 0x0,D0
  0x574f7be2: [c9 0c               ] BNE 0x574f7bee
  0x574f7be4: [dd 2b 36 00 00 00 00] CALL 0x574fb20f,[],0x0
  0x574f7beb: [cc 96 00            ] JMP 0x574f7c81
  0x574f7bee: [dd df 36 00 00 00 00] CALL 0x574fb2cd,[],0x0
  0x574f7bf5: [dd a0 08 00 00 00 00] CALL 0x574f8495,[],0x0
  0x574f7bfc: [cc 85 00            ] JMP 0x574f7c81
  0x574f7bff: [fb d8 77 02         ] CMP 0x2,R7
  0x574f7c03: [c9 35               ] BNE 0x574f7c38
  0x574f7c05: [80 02               ] MOV 0x2,D0
  0x574f7c07: [dd 22 3d 00 00 00 00] CALL 0x574fb929,[],0x0
  0x574f7c0e: [dd a7 29 01 00 00 00] CALL 0x5750a5b5,[],0x0
  0x574f7c15: [1c                  ] EXTHU D0
  0x574f7c16: [fa c8 fb 00         ] CMP 0xfb,D0
  0x574f7c1a: [c9 67               ] BNE 0x574f7c81
  0x574f7c1c: [fc a8 18 4a 18 53   ] MOVBU (0x53184a18),D0
  0x574f7c22: [a0 01               ] CMP 0x1,D0
  0x574f7c24: [c8 06               ] BEQ 0x574f7c2a
  0x574f7c26: [a0 03               ] CMP 0x3,D0
  0x574f7c28: [c9 cd               ] BNE 0x574f7bf5
  0x574f7c2a: [dd 63 1a 00 00 00 00] CALL 0x574f968d,[],0x0
  0x574f7c31: [10                  ] EXTB D0
  0x574f7c32: [a0 01               ] CMP 0x1,D0
  0x574f7c34: [c9 4d               ] BNE 0x574f7c81
  0x574f7c36: [ca bf               ] BRA 0x574f7bf5
  0x574f7c38: [fb d8 77 00         ] CMP 0x0,R7
  0x574f7c3c: [c9 29               ] BNE 0x574f7c65
  0x574f7c3e: [fe 2e 00 19 4a 18 53] MOVBU (0x53184a19),R0
  0x574f7c45: [fb d8 00 00         ] CMP 0x0,R0
  0x574f7c49: [c9 1c               ] BNE 0x574f7c65
  0x574f7c4b: [dd 8c 3d 00 00 00 00] CALL 0x574fb9d7,[],0x0
  0x574f7c52: [ca 2f               ] BRA 0x574f7c81
  0x574f7c54: [fb d8 77 00         ] CMP 0x0,R7
  0x574f7c58: [c8 0d               ] BEQ 0x574f7c65
  0x574f7c5a: [f5 dc               ] MOV R7,D0
  0x574f7c5c: [dd cd 3c 00 00 00 00] CALL 0x574fb929,[],0x0
  0x574f7c63: [ca 1e               ] BRA 0x574f7c81
  0x574f7c65: [dd 4b 3d 00 00 00 00] CALL 0x574fb9b0,[],0x0
  0x574f7c6c: [ca 15               ] BRA 0x574f7c81
  0x574f7c6e: [fc cd 53 2f ae 57   ] MOV 0x57ae2f53,D1
  0x574f7c74: [2c 5c 02            ] MOV 0x25c,D0
  0x574f7c77: [42 0c               ] MOV D0,(0xc,SP)
  0x574f7c79: [00                  ] CLR D0
  0x574f7c7a: [dd fd 86 bb ff 00 00] CALL 0x570b0377,[],0x0
  0x574f7c81: [f1 dc               ] MOV A3,D0
  0x574f7c83: [85 01               ] MOV 0x1,D1
  0x574f7c85: [dd 8b 3d 00 00 00 00] CALL 0x574fba10,[],0x0
  0x574f7c8c: [fc cc 69 2f ae 57   ] MOV 0x57ae2f69,D0
  0x574f7c92: [cd 43 01 00 00      ] CALL 0x574f7dd5,[],0x0
  0x574f7c97: [dd 9d 06 b9 ff 00 00] CALL 0x57088334,[],0x0
  0x574f7c9e: [dd 21 3e 00 00 00 00] CALL 0x574fbabf,[],0x0
-- HDD LBA48 clamp region (0x574DD156..0x574DD250) --
  0x574dd156: [ca 05               ] BRA 0x574dd15b
  0x574dd158: [f8 00 d2            ] MOV (-0x2e ,A0),D0
  0x574dd15b: [f8 c4 02            ] LSR 0x2,D0
  0x574dd15e: [54                  ] ASL2 D0
  0x574dd15f: [fc c0 00 00 fb ff   ] ADD -0x50000,D0
  0x574dd165: [f8 c4 02            ] LSR 0x2,D0
  0x574dd168: [fc 81 80 bc 11 53   ] MOV D0,(0x5311bc80)
  0x574dd16e: [3e                  ] MOV SP,A2
  0x574dd16f: [fa d2 90 00         ] ADD 0x90,A2
  0x574dd173: [4b 0c               ] MOV A2,(0xc,SP)
  0x574dd175: [80 04               ] MOV 0x4,D0
  0x574dd177: [85 08               ] MOV 0x8,D1
  0x574dd179: [dd e7 5c fc ff 00 00] CALL 0x574a2e60,[],0x0
  0x574dd180: [83                  ] MOV D0,D3
  0x574dd181: [af 00               ] CMP 0x0,D3
  0x574dd183: [c8 1e               ] BEQ 0x574dd1a1
  0x574dd185: [fc cc 0a e9 a5 57   ] MOV 0x57a5e90a,D0
  0x574dd18b: [8d                  ] MOV D3,D1
  0x574dd18c: [dd 4e 2c 01 00 00 00] CALL 0x574efdda,[],0x0
  0x574dd193: [fc cd fc e8 a5 57   ] MOV 0x57a5e8fc,D1
  0x574dd199: [2c 45 02            ] MOV 0x245,D0
  0x574dd19c: [ca 53               ] BRA 0x574dd1ef
  0x574dd19e: [0c                  ] CLR D3
  0x574dd19f: [ca 15               ] BRA 0x574dd1b4
  0x574dd1a1: [8f 01               ] MOV 0x1,D3
  0x574dd1a3: [08                  ] CLR D2
  0x574dd1a4: [5c 7c               ] MOV (0x7c,SP),A0
  0x574dd1a6: [f0 44               ] MOVBU (A0),D1
  0x574dd1a8: [41                  ] INC A0
  0x574dd1a9: [f0 42               ] MOVBU (A2),D0
  0x574dd1ab: [49                  ] INC A2
  0x574dd1ac: [a1                  ] CMP D0,D1
  0x574dd1ad: [c9 f1               ] BNE 0x574dd19e
  0x574dd1af: [48                  ] INC D2
  0x574dd1b0: [aa 07               ] CMP 0x7,D2
  0x574dd1b2: [c7 f4               ] BLS 0x574dd1a6
  0x574dd1b4: [af 00               ] CMP 0x0,D3
  0x574dd1b6: [c9 05               ] BNE 0x574dd1bb
  0x574dd1b8: [cc 86 00            ] JMP 0x574dd23e
  0x574dd1bb: [f9 e8 00            ] MOV SP,R0
  0x574dd1be: [fd 78 00 88 00 00   ] ADD 0x88,R0
  0x574dd1c4: [f5 c0               ] MOV R0,D0
  0x574dd1c6: [42 0c               ] MOV D0,(0xc,SP)
  0x574dd1c8: [80 0d               ] MOV 0xd,D0
  0x574dd1ca: [85 06               ] MOV 0x6,D1
  0x574dd1cc: [dd 94 5c fc ff 00 00] CALL 0x574a2e60,[],0x0
  0x574dd1d3: [83                  ] MOV D0,D3
  0x574dd1d4: [af 00               ] CMP 0x0,D3
  0x574dd1d6: [c8 26               ] BEQ 0x574dd1fc
  0x574dd1d8: [fc cc 30 e9 a5 57   ] MOV 0x57a5e930,D0
  0x574dd1de: [8d                  ] MOV D3,D1
  0x574dd1df: [dd fb 2b 01 00 00 00] CALL 0x574efdda,[],0x0
  0x574dd1e6: [fc cd fc e8 a5 57   ] MOV 0x57a5e8fc,D1
  0x574dd1ec: [2c 77 02            ] MOV 0x277,D0
  0x574dd1ef: [42 0c               ] MOV D0,(0xc,SP)
  0x574dd1f1: [00                  ] CLR D0
  0x574dd1f2: [dd 85 31 bd ff 00 00] CALL 0x570b0377,[],0x0
  0x574dd1f9: [8c                  ] MOV D3,D0
  0x574dd1fa: [ca 77               ] BRA 0x574dd271
  0x574dd1fc: [fd aa 00 8a 00 00   ] MOVBU (0x8a,SP),R0
  0x574dd202: [fb e9 00 80         ] BTST -0x80,R0
  0x574dd206: [c8 1d               ] BEQ 0x574dd223
  0x574dd208: [fc cc 74 3b a4 0b   ] MOV 0xba43b74,D0
  0x574dd20e: [fc 81 ac b9 11 53   ] MOV D0,(0x5311b9ac)
  0x574dd214: [fc cc 57 e9 a5 57   ] MOV 0x57a5e957,D0
  0x574dd21a: [dd c0 2b 01 00 00 00] CALL 0x574efdda,[],0x0
  0x574dd221: [ca 29               ] BRA 0x574dd24a
  0x574dd223: [fc cc ba 1d d2 05   ] MOV 0x5d21dba,D0
  0x574dd229: [fc 81 ac b9 11 53   ] MOV D0,(0x5311b9ac)
  0x574dd22f: [fc cc 71 e9 a5 57   ] MOV 0x57a5e971,D0
  0x574dd235: [dd a5 2b 01 00 00 00] CALL 0x574efdda,[],0x0
  0x574dd23c: [ca 0e               ] BRA 0x574dd24a
  0x574dd23e: [fc cc c8 d7 a6 04   ] MOV 0x4a6d7c8,D0
  0x574dd244: [fc 81 ac b9 11 53   ] MOV D0,(0x5311b9ac)
  0x574dd24a: [fc a5 ac b9 11 53   ] MOV (0x5311b9ac),D1
```

#### Prova em Disassembly da Seleção `400GB` (`0x57a5e957`) vs `200GB` (`0x57a5e971`) em `0x574dd1fc..0x574dd273` (`verified_extra_facts.txt` §5)

Demonstra que `FUN_574dd15b` possui ambos os ramos (`0xba43b74` -> `400GB` em `0x57a5e957`, com remoção falsa de `0x574dd208` pelo decompilador por saída de pilha não-modelada em `func_0x574a2e60`).

```text
  MAIN STR 0x57a5e957: '[RTSD] M7920(HDD 400GB) \n'
  MAIN STR 0x57a5e971: '[RTSD] M7920(HDD 200GB) \n'
  0x574dd156: [ca                   ] ?? CAh
  0x574dd157: [05                   ] ?? 05h
  0x574dd158: [f8                   ] ?? F8h
  0x574dd159: [00                   ] ?? 00h
  0x574dd15a: [d2                   ] ?? D2h
  0x574dd15b: [f8                   ] ?? F8h
  0x574dd15c: [c4                   ] ?? C4h
  0x574dd15d: [02                   ] ?? 02h
  0x574dd15e: [54                   ] ?? 54h    T
  0x574dd15f: [fc                   ] ?? FCh
  0x574dd160: [c0                   ] ?? C0h
  0x574dd161: [00                   ] ?? 00h
  0x574dd162: [00                   ] ?? 00h
  0x574dd163: [fb                   ] ?? FBh
  0x574dd164: [ff                   ] ?? FFh
  0x574dd165: [f8                   ] ?? F8h
  0x574dd166: [c4                   ] ?? C4h
  0x574dd167: [02                   ] ?? 02h
  0x574dd168: [fc                   ] ?? FCh
  0x574dd169: [81                   ] ?? 81h
  0x574dd16a: [80                   ] ?? 80h
  0x574dd16b: [bc                   ] ?? BCh
  0x574dd16c: [11                   ] ?? 11h
  0x574dd16d: [53                   ] ?? 53h    S
  0x574dd16e: [3e                   ] ?? 3Eh    >
  0x574dd16f: [fa                   ] ?? FAh
  0x574dd170: [d2                   ] ?? D2h
  0x574dd171: [90                   ] ?? 90h
  0x574dd172: [00                   ] ?? 00h
  0x574dd173: [4b                   ] ?? 4Bh    K
  0x574dd174: [0c                   ] ?? 0Ch
  0x574dd175: [80                   ] ?? 80h
  0x574dd176: [04                   ] ?? 04h
  0x574dd177: [85                   ] ?? 85h
  0x574dd178: [08                   ] ?? 08h
  0x574dd179: [dd                   ] ?? DDh
  0x574dd17a: [e7                   ] ?? E7h
  0x574dd17b: [5c                   ] ?? 5Ch    \
  0x574dd17c: [fc                   ] ?? FCh
  0x574dd17d: [ff                   ] ?? FFh
  0x574dd17e: [00                   ] ?? 00h
  0x574dd17f: [00                   ] ?? 00h
  0x574dd180: [83                   ] ?? 83h
  0x574dd181: [af                   ] ?? AFh
  0x574dd182: [00                   ] ?? 00h
  0x574dd183: [c8                   ] ?? C8h
  0x574dd184: [1e                   ] ?? 1Eh
  0x574dd185: [fc                   ] ?? FCh
  0x574dd186: [cc                   ] ?? CCh
  0x574dd187: [0a                   ] ?? 0Ah
  0x574dd188: [e9                   ] ?? E9h
  0x574dd189: [a5                   ] ?? A5h
  0x574dd18a: [57                   ] ?? 57h    W
  0x574dd18b: [8d                   ] ?? 8Dh
  0x574dd18c: [dd                   ] ?? DDh
  0x574dd18d: [4e                   ] ?? 4Eh    N
  0x574dd18e: [2c                   ] ?? 2Ch    ,
  0x574dd18f: [01                   ] ?? 01h
  0x574dd190: [00                   ] ?? 00h
  0x574dd191: [00                   ] ?? 00h
  0x574dd192: [00                   ] ?? 00h
  0x574dd193: [fc                   ] ?? FCh
  0x574dd194: [cd                   ] ?? CDh
  0x574dd195: [fc                   ] ?? FCh
  0x574dd196: [e8                   ] ?? E8h
  0x574dd197: [a5                   ] ?? A5h
  0x574dd198: [57                   ] ?? 57h    W
  0x574dd199: [2c                   ] ?? 2Ch    ,
  0x574dd19a: [45                   ] ?? 45h    E
  0x574dd19b: [02                   ] ?? 02h
  0x574dd19c: [ca                   ] ?? CAh
  0x574dd19d: [53                   ] ?? 53h    S
  0x574dd19e: [0c                   ] ?? 0Ch
  0x574dd19f: [ca                   ] ?? CAh
  0x574dd1a0: [15                   ] ?? 15h
  0x574dd1a1: [8f                   ] ?? 8Fh
  0x574dd1a2: [01                   ] ?? 01h
  0x574dd1a3: [08                   ] ?? 08h
  0x574dd1a4: [5c                   ] ?? 5Ch    \
  0x574dd1a5: [7c                   ] ?? 7Ch    |
  0x574dd1a6: [f0                   ] ?? F0h
  0x574dd1a7: [44                   ] ?? 44h    D
  0x574dd1a8: [41                   ] ?? 41h    A
  0x574dd1a9: [f0                   ] ?? F0h
  0x574dd1aa: [42                   ] ?? 42h    B
  0x574dd1ab: [49                   ] ?? 49h    I
  0x574dd1ac: [a1                   ] ?? A1h
  0x574dd1ad: [c9                   ] ?? C9h
  0x574dd1ae: [f1                   ] ?? F1h
  0x574dd1af: [48                   ] ?? 48h    H
  0x574dd1b0: [aa                   ] ?? AAh
  0x574dd1b1: [07                   ] ?? 07h
  0x574dd1b2: [c7                   ] ?? C7h
  0x574dd1b3: [f4                   ] ?? F4h
  0x574dd1b4: [af                   ] ?? AFh
  0x574dd1b5: [00                   ] ?? 00h
  0x574dd1b6: [c9                   ] ?? C9h
  0x574dd1b7: [05                   ] ?? 05h
  0x574dd1b8: [cc                   ] ?? CCh
  0x574dd1b9: [86                   ] ?? 86h
  0x574dd1ba: [00                   ] ?? 00h
  0x574dd1bb: [f9                   ] ?? F9h
  0x574dd1bc: [e8                   ] ?? E8h
  0x574dd1bd: [00                   ] ?? 00h
  0x574dd1be: [fd                   ] ?? FDh
  0x574dd1bf: [78                   ] ?? 78h    x
  0x574dd1c0: [00                   ] ?? 00h
  0x574dd1c1: [88                   ] ?? 88h
  0x574dd1c2: [00                   ] ?? 00h
  0x574dd1c3: [00                   ] ?? 00h
  0x574dd1c4: [f5                   ] ?? F5h
  0x574dd1c5: [c0                   ] ?? C0h
  0x574dd1c6: [42                   ] ?? 42h    B
  0x574dd1c7: [0c                   ] ?? 0Ch
  0x574dd1c8: [80                   ] ?? 80h
  0x574dd1c9: [0d                   ] ?? 0Dh
  0x574dd1ca: [85                   ] ?? 85h
  0x574dd1cb: [06                   ] ?? 06h
  0x574dd1cc: [dd                   ] ?? DDh
  0x574dd1cd: [94                   ] ?? 94h
  0x574dd1ce: [5c                   ] ?? 5Ch    \
  0x574dd1cf: [fc                   ] ?? FCh
  0x574dd1d0: [ff                   ] ?? FFh
  0x574dd1d1: [00                   ] ?? 00h
  0x574dd1d2: [00                   ] ?? 00h
  0x574dd1d3: [83                   ] ?? 83h
  0x574dd1d4: [af                   ] ?? AFh
  0x574dd1d5: [00                   ] ?? 00h
  0x574dd1d6: [c8                   ] ?? C8h
  0x574dd1d7: [26                   ] ?? 26h    &
  0x574dd1d8: [fc                   ] ?? FCh
  0x574dd1d9: [cc                   ] ?? CCh
  0x574dd1da: [30                   ] ?? 30h    0
  0x574dd1db: [e9                   ] ?? E9h
  0x574dd1dc: [a5                   ] ?? A5h
  0x574dd1dd: [57                   ] ?? 57h    W
  0x574dd1de: [8d                   ] ?? 8Dh
  0x574dd1df: [dd                   ] ?? DDh
  0x574dd1e0: [fb                   ] ?? FBh
  0x574dd1e1: [2b                   ] ?? 2Bh    +
  0x574dd1e2: [01                   ] ?? 01h
  0x574dd1e3: [00                   ] ?? 00h
  0x574dd1e4: [00                   ] ?? 00h
  0x574dd1e5: [00                   ] ?? 00h
  0x574dd1e6: [fc                   ] ?? FCh
  0x574dd1e7: [cd                   ] ?? CDh
  0x574dd1e8: [fc                   ] ?? FCh
  0x574dd1e9: [e8                   ] ?? E8h
  0x574dd1ea: [a5                   ] ?? A5h
  0x574dd1eb: [57                   ] ?? 57h    W
  0x574dd1ec: [2c                   ] ?? 2Ch    ,
  0x574dd1ed: [77                   ] ?? 77h    w
  0x574dd1ee: [02                   ] ?? 02h
  0x574dd1ef: [42                   ] ?? 42h    B
  0x574dd1f0: [0c                   ] ?? 0Ch
  0x574dd1f1: [00                   ] ?? 00h
  0x574dd1f2: [dd                   ] ?? DDh
  0x574dd1f3: [85                   ] ?? 85h
  0x574dd1f4: [31                   ] ?? 31h    1
  0x574dd1f5: [bd                   ] ?? BDh
  0x574dd1f6: [ff                   ] ?? FFh
  0x574dd1f7: [00                   ] ?? 00h
  0x574dd1f8: [00                   ] ?? 00h
  0x574dd1f9: [8c                   ] ?? 8Ch
  0x574dd1fa: [ca                   ] ?? CAh
  0x574dd1fb: [77                   ] ?? 77h    w
  0x574dd1fc: [fd                   ] ?? FDh
  0x574dd1fd: [aa                   ] ?? AAh
  0x574dd1fe: [00                   ] ?? 00h
  0x574dd1ff: [8a                   ] ?? 8Ah
  0x574dd200: [00                   ] ?? 00h
  0x574dd201: [00                   ] ?? 00h
  0x574dd202: [fb                   ] ?? FBh
  0x574dd203: [e9                   ] ?? E9h
  0x574dd204: [00                   ] ?? 00h
  0x574dd205: [80                   ] ?? 80h
  0x574dd206: [c8                   ] ?? C8h
  0x574dd207: [1d                   ] ?? 1Dh
  0x574dd208: [fc                   ] ?? FCh
  0x574dd209: [cc                   ] ?? CCh
  0x574dd20a: [74                   ] ?? 74h    t
  0x574dd20b: [3b                   ] ?? 3Bh    ;
  0x574dd20c: [a4                   ] ?? A4h
  0x574dd20d: [0b                   ] ?? 0Bh
  0x574dd20e: [fc                   ] ?? FCh
  0x574dd20f: [81                   ] ?? 81h
  0x574dd210: [ac                   ] ?? ACh
  0x574dd211: [b9                   ] ?? B9h
  0x574dd212: [11                   ] ?? 11h
  0x574dd213: [53                   ] ?? 53h    S
  0x574dd214: [fc                   ] ?? FCh
  0x574dd215: [cc                   ] ?? CCh
  0x574dd216: [57                   ] ?? 57h    W
  0x574dd217: [e9                   ] ?? E9h
  0x574dd218: [a5                   ] ?? A5h
  0x574dd219: [57                   ] ?? 57h    W
  0x574dd21a: [dd                   ] ?? DDh
  0x574dd21b: [c0                   ] ?? C0h
  0x574dd21c: [2b                   ] ?? 2Bh    +
  0x574dd21d: [01                   ] ?? 01h
  0x574dd21e: [00                   ] ?? 00h
  0x574dd21f: [00                   ] ?? 00h
  0x574dd220: [00                   ] ?? 00h
  0x574dd221: [ca                   ] ?? CAh
  0x574dd222: [29                   ] ?? 29h    )
  0x574dd223: [fc                   ] ?? FCh
  0x574dd224: [cc                   ] ?? CCh
  0x574dd225: [ba                   ] ?? BAh
  0x574dd226: [1d                   ] ?? 1Dh
  0x574dd227: [d2                   ] ?? D2h
  0x574dd228: [05                   ] ?? 05h
  0x574dd229: [fc                   ] ?? FCh
  0x574dd22a: [81                   ] ?? 81h
  0x574dd22b: [ac                   ] ?? ACh
  0x574dd22c: [b9                   ] ?? B9h
  0x574dd22d: [11                   ] ?? 11h
  0x574dd22e: [53                   ] ?? 53h    S
  0x574dd22f: [fc                   ] ?? FCh
  0x574dd230: [cc                   ] ?? CCh
  0x574dd231: [71                   ] ?? 71h    q
  0x574dd232: [e9                   ] ?? E9h
  0x574dd233: [a5                   ] ?? A5h
  0x574dd234: [57                   ] ?? 57h    W
  0x574dd235: [dd                   ] ?? DDh
  0x574dd236: [a5                   ] ?? A5h
  0x574dd237: [2b                   ] ?? 2Bh    +
  0x574dd238: [01                   ] ?? 01h
  0x574dd239: [00                   ] ?? 00h
  0x574dd23a: [00                   ] ?? 00h
  0x574dd23b: [00                   ] ?? 00h
  0x574dd23c: [ca                   ] ?? CAh
  0x574dd23d: [0e                   ] ?? 0Eh
  0x574dd23e: [fc                   ] ?? FCh
  0x574dd23f: [cc                   ] ?? CCh
  0x574dd240: [c8                   ] ?? C8h
```

---

## Parte IV — `/DRV_decomp.bin` & `/EH55_DRV_S211.bin` (Drive Front-End S211)

Firmware do processador Front-End (FE) do drive óptico Matshita. Esta parte reúne a tabela completa da CLI serial, os handlers `PASS` (`0x4D415453`/`0x48495441`), o comparativo cirúrgico entre `/DRV_decomp.bin` e `/EH55_DRV_S211.bin`, o despachante ATAPI (`0x3c` Write Buffer / Download Microcode), a tabela de sense codes em `0x400089dc` e as strings Shift-JIS.

### 4.1 Serial CLI (`0x400DF55C..0x400DF940`) & Comandos (`PASS`, `MMODE`, `SMODE`, `RB`, `WB`, `EEP`)

#### `FUN_400277de` (`drv_serial_rx_277de`) — `0x400277de` (197 B)

Receptor serial de linha da CLI do drive (`drv_serial_rx_277de`).

<!-- source: doc_drv_and_atapi.txt | FUNC FUN_400277de (drv_serial_rx_277de) @ 0x400277de (size=197 B, req=0x400277de) -->
```c
// Callers (21): 0x400274f1, 0x400274f8, 0x40027546, 0x40027571, 0x40027675, 0x400276af, 0x400276d3, 0x400277b7, 0x400277d1, 0x40027ad0, 0x40027ae4, 0x40027b0d, 0x40027b16, 0x40027c03, 0x40027c18, 0x40027f4a
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

int FUN_400277de(int param_1)

{
  undefined1 uVar1;
  undefined *puVar2;
  short sVar3;
  ushort uVar4;
  undefined1 uVar5;
  
  if (0x27ff < _DAT_00000ce2) {
    _DAT_00000ce2 = 0;
  }
  (&UNK_889f1fe0)[_DAT_00000ce2] = (char)param_1;
  _DAT_00000ce2 = _DAT_00000ce2 + 1;
  if ((DAT_00000cd8 & 1) != 0) {
    if (((DAT_0000b003 & 0x80) != 0) || (uVar1 = (char)param_1, _DAT_00000cdc != _DAT_00000cda)) {
      sVar3 = FUN_40027b8e(1);
      uVar5 = (undefined1)param_1;
      if (sVar3 == 0) {
        param_1 = 0;
        uVar1 = DAT_0000b004;
        if ((DAT_00000cd8 & 2) != 0) {
          _DAT_00000cdc = 0;
          _DAT_00000cda = 0;
          puVar2 = &UNK_889f47e0;
          FUN_40010950(0xf);
          uVar4 = _DAT_00000cdc + 0xf;
          param_1 = uVar4 + 1;
          _DAT_00000cdc = (ushort)param_1;
          puVar2[uVar4] = uVar5;
          uVar1 = DAT_0000b004;
        }
      }
      else {
        (&UNK_889f47e0)[_DAT_00000cdc] = uVar5;
        uVar1 = DAT_0000b004;
        if (_DAT_00000cdc == 0x7ff) {
          param_1 = 0;
          _DAT_00000cdc = 0;
        }
        else {
          param_1 = _DAT_00000cdc + 1;
          _DAT_00000cdc = (ushort)param_1;
        }
      }
    }
    DAT_0000b004 = uVar1;
    return param_1;
  }
  return param_1;
}
```

#### `FUN_4002803a` (`drv_serial_cli_dispatch_2803a`) — `0x4002803a` (144 B)

Despachante da tabela de comandos da CLI serial (`drv_serial_cli_dispatch_2803a`).

<!-- source: doc_drv_and_atapi.txt | FUNC FUN_4002803a (drv_serial_cli_dispatch_2803a) @ 0x4002803a (size=144 B, req=0x4002803a) -->
```c
// Callers (1): 0x40027ff1
undefined4 FUN_4002803a(void)

{
  char *in_A0;
  char *pcVar1;
  undefined *puVar2;
  byte bVar3;
  byte bVar4;
  
  puVar2 = &DAT_889f501c;
  FUN_40010992(0,0x8f);
  bVar3 = 0;
  bVar4 = 0;
  pcVar1 = in_A0;
  do {
    if ((((*pcVar1 == ',') || (*pcVar1 == ' ')) || (*pcVar1 == ':')) || (*pcVar1 == '\r')) {
      puVar2[(uint)bVar4 + (uint)bVar3 * 0xb] = 0;
      if (bVar4 != 0) {
        bVar3 = bVar3 + 1;
        if (0xd < bVar3) {
          return 2;
        }
        bVar4 = 0;
      }
      if (*pcVar1 == '\r') {
        DAT_889f5523 = bVar3 - 1;
        return 0;
      }
    }
    else {
      puVar2[(uint)bVar4 + (uint)bVar3 * 0xb] = *pcVar1;
      bVar4 = bVar4 + 1;
      if (10 < bVar4) {
        return 7;
      }
    }
    pcVar1 = pcVar1 + 1;
    if (0xa0 < (int)pcVar1 - (int)in_A0) {
      return 7;
    }
  } while( true );
}
```

#### `FUN_40029266` (`drv_unlock_helper_29266`) — `0x40029266` (220 B)

Helper de validação/desbloqueio na CLI do drive (`drv_unlock_helper_29266`).

<!-- source: doc_drv_and_atapi.txt | FUNC FUN_40029266 (drv_unlock_helper_29266) @ 0x40029266 (size=220 B, req=0x40029266) -->
```c
// Callers (0): 
undefined4 FUN_40029266(void)

{
  byte bVar1;
  undefined1 uVar2;
  int iVar3;
  uint uVar4;
  uint uVar5;
  uint uVar6;
  int iStack_24;
  
  uVar4 = FUN_40027cd8();
  uVar4 = uVar4 & 0xf5ff;
  uVar5 = FUN_40027cd8();
  uVar6 = uVar4 & 0xffff;
  if (0x5d < uVar6) {
    return 4;
  }
  bVar1 = DAT_889f5523;
  if (bVar1 < 2) {
    if (bVar1 == 1) {
      uVar5 = 0x40;
    }
    else if (bVar1 == 0) {
      uVar4 = 0;
      uVar5 = 0x5e;
    }
  }
  else {
    FUN_40027cd8();
    uVar2 = DAT_889f5523;
    func_0x40029482(1,uVar6 & 0xffff | 0xa00);
    uVar5 = 1;
  }
  uVar6 = uVar4 & 0xffff;
  if (0x5e < (uVar5 & 0xffff) + uVar6) {
    uVar5 = 0x5e - uVar6;
  }
  iVar3 = uVar6 + (uVar5 & 0xffff);
  while ((int)(uVar4 & 0xffff) < iVar3) {
    func_0x40029482(0,uVar4 & 0xffff | 0xa00);
    uVar2 = DAT_889fc9e0;
    (&UNK_889fca3e)[iStack_24] = uVar2;
    uVar4 = iStack_24 + 1;
  }
  FUN_4002829c(8,&UNK_889fca3e + uVar6);
  return 0;
}
```

#### `FUN_40133ed0` (`drv_cli_slogon_133ed0`) — `0x40133ed0` (14 B)

Comando CLI `SLOGON` (`drv_cli_slogon_133ed0`).

<!-- source: doc_drv_and_atapi.txt | FUNC FUN_40133ed0 (drv_cli_slogon_133ed0) @ 0x40133ed0 (size=14 B, req=0x40133ed0) -->
```c
// Callers (1): 0x40010153
undefined2 FUN_40133ed0(void)

{
  undefined2 uVar1;
  
  uVar1 = FUN_40133eeb(1);
  return uVar1;
}
```

#### `FUN_40133ede` (`drv_cli_slogoff_133ede`) — `0x40133ede` (13 B)

Comando CLI `SLOGOFF` (`drv_cli_slogoff_133ede`).

<!-- source: doc_drv_and_atapi.txt | FUNC FUN_40133ede (drv_cli_slogoff_133ede) @ 0x40133ede (size=13 B, req=0x40133ede) -->
```c
// Callers (0): 
// Referenced strings/literals:
//   0x40133ede: 'øþü'
undefined2 FUN_40133ede(void)

{
  undefined2 uVar1;
  
  uVar1 = FUN_40133eeb(0);
  return uVar1;
}
```

#### `FUN_4013420a` (`drv_cli_pass_13420a`) — `0x4013420a` (57 B)

Comando CLI `PASS` em `/DRV_decomp.bin`: exige `0x4d415453` (`"MATS"`) / `0x48495441` (`"HITA"`) e seta `DAT_00000cd8 |= 1` (`drv_cli_pass_13420a`).

<!-- source: doc_drv_and_atapi.txt | FUNC FUN_4013420a (drv_cli_pass_13420a) @ 0x4013420a (size=57 B, req=0x4013420a) -->
```c
// Callers (0): 
// Referenced strings/literals:
//   0x4013420a: "Ï\x80øþüüÜ'P\x9f\x88ÝÃ:ïÿ"
undefined4 FUN_4013420a(void)

{
  int iVar1;
  int iVar2;
  
  iVar1 = FUN_40027cd8();
  iVar2 = FUN_40027cd8();
  if ((iVar1 == 0x4d415453) && (iVar2 == 0x48495441)) {
    DAT_00000cd8 = DAT_00000cd8 | 1;
  }
  return 0;
}
```

#### `FUN_40134243` (`drv_cli_ver_134243`) — `0x40134243` (3 B)

Comando CLI `VER` (`drv_cli_ver_134243`).

<!-- source: doc_drv_and_atapi.txt | FUNC FUN_40134243 (drv_cli_ver_134243) @ 0x40134243 (size=3 B, req=0x40134243) -->
```c
// Callers (0): 
undefined4 FUN_40134243(undefined4 param_1,undefined4 param_2)

{
  undefined4 in_D2;
  undefined4 uStackY_3c;
  undefined4 uStackY_38;
  
  drv_cli_io_helper(0);
  drv_cli_io_helper(0,param_2,uStackY_38,in_D2);
  drv_cli_io_helper(0,param_2,uStackY_38,uStackY_3c);
  drv_cli_io_helper(0,param_2,uStackY_38,uStackY_3c);
  drv_cli_io_helper(0,param_2,uStackY_38,uStackY_3c);
  drv_cli_io_helper(0,param_2,uStackY_38,uStackY_3c);
  drv_cli_io_helper(0,param_2,uStackY_38,uStackY_3c);
  drv_cli_io_helper(0,param_2,uStackY_38,uStackY_3c);
  drv_cli_io_helper(0,param_2,uStackY_38,uStackY_3c);
  return 0;
}
```

#### `FUN_40134382` (`drv_cli_mmode_134382`) — `0x40134382` (64 B)

Comando CLI `MMODE` (`drv_cli_mmode_134382`).

<!-- source: doc_drv_and_atapi.txt | FUNC FUN_40134382 (drv_cli_mmode_134382) @ 0x40134382 (size=64 B, req=0x40134382) -->
```c
// Callers (0): 
undefined2 FUN_40134382(void)

{
  char cVar1;
  undefined2 uVar2;
  
  DAT_00000e41 = DAT_00000e41 | 0x80;
  drv_cli_io_helper(0);
  cVar1 = DAT_889f5523;
  if ((cVar1 == '\x01') && (cVar1 = FUN_40027cd8(), cVar1 != '\0')) {
    uVar2 = func_0x40134402();
    return uVar2;
  }
  return 0;
}
```

#### `FUN_401343c2` (`drv_cli_smode_1343c2`) — `0x401343c2` (64 B)

Comando CLI `SMODE` (`drv_cli_smode_1343c2`).

<!-- source: doc_drv_and_atapi.txt | FUNC FUN_401343c2 (drv_cli_smode_1343c2) @ 0x401343c2 (size=64 B, req=0x401343c2) -->
```c
// Callers (0): 
undefined2 FUN_401343c2(void)

{
  char cVar1;
  undefined2 uVar2;
  
  DAT_00000e41 = DAT_00000e41 | 0x7f;
  drv_cli_io_helper(0);
  cVar1 = DAT_889f5523;
  if ((cVar1 == '\x01') && (cVar1 = FUN_40027cd8(), cVar1 != '\0')) {
    uVar2 = func_0x40134402();
    return uVar2;
  }
  return 0;
}
```

#### `FUN_40134483` (`drv_cli_rb_134483`) — `0x40134483` (44 B)

Comando CLI `RB` — Read Byte (`drv_cli_rb_134483`).

<!-- source: doc_drv_and_atapi.txt | FUNC FUN_40134483 (drv_cli_rb_134483) @ 0x40134483 (size=44 B, req=0x40134483) -->
```c
// Callers (0): 
// Referenced strings/literals:
//   0x40134483: "øþðüÜ'P\x9f\x88ÝL8ïÿ"
//   0x400e9dfe: '  %lh: %bh\n'
undefined4 FUN_40134483(void)

{
  undefined4 in_A2;
  undefined4 in_A3;
  
  FUN_40027cd8();
  drv_cli_io_helper(0,0x400e9dfe,in_A2,in_A3);
  return 0;
}
```

#### `FUN_401344e0` (`drv_cli_wb_1344e0`) — `0x401344e0` (85 B)

Comando CLI `WB` — Write Byte (`drv_cli_wb_1344e0`).

<!-- source: doc_drv_and_atapi.txt | FUNC FUN_401344e0 (drv_cli_wb_1344e0) @ 0x401344e0 (size=85 B, req=0x401344e0) -->
```c
// Callers (0): 
// Referenced strings/literals:
//   0x401344e0: "ÏàøþðüÜ'P\x9f\x88Ýí7ïÿ"
undefined4 FUN_401344e0(void)

{
  undefined1 uVar1;
  undefined1 *puVar2;
  uint uStackY_30;
  byte *pbStackY_2c;
  
  puVar2 = (undefined1 *)FUN_40027cd8();
  uVar1 = FUN_40027cd8();
  *puVar2 = uVar1;
  drv_cli_io_helper(0);
  if (*pbStackY_2c != uStackY_30) {
    drv_cli_io_helper(0,(uint)*pbStackY_2c,pbStackY_2c,uVar1);
  }
  return 0;
}
```

#### `FUN_401345de` (`drv_cli_eep_1345de`) — `0x401345de` (626 B)

Comando CLI `EEP` — leitura/gravação EEPROM do drive (`drv_cli_eep_1345de`).

<!-- source: doc_drv_and_atapi.txt | FUNC FUN_401345de (drv_cli_eep_1345de) @ 0x401345de (size=626 B, req=0x401345de) -->
```c
// Callers (0): 
// Referenced strings/literals:
//   0x401345de: 'Ïðøþðü¨#U\x9f\x88\x9a'
undefined4 FUN_401345de(void)

{
  char cVar1;
  uint uVar2;
  uint uVar3;
  uint uVar4;
  uint uVar5;
  uint uStack_40;
  char cStack_3c;
  uint uStack_38;
  uint uStack_34;
  uint uStack_30;
  undefined4 uStack_18;
  
  cVar1 = DAT_889f5523;
  uVar2 = 0;
  uVar5 = 0;
  if (cVar1 != '\0') {
    uVar2 = FUN_40027cd8();
    FUN_40027cd8();
  }
  cVar1 = DAT_889f5523;
  if (cVar1 == '\x02') {
    uVar2 = FUN_400d71e3(uVar2 & 0xffff,1);
    if ((uVar2 & 0xffff) != uVar5) {
      return 4;
    }
    uVar2 = FUN_400d7378();
    uVar4 = 1;
    if ((uVar2 & 0xffff) != uStack_34) {
      return 4;
    }
  }
  else {
    if (cVar1 != '\x01') {
      drv_cli_io_helper(8);
      drv_cli_io_helper(8);
      drv_cli_io_helper(8);
      drv_cli_io_helper(8);
      drv_cli_io_helper(8);
      drv_cli_io_helper(8);
      drv_cli_io_helper(8);
      drv_cli_io_helper(8);
      return 0;
    }
    uVar3 = FUN_40010974(3);
    if (((int)uVar3 >> 0x1f & uVar3 & (int)uVar3 >> 0x1f & 0xffff0000U) == uVar5) {
      uVar4 = 0x100;
      uStack_30 = 0;
    }
    else {
      uVar3 = FUN_40010974(4);
      if (((int)uVar3 >> 0x1f & uVar3 & (int)uVar3 >> 0x1f & 0xffff0000U) == uVar5) {
        uStack_30 = 0x100;
        uVar4 = 0x400;
      }
      else {
        uVar3 = FUN_40010974(4);
        if (((int)uVar3 >> 0x1f & uVar3 & (int)uVar3 >> 0x1f & 0xffff0000U) == uVar5) {
          uStack_30 = 0x500;
          uVar4 = 0x200;
        }
        else {
          uVar3 = FUN_40010974(3);
          if (((int)uVar3 >> 0x1f & uVar3 & (int)uVar3 >> 0x1f & 0xffff0000U) == uVar5) {
            uVar4 = 0x200;
            uStack_30 = 0x700;
          }
          else {
            uVar3 = FUN_40010974(4);
            uVar4 = 0x80;
            uStack_30 = uVar2;
            if (((int)uVar3 >> 0x1f & uVar3 & (int)uVar3 >> 0x1f & 0xffff0000U) == uVar5) {
              uStack_30 = 0x900;
              uVar4 = 0x300;
            }
          }
        }
      }
    }
  }
  uVar2 = uStack_30 & 0xffff;
  if (0x1000 < uVar4 + uVar2) {
    uVar4 = 0x1000 - uVar2;
  }
  FUN_400d70c0(uVar2,uVar4 & 0xffff);
  FUN_4002829c(8,&DAT_889fc9e0);
  if (cStack_3c != '\x01') {
    return 0;
  }
  uVar2 = FUN_40010974(4);
  if (((int)uVar2 >> 0x1f & uVar2 & (int)uVar2 >> 0x1f & 0xffff0000U) == 0) {
    uStack_38 = 0xc00;
  }
  else {
    uVar2 = FUN_40010974(4);
    if (((int)uVar2 >> 0x1f & uVar2 & (int)uVar2 >> 0x1f & 0xffff0000U) == 0) {
      uStack_38 = 0xd00;
    }
    else {
      uVar2 = FUN_40010974(4);
      if (((int)uVar2 >> 0x1f & uVar2 & (int)uVar2 >> 0x1f & 0xffff0000U) != 0) goto LAB_40134807;
      uStack_38 = 0xe00;
    }
  }
  uStack_40 = 0x100;
LAB_40134807:
  uStack_38 = uStack_38 & 0xffff;
  if (0x1000 < (uStack_40 & 0xffff) + uStack_38) {
    uStack_40 = 0x1000 - uStack_38;
  }
  FUN_400d70c0(uStack_38,uStack_40 & 0xffff);
  drv_cli_io_helper(8);
  FUN_4002829c(8,uStack_18);
  return 0;
}
```

#### Tabela Completa da CLI Serial (`/DRV_decomp.bin` @ `0x400DF55C..0x400DF940`, entradas de 20 B)

Mapeamento completo nome ASCII -> ponteiro de função -> flags/permissões.

<!-- source: doc_drv_and_atapi.txt | DRV_decomp.bin CLI TABLE @ 0x400DF55C..0x400DF940 (20B entries) -->
```text
  0x400df55c: cmd='\x0c'     flags=0x03000000 attr=0x00400300 fn=0x00300000
  0x400df570: cmd='\x10'     flags=0x00000040 attr=0x00100030 fn=0x01f00000
  0x400df584: cmd='P\x02\x0c' flags=0x00000000 attr=0x02c002c0 fn=0x00000040
  0x400df598: cmd='0'        flags=0x00030001 attr=0x000f0007 fn=0x003f001f
  0x400df5ac: cmd='\x7f'     flags=0x0fff07ff attr=0x3fff1fff fn=0xffff7fff
  0x400df5c0: cmd='SLOGON'   flags=0x0c000000 attr=0x00000000 fn=0x40133ed0
  0x400df5d4: cmd='SLOGOFF'  flags=0x0c000000 attr=0x00000000 fn=0x40133ede
  0x400df5e8: cmd='PASS'     flags=0x22000000 attr=0x00000080 fn=0x4013420a
  0x400df5fc: cmd='VER'      flags=0x00000000 attr=0x00000008 fn=0x40134243
  0x400df610: cmd='RESET'    flags=0x00000000 attr=0x00000080 fn=0x40134335
  0x400df624: cmd='LOGCUT'   flags=0x11000000 attr=0x00000000 fn=0x40134343
  0x400df638: cmd='MMODE'    flags=0x01000000 attr=0x00000000 fn=0x40134382
  0x400df64c: cmd='SMODE'    flags=0x01000000 attr=0x00000000 fn=0x401343c2
  0x400df660: cmd='PRMPT'    flags=0x11000000 attr=0x00000008 fn=0x4013445d
  0x400df674: cmd='RB'       flags=0x11000000 attr=0x00000008 fn=0x40134483
  0x400df688: cmd='RW'       flags=0x11000000 attr=0x00000008 fn=0x401344af
  0x400df69c: cmd='WB'       flags=0x22000000 attr=0x00000008 fn=0x401344e0
  0x400df6b0: cmd='WW'       flags=0x22000000 attr=0x00000008 fn=0x40134535
  0x400df6c4: cmd='EEP'      flags=0x02000000 attr=0x00000008 fn=0x401345de
  0x400df6d8: cmd='D'        flags=0x13000000 attr=0x00000008 fn=0x40134850
  0x400df6ec: cmd='MLOG'     flags=0x01000000 attr=0x00000008 fn=0x401348d7
  0x400df700: cmd='DSCLOG'   flags=0x01000000 attr=0x00000008 fn=0x40134902
  0x400df714: cmd='CLOG'     flags=0x01000000 attr=0x00000008 fn=0x4013492e
  0x400df728: cmd='ACSLOG'   flags=0x01000000 attr=0x00000008 fn=0x4013495a
  0x400df73c: cmd='TMPLOG'   flags=0x01000000 attr=0x00000008 fn=0x40134986
  0x400df750: cmd='LFMLOG'   flags=0x01000000 attr=0x00000008 fn=0x401349b2
  0x400df764: cmd='LINFO'    flags=0x01000000 attr=0x00000008 fn=0x40134cf9
  0x400df778: cmd='RMD'      flags=0x12000000 attr=0x00000008 fn=0x40134f36
  0x400df78c: cmd='RZONE'    flags=0x01000000 attr=0x00000008 fn=0x4013583a
  0x400df7a0: cmd='LPP'      flags=0x00000000 attr=0x00000008 fn=0x40135020
  0x400df7b4: cmd='ADIP'     flags=0x00000000 attr=0x00000008 fn=0x401353d1
  0x400df7c8: cmd='TOC'      flags=0x01000000 attr=0x00000008 fn=0x40136014
  0x400df7dc: cmd='TRACK'    flags=0x01000000 attr=0x00000008 fn=0x4013607c
  0x400df7f0: cmd='PON'      flags=0x00000000 attr=0x00000008 fn=0x40136137
  0x400df804: cmd='HON'      flags=0x00000000 attr=0x00000008 fn=0x40136137
  0x400df818: cmd='POFF'     flags=0x00000000 attr=0x00000008 fn=0x40136156
  0x400df82c: cmd='HOFF'     flags=0x00000000 attr=0x00000008 fn=0x40136156
  0x400df840: cmd='PCONT'    flags=0x00000000 attr=0x00000008 fn=0x40136175
  0x400df854: cmd='PDOWN'    flags=0x00000000 attr=0x00000008 fn=0x4013619f
  0x400df868: cmd='PTIME'    flags=0x02000000 attr=0x00000008 fn=0x401361c8
  0x400df87c: cmd='ZTBL'     flags=0x05000000 attr=0x00000008 fn=0x4013627b
  0x400df890: cmd='TSTWT'    flags=0x11000000 attr=0x00000008 fn=0x40136405
  0x400df8a4: cmd='MSF2PBA'  flags=0x13000000 attr=0x00000008 fn=0x4013649b
  0x400df8b8: cmd='PBA2MSF'  flags=0x11000000 attr=0x00000008 fn=0x401364bd
  0x400df8cc: cmd='REVADR'   flags=0x11000000 attr=0x00000008 fn=0x400298e2
  0x400df8e0: cmd='CONT'     flags=0x00000000 attr=0x00000008 fn=0x401364fd
  0x400df8f4: cmd='BREAK'    flags=0x00000000 attr=0x00000008 fn=0x4013652f
  0x400df908: cmd='DSCC'     flags=0x13000000 attr=0x00000008 fn=0x400290c4
  0x400df91c: cmd='DSC'      flags=0x02000000 attr=0x00000008 fn=0x4002912d
  0x400df930: cmd='FEP'      flags=0x03000000 attr=0x00000008 fn=0x40029266
```

#### Constantes e Offsets Verificados (`PASS` / `MATS` / `HITA` / `SenseTable`)

Localizações exatas em `/DRV_decomp.bin`.

<!-- source: doc_drv_and_atapi.txt | DRV_decomp.bin CONSTANTS FOR PASS / MATS / HITA / SENSE -->
```text
  Checking callers of 0x4013420a and constants near PASS handler:
```

### 4.2 Comparativo `/DRV_decomp.bin` vs `/EH55_DRV_S211.bin` (`FUN_40130efa` vs `FUN_4013420a`)

Demonstra com 100% de precisão o deslocamento de endereços entre revisões de firmware do drive FE: em `/EH55_DRV_S211.bin`, `0x40130efa` (57 B) é o handler real do comando `PASS` (verificando `0x4d415453` e `0x48495441` e executando `0x40130f2a: BSET 0x1,(0xd78)`). Em `/DRV_decomp.bin`, o endereço `0x40130efa` cai dentro de strings format (`"a  = 0x%lh\n"`), enquanto o handler `PASS` correspondente está deslocado para `0x4013420a` (`DAT_00000cd8 |= 1`).

#### Nota de Mapeamento `NO_FUNC drv_alt_130efa @ 0x40130efa` (`doc_drv_and_atapi.txt`)

Confirma que em `/DRV_decomp.bin` não existe função em `0x40130efa` (o handler está em `/EH55_DRV_S211.bin`).

<!-- source: doc_drv_and_atapi.txt | NO_FUNC drv_alt_130efa @ 0x40130efa -->
*(Sem corpo na exportação original; referência cruzada detalhada na subseção correspondente.)*

#### Todas as 15 Funções e Handler `PASS` de `/EH55_DRV_S211.bin` (`verified_extra_facts.txt` §3)

```text
  S211 FUNC thunk_FUN_4000112e @ 0x40001000 size=3 B
  S211 FUNC FUN_4000112e @ 0x4000112e size=647 B
  S211 FUNC FUN_40030080 @ 0x40030080 size=28 B
  S211 FUNC FUN_40130bc0 @ 0x40130bc0 size=14 B
  S211 FUNC FUN_40130bce @ 0x40130bce size=13 B
  S211 FUNC FUN_40130efa @ 0x40130efa size=57 B
  S211 FUNC FUN_40130f33 @ 0x40130f33 size=242 B
  S211 FUNC FUN_40131025 @ 0x40131025 size=14 B
  S211 FUNC FUN_4013114d @ 0x4013114d size=38 B
  S211 FUNC FUN_40131173 @ 0x40131173 size=44 B
  S211 FUNC FUN_4013119f @ 0x4013119f size=49 B
  S211 FUNC FUN_401311d0 @ 0x401311d0 size=85 B
  S211 FUNC FUN_40131225 @ 0x40131225 size=90 B
  S211 FUNC FUN_4013127f @ 0x4013127f size=626 B
  S211 FUNC FUN_401314f1 @ 0x401314f1 size=135 B

-- S211 DECOMP FUN_40130efa @ 0x40130efa --
undefined4 FUN_40130efa(void)

{
  byte bVar1;
  int iVar2;
  int iVar3;
  
  iVar2 = func_0x40027e14();
  iVar3 = func_0x40027e14();
  if ((iVar2 == 0x4d415453) && (iVar3 == 0x48495441)) {
    bVar1 = UNK_00000d78;
    UNK_00000d78 = bVar1 | 1;
  }
  return 0;
}
-- S211 DISASM 0x40130efa..0x40130f32 --
  0x40130efa: [cf 80                ] MOVM [D2],(SP)
  0x40130efc: [f8 fe fc             ] ADD -0x4,SP
  0x40130eff: [fc dc 27 4c 9f 88    ] MOV 0x889f4c27,A0
  0x40130f05: [dd 0f 6f ef ff 00 04 ] CALL 0x40027e14,[],0x4
  0x40130f0c: [82                   ] MOV D0,D2
  0x40130f0d: [fc dc 32 4c 9f 88    ] MOV 0x889f4c32,A0
  0x40130f13: [dd 01 6f ef ff 00 04 ] CALL 0x40027e14,[],0x4
  0x40130f1a: [fc ca 53 54 41 4d    ] CMP 0x4d415453,D2
  0x40130f20: [c9 0f                ] BNE 0x40130f2f
  0x40130f22: [fc c8 41 54 49 48    ] CMP 0x48495441,D0
  0x40130f28: [c9 07                ] BNE 0x40130f2f
  0x40130f2a: [fe 80 78 0d 01       ] BSET 0x1,(0xd78)
  0x40130f2f: [00                   ] CLR D0
  0x40130f30: [df 80 08             ] RET [D2],0x8

-- S211 DECOMP FUN_40130bc0 @ 0x40130bc0 --
undefined2 FUN_40130bc0(void)

{
  undefined2 uVar1;
  
  uVar1 = func_0x40130bdb(1);
  return uVar1;
}
-- S211 DISASM 0x40130bc0..0x40130bcd --
  0x40130bc0: [f8 fe fc             ] ADD -0x4,SP
  0x40130bc3: [80 01                ] MOV 0x1,D0
  0x40130bc5: [cd 16 00 f0 48       ] CALL 0x40130bdb,[D2,D3,A2,A3],0x48
  0x40130bca: [1c                   ] EXTHU D0
  0x40130bcb: [df 00 04             ] RET [],0x4

-- S211 DECOMP FUN_40030080 @ 0x40030080 --
undefined4 FUN_40030080(undefined4 param_1)

{
  undefined2 uVar1;
  int in_D2;
  int in_D3;
  
  Ram0000f81e = param_1;
  if (in_D2 + in_D3 == 0) {
    uVar1 = 0x30b;
  }
  else {
    uVar1 = 0x30c;
  }
  Ram00000f5c = uVar1;
  return 0;
}
-- S211 DISASM 0x40030080..0x400301d0 --
  0x40030080: [01 1e f8             ] MOV D0,(0xf81e)
  0x40030083: [ee                   ] ADD D3,D2
  0x40030084: [10                   ] EXTB D0
  0x40030085: [c8 08                ] BEQ 0x4003008d
  0x40030087: [2c 0c 03             ] MOV 0x30c,D0
  0x4003008a: [cc 2b 01             ] JMP 0x400301b5
  0x4003008d: [2c 0b 03             ] MOV 0x30b,D0
  0x40030090: [cc 25 01             ] JMP 0x400301b5
  0x40030093: [80                   ] ?? 80h
  0x40030094: [01                   ] ?? 01h
  0x40030095: [cd                   ] ?? CDh
  0x40030096: [ab                   ] ?? ABh
  0x40030097: [11                   ] ?? 11h
  0x40030098: [00                   ] ?? 00h
  0x40030099: [00                   ] ?? 00h
  0x4003009a: [2c                   ] ?? 2Ch    ,
  0x4003009b: [23                   ] ?? 23h    #
  0x4003009c: [05                   ] ?? 05h
  0x4003009d: [cc                   ] ?? CCh
  0x4003009e: [18                   ] ?? 18h
  0x4003009f: [01                   ] ?? 01h
  0x400300a0: [2c                   ] ?? 2Ch    ,
  0x400300a1: [24                   ] ?? 24h    $
  0x400300a2: [05                   ] ?? 05h
  0x400300a3: [cc                   ] ?? CCh
  0x400300a4: [12                   ] ?? 12h
  0x400300a5: [01                   ] ?? 01h
  0x400300a6: [1e                   ] ?? 1Eh
  0x400300a7: [f8                   ] ?? F8h
  0x400300a8: [ee                   ] ?? EEh
  0x400300a9: [10                   ] ?? 10h
  0x400300aa: [c8                   ] ?? C8h
  0x400300ab: [38                   ] ?? 38h    8
  0x400300ac: [fe                   ] ?? FEh
  0x400300ad: [82                   ] ?? 82h
  0x400300ae: [4c                   ] ?? 4Ch    L
  0x400300af: [1b                   ] ?? 1Bh
  0x400300b0: [20                   ] ?? 20h     
  0x400300b1: [c9                   ] ?? C9h
  0x400300b2: [09                   ] ?? 09h
  0x400300b3: [fe                   ] ?? FEh
  0x400300b4: [82                   ] ?? 82h
  0x400300b5: [4c                   ] ?? 4Ch    L
  0x400300b6: [1b                   ] ?? 1Bh
  0x400300b7: [40                   ] ?? 40h    @
  0x400300b8: [c8                   ] ?? C8h
  0x400300b9: [24                   ] ?? 24h    $
  0x400300ba: [fe                   ] ?? FEh
  0x400300bb: [82                   ] ?? 82h
  0x400300bc: [04                   ] ?? 04h
  0x400300bd: [14                   ] ?? 14h
  0x400300be: [20                   ] ?? 20h     
  0x400300bf: [c9                   ] ?? C9h
  0x400300c0: [1d                   ] ?? 1Dh
  0x400300c1: [fc                   ] ?? FCh
  0x400300c2: [cc                   ] ?? CCh
  0x400300c3: [ac                   ] ?? ACh
  0x400300c4: [80                   ] ?? 80h
  0x400300c5: [0e                   ] ?? 0Eh
  0x400300c6: [40                   ] ?? 40h    @
  0x400300c7: [42                   ] ?? 42h    B
  0x400300c8: [04                   ] ?? 04h
  0x400300c9: [fc                   ] ?? FCh
  0x400300ca: [cc                   ] ?? CCh
  0x400300cb: [10                   ] ?? 10h
  0x400300cc: [80                   ] ?? 80h
  0x400300cd: [00                   ] ?? 00h
  0x400300ce: [00                   ] ?? 00h
  0x400300cf: [dd                   ] ?? DDh
  0x400300d0: [f5                   ] ?? F5h
  0x400300d1: [79                   ] ?? 79h    y
  0x400300d2: [ff                   ] ?? FFh
  0x400300d3: [ff                   ] ?? FFh
  0x400300d4: [b0                   ] ?? B0h
  0x400300d5: [14                   ] ?? 14h
  0x400300d6: [2c                   ] ?? 2Ch    ,
  0x400300d7: [0a                   ] ?? 0Ah
  0x400300d8: [03                   ] ?? 03h
  0x400300d9: [cc                   ] ?? CCh
  0x400300da: [dc                   ] ?? DCh
  0x400300db: [00                   ] ?? 00h
  0x400300dc: [2c                   ] ?? 2Ch    ,
  0x400300dd: [06                   ] ?? 06h
  0x400300de: [03                   ] ?? 03h
  0x400300df: [cc                   ] ?? CCh
  0x400300e0: [d6                   ] ?? D6h
  0x400300e1: [00                   ] ?? 00h
  0x400300e2: [2c                   ] ?? 2Ch    ,
  0x400300e3: [05                   ] ?? 05h
  0x400300e4: [03                   ] ?? 03h
  0x400300e5: [cc                   ] ?? CCh
  0x400300e6: [d0                   ] ?? D0h
  0x400300e7: [00                   ] ?? 00h
  0x400300e8: [2c                   ] ?? 2Ch    ,
  0x400300e9: [15                   ] ?? 15h
  0x400300ea: [04                   ] ?? 04h
  0x400300eb: [cc                   ] ?? CCh
  0x400300ec: [ca                   ] ?? CAh
  0x400300ed: [00                   ] ?? 00h
  0x400300ee: [1e                   ] ?? 1Eh
  0x400300ef: [f8                   ] ?? F8h
  0x400300f0: [ee                   ] ?? EEh
  0x400300f1: [10                   ] ?? 10h
  0x400300f2: [c8                   ] ?? C8h
  0x400300f3: [07                   ] ?? 07h
  0x400300f4: [2c                   ] ?? 2Ch    ,
  0x400300f5: [08                   ] ?? 08h
  0x400300f6: [03                   ] ?? 03h
  0x400300f7: [ca                   ] ?? CAh
  0x400300f8: [05                   ] ?? 05h
  0x400300f9: [2c                   ] ?? 2Ch    ,
  0x400300fa: [07                   ] ?? 07h
  0x400300fb: [03                   ] ?? 03h
  0x400300fc: [03                   ] ?? 03h
  0x400300fd: [5c                   ] ?? 5Ch    \
  0x400300fe: [0f                   ] ?? 0Fh
  0x400300ff: [fe                   ] ?? FEh
  0x40030100: [82                   ] ?? 82h
  0x40030101: [84                   ] ?? 84h
  0x40030102: [0f                   ] ?? 0Fh
  0x40030103: [03                   ] ?? 03h
  0x40030104: [c9                   ] ?? C9h
  0x40030105: [20                   ] ?? 20h     
  0x40030106: [cc                   ] ?? CCh
  0x40030107: [c7                   ] ?? C7h
  0x40030108: [00                   ] ?? 00h
  0x40030109: [1e                   ] ?? 1Eh
  0x4003010a: [f8                   ] ?? F8h
  0x4003010b: [ee                   ] ?? EEh
  0x4003010c: [10                   ] ?? 10h
  0x4003010d: [c8                   ] ?? C8h
  0x4003010e: [07                   ] ?? 07h
  0x4003010f: [2c                   ] ?? 2Ch    ,
  0x40030110: [0a                   ] ?? 0Ah
  0x40030111: [03                   ] ?? 03h
  0x40030112: [ca                   ] ?? CAh
  0x40030113: [05                   ] ?? 05h
  0x40030114: [2c                   ] ?? 2Ch    ,
  0x40030115: [09                   ] ?? 09h
  0x40030116: [03                   ] ?? 03h
  0x40030117: [03                   ] ?? 03h
  0x40030118: [5c                   ] ?? 5Ch    \
  0x40030119: [0f                   ] ?? 0Fh
  0x4003011a: [fe                   ] ?? FEh
  0x4003011b: [82                   ] ?? 82h
  0x4003011c: [84                   ] ?? 84h
  0x4003011d: [0f                   ] ?? 0Fh
  0x4003011e: [03                   ] ?? 03h
  0x4003011f: [c9                   ] ?? C9h
  0x40030120: [05                   ] ?? 05h
  0x40030121: [cc                   ] ?? CCh
  0x40030122: [ac                   ] ?? ACh
  0x40030123: [00                   ] ?? 00h
  0x40030124: [2c                   ] ?? 2Ch    ,
  0x40030125: [ff                   ] ?? FFh
  0x40030126: [00                   ] ?? 00h
  0x40030127: [dd                   ] ?? DDh
  0x40030128: [ee                   ] ?? EEh
  0x40030129: [c2                   ] ?? C2h
  0x4003012a: [07                   ] ?? 07h
  0x4003012b: [00                   ] ?? 00h
  0x4003012c: [f0                   ] ?? F0h
  0x4003012d: [38                   ] ?? 38h    8
  0x4003012e: [cc                   ] ?? CCh
  0x4003012f: [9f                   ] ?? 9Fh
  0x40030130: [00                   ] ?? 00h
  0x40030131: [2c                   ] ?? 2Ch    ,
  0x40030132: [f6                   ] ?? F6h
  0x40030133: [00                   ] ?? 00h
  0x40030134: [02                   ] ?? 02h
  0x40030135: [32                   ] ?? 32h    2
  0x40030136: [14                   ] ?? 14h
  0x40030137: [cc                   ] ?? CCh
  0x40030138: [92                   ] ?? 92h
  0x40030139: [00                   ] ?? 00h
  0x4003013a: [80                   ] ?? 80h
  0x4003013b: [02                   ] ?? 02h
  0x4003013c: [cd                   ] ?? CDh
  0x4003013d: [04                   ] ?? 04h
  0x4003013e: [11                   ] ?? 11h
  0x4003013f: [00                   ] ?? 00h
  0x40030140: [00                   ] ?? 00h
  0x40030141: [1e                   ] ?? 1Eh
  0x40030142: [f8                   ] ?? F8h
  0x40030143: [ee                   ] ?? EEh
  0x40030144: [10                   ] ?? 10h
  0x40030145: [c8                   ] ?? C8h
  0x40030146: [1a                   ] ?? 1Ah
  0x40030147: [fe                   ] ?? FEh
  0x40030148: [82                   ] ?? 82h
  0x40030149: [4c                   ] ?? 4Ch    L
  0x4003014a: [1b                   ] ?? 1Bh
  0x4003014b: [20                   ] ?? 20h     
  0x4003014c: [c9                   ] ?? C9h
  0x4003014d: [66                   ] ?? 66h    f
  0x4003014e: [fe                   ] ?? FEh
  0x4003014f: [82                   ] ?? 82h
  0x40030150: [4c                   ] ?? 4Ch    L
  0x40030151: [1b                   ] ?? 1Bh
  0x40030152: [40                   ] ?? 40h    @
  0x40030153: [c8                   ] ?? C8h
  0x40030154: [04                   ] ?? 04h
  0x40030155: [ca                   ] ?? CAh
  0x40030156: [5d                   ] ?? 5Dh    ]
  0x40030157: [2c                   ] ?? 2Ch    ,
  0x40030158: [f5                   ] ?? F5h
  0x40030159: [00                   ] ?? 00h
  0x4003015a: [02                   ] ?? 02h
  0x4003015b: [32                   ] ?? 32h    2
  0x4003015c: [14                   ] ?? 14h
  0x4003015d: [ca                   ] ?? CAh
  0x4003015e: [6c                   ] ?? 6Ch    l
  0x4003015f: [fe                   ] ?? FEh
  0x40030160: [82                   ] ?? 82h
  0x40030161: [4c                   ] ?? 4Ch    L
  0x40030162: [1b                   ] ?? 1Bh
  0x40030163: [02                   ] ?? 02h
  0x40030164: [c8                   ] ?? C8h
  0x40030165: [24                   ] ?? 24h    $
  0x40030166: [3e                   ] ?? 3Eh    >
  0x40030167: [22                   ] ?? 22h    "
  0x40030168: [18                   ] ?? 18h
  0x40030169: [00                   ] ?? 00h
  0x4003016a: [98                   ] ?? 98h
  0x4003016b: [85                   ] ?? 85h
  0x4003016c: [18                   ] ?? 18h
  0x4003016d: [dd                   ] ?? DDh
  0x4003016e: [7b                   ] ?? 7Bh    {
  0x4003016f: [08                   ] ?? 08h
  0x40030170: [fe                   ] ?? FEh
  0x40030171: [ff                   ] ?? FFh
  0x40030172: [00                   ] ?? 00h
  0x40030173: [00                   ] ?? 00h
  0x40030174: [2c                   ] ?? 2Ch    ,
  0x40030175: [c1                   ] ?? C1h
  0x40030176: [00                   ] ?? 00h
  0x40030177: [f8                   ] ?? F8h
  0x40030178: [93                   ] ?? 93h
  0x40030179: [2c                   ] ?? 2Ch    ,
  0x4003017a: [00                   ] ?? 00h
  0x4003017b: [f8                   ] ?? F8h
  0x4003017c: [93                   ] ?? 93h
  0x4003017d: [2e                   ] ?? 2Eh    .
  0x4003017e: [98                   ] ?? 98h
  0x4003017f: [dd                   ] ?? DDh
  0x40030180: [b3                   ] ?? B3h
  0x40030181: [28                   ] ?? 28h    (
  0x40030182: [0a                   ] ?? 0Ah
  0x40030183: [00                   ] ?? 00h
  0x40030184: [f0                   ] ?? F0h
  0x40030185: [24                   ] ?? 24h    $
  0x40030186: [f0                   ] ?? F0h
  0x40030187: [73                   ] ?? 73h    s
  0x40030188: [2c                   ] ?? 2Ch    ,
  0x40030189: [26                   ] ?? 26h    &
  0x4003018a: [05                   ] ?? 05h
  0x4003018b: [ca                   ] ?? CAh
  0x4003018c: [2a                   ] ?? 2Ah    *
  0x4003018d: [1e                   ] ?? 1Eh
  0x4003018e: [f8                   ] ?? F8h
  0x4003018f: [ee                   ] ?? EEh
  0x40030190: [10                   ] ?? 10h
  0x40030191: [c9                   ] ?? C9h
  0x40030192: [07                   ] ?? 07h
  0x40030193: [2c                   ] ?? 2Ch    ,
  0x40030194: [0e                   ] ?? 0Eh
  0x40030195: [03                   ] ?? 03h
  0x40030196: [ca                   ] ?? CAh
  0x40030197: [1f                   ] ?? 1Fh
  0x40030198: [38                   ] ?? 38h    8
  0x40030199: [4e                   ] ?? 4Eh    N
  0x4003019a: [1b                   ] ?? 1Bh
  0x4003019b: [f8                   ] ?? F8h
  0x4003019c: [ec                   ] ?? ECh
  0x4003019d: [10                   ] ?? 10h
  0x4003019e: [c8                   ] ?? C8h
  0x4003019f: [0f                   ] ?? 0Fh
  0x400301a0: [2c                   ] ?? 2Ch    ,
  0x400301a1: [08                   ] ?? 08h
  0x400301a2: [03                   ] ?? 03h
  0x400301a3: [03                   ] ?? 03h
  0x400301a4: [5c                   ] ?? 5Ch    \
  0x400301a5: [0f                   ] ?? 0Fh
  0x400301a6: [fe                   ] ?? FEh
  0x400301a7: [80                   ] ?? 80h
  0x400301a8: [85                   ] ?? 85h
  0x400301a9: [0f                   ] ?? 0Fh
  0x400301aa: [10                   ] ?? 10h
  0x400301ab: [ca                   ] ?? CAh
  0x400301ac: [22                   ] ?? 22h    "
  0x400301ad: [2c                   ] ?? 2Ch    ,
  0x400301ae: [2a                   ] ?? 2Ah    *
  0x400301af: [06                   ] ?? 06h
  0x400301b0: [ca                   ] ?? CAh
  0x400301b1: [05                   ] ?? 05h
  0x400301b2: [2c                   ] ?? 2Ch    ,
  0x400301b3: [2d                   ] ?? 2Dh    -
  0x400301b4: [06                   ] ?? 06h
  0x400301b5: [03 5c 0f             ] MOVHU D0,(0xf5c)
  0x400301b8: [ca 15                ] BRA 0x400301cd
  0x400301ba: [80                   ] ?? 80h
  0x400301bb: [3e                   ] ?? 3Eh    >
  0x400301bc: [02                   ] ?? 02h
  0x400301bd: [32                   ] ?? 32h    2
  0x400301be: [14                   ] ?? 14h
  0x400301bf: [80                   ] ?? 80h
  0x400301c0: [2e                   ] ?? 2Eh    .
  0x400301c1: [ca                   ] ?? CAh
  0x400301c2: [0a                   ] ?? 0Ah
  0x400301c3: [2c                   ] ?? 2Ch    ,
  0x400301c4: [f7                   ] ?? F7h
  0x400301c5: [00                   ] ?? 00h
  0x400301c6: [02                   ] ?? 02h
  0x400301c7: [32                   ] ?? 32h    2
  0x400301c8: [14                   ] ?? 14h
  0x400301c9: [80                   ] ?? 80h
  0x400301ca: [3d                   ] ?? 3Dh    =
  0x400301cb: [f0                   ] ?? F0h
  0x400301cc: [73                   ] ?? 73h    s
  0x400301cd: [00                   ] CLR D0
  0x400301ce: [df f0 40             ] RET [A3,A2,D3,D2],0x40
```

### 4.3 Despachante ATAPI (`0x3c` Write Buffer), Sense Table (`0x400089dc`) & Strings Shift-JIS (`DRV12`)

#### `FUN_40004852` (`drv_sense_lookup_4852`) — `0x40004852` (150 B)

Lookup na tabela de Sense (`0x400089dc`) do drive (`drv_sense_lookup_4852`).

<!-- source: doc_drv_and_atapi.txt | FUNC FUN_40004852 (drv_sense_lookup_4852) @ 0x40004852 (size=150 B, req=0x40004852) -->
```c
// Callers (1): 0x40004cc3
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

uint FUN_40004852(void)

{
  byte bVar1;
  undefined *puVar2;
  uint uVar3;
  
  FUN_400017ca(0,0x12);
  DAT_889ed2d6 = 0x70;
  DAT_889ed2dd = 10;
  if (_DAT_000001b0 == 0) {
    return 10;
  }
  puVar2 = &DAT_400089dc;
  DAT_889ed2d8 = (&DAT_400089dc)[(uint)_DAT_000001b0 * 3];
  DAT_889ed2de = DAT_000001a8;
  DAT_889ed2df = DAT_000001a9;
  FUN_4000186d(_DAT_000001aa);
  DAT_889ed2e2 = puVar2[(uint)_DAT_000001b0 * 3 + 1];
  DAT_889ed2e3 = puVar2[(uint)_DAT_000001b0 * 3 + 2];
  uVar3 = (uint)_DAT_000001e8;
  if ((_DAT_000001e8 & 0x400) != 0) {
    bVar1 = DAT_889ed2d6;
    DAT_889ed2d6 = bVar1 | 0x80;
    uVar3 = FUN_40001831(_DAT_000001ac);
  }
  return uVar3;
}
```

#### `FUN_40005772` (`drv_mecha_helper_5772`) — `0x40005772` (114 B)

Helper de erro/estado mecânico com uso de strings em tabela (`drv_mecha_helper_5772`).

<!-- source: doc_drv_and_atapi.txt | FUNC FUN_40005772 (drv_mecha_helper_5772) @ 0x40005772 (size=114 B, req=0x40005772) -->
```c
// Callers (1): 0x400054ed
undefined4 FUN_40005772(uint param_1)

{
  int *in_A0;
  uint uVar1;
  undefined4 uVar2;
  short sVar3;
  int iVar4;
  ushort *puVar5;
  
  sVar3 = 0;
  if ((*in_A0 == 0) && (in_A0[2] == 0)) {
    iVar4 = in_A0[4];
  }
  else if (*in_A0 == 0) {
    iVar4 = in_A0[2];
  }
  else {
    iVar4 = *in_A0;
  }
  puVar5 = (ushort *)(&DAT_88800000 + iVar4);
  if (param_1 == 0) {
    param_1 = in_A0[1] + in_A0[3] + in_A0[5];
  }
  for (uVar1 = 0; uVar1 < param_1; uVar1 = uVar1 + 2) {
    sVar3 = sVar3 + *puVar5 * 0x100 + (*puVar5 >> 8);
    puVar5 = puVar5 + 1;
  }
  if ((ushort)(sVar3 + (puVar5[-1] & 0xff) * -0x100) == puVar5[-1] >> 8) {
    uVar2 = 0;
  }
  else {
    uVar2 = 0x46;
  }
  return uVar2;
}
```

#### `FUN_40005aba` (`drv_atapi_sense_5aba`) — `0x40005aba` (570 B)

Construção de resposta `REQUEST SENSE` ATAPI (`drv_atapi_sense_5aba`).

<!-- source: doc_drv_and_atapi.txt | FUNC FUN_40005aba (drv_atapi_sense_5aba) @ 0x40005aba (size=570 B, req=0x40005aba) -->
```c
// Callers (1): 0x40005023
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

uint FUN_40005aba(void)

{
  undefined2 uVar1;
  undefined1 uVar2;
  undefined1 uVar3;
  undefined1 uVar4;
  undefined1 uVar5;
  undefined2 *puVar6;
  char *pcVar7;
  ushort uVar8;
  short sVar9;
  uint uVar10;
  uint uVar11;
  undefined2 *puVar12;
  uint uVar13;
  uint extraout_var;
  uint extraout_var_00;
  int extraout_var_01;
  short sStack0000000c;
  int iStack00000010;
  byte bStack00000014;
  byte bStack00000015;
  
  uVar10 = FUN_40005f0e();
  if ((uVar10 < 0x7fc) && (0xf4 < uVar10)) {
    puVar6 = (undefined2 *)&DAT_88800000;
    puVar12 = (undefined2 *)(&DAT_4000f800 + _DAT_00000028);
    for (uVar11 = 0; (uVar11 & 0xffff) < (uVar10 & 0xffff); uVar11 = (uVar11 & 0xffff) + 2) {
      uVar1 = *puVar12;
      puVar12 = puVar12 + 1;
      *puVar6 = uVar1;
      puVar6 = puVar6 + 1;
    }
  }
  else {
    FUN_40005d46();
    uVar10 = 0xf4;
  }
  puVar6 = (undefined2 *)&DAT_8880008a;
  *(undefined2 *)(&DAT_88800000 + uVar10) = 0;
  uVar2 = DAT_888000e2;
  uVar3 = DAT_888000e3;
  uVar4 = DAT_888000e4;
  uVar5 = DAT_888000e5;
  iStack00000010 = CONCAT31(CONCAT21(CONCAT11(uVar2,uVar3),uVar4),uVar5);
  uVar10 = FUN_40006154(0x10);
  uVar8 = 0;
  if (((int)uVar10 >> 0x1f & uVar10 & (int)uVar10 >> 0x1f & 0xffff0000U) == 0) {
    pcVar7 = s_DMR_E550C_MATSHITAR6_MV4_A0_B014_4000bfd0 + 0x10;
    for (; uVar8 < 0x1c; uVar8 = uVar8 + 2) {
      uVar1 = *(undefined2 *)pcVar7;
      pcVar7 = pcVar7 + 2;
      *puVar6 = uVar1;
      puVar6 = puVar6 + 1;
    }
    (&DAT_888000f4)[iStack00000010] = 0;
    (&UNK_888000f5)[iStack00000010] = 1;
  }
  else {
    for (; uVar8 < 0x1c; uVar8 = uVar8 + 2) {
      *puVar6 = 0;
      puVar6 = puVar6 + 1;
    }
  }
  puVar6 = (undefined2 *)&DAT_888000ae;
  uVar10 = FUN_40006154(0x10);
  uVar8 = 0;
  if (((int)uVar10 >> 0x1f & uVar10 & (int)uVar10 >> 0x1f & 0xffff0000U) == 0) {
    puVar12 = &DAT_4000fff8;
    for (; uVar8 < 4; uVar8 = uVar8 + 2) {
      uVar1 = *puVar12;
      puVar12 = puVar12 + 1;
      *puVar6 = uVar1;
      puVar6 = puVar6 + 1;
    }
    (&UNK_8880010c)[iStack00000010] = 0;
    (&UNK_8880010d)[iStack00000010] = 1;
  }
  else {
    for (; uVar8 < 4; uVar8 = uVar8 + 2) {
      *puVar6 = 0;
      puVar6 = puVar6 + 1;
    }
  }
  (&DAT_88800100)[iStack00000010] = 0;
  (&UNK_88800101)[iStack00000010] = 1;
  sStack0000000c = FUN_40005e39();
  uVar11 = 0;
  puVar6 = (undefined2 *)&DAT_88800012;
  uVar10 = extraout_var - 0x14;
  for (uVar13 = 0; (uVar13 & 0xffff) < extraout_var - 0x14; uVar13 = (uVar13 & 0xffff) + 2) {
    bStack00000014 = (byte)*puVar6;
    bStack00000015 = (byte)((ushort)*puVar6 >> 8);
    uVar10 = (uint)bStack00000014 * 0x100;
    uVar11 = (uVar11 & 0xffff) + uVar10 + (uint)bStack00000015;
    puVar6 = puVar6 + 1;
  }
  bStack00000015 = (byte)(uVar11 >> 8);
  *(byte *)(extraout_var_01 + -2) = bStack00000015;
  bStack00000014 = (byte)uVar11;
  *(byte *)(extraout_var_01 + -1) = bStack00000014;
  bStack00000015 = (byte)((ushort)sStack0000000c >> 8);
  DAT_88800010 = bStack00000015;
  bStack00000014 = (byte)sStack0000000c;
  DAT_88800011 = bStack00000014;
  uVar13 = extraout_var;
  if ((sStack0000000c == -1) ||
     (sVar9 = FUN_40006699(sStack0000000c,uVar11,uVar10), uVar13 = extraout_var_00, sVar9 == 0)) {
    uVar13 = uVar13 & 0xffff;
  }
  else {
    uVar13 = 0;
  }
  return uVar13;
}
```

#### `FUN_40005d46` (`drv_atapi_err_5d46`) — `0x40005d46` (243 B)

Registro/propagação de erro ATAPI (`drv_atapi_err_5d46`).

<!-- source: doc_drv_and_atapi.txt | FUNC FUN_40005d46 (drv_atapi_err_5d46) @ 0x40005d46 (size=243 B, req=0x40005d46) -->
```c
// Callers (1): 0x40005afc
void FUN_40005d46(void)

{
  undefined2 *puVar1;
  ushort uVar2;
  
  puVar1 = (undefined2 *)&DAT_88800000;
  for (uVar2 = 0; uVar2 < 0xf4; uVar2 = uVar2 + 2) {
    *puVar1 = 0;
    puVar1 = puVar1 + 1;
  }
  FUN_40001788(0x10);
  DAT_88800012 = 0;
  DAT_88800013 = 0;
  DAT_88800014 = 0;
  DAT_88800015 = 0xde;
  DAT_88800016 = 0;
  DAT_88800017 = 0;
  DAT_88800018 = 0;
  DAT_88800019 = 200;
  DAT_888000ea = 0;
  DAT_888000eb = 0;
  DAT_888000ec = 0;
  DAT_888000ed = 4;
  FUN_40001788(0x10);
  FUN_40001788(4);
  FUN_40001788(4);
  FUN_40001788(4);
  return;
}
```

#### `FUN_4001fcaf` (`drv_cmd_exec_1fcaf`) — `0x4001fcaf` (394 B)

Executor interno de comandos do drive (`drv_cmd_exec_1fcaf`).

<!-- source: doc_drv_and_atapi.txt | FUNC FUN_4001fcaf (drv_cmd_exec_1fcaf) @ 0x4001fcaf (size=394 B, req=0x4001fcaf) -->
```c
// Callers (1): 0x400dea24
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

short FUN_4001fcaf(void)

{
  byte bVar1;
  short sVar2;
  
  if ((_DAT_00000306 & 0x2000) == 0) {
    bVar1 = DAT_0000023d & 1;
  }
  else {
    if (((_DAT_0000043a & 0x80) == 0) && ((DAT_00000241 & 0xf) != 0)) {
      return 0x45;
    }
    if ((DAT_0000023d & 1) != 0) goto LAB_4001fce3;
    bVar1 = DAT_00000245 & 0xf;
  }
  if (bVar1 != 0) {
    return 0x45;
  }
LAB_4001fce3:
  if (((_DAT_00000306 & 2) != 0) && ((_DAT_0000043a & 0x80) == 0)) {
    return 0x45;
  }
  _DAT_00000468 = FUN_400109a0();
  _DAT_0000046c = FUN_400109a0();
  _DAT_00000470 = 0;
  _DAT_00000464 = 0;
  _DAT_00000474 = 0;
  _DAT_00000454 = 0;
  _DAT_00000458 = 0;
  _DAT_0000045c = 0;
  _DAT_00000450 = 0;
  _DAT_00000460 = 0;
  DAT_00000462 = 0;
  DAT_00000248 = 0x67;
  DAT_00000249 = 0;
  if ((DAT_0000023d & 1) != 0) {
    DAT_00000249 = 0x80;
  }
  if ((DAT_0000023d & 2) != 0) {
    DAT_00000249 = DAT_00000249 | 1;
  }
  if ((_DAT_0000043a & 0x80) != 0) {
    DAT_00000249 = DAT_00000249 | 0x40;
  }
  if (((_DAT_0000043a & 0x80) != 0) && ((_DAT_0000043a & 2) != 0)) {
    DAT_00000249 = DAT_00000249 | 0x20;
  }
  if ((DAT_0000023d & 1) != 0) {
    _DAT_0000043a = _DAT_0000043a | 0x10;
  }
  while( true ) {
    if ((char)('\x10' - DAT_0000045c) == '\0') break;
    sVar2 = FUN_4001273d('\x10' - DAT_0000045c);
    if (sVar2 != 0) {
      return sVar2;
    }
    FUN_400dd3ac(0,0x1800);
    sVar2 = FUN_40012916();
    if (sVar2 != 0) {
      return sVar2;
    }
  }
  _DAT_0000036c = _DAT_0000046c;
  FUN_40015881(1);
  sVar2 = FUN_4001418b();
  if (sVar2 != 0) {
    return sVar2;
  }
  if ((DAT_00000247 & 0x80) != 0) {
    DAT_00000424 = 1;
  }
  sVar2 = FUN_40014271(1);
  if (sVar2 == 0) {
    DAT_00000424 = 0;
    return 0;
  }
  _DAT_000003d0 = _DAT_000003d0 | 0x400;
  _DAT_00000394 = _DAT_00000468 + _DAT_00000470;
  if ((_DAT_00000306 & 0x1000) == 0) {
    DAT_00000424 = 0;
    return sVar2;
  }
  if ((_DAT_0000043a & 0x80) != 0) {
    DAT_00000424 = 0;
    return sVar2;
  }
  _DAT_00000394 = _DAT_00000394 + 0x10096;
  DAT_00000424 = 0;
  return sVar2;
}
```

#### `FUN_4002025e` (`drv_cmd_tbl_2025e`) — `0x4002025e` (1484 B)

Despacho por tabela de opcodes/subcomandos do drive (`drv_cmd_tbl_2025e`).

<!-- source: doc_drv_and_atapi.txt | FUNC FUN_4002025e (drv_cmd_tbl_2025e) @ 0x4002025e (size=1484 B, req=0x4002025e) -->
```c
// Callers (1): 0x400de7e8
// Referenced strings/literals:
//   0x400d7c67: 'üÞ'
//   0x400d7bc3: ',ÿ'
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

short FUN_4002025e(undefined4 param_1,undefined4 param_2)

{
  undefined *puVar1;
  undefined1 extraout_D0;
  undefined1 extraout_D0_00;
  undefined1 extraout_D0_01;
  undefined1 extraout_D0_02;
  undefined1 extraout_D0_03;
  undefined1 extraout_D0_04;
  ushort uVar2;
  short sVar3;
  undefined2 uVar4;
  short sVar5;
  undefined *puVar6;
  uint uVar7;
  int iVar8;
  byte bVar9;
  uint uVar10;
  undefined1 uVar11;
  uint uVar12;
  uint uStackY_70;
  uint uStackY_6c;
  int iStackY_68;
  int iStackY_64;
  int iStackY_50;
  int extraout_var;
  undefined *puStack_20;
  int extraout_var_00;
  undefined4 extraout_var_01;
  short extraout_var_02;
  short extraout_var_03;
  short extraout_var_04;
  short extraout_var_05;
  ushort uStack_16;
  ushort uStack_14;
  
  if ((DAT_00000e41 & 0x80) == 0) {
    return 0x41;
  }
  FUN_40015869(0,param_2,0);
  FUN_400109a0();
  uVar12 = (uint)DAT_00000242;
  uVar2 = FUN_400109e0();
  if (uVar12 == 1) {
    if (0x1000 < uVar2) {
      return 0x45;
    }
    if ((uStack_14 & 1) != 0) {
      sVar3 = FUN_4001294e(uVar2,0x1fc9e0);
      for (uVar12 = 0; uVar12 = uVar12 & 0xffff, (int)uVar12 < (int)puStack_20; uVar12 = uVar12 + 1)
      {
        FUN_40020953();
        sVar3 = extraout_var_03;
      }
      return sVar3;
    }
    sVar3 = FUN_4001a1bd();
    if ((sVar3 != 0) && (((_DAT_00000306 & 1) != 0 || ((_DAT_0000043c & 0x40) != 0)))) {
      DAT_00000390 = 0x11;
      return 0x57;
    }
    puVar6 = &DAT_889fc9e0;
    for (uVar12 = 0; uVar12 = uVar12 & 0xffff, uVar12 < uVar2; uVar12 = uVar12 + 1) {
      FUN_40020913();
      *puVar6 = extraout_D0;
      puVar6 = puVar6 + 1;
    }
  }
  else {
    if (uVar12 != 3) {
      if (uVar12 == 4) {
        sVar3 = FUN_400119d7(0);
        if (sVar3 != 0) {
          return sVar3;
        }
        for (bVar9 = 0; uVar2 = 0, bVar9 < 0x1e; bVar9 = bVar9 + 1) {
          for (; uVar2 < 0x1290; uVar2 = uVar2 + 1) {
          }
        }
        _DAT_0000b000 = _DAT_0000b000 & 0x3fff;
        FUN_400114fd(_DAT_0000b000,bVar9,0);
        thunk_FUN_40000010();
        return extraout_var_04;
      }
      if (uVar12 == 6) {
        return extraout_var_02;
      }
      if (uVar12 == 7) {
        if (uStack_14 == 4) {
          uVar12 = FUN_400109e0();
          uVar4 = FUN_400109e0();
          FUN_40054fce(uVar4,uVar12 & 0xffff);
          return extraout_var_05;
        }
        if (uStack_14 == 0x11) {
          sVar3 = FUN_40028f69();
          return sVar3;
        }
        return 0x45;
      }
      if (uVar12 == 0x62) {
        uVar12 = (uint)uStack_14;
        if ((uStack_14 & 4) != 0) {
          return 0;
        }
        if ((uStack_14 & 8) == 0) {
          if ((uStack_14 & 2) == 0) {
            if ((uStack_14 & 0x10) == 0) {
              uVar4 = FUN_400109e0();
              if ((uVar12 & 1) == 0) {
                if ((uVar12 & 0x80) == 0) {
                  sVar5 = FUN_400d70c0(uVar4,uStack_16);
                }
                else {
                  sVar5 = FUN_400d70cb(uVar4,uStack_16);
                }
                if (sVar5 == 0) {
                  sVar5 = FUN_4001294e(uStack_16,0x1fc9e0);
                }
                sVar3 = 0;
                if (sVar5 != 0) {
                  return sVar5;
                }
              }
              else {
                sVar3 = FUN_4001294e(uStack_16,0x1fc9e0);
                if (sVar3 != 0) {
                  return sVar3;
                }
                if ((uStackY_70 & 0x80) == 0) {
                  sVar3 = FUN_400d71e3(uStackY_6c & 0xffff,uStack_16,extraout_var_01,0);
                }
                else {
                  sVar3 = FUN_400d71ee(uStackY_6c & 0xffff,uStack_16,uStackY_70,uStackY_6c & 0xffff,
                                       iStackY_68,iStackY_64);
                }
              }
            }
            else if ((uStack_14 & 0x80) == 0) {
              sVar3 = FUN_400d7c67();
            }
            else {
              sVar3 = FUN_400d7bc3();
            }
          }
          else {
            sVar3 = FUN_400d7378();
          }
          if (sVar3 == 0) {
            return 0;
          }
          DAT_00000390 = 0x41;
          _DAT_00000392 = _DAT_00000de8;
          DAT_0000039a = 0x41;
          return sVar3;
        }
        return 0x45;
      }
      if (uVar12 == 0x72) {
        sVar3 = FUN_4003c9b9();
        return sVar3;
      }
      if (uVar12 == 0x74) {
        sVar3 = FUN_4005202a();
        return sVar3;
      }
      if (uVar12 == 0x76) {
        sVar3 = FUN_400d347e();
        return sVar3;
      }
      if ((((uVar12 == 0x31) || (uVar12 == 0x4b)) || (uVar12 == 0x4d)) ||
         ((uVar12 == 0x4e || (uVar12 == 0x60)))) {
        sVar3 = FUN_400259d8(0x80);
        if (sVar3 != 0) {
          return sVar3;
        }
        FUN_4002453a(4);
        if ((iStackY_50 == 0x31) && (((DAT_0000023d & 0x1f) == 6 || ((DAT_0000023d & 0x1f) == 7))))
        {
          bVar9 = 0x10;
          _DAT_00000460 = 0;
          _DAT_0000045c = 0;
          DAT_00000462 = 0;
          if ((DAT_0000023d & 3) == 3) {
            bVar9 = 0x20;
          }
          uVar12 = (uint)bVar9;
          do {
            sVar3 = FUN_4001273d(uVar12);
            if (sVar3 != 0) {
              return sVar3;
            }
            iVar8 = extraout_var;
            FUN_400dd3ac(0,0x1800);
            sVar3 = FUN_40012916();
            if (sVar3 != 0) {
              return sVar3;
            }
          } while (iVar8 != _DAT_0000045c);
        }
      }
      sVar3 = FUN_4002082a();
      return sVar3;
    }
    uVar12 = (uint)uStack_14;
    if (uVar12 == 0) {
      puVar6 = (undefined *)FUN_4008526f(1);
      puStack_20 = (undefined *)FUN_4008526f(0);
      FUN_4008526f(2);
      uVar11 = extraout_D0_00;
      uVar7 = FUN_4008526f(3);
    }
    else if (uVar12 == 1) {
      puVar6 = &UNK_889c89e0;
      puStack_20 = &UNK_889c89e0;
      uVar11 = 0x10;
      uVar7 = 0x1000;
    }
    else if (uVar12 == 2) {
      puVar6 = _DAT_00000cd0;
      if (_DAT_00000cd0 == (undefined *)0x0) {
        puVar6 = &UNK_889f65e0;
      }
      puStack_20 = &UNK_889f65e0;
      uVar11 = 0x10;
      uVar7 = 0x800;
    }
    else if (uVar12 == 4) {
      puVar6 = (undefined *)FUN_40054fab(1);
      puStack_20 = (undefined *)FUN_40054fab(0);
      FUN_40054fab(2);
      uVar11 = extraout_D0_01;
      uVar7 = FUN_40054fab(3);
    }
    else if (uVar12 == 8) {
      puVar6 = (undefined *)FUN_400bd9c3(1);
      puStack_20 = (undefined *)FUN_400bd9c3(0);
      FUN_400bd9c3(2);
      uVar11 = extraout_D0_02;
      uVar7 = FUN_400bd9c3(3);
    }
    else if (uVar12 == 9) {
      puVar6 = (undefined *)FUN_40070c32(1);
      puStack_20 = (undefined *)FUN_40070c32(0);
      FUN_40070c32(2);
      uVar11 = extraout_D0_03;
      uVar7 = FUN_40070c32(3);
    }
    else if (uVar12 == 0x10) {
      puStack_20 = &UNK_889f6de0;
      uVar11 = 0x10;
      uVar7 = 0xc00;
      puVar6 = _DAT_0000003c;
    }
    else {
      if (uVar12 != 0x11) {
        return 0x45;
      }
      puVar6 = (undefined *)FUN_40028f3e(1);
      puStack_20 = (undefined *)FUN_40028f3e(0);
      FUN_40028f3e(2);
      uVar11 = extraout_D0_04;
      uVar7 = FUN_40028f3e(3);
    }
    puVar1 = puStack_20 + (uVar7 & 0xffff);
    DAT_889fc9e0 = (char)uStack_14;
    DAT_889fc9e1 = uVar11;
    uVar7 = uVar7 & 0xffff;
    FUN_40010a35(uVar7);
    if ((puVar6 < puStack_20) || (puVar1 < puVar6)) {
      DAT_00000390 = 0x26;
      return 0x3d;
    }
    if ((uVar12 == 0) || (uVar12 == 0x11)) {
      uVar2 = 4;
      if (uStack_16 < 4) {
        uVar2 = uStack_16;
      }
      sVar3 = FUN_4001294e(uVar2,0x1fc9e0);
      if (sVar3 != 0) {
        return sVar3;
      }
      uVar12 = uStackY_70 - (uStackY_6c & 0xffff) & 0xffff;
      if (uVar12 == 0) {
        return 0;
      }
      uVar7 = extraout_var_00 - iStackY_64;
      if (uVar12 < (extraout_var_00 - iStackY_64 & 0xffffU)) {
        uVar7 = uVar12;
      }
      sVar3 = FUN_4001294e(uVar7 & 0xffff);
      uVar7 = iStackY_68 - (uStackY_70 & 0xffff);
      if (sVar3 != 0) {
        return sVar3;
      }
      uVar10 = uVar7 & 0xffff;
      if (uVar10 == 0) {
        return 0;
      }
      uVar12 = iStackY_64 - (int)puStack_20;
      if (uVar10 < (iStackY_64 - (int)puStack_20 & 0xffffU)) {
        uVar12 = uVar7;
      }
      uVar12 = uVar12 & 0xffff;
      iVar8 = (int)puStack_20 - uStackY_6c;
      goto LAB_400205e9;
    }
    FUN_40010950();
    FUN_40010950((int)puVar6 - (int)puStack_20);
    if ((int)(uVar7 + 4) < (int)(uint)uStack_16) {
      uStack_16 = (ushort)(uVar7 + 4);
    }
  }
  uVar12 = (uint)uStack_16;
  iVar8 = 0x1fc9e0;
LAB_400205e9:
  sVar3 = FUN_4001294e(uVar12,iVar8);
  return sVar3;
}
```

#### `FUN_400209f3` (`drv_atapi_dispatch_209f3`) — `0x400209f3` (1755 B)

Despachante ATAPI principal — inclui opcode `0x3c` Write Buffer / Download Microcode (`drv_atapi_dispatch_209f3`).

<!-- source: doc_drv_and_atapi.txt | FUNC FUN_400209f3 (drv_atapi_dispatch_209f3) @ 0x400209f3 (size=1755 B, req=0x400209f3) -->
```c
// Callers (1): 0x400de9ec
// Referenced strings/literals:
//   0x400df460: '.IOP\rIOP=       \r\n%'
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

uint FUN_400209f3(void)

{
  undefined1 uVar1;
  undefined1 *puVar2;
  undefined1 *puVar3;
  undefined4 in_A2;
  undefined4 *puVar4;
  undefined4 in_A3;
  undefined **ppuVar5;
  byte bVar6;
  short sVar7;
  undefined2 uVar8;
  uint uVar9;
  uint uVar10;
  undefined4 uVar11;
  int iVar12;
  int iVar13;
  byte bVar14;
  undefined *puVar15;
  uint in_D3;
  uint uVar16;
  undefined4 uStackY_88;
  int iStackY_84;
  int iStackY_80;
  undefined4 uStackY_68;
  int iStackY_5c;
  int iStackY_58;
  uint uStackY_50;
  undefined4 *puStackY_4c;
  uint in_stack_ffffffb8;
  undefined4 extraout_var;
  int iStack_34;
  char cStack_30;
  undefined2 uStack_2c;
  undefined2 uStack_2a;
  undefined1 auStack_28 [8];
  undefined2 uStack_20;
  
  puVar3 = &DAT_400df460;
  puVar2 = auStack_28;
  iVar13 = -5;
  do {
    *puVar2 = *puVar3;
    puVar3 = puVar3 + 1;
    puVar2 = puVar2 + 1;
    iVar13 = iVar13 + 1;
  } while (iVar13 != 0);
  if (DAT_00000247 == '<') {
    if (DAT_00000242 == 1) {
      if ((DAT_0000023d & 1) == 1) {
        FUN_400134f1();
        FUN_40013537();
      }
      else {
        FUN_400134f1();
        FUN_40013572();
      }
      sVar7 = FUN_400119d7(0);
      if (sVar7 == 0) {
        uVar9 = (*(code *)PTR_FUN_40001084)();
        return uVar9;
      }
      uVar9 = 0x45;
    }
    else {
      if (DAT_00000242 == 0x10) {
        FUN_400109e0();
        uVar9 = FUN_400109e0();
        if ((_DAT_0000043c & 7) != 0) {
          return 0x47;
        }
        uVar10 = FUN_4002c2f0();
        if (((int)uVar10 >> 0x1f & uVar10 & (int)uVar10 >> 0x1f & 0xffff0000U) == 0) {
          uStack_2c = (*(code *)PTR_FUN_40001090)(in_stack_ffffffb8 & 0xffff);
        }
        else {
          uStack_2c = 0;
        }
        if (uStack_2c == 0) {
          return 0x45;
        }
        if ((uVar9 & 0xffff) == 0) {
          return 0;
        }
        if ((uVar9 & 0xffff) < (uint)uStack_2c) {
          uStack_2c = (ushort)uVar9;
        }
        FUN_4001294e(uStack_2c,0);
      }
      else if (DAT_00000242 == 0x91) {
        uStack_20 = CONCAT11(DAT_00000245,DAT_00000246);
        if (uStack_20 < 0x801) {
          if (uStack_20 == 0) {
            DAT_889fc9e0 = 8;
            DAT_889fc9e1 = 0;
            FUN_4001294e(0x802,0x1fc9e0);
          }
          else {
            DAT_889fc9e0 = DAT_00000245;
            DAT_889fc9e1 = DAT_00000246;
            FUN_4001294e(2,0x1fc9e0);
            FUN_4001294e(uStack_20,uStackY_88);
          }
        }
        else if (DAT_0000023f == 1) {
          FUN_40021122(&DAT_00002800);
          FUN_40021156(0x2800 - _DAT_00000ce2,_DAT_00000ce2,in_D3,&UNK_889f1fe0,in_A3);
          FUN_40021156(_DAT_00000ce2);
        }
        else if (_DAT_00000ce2 == _DAT_00000ce4) {
          DAT_889fc9e0 = 0;
          DAT_889fc9e1 = 1;
          DAT_889fc9e2 = 0x25;
          DAT_889fc9e3 = 0x20;
          uVar11 = 4;
          if (uStack_20 == 0) {
            DAT_889fc9e1 = 0;
            uVar11 = 2;
          }
          FUN_4001294e(uVar11,0x1fc9e0);
        }
        else {
          if (_DAT_00000ce4 < _DAT_00000ce2) {
            uVar9 = (uint)_DAT_00000ce2 - (uint)_DAT_00000ce4 & 0xffff;
            FUN_40021122(uVar9);
            FUN_40021156(uVar9,_DAT_00000ce4,in_D3,in_A2,in_A3);
            in_A3 = extraout_var;
            in_D3 = uStackY_50;
          }
          if (_DAT_00000ce2 < _DAT_00000ce4) {
            puVar15 = &DAT_00002800;
            FUN_40021122(0x2800 - (_DAT_00000ce4 - _DAT_00000ce2));
            FUN_40021156((int)puVar15 - (uint)_DAT_00000ce4 & 0xffff,_DAT_00000ce4,in_D3,
                         &UNK_889f1fe0,in_A3);
            FUN_40021156(_DAT_00000ce2);
          }
          _DAT_00000ce4 = _DAT_00000ce2;
        }
      }
      else if (DAT_00000242 == 0x12) {
        FUN_4001294e(0x28,0x1fc9e0);
        sVar7 = FUN_400210d1();
        if (sVar7 == 0) {
          return 0x45;
        }
        FUN_40010992(0,0xc);
        if ((DAT_00000e41 & 0x80) == 0) {
          DAT_00000e41 = DAT_00000e41 | 0x80;
          FUN_40137eb9(0);
          DAT_00000e41 = DAT_00000e41 | 0x7f;
        }
        else {
          FUN_40137eb9(0);
        }
        FUN_4002a178();
        FUN_400d70c0(0xbc,1);
        uVar8 = 0xbc;
        if (cStack_30 != -1) {
          uVar8 = 0xbd;
        }
        if ((DAT_00000392 < 0xa0) || (0xbf < DAT_00000392)) {
          _DAT_00000392 = 0;
        }
        FUN_400d71e3(uVar8,1);
        FUN_400d7378();
        for (uVar9 = 0; uVar9 = uVar9 & 0xffff, uVar9 < 0x40; uVar9 = uVar9 + 1) {
          *(undefined1 *)(uVar9 + 0xf06 + iStackY_5c) = 0;
        }
        DAT_889fd8e6 = 0;
        DAT_889fd8e7 = 1;
        DAT_889fd8e8 = 7;
        FUN_40010992(0,0xc);
        FUN_40137eb9(0);
        FUN_40010950(0xe);
        FUN_400d70c0(uStackY_68,1);
        uVar1 = DAT_889fc9f0;
        FUN_400210ef(uVar1);
        DAT_889fc9e0 = 0;
        DAT_889fc9e1 = 0x10;
      }
      else {
        if ((DAT_00000242 == 0x93) || (DAT_00000242 == 0x13)) {
          if ((DAT_00000242 & 0x80) == 0) {
            DAT_0000023d = 3;
          }
          else {
            DAT_0000023d = 2;
          }
          DAT_00000242 = 100;
          DAT_00000243 = DAT_00000245;
          DAT_00000244 = DAT_00000246;
          if ((DAT_00000e41 & 0x80) == 0) {
            DAT_00000e41 = DAT_00000e41 | 0x80;
            uVar9 = FUN_4002082a();
            DAT_00000e41 = DAT_00000e41 | 0x7f;
          }
          else {
            uVar9 = FUN_4002082a();
          }
          return uVar9 & 0xffff;
        }
        if (DAT_00000242 == 0x94) {
          bVar14 = DAT_0000023e ^ DAT_0000023d ^ DAT_00000240;
          uStack_2a = CONCAT11(bVar14,DAT_0000023e ^ DAT_00000241);
          bVar6 = DAT_0000023f ^ DAT_0000023d ^ DAT_00000243;
          uStack_2c = CONCAT11(bVar6,DAT_0000023f ^ DAT_00000244);
          if (((DAT_00000245 == (byte)(DAT_0000023e ^ DAT_00000241 ^ bVar6)) &&
              (DAT_00000246 == (byte)(bVar14 ^ DAT_0000023f ^ DAT_00000244))) && (uStack_2c < 0x801)
             ) {
            if ((DAT_0000023d & 1) == 0) {
              FUN_400d70c0(uStack_2a);
            }
            else {
              FUN_400d71c0(uStack_2a);
            }
          }
          else {
            DAT_889fc9e0 = DAT_0000023d;
            DAT_889fc9e1 = DAT_0000023e;
            uStack_2c = 2;
          }
          FUN_4001294e(uStack_2c,0x1fc9e0);
        }
        else {
          if (DAT_00000242 != 0xfe) {
            return 0x45;
          }
          uVar10 = 0;
          for (uVar9 = 0; uVar9 = uVar9 & 0xffff, uVar9 < 8; uVar9 = uVar9 + 1) {
            uVar8 = (*(code *)(&PTR_FUN_400df440)[uVar9])(3);
            FUN_40010a35(uVar8);
            uVar10 = (uVar10 & 0xffff) + 2;
          }
          uVar10 = uVar10 & 0xffff;
          FUN_40010a35(uVar10);
          uVar10 = uVar10 + 2;
          iVar13 = 0;
          uVar9 = 0;
          while (uVar16 = uVar9 & 0xffff, uVar16 < 8) {
            iVar12 = (*(code *)(&PTR_FUN_400df440)[uVar9 & 0xffff])(3);
            iVar13 = iVar13 + iVar12;
            uVar9 = uVar16 + 1;
          }
          iVar13 = (uVar10 & 0xffff) + iVar13;
          FUN_400109f9(iVar13);
          uVar10 = (uVar10 & 0xffff) + 4;
          for (uVar9 = 0; (uVar9 & 0xffff) < uVar10; uVar9 = (uVar9 & 0xffff) + 1) {
            (&DAT_889fc9e0)[uVar9 & 0xffff] = 0;
          }
          uVar9 = FUN_400109c1();
          if (uVar9 == 0) {
            uVar9 = iVar13 + 4;
          }
          else if ((uVar9 & 1) != 0) {
            uVar9 = uVar9 - 1;
          }
          if (uVar10 < uVar9) {
            uVar9 = uVar10;
          }
          uVar9 = FUN_4001294e(uVar9,0x1fc9e0);
          if ((uVar9 & 0xffff) != 0) {
            return uVar9 & 0xffff;
          }
          puVar4 = (undefined4 *)(iStackY_80 - iStackY_84);
          if (puVar4 != (undefined4 *)0x0) {
            uVar9 = 0;
            while (uVar9 = uVar9 & 0xffff, uVar9 < 8) {
              if ((uVar9 == 0) && (uVar10 = (*(code *)PTR_FUN_400df440)(1), (uVar10 & 1) != 0)) {
                drv_cli_io_helper(0);
                puVar4 = puStackY_4c;
                uVar9 = uStackY_50;
              }
              ppuVar5 = &PTR_FUN_400df440 + (uVar9 & 0xffff);
              iVar13 = (*(code *)*ppuVar5)(0);
              iVar12 = (*(code *)*ppuVar5)(3);
              iVar13 = iVar13 + iVar12;
              iVar12 = (*(code *)*ppuVar5)(1);
              if ((uint)(iVar13 - iVar12) < puVar4) {
                iVar12 = (*(code *)*ppuVar5)(1);
                puVar4 = (undefined4 *)(iVar13 - iVar12);
              }
              (*(code *)*ppuVar5)(1);
              uVar9 = FUN_400211c2((uint)puVar4 & 0xffff);
              if ((uVar9 & 0xffff) != 0) {
                return uVar9 & 0xffff;
              }
              uVar9 = uStackY_50 - iStackY_58;
              if (uVar9 == 0) break;
              iVar13 = (*(code *)*puStackY_4c)(1);
              iVar12 = (*(code *)*puStackY_4c)(0);
              if ((uint)(iVar13 - iVar12) < uVar9) {
                iVar13 = (*(code *)*puStackY_4c)(1);
                iVar12 = (*(code *)*puStackY_4c)(0);
                uVar9 = iVar13 - iVar12;
              }
              (*(code *)*puStackY_4c)(0);
              uVar9 = FUN_400211c2(uVar9 & 0xffff);
              if ((uVar9 & 0xffff) != 0) {
                return uVar9 & 0xffff;
              }
              puVar4 = (undefined4 *)(uStackY_50 - iStackY_58);
              if (puVar4 == (undefined4 *)0x0) break;
              uVar9 = iStack_34 + 1;
            }
          }
        }
      }
      uVar9 = 0;
    }
  }
  else {
    uVar9 = 0x45;
  }
  return uVar9;
}
```

#### `FUN_40021476` (`drv_atapi_pkt_21476`) — `0x40021476` (535 B)

Recepção/preparo de pacote CDB ATAPI (`drv_atapi_pkt_21476`).

<!-- source: doc_drv_and_atapi.txt | FUNC FUN_40021476 (drv_atapi_pkt_21476) @ 0x40021476 (size=535 B, req=0x40021476) -->
```c
// Callers (1): 0x400de814
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

short FUN_40021476(void)

{
  char cVar1;
  char cVar2;
  short sVar3;
  uint uVar4;
  uint uVar5;
  byte bVar6;
  uint uStackY_24;
  uint uStackY_20;
  
  uVar4 = FUN_400109c1();
  uVar5 = FUN_400109c1();
  bVar6 = DAT_0000023d & 7;
  if (bVar6 == 3) {
    if ((DAT_00000dd2 != '\x10') || (DAT_00000dd1 != '\0')) {
      return 0x47;
    }
    if (uVar4 != 0) {
      return 0x45;
    }
    FUN_40010950(4);
    if (4 < uVar5) {
      uVar5 = 4;
    }
    uVar4 = 0x1fc9e0;
    uStackY_20 = uVar5;
  }
  else {
    if (bVar6 != 2) {
      if (bVar6 == 1) {
        if (0xe830 < uVar5 + uVar4) {
          return 0x45;
        }
        if (uVar5 == 0) {
          return 0;
        }
        if (0x1010 < uVar5) {
          return 0x45;
        }
        FUN_40010950(uVar5 & 0xffff);
        uVar4 = 0x1fc9e0;
        uStackY_20 = uVar5;
        goto LAB_4002167e;
      }
      if (bVar6 != 6) {
        return 0x45;
      }
      if ((_DAT_00000306 & 2) == 0) {
        return 0x45;
      }
      if ((DAT_00000dd4 & 0x80) != 0) {
        return 0x45;
      }
      if (DAT_00000dd2 != '\b') {
        return 0x45;
      }
      if ((DAT_00001364 & 0x20) == 0) {
        return 0x45;
      }
      if ((_DAT_00000de2 & 0xee) != 0) {
        return 0x45;
      }
      if (DAT_00001365 == '\0') {
        cVar1 = FUN_400a73cb(DAT_00000308,(int)(uint)DAT_00000309 >> 1 & 1,uVar4,uVar5);
      }
      else {
        cVar2 = FUN_400a73cb(DAT_00000308,(int)(uint)DAT_00000309 >> 1 & 1,uVar4,uVar5);
        if (cVar2 == '\0') {
          cVar2 = FUN_400a73cb(DAT_00000308,(int)(uint)DAT_00000309 >> 1 & 1,uStackY_24,uStackY_20);
          cVar1 = '\0';
          if (cVar2 == '\0') goto LAB_400215fb;
        }
        cVar1 = '\x01';
      }
LAB_400215fb:
      if (((cVar1 == '\0') ||
          (((uStackY_20 != 0x17000 && (uStackY_20 != 0x8000)) && (uStackY_20 != 0x7000)))) ||
         (((uStackY_24 != 0 && (uStackY_24 != 0x8000)) && (uStackY_24 != 0x10000)))) {
        return 0x45;
      }
      FUN_400dd585();
      FUN_400956cc(0);
      FUN_400dd5af();
      DAT_00000248 = 0x33;
      DAT_00000249 = 0;
      sVar3 = FUN_4001418b();
      if (sVar3 != 0) {
        return sVar3;
      }
      uVar4 = uStackY_24;
      sVar3 = FUN_40014271(1);
      if (sVar3 != 0) {
        return sVar3;
      }
      if ((uStackY_20 == uStackY_24) || (uStackY_20 == 0x7000)) {
        FUN_40021c01(1);
      }
      goto LAB_4002167e;
    }
    if ((DAT_00000e41 & 0x80) == 0) {
      return 0x45;
    }
    sVar3 = FUN_4001a1bd();
    if ((sVar3 != 0) && (((_DAT_00000306 & 1) != 0 || ((_DAT_0000043c & 0x40) != 0)))) {
      DAT_00000390 = 0x11;
      return 0x57;
    }
    if ((uVar4 & 3) != 0) {
      return 0x45;
    }
    if ((uVar5 & 3) != 0) {
      return 0x45;
    }
    uStackY_20 = uVar5;
    if (0x1d0b00 < uVar5 + uVar4) {
      return 0x45;
    }
  }
  if (uStackY_20 == 0) {
    return 0;
  }
LAB_4002167e:
  sVar3 = FUN_4001294e(uStackY_20,uVar4);
  return sVar3;
}
```

#### `FUN_4002168d` (`drv_atapi_irq_2168d`) — `0x4002168d` (646 B)

Rotina de interrupção/conclusão ATAPI (`drv_atapi_irq_2168d`).

<!-- source: doc_drv_and_atapi.txt | FUNC FUN_4002168d (drv_atapi_irq_2168d) @ 0x4002168d (size=646 B, req=0x4002168d) -->
```c
// Callers (1): 0x400de81c
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

uint FUN_4002168d(void)

{
  undefined4 *puVar1;
  char cVar2;
  ushort uVar3;
  undefined4 *puVar4;
  uint uVar5;
  int iVar6;
  byte bVar7;
  int iStack_60;
  int iStack_58;
  int iStack_54;
  
  _DAT_000003d0 = _DAT_000003d0 & 0xfffe;
  puVar4 = (undefined4 *)FUN_400109c1();
  uVar5 = FUN_400109c1();
  bVar7 = DAT_0000023d & 7;
  if (bVar7 == 7) {
    if ((_DAT_000003d0 & 2) == 0) {
      _DAT_00000428 = (undefined4 *)0x0;
      _DAT_000003d0 = _DAT_000003d0 | 2;
    }
    if ((DAT_00000dd2 != '\x10') || (DAT_00000dd1 != '\0')) {
      uVar5 = 0x47;
      goto LAB_40021900;
    }
    if (((((uint)puVar4 & 3) == 0) && ((uVar5 & 3) == 0)) && (_DAT_00000428 == puVar4)) {
      if (uVar5 == 0) {
        return 0;
      }
      uVar5 = FUN_4001294e(uVar5,puVar4);
      if ((uVar5 & 0xffff) != 0) goto LAB_40021900;
      _DAT_00000428 = (undefined4 *)((int)_DAT_00000428 + iStack_58);
      if (_DAT_00000428 < (undefined4 *)0x30) {
        uVar5 = 0;
        goto LAB_40021900;
      }
      iVar6 = FUN_400109a0();
      puVar4 = (undefined4 *)(iVar6 + 0x30);
      if ((undefined4 *)(iStack_60 + iStack_58) <= puVar4) {
        if (_DAT_00000428 == puVar4) {
          cVar2 = '\0';
          for (uVar3 = 0; uVar3 < 0x30; uVar3 = uVar3 + 1) {
            cVar2 = cVar2 + (&DAT_88800000)[uVar3];
          }
          if (cVar2 == '\0') {
            FUN_40010950(0x30);
            for (puVar1 = (undefined4 *)&DAT_88800000; puVar1 + 0x1de00000 < puVar4;
                puVar1 = puVar1 + 1) {
              *puVar1 = puVar1[0xc];
            }
            uVar5 = FUN_40021a59(0);
          }
          else {
            DAT_00000390 = 0x1f;
            uVar5 = 0x46;
          }
        }
        goto LAB_40021900;
      }
    }
  }
  else {
    if (bVar7 == 2) {
      if ((((DAT_00000e41 & 0x80) == 0) ||
          (_DAT_000003d0 = _DAT_000003d0 & 0xfffd, ((uint)puVar4 & 3) != 0)) ||
         (((uVar5 & 3) != 0 || ((undefined1 *)0x1d0b00 < (undefined1 *)(uVar5 + (int)puVar4))))) {
        return 0x45;
      }
      if (uVar5 == 0) {
        return 0;
      }
      uVar5 = FUN_4001294e(uVar5,puVar4);
      goto LAB_40021900;
    }
    if (bVar7 == 6) {
      _DAT_000003d0 = _DAT_000003d0 & 0xfffd;
      if (((uVar5 != 0x17000) && (uVar5 != 0xffff8000)) && (uVar5 != 0x7000)) {
        return 0x45;
      }
      if (((puVar4 != (undefined4 *)0x0) && (puVar4 != (undefined4 *)&DAT_00008000)) &&
         (puVar4 != (undefined4 *)0x10000)) {
        return 0x45;
      }
      if ((DAT_00000dd0 & 0x80) == 0) {
        return 0x45;
      }
      if (DAT_00000dd2 != '\x10') {
        return 0x45;
      }
      cVar2 = FUN_4008ba47();
      if (cVar2 != '\0') {
        return 0x45;
      }
      uVar5 = FUN_4001294e(uVar5,puVar4);
      if ((((uVar5 & 0xffff) == 0) && ((iStack_58 == iStack_54 || (iStack_58 == 0x7000)))) &&
         (uVar5 = FUN_40021c01(0), (uVar5 & 0xffff) == 0)) {
        DAT_00000248 = 0x33;
        DAT_00000249 = 1;
        uVar5 = FUN_4001418b();
        if (((uVar5 & 0xffff) == 0) && (uVar5 = FUN_40014271(1), (uVar5 & 0xffff) == 0)) {
          uVar3 = Ram889efbec;
          if (((uVar3 & 2) == 0) ||
             ((bVar7 = DAT_889effcc, (bVar7 & 0x20) == 0 ||
              (uVar3 = Ram889efbf2, (uVar3 & 0xee) != 0)))) {
            uVar5 = 0x10;
            FUN_400dd585();
            FUN_400956c2();
          }
          else {
            FUN_400dd585();
            FUN_400956cc(1);
            FUN_4008bb2a(1);
          }
          FUN_400dd5af();
        }
      }
      goto LAB_40021900;
    }
  }
  uVar5 = 0x45;
LAB_40021900:
  if ((uVar5 & 0xffff) != 0) {
    _DAT_000003d0 = _DAT_000003d0 & 0xfffd;
  }
  return uVar5 & 0xffff;
}
```

#### `FUN_400d92b2` (`drv_ref_0x400d92b2`) — `0x400d92b2` (82 B)

Referência cruzada ao subsistema Sense / erro em `0x400d92b2` (`drv_ref_0x400d92b2`).

<!-- source: doc_drv_and_atapi.txt | FUNC FUN_400d92b2 (drv_ref_0x400d92b2) @ 0x400d92b2 (size=82 B, req=0x400d92b2) -->
```c
// Callers (1): 0x40058912
// Referenced strings/literals:
//   0x400d8d31: '\x83\x92Í!'
//   0x400d9009: '>R'
short FUN_400d92b2(uint param_1)

{
  ushort uVar1;
  short sVar2;
  
  FUN_40010950(8);
  sVar2 = FUN_400d8d31(param_1 & 0xffff);
  if ((sVar2 == 0) && ((DAT_0000028b & 0xf0) == 0x70)) {
    DAT_0000138e = 0xf6;
    sVar2 = 0x3d;
  }
  else {
    uVar1 = Ram88000004;
    if ((uVar1 & 1) != 0) {
      sVar2 = FUN_400d9009();
    }
  }
  return sVar2;
}
```

#### Fatia da Tabela de Sense (`0x400DE3E4..0x400DE444` — `doc_drv_and_atapi.txt`)

Dump hexadecimal/textual da fatia de tabela em `0x400DE3E4..0x400DE444`.

<!-- source: doc_drv_and_atapi.txt | SENSE TABLE SLICE 0x400DE3E4..0x400DE444 -->
```text
  0x400de3e4: 03 00 04 03 01 04 03 02 04 03 03 04 03 04 04 08
  0x400de3f4: 00 04 08 03 04 09 00 04 09 01 04 09 02 04 09 03
  0x400de404: 04 09 04 04 09 90 04 10 00 04 3e 02 04 40 00 04
  0x400de414: 44 00 04 53 00 04 93 00 05 1a 00 05 20 00 05 21
  0x400de424: 00 05 21 02 05 21 03 05 24 00 05 26 00 05 2c 00
  0x400de434: 05 30 01 05 30 02 05 30 04 05 30 05 05 30 06 05
```

#### Prova Adicional em `/DRV_decomp.bin`: Disassembly `0x4006ef00..0x4006ef35`, Xrefs `0x400089dc` e Strings em `0x40130ee0..0x40130f2f` (`verified_extra_facts.txt` §6)

```text
-- DISASM 0x4006ef00..0x4006ef35 --
  0x4006ef00: [00] <undef>
  0x4006ef01: [d9] <undef>
  0x4006ef02: [e3] <undef>
  0x4006ef03: [9e] <undef>
  0x4006ef04: [88] <undef>
  0x4006ef05: [04] <undef>
  0x4006ef06: [ca 09                ] BRA 0x4006ef0f
  0x4006ef08: [fe 01 d9 e3 9e 88 04 ] BCLR 0x4,(0x889ee3d9)
  0x4006ef0f: [f1 d8                ] MOV A2,D0
  0x4006ef11: [1c                   ] EXTHU D0
  0x4006ef12: [df f0 18             ] RET [A3,A2,D3,D2],0x18
  0x4006ef15: [83                   ] MOV D0,D3
  0x4006ef16: [2c ff 00             ] MOV 0xff,D0
  0x4006ef19: [f8 92 29             ] MOVBU D0,(0x29 ,SP)
  0x4006ef1c: [00                   ] CLR D0
  0x4006ef1d: [f8 92 24             ] MOVBU D0,(0x24 ,SP)
  0x4006ef20: [f8 93 32             ] MOVHU D0,(0x32 ,SP)
  0x4006ef23: [f8 92 26             ] MOVBU D0,(0x26 ,SP)
  0x4006ef26: [f8 92 2b             ] MOVBU D0,(0x2b ,SP)
  0x4006ef29: [38 dc 0d             ] MOVHU (0xddc),D0
  0x4006ef2c: [f8 ec 08             ] BTST 0x8,D0
  0x4006ef2f: [c9 0e                ] BNE 0x4006ef3d
  0x4006ef31: [38 dc 0d             ] MOVHU (0xddc),D0
  0x4006ef34: [fa ec 00 01          ] BTST 0x100,D0
-- XREFS TO 0x400089dc (SenseTable) --
  40004884 -> 400089dc (DATA)
  40003943 -> 400089dc (DATA)
  4000394e -> 400089dc (DATA)
  4000393d -> 400089dc (DATA)
  4000487e -> 400089dc (DATA)
-- DRV_decomp 0x40130ee0..0x40130f2f --
  0x40130ee0: 72 65 74 20 20 20 3d 20 25 77 68 0a 00 09 09 52 |ret   = %wh....R|
  0x40130ef0: 5a 6f 6e 65 4e 6f 20 0a 00 09 09 20 2d 3e 70 62 |ZoneNo .... ->pb|
  0x40130f00: 61 20 20 3d 20 30 78 25 6c 68 0a 00 09 09 20 2d |a  = 0x%lh.... -|
  0x40130f10: 3e 73 69 7a 65 20 3d 20 30 78 25 6c 68 0a 00 09 |>size = 0x%lh...|
  0x40130f20: 09 20 2d 3e 72 65 74 20 20 3d 20 30 78 25 62 68 |. ->ret  = 0x%bh|
```

#### Análise Complementar `DRV12` (`FUN_40005772`, `Err05mWTable` Shift-JIS, `SenseTable` `0x40008900` e busca LE16 `a7 83` em `0x4006f5d6..0x4006f5ee` — `res_followup_drv12.txt`)

```text
=== DRV12A: FUN_40005772 & CALLERS ===

--- DISASM 0x40005772..0x40005850 ---
  0x40005772: 91                 MOV A0,A1
  0x40005773: 79                 MOV (A1),D2
  0x40005774: 04                 CLR D1
  0x40005775: a6                 CMP D1,D2
  0x40005776: c9 0d              BNE 0x40005783
  0x40005778: f8 09 08           MOV (0x8 ,A1),D2
  0x4000577b: a6                 CMP D1,D2
  0x4000577c: c9 07              BNE 0x40005783
  0x4000577e: f8 0d 10           MOV (0x10 ,A1),D3
  0x40005781: ca 0c              BRA 0x4000578d
  0x40005783: 79                 MOV (A1),D2
  0x40005784: a6                 CMP D1,D2
  0x40005785: c9 07              BNE 0x4000578c
  0x40005787: f8 0d 08           MOV (0x8 ,A1),D3
  0x4000578a: ca 03              BRA 0x4000578d
  0x4000578c: 7d                 MOV (A1),D3
  0x4000578d: fc c3 00 00 80 88  ADD -0x77800000,D3
  0x40005793: f1 ec              MOV D3,A0
  0x40005795: 83                 MOV D0,D3
  0x40005796: a7                 CMP D1,D3
  0x40005797: c9 0d              BNE 0x400057a4
  0x40005799: f8 0d 04           MOV (0x4 ,A1),D3
  0x4000579c: f8 01 0c           MOV (0xc ,A1),D0
  0x4000579f: f8 09 14           MOV (0x14 ,A1),D2
  0x400057a2: e3                 ADD D0,D3
  0x400057a3: eb                 ADD D2,D3
  0x400057a4: 95 00              MOV 0x0,A1
  0x400057a6: ca 17              BRA 0x400057bd
  0x400057a8: f0 60              MOVHU (A0),D0
  0x400057aa: 1d                 EXTHU D1
  0x400057ab: f8 93 00           MOVHU D0,(0x0 ,SP)
  0x400057ae: f8 b8 00           MOVBU (0x0 ,SP),D0
  0x400057b1: f8 ba 01           MOVBU (0x1 ,SP),D2
  0x400057b4: f8 c0 08           ASL 0x8,D0
  0x400057b7: e1                 ADD D0,D1
  0x400057b8: e9                 ADD D2,D1
  0x400057b9: 20 02              ADD 0x2,A0
  0x400057bb: 21 02              ADD 0x2,A1
  0x400057bd: f1 ad              CMP D3,A1
  0x400057bf: c4 e9              BCS 0x400057a8
  0x400057c1: 80 02              MOV 0x2,D0
  0x400057c3: f1 20              SUB D0,A0
  0x400057c5: f0 60              MOVHU (A0),D0
  0x400057c7: f8 93 00           MOVHU D0,(0x0 ,SP)
  0x400057ca: f8 b8 00           MOVBU (0x0 ,SP),D0
  0x400057cd: f8 c0 08           ASL 0x8,D0
  0x400057d0: f1 01              SUB D0,D1
  0x400057d2: f8 b8 01           MOVBU (0x1 ,SP),D0
  0x400057d5: f1 01              SUB D0,D1
  0x400057d7: 1d                 EXTHU D1
  0x400057d8: a5 00              CMP 0x0,D1
  0x400057da: c8 06              BEQ 0x400057e0
  0x400057dc: 80 46              MOV 0x46,D0
  0x400057de: ca 03              BRA 0x400057e1
  0x400057e0: 00                 CLR D0
  0x400057e1: de c0 0c           RETF [D3,D2],0xc
  0x400057e4: f1 e1              MOV D0,A1
  0x400057e6: f0 41              MOVBU (A1),D0
  0x400057e8: f0 70              MOVHU D0,(A0)
  0x400057ea: f0 fc              RETS
  0x400057ec: 3e                 MOV SP,A2
  0x400057ed: 52                 INC4 A2
  0x400057ee: 82                 MOV D0,D2
  0x400057ef: 48                 INC D2
  0x400057f0: 93                 MOV A0,A3
  0x400057f1: 98                 MOV A2,A0
  0x400057f2: cd f2 ff 00 00     CALL 0x400057e4,[],0x0
  0x400057f7: f8 bf 04           MOVHU (0x4 ,SP),D3
  0x400057fa: 98                 MOV A2,A0
  0x400057fb: 88                 MOV D2,D0
  0x400057fc: cd e8 ff 00 00     CALL 0x400057e4,[],0x0
  0x40005801: f8 bc 04           MOVHU (0x4 ,SP),D0
  0x40005804: f8 c0 08           ASL 0x8,D0
  0x40005807: e3                 ADD D0,D3
  0x40005808: f0 7f              MOVHU D3,(A3)
  0x4000580a: df f0 18           RET [A3,A2,D3,D2],0x18
  0x4000580d: 3f                 MOV SP,A3
  0x4000580e: 43 04              MOV A0,(0x4,SP)
  0x40005810: 23 08              ADD 0x8,A3
  0x40005812: 9c                 MOV A3,A0
  0x40005813: 83                 MOV D0,D3
  0x40005814: cd d0 ff 00 00     CALL 0x400057e4,[],0x0
  0x40005819: f8 bc 08           MOVHU (0x8 ,SP),D0
  0x4000581c: 4c                 INC D3
  0x4000581d: 9c                 MOV A3,A0
  0x4000581e: 18                 EXTH D0
  0x4000581f: f1 e2              MOV D0,A2
  0x40005821: 8c                 MOV D3,D0
  0x40005822: cd c2 ff 00 00     CALL 0x400057e4,[],0x0
  0x40005827: f8 be 08           MOVHU (0x8 ,SP),D2
  0x4000582a: 9c                 MOV A3,A0
  0x4000582b: 1a                 EXTH D2
  0x4000582c: f8 c2 08           ASL 0x8,D2
  0x4000582f: f1 6a              ADD D2,A2
  0x40005831: 8e                 MOV D3,D2
  0x40005832: 48                 INC D2
  0x40005833: 88                 MOV D2,D0
  0x40005834: cd b0 ff 00 00     CALL 0x400057e4,[],0x0
  0x40005839: f8 bf 08           MOVHU (0x8 ,SP),D3
  0x4000583c: 48                 INC D2
  0x4000583d: 9c                 MOV A3,A0
  0x4000583e: 88                 MOV D2,D0
  0x4000583f: cd a5 ff 00 00     CALL 0x400057e4,[],0x0
  0x40005844: f8 bc 08           MOVHU (0x8 ,SP),D0
  0x40005847: 5c 04              MOV (0x4,SP),A0
  0x40005849: f8 c3 10           ASL 0x10,D3
  0x4000584c: f1 5b              ADD A2,D3
  0x4000584e: f8 c0 18           ASL 0x18,D0

=== FUNC FUN_40005772 @ 0x40005772 (req 0x40005772) ===


undefined4 FUN_40005772(uint param_1)



{

  int *in_A0;

  uint uVar1;

  undefined4 uVar2;

  short sVar3;

  int iVar4;

  ushort *puVar5;

  

  sVar3 = 0;

  if ((*in_A0 == 0) && (in_A0[2] == 0)) {

    iVar4 = in_A0[4];

  }

  else if (*in_A0 == 0) {

    iVar4 = in_A0[2];

  }

  else {

    iVar4 = *in_A0;

  }

  puVar5 = (ushort *)(&DAT_88800000 + iVar4);

  if (param_1 == 0) {

    param_1 = in_A0[1] + in_A0[3] + in_A0[5];

  }

  for (uVar1 = 0; uVar1 < param_1; uVar1 = uVar1 + 2) {

    sVar3 = sVar3 + *puVar5 * 0x100 + (*puVar5 >> 8);

    puVar5 = puVar5 + 1;

  }

  if ((ushort)(sVar3 + (puVar5[-1] & 0xff) * -0x100) == puVar5[-1] >> 8) {

    uVar2 = 0;

  }

  else {

    uVar2 = 0x46;

  }

  return uVar2;

}




  CALLERS of 0x40005772: []

=== DRV12B: DECODE SHIFT-JIS STRINGS AROUND 0x400e8900..0x400e9000 ===
  0x400e8b00: 91 65 92 b2 90 ae 29 00 00 00 00 00 00 00 00 00  |.e....).........|
  0x400e8b10: 00 00 00 00 00 00 00 00 00 00 a2 83 30 2e 35 6d  |............0.5m|
  0x400e8b20: 57 94 ad 8c f5 82 aa 82 c5 82 ab 82 c8 82 a2 28  |W..............(|
  0x400e8b30: 48 46 b5 cc 91 65 92 b2 90 ae 29 00 00 00 00 00  |HF...e....).....|
  0x400e8b40: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 a3 83  |................|
  0x400e8b50: 31 2e 35 6d 57 94 ad 8c f5 82 aa 82 c5 82 ab 82  |1.5mW...........|
  0x400e8b60: c8 82 a2 28 48 46 b5 cc 91 65 92 b2 90 ae 29 00  |...(HF...e....).|
  0x400e8b70: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00  |................|
  0x400e8b80: 00 00 a4 83 e8 87 92 6c 49 74 68 20 91 9d 89 c1  |.......lIth ....|
  0x400e8b90: 28 48 46 b5 cc 8e 9e 29 00 00 00 00 00 00 00 00  |(HF....)........|
  0x400e8ba0: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00  |................|
  0x400e8bb0: 00 00 00 00 00 00 a5 83 bd db b0 cc df 8c f8 97  |................|
  0x400e8bc0: a6 83 c5 8c b8 8f ad 28 48 46 b5 cc 8e 9e 29 00  |.......(HF....).|
  0x400e8bd0: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00  |................|
  0x400e8be0: 00 00 00 00 00 00 00 00 00 00 a6 83 48 46 90 d8  |............HF..|
  0x400e8bf0: 82 ea 8d 9e 82 dd 93 64 97 ac 8c b8 8f ad 00 00  |.......d........|
  0x400e8c00: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00  |................|
  0x400e8c10: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 a7 83  |................|
  0x400e8c20: 30 2e 35 6d 57 94 ad 8c f5 82 aa 82 c5 82 ab 82  |0.5mW...........|
  0x400e8c30: c8 82 a2 28 48 46 b5 dd 96 a7 92 b2 90 ae 29 00  |...(HF........).|
  0x400e8c40: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00  |................|
  0x400e8c50: 00 00 a8 83 31 2e 35 6d 57 94 ad 8c f5 82 aa 82  |....1.5mW.......|
  0x400e8c60: c5 82 ab 82 c8 82 a2 28 48 46 b5 dd 96 a7 92 b2  |.......(HF......|
  0x400e8c70: 90 ae 29 00 00 00 00 00 00 00 00 00 00 00 00 00  |..).............|
  0x400e8c80: 00 00 00 00 00 00 b0 83 35 2e 30 6d 57 94 ad 8c  |........5.0mW...|
  0x400e8c90: f5 82 aa 82 c5 82 ab 82 c8 82 a2 28 48 46 b5 cc  |...........(HF..|
  0x400e8ca0: 91 65 92 b2 90 ae 29 00 00 00 00 00 00 00 00 00  |.e....).........|
  0x400e8cb0: 00 00 00 00 00 00 00 00 00 00 b1 83 31 30 2e 30  |............10.0|
  0x400e8cc0: 6d 57 94 ad 8c f5 82 aa 82 c5 82 ab 82 c8 82 a2  |mW..............|
  0x400e8cd0: 28 48 46 b5 cc 91 65 92 b2 90 ae 29 00 00 00 00  |(HF...e....)....|
  0x400e8ce0: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 b2 83  |................|
  0x400e8cf0: bd db b0 cc df 8c f8 97 a6 83 c5 8c b8 8f ad 28  |...............(|

  CP932 strings around 0x400e8a00..0x400e9000:
    0x400e8a16 (hex[:8]=918391aa92e88c8b): '巣測定結果の不備により演算ができない'
    0x400e8a4a (hex[:8]=9283544b4f4e82cc): '茶TKONの為、実行できない'
    0x400e8a7e (hex[:8]=93834456442d5241): '塔DVD-RAMでRTTADJを実行しようとした'
    0x400e8ab2 (hex[:8]=a083302e356d5794): '\uf8f0�0.5mW発光ができない(HFｵﾝ粗調整)'
    0x400e8ae6 (hex[:8]=a183312e356d5794): '｡�1.5mW発光ができない(HFｵﾝ粗調整)'
    0x400e8b1a (hex[:8]=a283302e356d5794): '｢�0.5mW発光ができない(HFｵﾌ粗調整)'
    0x400e8b4e (hex[:8]=a383312e356d5794): '｣�1.5mW発光ができない(HFｵﾌ粗調整)'
    0x400e8b82 (hex[:8]=a483e887926c4974): '､�閾値Ith 増加(HFｵﾌ時)'
    0x400e8bb6 (hex[:8]=a583bddbb0ccdf8c): '･�ｽﾛｰﾌﾟ効率η減少(HFｵﾌ時)'
    0x400e8bea (hex[:8]=a683484690d882ea): 'ｦォF切れ込み電流減少'
    0x400e8c1e (hex[:8]=a783302e356d5794): 'ｧ�0.5mW発光ができない(HFｵﾝ密調整)'
    0x400e8c52 (hex[:8]=a883312e356d5794): 'ｨ�1.5mW発光ができない(HFｵﾝ密調整)'
    0x400e8c86 (hex[:8]=b083352e306d5794): 'ｰ�5.0mW発光ができない(HFｵﾌ粗調整)'
    0x400e8cba (hex[:8]=b18331302e306d57): 'ｱ�10.0mW発光ができない(HFｵﾌ粗調整)'
    0x400e8cee (hex[:8]=b283bddbb0ccdf8c): 'ｲ�ｽﾛｰﾌﾟ効率η減少(HFｵﾌ時)'
    0x400e8d22 (hex[:8]=b383352e306d5794): 'ｳ�5.0mW発光ができない(HFｵﾌ密調整)'
    0x400e8d56 (hex[:8]=b48331302e306d57): 'ｴ�10.0mW発光ができない(HFｵﾌ密調整)'
    0x400e8d8a (hex[:8]=b683352e306d5794): 'ｶ�5.0mW発光ができない(HFｵﾌ粗調整)'
    0x400e8dbe (hex[:8]=b78331302e306d57): 'ｷ�10.0mW発光ができない(HFｵﾌ粗調整)'
    0x400e8df2 (hex[:8]=b883bddbb0ccdf8c): 'ｸ�ｽﾛｰﾌﾟ効率η減少(HFｵﾌ時)'
    0x400e8e26 (hex[:8]=b983352e306d5794): 'ｹ�5.0mW発光ができない(HFｵﾌ密調整)'
    0x400e8e5a (hex[:8]=ba8331302e306d57): 'ｺ�10.0mW発光ができない(HFｵﾌ密調整)'
    0x400e8e8e (hex[:8]=ff834c445f4f4653): '\uf8f3キD_OFSCANｴﾗｰ(引数おかしい)'
    0x400e8ec2 (hex[:8]=0a0d2d2d2d204375): '\n\r--- Cut ---\n\r'
    0x400e8ed2 (hex[:8]=202043616e63656c): '  Cancel [drive task busy]\n'
    0x400e8eee (hex[:8]=202043616e63656c): '  Cancel [serial command busy]\n'
    0x400e8f0e (hex[:8]=2020414444524553): '  ADDRESS    00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F  0123456789ABCDEF\n'
    0x400e8f5e (hex[:8]=20202d2d2d2d2d2d): '  --------   -----------------------------------------------  ----------------\n'
    0x400e8fae (hex[:8]=2020414444524553): '  ADDRESS    00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F\n'
    0x400e8fec (hex[:8]=20202d2d2d2d2d2d): '  --------   -------'

=== DRV12C: SENSE TABLE (0x40008900..0x40008b00) & PTRS ===
  0x40008980: 20 20 20 20 20 20 00 00 00 00 0f 00 00 00 02 00  |      ..........|
  0x40008990: 02 00 00 06 00 00 00 00 00 00 00 00 00 00 00 00  |................|
  0x400089a0: 00 00 00 00 00 07 00 07 00 03 00 78 00 78 00 f0  |...........x.x..|
  0x400089b0: 00 78 00 00 00 00 00 00 00 00 00 00 00 00 00 00  |.x..............|
  0x400089c0: 00 00 00 00 00 00 00 00 00 78 00 00 00 00 00 00  |.........x......|
  0x400089d0: 00 00 00 00 00 00 00 00 00 07 00 00 00 00 00 00  |................|
  0x400089e0: 00 11 00 00 12 00 00 13 00 00 14 00 04 04 01 11  |................|
  0x400089f0: 80 01 18 05 01 5b 80 01 5d 03 01 73 06 02 04 01  |.....[..]..s....|
  0x40008a00: 02 04 04 02 04 07 02 04 08 02 04 80 02 30 00 02  |.............0..|
  0x40008a10: 30 02 02 3a 00 02 3a 01 02 3a 02 02 53 02 02 57  |0..:..:..:..S..W|
  0x40008a20: 00 03 02 00 03 0c 00 03 0c 02 03 10 00 03 11 00  |................|
  0x40008a30: 03 11 05 03 14 00 03 16 00 03 31 00 03 31 01 03  |..........1..1..|
  PTRS into SenseTable [0x40008980..0x40008b20]: count=1 first=[('0x40004880', '0x400089dc', 'FUN_40004852')]
  PTRS into Err05mWTable [0x400e8a00..0x400e9000]: count=4 first=[('0x4002814e', '0x400e8ed2', 'drv_serial_cli_dispatch'), ('0x4002816e', '0x400e8eee', 'drv_serial_cli_dispatch'), ('0x400282c2', '0x400e8f0e', 'FUN_4002829c'), ('0x400282e8', '0x400e8fec', 'FUN_4002829c')]

=== DRV12D: LE16(0x83a7) = a7 83 SEARCH & REFS TO 0x400de404 / 0x40129307 ===
  0x83a7 LE/bytes: count=3 first=[('0x4006f5e2', 'FUN_4006ef15'), ('0x400e8c1e', '?'), ('0x401385ec', '?')]
    LE hit 0x4006f5e2 inside FUN_4006ef15 @ 0x4006ef15

--- DISASM 0x4006f5d6..0x4006f5ee ---
  0x4006f5da: 46 04              MOV D1,(0x4,SP)
  0x4006f5dc: 43 08              MOV A0,(0x8,SP)
  0x4006f5de: 42 0c              MOV D0,(0xc,SP)
  0x4006f5e0: 00                 CLR D0
  0x4006f5e1: dd a7 83 fb ff b0 14 CALL 0x40027988,[D2,A2,A3],0x14
  0x4006f5e8: f1 dc              MOV A3,D0
  0x4006f5ea: 1c                 EXTHU D0
  0x4006f5eb: 04                 CLR D1
  0x4006f5ec: fa c3 e8 03        ADD 0x3e8,D3
  0x400de404 inside ?
  0x400de3e4: 03 00 04 03 01 04 03 02 04 03 03 04 03 04 04 08  |................|
  0x400de3f4: 00 04 08 03 04 09 00 04 09 01 04 09 02 04 09 03  |................|
  0x400de404: 04 09 04 04 09 90 04 10 00 04 3e 02 04 40 00 04  |..........>..@..|
  0x400de414: 44 00 04 53 00 04 93 00 05 1a 00 05 20 00 05 21  |D..S........ ..!|
  0x400de424: 00 05 21 02 05 21 03 05 24 00 05 26 00 05 2c 00  |..!..!..$..&..,.|
  0x400de434: 05 30 01 05 30 02 05 30 04 05 30 05 05 30 06 05  |.0..0..0..0..0..|
  0x40129307 inside ?
  0x401292e7: ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff  |................|
  0x401292f7: ff ff ff ff ff ff ff ff ff ff ff 07 3f 05 04 17  |............?...|
  0x40129307: 04 09 04 1b 04 09 04 1b 04 0d 04 1f 04 05 04 17  |................|
  0x40129317: 04 11 04 22 04 13 04 24 04 15 04 26 04 09 ff 28  |..."...$...&...(|
  0x40129327: 04 2e 04 29 04 2f 04 2a 04 30 04 2b 04 31 04 28  |...)./.*.0.+.1.(|
  0x40129337: 04 2e 04 2c 04 32 04 2d 04 33 04 b8 04 b9 04 0b  |...,.2.-.3......|

=== DRV12E: DECOMPILE CALLERS OF 0x40005772 & PTR TARGET FUNCTIONS ===
  CALLERS of 0x40005772: []
```

---

## Parte V — Catálogo de Armadilhas do Decompilador Ghidra no MN103S

Durante a auditoria forense dos 5 binários `mn10300:LE:32:default`, foram comprovadas **7 classes de distorções sistemáticas** no decompilador do Ghidra. Toda interpretação de pseudocódigo C neste repositório deve aplicar estas salvaguardas e conferir o disassembly subjacente nos pontos críticos:

| # | Categoria da Armadilha | Evidência Verificada (Programa & Endereço) | Sintoma no Pseudocódigo C | Disassembly / Fato Real Subjacente | Regra de Leitura Segura |
|---|---|---|---|---|---|
| 1 | **Ausência de seeds / fluxo órfão em `.bin` crus** | `/eh55_frm_PROG061_BOOT.bin` (5 funcs), `/EH55_MAIN_3410PQ.bin` (33 funcs), `/EH55_DRV_S211.bin` (15 funcs), `FUN_574dd010` antes de `createFunction` | Funções críticas (`FUN_5040ea30`, `FUN_574dd010`) ausentes do auto-analysis até criação manual; `in_D2`/`in_R4`/`in_A3` não resolvidos. | O carregador binário cru não possui tabela de símbolos ELF; partes de `.rodata`/tabelas de salto não disparam descoberta automática de código. | Criar funções explicitamente em modo read-only em análise e validar ponteiros em tabelas de despacho (`0x400DF55C`, jump tables). |
| 2 | **`BCLR` / `BSET` com máscara/ponteiro mal tipado (`0xff` ou `(A2)`)** | `/EH55_MAIN_3410PQ.bin`: `0x5771ac87` em `FUN_5771abfb` (`FPC_TestModeControl`) | Linha degenerada `*param_3 = *param_3;` sem efeito aparente em C. | Instrução real `0x5771ac87`: `BCLR 0xff,(0x0,A2)` (limpa todos os 8 bits do byte apontado por `A2`, equivalente a escrita `0x00`). | Sempre confirmar no disassembly instruções `BCLR`/`BSET` em registradores ou ponteiros indiretos antes de inferir no-op. |
| 3 | **Remoção falsa de bloco (`unreachable block`) + `mn_todo()`** | `/EH55_MAIN_3410PQ.bin`: `0x574dd1fc..0x574dd208` em `FUN_574dd15b` | Comentário `WARNING: Removing unreachable block (ram,0x574dd208)`, chamada `mn_todo()`, exibindo apenas `200GB` (`0x5d21dba`) e ocultando `400GB`. | No disassembly existe ramo vivo testando retorno de `func_0x574a2e60`: quando a capacidade corresponde, passa `0xba43b74` e a string `"[RTSD] M7920(HDD 400GB) \n"` (`0x57a5e957`). | Toda vez que aparecer `Removing unreachable block`, `mn_todo()` ou parâmetro faltante após chamada, inspecionar a convenção de retorno na pilha em assembly. |
| 4 | **Instruções sobrepostas (1 byte antes do rótulo de função)** | `/eh55_frm_PROG061_BOOT.bin`: `0x5040ea2f` (`f8`) / `0x5040ea30` (`07 08 84`) em `FUN_5040ea30` (`FMU_list_sum`) | `WARNING: Instruction at (ram,0x5040ea32) overlaps instruction at (ram,0x5040ea30)`; `while (uRam00008408 = (undefined2)param_2, ...)` na condição do loop. | Em `0x5040ea2f` há o byte `f8` (`?? F8h` quando desmontado em falso alinhamento após `0x5040ea30: MOVHU D1,(0x8408)`), enquanto `0x5040ea52` executa `BRA 0x5040ea2f` (reentrando 1 byte antes do rótulo de função). | Ao ver `overlaps instruction at`, desmontar a partir de `entry - 4` byte a byte e traçar o destino exato de cada branch. |
| 5 | **Podagem (Dead-Code / Unreachable Block) em escritas MMIO / Flash CFI** | `/BOOT.BIN`: `FUN_440236ed` (size=205 B, 18 warnings `Removing unreachable block`) e `0x4400133e` (`crt0`) | `FUN_440236ed` colapsa para apenas `_DAT_84200554 = 0x55; _DAT_84200aaa = 0x90; _DAT_84200000 = 0xf0; return 2;`, removendo 18 blocos de protocolo NOR Flash em `0x44023721..0x440237a4`. | O disassembly preserva integralmente os loops de leitura/escrita e sequências de desbloqueio/identificação em endereço fixo de barramento `0x8420xxxx` (`volatile`). | Marcar regiões MMIO/NOR Flash (`0x10xxxxxx`, `0x84xxxxxx`, `0x88xxxxxx`) como `volatile` na análise e conferir o disassembly de drivers Flash/IDE/SD. |
| 6 | **Indexação negativa de frame (`*(in_A3 + -N)` / `puVar1[-N]`) e registradores globais não preservados (`in_A3`, `in_R5`, `in_R7`, `in_D2`, `in_LAR`)** | `PROG061` (`FUN_5040ea30`, `FUN_50404f12` `in_LAR + -4`), `MAIN` (`FUN_574f0590`, `FUN_574dd0fd`) | Variáveis locais aparecem como `*(uint *)(in_A3 + -4)` ou `(*(code *)(in_LAR + -4))()` tratando loop de hardware `LAR` / frame pointer `A3` como ponteiro externo. | O MN103S usa `A3` como frame pointer (`MOVM` / `ADD -imm,SP`) e `LAR`/`Lcc` para loops de hardware zero-overhead. | Ler `in_A3 - k` como variável local de frame e `(*(code *)(in_LAR + -4))()` como reiteração de hardware loop (`SETLB`/`Lcc`). |
| 7 | **Desalinhamento de endereços e strings entre revisões + encodings Shift-JIS / falsos positivos em bytes** | `/DRV_decomp.bin` vs `/EH55_DRV_S211.bin` (`0x40130efa` vs `0x4013420a`); `DRV_decomp` `0x4006ef08` (`BCLR 0x4,(0x889ee3d9)` = 7 B) & `0x4006f5e1` (`dd a7 83 fb ff b0 14`) | Importar símbolo de uma revisão para outra aponta `0x40130efa` no meio de `"a  = 0x%lh\n"` em `DRV_decomp`; busca por `LE16(0x83a7)` (`a7 83`) dá match falso em `0x4006f5e2` dentro de função de código. | Em `EH55_DRV_S211.bin`, `0x40130efa` é código ativo (`PASS` `MATS`/`HITA` -> `BSET 0x1,(0xd78)`); em `DRV_decomp.bin`, o equivalente é `0x4013420a` (`DAT_00000cd8 |= 1`). Já em `0x4006f5e1`, os bytes `a7 83` estão dentro de `CALL 0x40027988,[D2,A2,A3],0x14` (`dd a7 83 ...`). Strings de erro mecha em `0x400e8a16` usam **Shift-JIS**. | Sempre confirmar SHA-256 do binário alvo e validar `xrefs`/bytes em disassembly antes de transpor endereços entre dumps `DRV`/`S211`/`MAIN`. |

---

## Verificação de Cobertura

Este documento foi gerado automaticamente por script determinístico com validação pós-geração garantindo **zero funções omitidas**, **zero blocos de tabela/disassembly omitidos**, **cercas de código balanceadas** e **preservação integral das linhas de C e assembly verificadas no projeto read-only `Panadiga_MN103S`**.
