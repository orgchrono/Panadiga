> 🌐 **Idioma / Language:** **Português (Brasil)** · [English Version (`en-US`)](../en-US/RESEARCH_VALIDATION_GATES_SATA_STAGE2_AND_8MB_BGO_RETARGET.md) · [Índice de Documentação](../INDEX.md)

# Pesquisa Técnica: Portões de Validação `0..4`, Investigação (~10h), Retarget `MAIN3060NP` BGO `8 MiB` & Hardening SSOT/RAG

> `[PROVADO EM HARDWARE REAL]` · `[EVD: EV-20261122-019..EV-20261123-029 + Mn103Vm + P7501 COM10]`

> **Nota de Nomenclatura Canônica (Deprecada $\rightarrow$ Atual — Legado Removido):** Os apelidos históricos de campanha deste dossiê foram substituídos no código, firmware, SSOT e CLI/MCP (aliases legados removidos): **`Portão 4` $\rightarrow$ `NorAutoboot` / `NORFROM`** (`--nor-autoboot-capsule`, `BGO_INSPECT NORFROM`, tag `NAUT` / `0x5455414E` em `+0xFC`), **`Stage-1.5` $\rightarrow$ `DIAG_PROG_RAM`** (`0x50400000`, `digadiag_ram_tbc`) e **`Stage-2` $\rightarrow$ `HddAutoboot` / `MAIN_OS_RAM`** (`--sata-hdd-autoboot`, tag `[HDD_AUTOBOOT]`, chave `hdd_autoboot=`, `0x57000000`).

Este dossiê registra a investigação técnica completa (~10 horas de depuração sistemática sem risco ao IPL da NOR) que resolveu o boot direto de um HDD SATA atrás da ponte `StarTech IDE2SAT2` no **Panasonic DMR-EH55LB-S** (`8 MiB` NOR `S29JL064H`, `Loader ver0.61`, placa `VEP79138`), documenta os **Portões de Validação `0..4`**, detalha o **retarget binário `4 MiB -> 8 MiB` BGO** do `MAIN3060NP`, consolida as regras de **hardening anti-regressão**, estende o suporte ao **`MAIN3410PQ`** (build europeia `M6:PAL:DVD.HDD` com chaveamento `NTSC`/`PAL`) e documenta a prova do **Portão 4 (`NorAutoboot` / `Nor-FROM` `P4ZU` sem gravação na NOR)**.

---

## 1. Cronologia da Investigação (~10h) e Isolamento de Causas Raiz

O objetivo era tirar um DMR-EH55LB-S (`M6930LB`, `PAL-M`) com disco SATA (`SAMSUNG HM251JI` `250 GB` + `StarTech IDE2SAT2`) do ciclo de erro `U99` até o boot autônomo do `PiE-OS` (`MAIN`), **sem nunca executar gravação cega nos setores físicos `SA0..SA14` da Flash NOR `IC5001`**. Três camadas sobrepostas de incompatibilidade foram isoladas e eliminadas em sequência:

| Fase | Sintoma Observado em Bancada (`COM10` `P7501`) | Causa Raiz Identificada | Correção & Blindagem Aplicada |
| :--- | :--- | :--- | :--- |
| **Camada A — Reset da Ponte SATA** | `MAIN` perdia a janela de readiness da ponte atrás do `StarTech IDE2SAT2` em fluxos com reset ATA. | `Marvell 88SA8052` leva **`~32 ms`** para afirmar `BSY` e **`~555 ms`** até `DRDY`, mas o firmware stock espera apenas **`~2 ms`**. | Patch `SataBridgeResetFix` (`1000 ms`) em [`sata_bridge_fix.rs`](../../crates/panadiga-core/src/re/sata_bridge_fix.rs) (`MAIN3060NP` e `MAIN3410PQ`). |
| **Camada B — Boot em Cadeia do HDD (Stage-2)** | Portão 2 (boot SD) funcionava, mas no primeiro teste do Portão 3 o cold boot direto do HDD abortava no meio da carga de `MAIN3060NP`. | **`MAIN3060NP`** foi compilado para geometria **`4 MiB` Top-Boot** (`0x84200000..0x84400000`, `35` setores, `P1=3`, `P2=28`, `max_did=0x0A1A`), falhando sobre a **`S29JL064H` de `8 MiB`** (`0x84130000..0x84400000`, `45` setores, `P1=1`, `P2=40`, `max_did=0x0F24`) em **`src/nfd_api.c:228`** (`[nfrom] BGO driver init failed[1].`). | Retarget completo `4 MiB -> 8 MiB` em [`main_bgo_retarget/mod.rs`](../../crates/panadiga-core/src/re/main_bgo_retarget/mod.rs) + validação prévia determinística em **`Mn103Vm`**. |
| **Camada C — Regressão Silenciosa de Empacotamento** | Era possível invocar montagem `--sata-bridge-stage2-boot` gerando `PANA__SD.FRM` com `MAIN3060NP` ainda em `4 MiB` para alvo `8 MiB`. | Falta de trava fail-closed cruzando o alvo NOR (`8 MiB`) com a geometria BGO efetiva embutida no `MAIN`. | Trava obrigatória **`--main-bgo-retarget`**, novo identificador **`CHK_ID_MAIN_BGO_GEOMETRY`** em `verify-sd --audit-profile hardened` e catálogo SSOT em `profiles/traps/`. |

---

## 2. Matriz dos 5 Portões de Validação (`Portão 0 .. Portão 4`)

| Portão | Escopo & Regra de Contenção | Evidência / Mecanismo de Aprovação | Status em `2026-11-23` |
| :--- | :--- | :--- | :--- |
| **Portão 0 — Blindagem Estática, `Mn103Vm` & SSOT** | Nenhuma imagem segue para bancada sem auditoria `verify-sd --audit-profile hardened` (`CHK_ID_MAIN_BGO_GEOMETRY`, `0` falhas) + prova em `Mn103Vm`. | [`main_bgo_retarget/mod.rs`](../../crates/panadiga-core/src/re/main_bgo_retarget/mod.rs), [`validate_bgo.rs`](../../crates/panadiga-profiles/src/validate_bgo.rs) e [`geometry_and_binary_traps.json`](../../profiles/traps/geometry_and_binary_traps.json). | **APROVADO** |
| **Portão 1 — Integridade Elétrica & ATA (`P58001` / `P1503`)** | Continuidade `1:1` validada antes de ligar, jumper **`MASTER`** na `StarTech IDE2SAT2`, ausência de erro de orientação **`0x00002004`**. | Documentado em [`GUIA_HARDWARE_STARTECH_IDE2SAT2_E_CABO_FFC_TYPE_B.md`](GUIA_HARDWARE_STARTECH_IDE2SAT2_E_CABO_FFC_TYPE_B.md). | **APROVADO (HW)** |
| **Portão 2 — Boot via SD com Ponte SATA Ativa (Sem Gravar Flash/HDD)** | Valida enumeração do disco `SAMSUNG HM251JI` sob o `Loader ver0.61` real sem alterar `SA0..SA14` nem gravar espelhos MEIHFS. | [`boottrace_sata_bridge_gate2_20261122.log`](../evidence/boottrace_sata_bridge_gate2_20261122.log) + [`boottrace_sata_bridge_nosd_only_20261122.log`](../evidence/boottrace_sata_bridge_nosd_only_20261122.log) + [`boottrace_sata_bridge_nosd_warm_20261122.log`](../evidence/boottrace_sata_bridge_nosd_warm_20261122.log). | **APROVADO (HW)** |
| **Portão 3 — Stage-2 nos Espelhos MEIHFS + Boot Direto do HDD SATA** | **`FVU)`** grava Stage-2 + `MAIN3060NP` (`8 MiB` BGO) nos 2 espelhos do HDD; Boot 2 arranca **`PiE-OS 3060NP`** diretamente do SATA com SD sem gravação. | **Boot 1:** [`boottrace_sata_bridge_gate3_8mb_boot1_fvu_20261123.log`](../evidence/boottrace_sata_bridge_gate3_8mb_boot1_fvu_20261123.log) (`check_sum = 0x32836a32`, `Vefify OK` × 2)<br>**Boot 2:** [`boottrace_sata_bridge_gate3_8mb_boot2_main_20261123.log`](../evidence/boottrace_sata_bridge_gate3_8mb_boot2_main_20261123.log) (`39.652 B` de `PiE-OS 3060NP`). | **APROVADO AO VIVO (`2026-11-23`)** |
| **Portão 4 — Boot Sem Cartão SD via `Loader ver0.61` `Nor-FROM` (`SA11..SA14`)** | Carregar o Stage-2 diretamente dos setores **`SA11..SA14` (`0x84040000..0x8407FFFF`)** quando `"fvu.exe"` existe no BGO, mantendo **`SA0..SA10` RO**. | Prova determinística completa em `Mn103Vm` + `SpansionNorFlash` em [`sata_hdd_autoboot/mod.rs`](../../crates/panadiga-core/src/re/sata_hdd_autoboot/mod.rs). **Nenhuma escrita física executada em NOR.** | **PROVADO EM EMULADOR (`Mn103Vm`)** |

---

## 3. Portão 3: Da Falha `src/nfd_api.c:228` ao Passe Ao Vivo (`2026-11-23`)

### 3.1 Falha inicial (`4 MiB` vs `8 MiB`) capturada em bancada
Na primeira tentativa do Portão 3 ([`boottrace_sata_bridge_gate3_boot1_20261122.log`](../evidence/boottrace_sata_bridge_gate3_boot1_20261122.log) e [`boottrace_sata_bridge_gate3_boot2_20261122.log`](../evidence/boottrace_sata_bridge_gate3_boot2_20261122.log)), o `MAIN3060NP` foi empacotado sem `--main-bgo-retarget`. Ao tentar inicializar o driver `NFROM/BGO` no silício real com Flash `S29JL064H` de `8 MiB`, a inicialização abortou exatamente em:
- **`[nfrom] BGO driver init failed[1].` (`src/nfd_api.c:228`)**

### 3.2 Anatomia completa do retarget `MAIN3060NP` (`4 MiB -> 8 MiB` BGO)
Em [`main_bgo_retarget/mod.rs`](../../crates/panadiga-core/src/re/main_bgo_retarget/mod.rs), o retargeting de `MAIN3060NP` (`13.427.852 B`) preserva compatibilidade binária exata e verificação determinística:
- **Expansão de estruturas `NFD`/`BGO` em BSS reservada:** tamanho expandido de **`0x1CC4`** para **`0x26C4`** bytes na cauda BSS recém-reservada acima de `_end` (**`0x531B7774 -> 0x531BC000`**, região realocada **`0x531B8000..0x531BC000`**).
- **Code cave e trampolins:** cave dedicada de **`165 bytes` em `VA 0x57000320`**, alcançada por **`12` trampolins** e ajustando **`7` funções/sítios `NFD_API`** junto com **`48` offsets em `nfd_ctrl`**.
- **Parâmetros geométricos e tabela FTL:** base BGO alterada de **`0x84200000`** (`35` setores, `P1=3`, `P2=28`, `max_did=0x0A1A`) para **`0x84130000`** (`45` setores `SA26..SA70`, `P1=1`, `P2=40`, `max_did=0x0F24`), com atualização coerente da tabela/constante **`0x2202`**.
- **Prova pré-hardware em `Mn103Vm`:** execução emulada de **`NFD_Read(17/22/12)`** e ausência da falha `src/nfd_api.c:228` antes de gerar a mídia física SD.

### 3.3 Passe Ao Vivo do Portão 3 (`2026-11-23`)
O kit `firmware_and_images/sd_unbrick_kit_sata_bridge_stage2` foi recompilado com `--loader-profile ver061 --bgo-mode uart_monitor --passthrough-mode factory_original --target-gb 250 --sata-bridge-reset-fix --sata-bridge-stage2-boot --main-bgo-retarget --expected-main-build 3060NP` e validado por `verify-sd --audit-profile hardened` (`main_bgo_geometry=BGO_45SEC_8MB_0X84130000`, `0` falhas):

1. **Boot 1 (`FVU)` — Gravação dos dois espelhos no `SAMSUNG HM251JI`):**
   - Comando `WAITTX 25000 NORMAL "FVU)..." 4646 30000` em `COM10` → [`boottrace_sata_bridge_gate3_8mb_boot1_fvu_20261123.log`](../evidence/boottrace_sata_bridge_gate3_8mb_boot1_fvu_20261123.log).
   - Resultado verificado: **`triggered=YES`**, **`size=0x645028`**, **`check_sum = 0x32836a32`**, **`Vefify OK` × 2** nas faixas **`0x00010000-0x00013228`** e **`0x1D1A5970-0x1D1A8B98`**.
2. **Boot 2 (`BOOTTRACE 60000 NORMAL` — Execução direta do HDD SATA):**
   - Log completo: [`boottrace_sata_bridge_gate3_8mb_boot2_main_20261123.log`](../evidence/boottrace_sata_bridge_gate3_8mb_boot2_main_20261123.log) (`55.917 B`).
   - Descompressão Stage-2 e salto: **`HEADER:81132B55, DECOMP:81132B55`**, **`[STAGE2] Boot MAIN @ 0x57000000`**.
   - Telemetria do sistema operacional principal: **`39.652` bytes** de saída UART do **`PiE-OS / 3060NP`**, `[nfrom] DeviceInfo(EEPROM : 8Kbit)`, chip `M6930LB`, **`SAMSUNG HM251JI`**, `AVOUT_V_Mute: onoff=0`, `SYS_CNTL_SetSysState[EE]`, `FactoryMode = Start`, `Media Confirm = OK`, **zero `BGO driver init failed`**, **zero `U99`**.
   - **Estado físico observado no painel do aparelho:** LED **`HDD`** aceso, LED **`DVD`** acendeu e apagou, LED **`SD`** aceso e visor VFD exibindo **`TM LI`** — comportamento nominal esperado para `WakeupReason:[FACT(18)]` com `FactoryMode = Start` operando sobre um disco com superbloco HDFS ainda vazio (`Hdd illegal (HDD ID mismatched)`).

---

## 4. Extensão `MAIN3410PQ` (Europeu `M6:PAL:DVD.HDD` c/ `NTSC`/`PAL`) & Hardening Anti-Regressão

### 4.1 Suporte ao `MAIN3410PQ` (`13.003.052 B`)
O firmware extraído da partição de firmware do HDD de referência europeia (`hdd_partition_firmware_0x02000000.bin` `6.173.264 B` → `eh55_main3410pq_unpacked.bin` `13.003.052 B`, SHA-256 `c3275d39549b68f48db16bc5fd043c6f8a373d4ba416826de09be5c55dc991b1`, base europeia `M6:PAL:DVD.HDD` com suporte a chaveamento `PAL/NTSC` além dos símbolos compartilhados de driver `AIOF` `PAL_M`) difere do `MAIN3060NP` em dois pontos essenciais verificados no módulo `re/`:
1. **Geometria BGO nativa de `8 MiB`:** diferentemente do `MAIN3060NP`, o **`MAIN3410PQ` já vem nativamente configurado para `0x84130000`** (`VA 0x5756EFD4`, verificada por `verify_main3410pq_native_8mb_bgo`), dispensando retarget `4 MiB -> 8 MiB` e validado em `compose` e `verify-sd`.
2. **Correção da janela de reset SATA (`1000 ms`):** com deslocamento exato `-0x78E2` no módulo `RTSD`, o sítio **`VA 0x5710A9D0`** (`OFF 0x0010A9D0`, original `[2C D0 07 89 DD A7 C9 FF FF 00 00]`) recebe `CALL 0x574DF538` (`rtsd_sleep_10ms`), validado entre os 4 chamadores irmãos (`0x5710A452`, `0x5710B086`, `0x5710B5AF`, `0x5710C7E5`) no teste emulado `mn103vm_proves_main3410pq_and_main3060np_sata_reset_wait_1000ms` em [`sata_bridge_fix.rs`](../../crates/panadiga-core/src/re/sata_bridge_fix.rs).

### 4.2 Hardening Cross-Stack (SSOT, Armadilhas, Dicionários de Símbolos e RAG)
Para impedir que regressões como `4 MiB` vs `8 MiB` ou espera `2 ms` × `555 ms` voltem a ocorrer, as descobertas foram promovidas à SSOT declarativa e ao índice RAG/MCP:
- **Catálogo de Armadilhas ([`profiles/traps/geometry_and_binary_traps.json`](../../profiles/traps/geometry_and_binary_traps.json)):** ampliado para **`11` armadilhas** com inclusão de `TRAP_MAIN3060NP_4MB_BGO_ON_8MB_EH55`, `TRAP_SATA_BRIDGE_2MS_POST_RESET_RACE` e `TRAP_NORFROM_FVU_BGO_MARKER_VER061`.
- **Dicionários de Símbolos Curados (`profiles/symbols/loader_ver061_toc48_main57/`):** criados [`LOADER_VER061.json`](../../profiles/symbols/loader_ver061_toc48_main57/LOADER_VER061.json), [`MAIN3060NP.json`](../../profiles/symbols/loader_ver061_toc48_main57/MAIN3060NP.json) e [`MAIN3410PQ.json`](../../profiles/symbols/loader_ver061_toc48_main57/MAIN3410PQ.json) expondo endereços, callers, constantes hex e side-effects via `panadiga://symbols/<train_id>/<module>`.
- **Indexação RAG Direta de Alta Velocidade ([`crates/panadiga-rag/src/indexer/ssot_atlas.rs`](../../crates/panadiga-rag/src/indexer/ssot_atlas.rs)):** indexa diretamente `profiles/traps/` e `profiles/forensic_atlas/` para respostas imediatas de baixo custo via RAG e servidor MCP nativo (`panadiga mcp`).

---

## 5. Portão 4: Prova em Emulador `Mn103Vm` da Carga `Nor-FROM` (`Loader ver0.61`)

O teste **`mn103vm_proves_nor_autoboot_loader_ver061_norfrom_boot_and_escape_hatch`** em [`sata_hdd_autoboot/mod.rs`](../../crates/panadiga-core/src/re/sata_hdd_autoboot/mod.rs) executa as instruções reais do **`Loader ver0.61`** (`eh55_loader_ver061_256k.bin`) sobre uma cópia **Copy-on-Write** da NOR completa de `8 MiB` (`eh55_nor_8mb_ic5001_repaired.bin`) no emulador **`Mn103Vm` + `SpansionNorFlash`**, garantindo **`SA0..SA10` intactos**:

1. **Descoberta exata de ABI em `FUN_4400894B` (`0x4400894B..0x44008B98`):**
   - `loaded_from_nor` (`-12(A3)`) inicia em `0`.
   - Chama **`FUN_4400A1F1`** (`NFD_FileExists` — atenção ao endereço exato **`0x4400A1F1`**, não `0xA1F2`) com o ponteiro de `.data` **`0x504800AF`** (`"fvu.exe\0"`).
   - **Caso negativo (`D0 != 0`, `"fvu.exe"` ausente):** desvia para **`0x440089F5`** (`Update-Disk` via SD/DVD). Confirmado que o dump real `eh55_nor_8mb_ic5001_repaired.bin[0x40000..0x80000]` é `100% 0xFF` e o BGO não contém `"fvu.exe"`.
   - **Caso positivo (`D0 == 0`, `"fvu.exe"` presente no BGO):** imprime `"Read fvu program from Nor-FROM to 0x%x.\n"`, copia **`256 KiB`** de **`0x84040000` (`SA11..SA14`)** para **`0x90400000`** via `memcpy` (`0x4402A34A`) + verificação integral via `memcmp` (`0x4402A30A`), marca `-12(A3) = 1` e salta para **`0x44008B98`**, desviando da leitura de SD/DVD com correspondência SHA-256 exata e encadeamento até o Stage-2 no HDD.
2. **Armadilha de limpeza (`0x44009BED`) & 3ª rota de escape:**
   - Quando `startup reason = [70]`, `0x44008914` chama `0x44009BED` (`cleanup_bgo_temp_files`), deixando os setores virgens `SA11..SA14` (`0x40000..0x80000`) intocados. Em caso de contingência, o fluxo valida também a **3ª rota de escape (`PANA__SD.FRM` / escape hatch)**.

### 5.1 Prova Ao Vivo Etapa 1 (Dry-Run via SD `P4ZU`) & Auditoria `config1` / `IC7501` (`2026-11-23`)
- **Prova Etapa 1 em silício real ([`com10_portao4_etapa1_sd_dryrun_20261123.log`](../evidence/com10_portao4_etapa1_sd_dryrun_20261123.log), `EV-20261123-025`):** com **`0` escritas na NOR**, o auto-descompactador `P4ZU` (`256 B` em SDRAM) encadeado ao `Loader ver0.61` (`zlib 1.2.2`, trampolim `0x91040000`) descompactou e executou `Fvu ver0.61` (`check-sum[0x316E619]`) e concluiu o boot completo do `MAIN 3410PQ` pelo `SAMSUNG HM251JI` atrás da `IDE2SAT2` (`HEADER:0094407E`, `91.312 B` UART, `overflow=NO`).
- **Auditoria `config1` (`ID 0x0117`) & postmortem da BGO (`EV-20261123-026`):** na tabela `0x57AED0A8` (base `0x53184B44` em `0x5751702A`), **`ID 0x0117` (TV System)** fica em **`+0x12F0` (`página 9 + 0x00F0`)**, com polaridade verificada **`0x00 = TV_NTSC` (`480i 60 Hz`)** e **`0x01 = TV_PAL` (`576i 50 Hz`)**. O script prévio em [`bgo_heal_nfrom_ntsc_20261123.log`](../evidence/bgo_heal_nfrom_ntsc_20261123.log) sanou os furos de tag FTL (`bgo_writeD(ID=0xF01)` eliminado), porém gravou `0x01` no `PID 0x000C` (Página 0, `+0x00EC` = **`ID 0x00C6`** `0xFF -> 0x01`) deixando a Página 9 real intacta em `TV_PAL`; além disso, `PID 0x01AC` (`fs4.dat` pág. 131) aponta via L2P para `SA55 (F2_02) slot 18` com tag `0x0000` (`bgo_read_D(ID=0x1AC) return error(2)`). Em paralelo, o `IC7501` brasileiro (`MN67788RA`) devolveu `0x7F` para opcodes europeus do `3410PQ` (`0xA9`, `0xC9`, `0xE3`, `0xE4`, `0xF9`, `0xFA`).

### 5.2 Cirurgia CoW BGO Bloco A ao Vivo (Zero Erases, `TV_NTSC` + Restauração de `config1[0]` e `PID 0x01AC` — `EV-20261123-027`)

- **Pré-voo físico & integridade FTL (`COM10`):** `VDD=3.26V`, `NSW5V=1` e `BGO_INSPECT` saudável (`sectors=45`, `erased=4`, `valid=41`, `corrupt=0`, `dup_lbn=0`, `zmarker_bad=0`, `healthy=YES`), com `0` furos intermediários `0xFFFF` nos 40 setores `F2`. O backup pré-cirurgia (`178.176 B`, SHA-256 `5A6BDC8C5AEBB4EB0FB80AEA549DCFD8CEDCC102528011B2ADBD40B15767B099`) foi salvo antes de qualquer escrita.
- **Resolução dinâmica da FAT ativa (`0` erases de setor):** a travessia viva localizou `P1=SA39` e o bloco aberto `open=SA37` (`active_l2p=1037`, `blank_sps_total=22`), gravando exclusivamente nos próximos slots virgens de `SA37`:
  1. **`config1[0]` (rotacionado em `PID 0x0044`, `SA48 slot 83`):** o diff exato contra o backup revelou exclusivamente `+0x0EC: bak=0xFF live=0x01` — restaurado para `+0x0EC=0xFF` via CoW em `SA37 slot 32` (`L2P tid=0x0004 (sp=232,s=1)->(sp=234,s=0, word=0x0240)`).
  2. **`config1[9]` (rotacionado em `PID 0x003E`, `SA60 slot 60`, `+0x0F0=01 00 00 00`):** gravado `00 00 00 00` (`TV_NTSC`, 480i 60 Hz) via CoW em `SA37 slot 33` (`L2P tid=0x0003 (sp=202,s=4)->(sp=235,s=0, word=0x0242)`).
  3. **`fs4.dat[131]` (`PID 0x01AC`, em `SA37 slot 18` com `tag=0x0000`):** restaurado a partir do payload limpo (`SHA-256 B58B55FC010B651869BE74EA35FF6B5D7E413F4FA0F38DDF91C767391AA6271F`, mesmo da imagem reparada `SA53 slot 112`) via CoW em `SA37 slot 34` (`L2P tid=0x001A (sp=178,s=3)->(sp=236,s=0, word=0x0244)`).
- **Read-back pós-escrita (antes do reboot):** `BGO_INSPECT healthy=YES`, `corrupt=0`, `dup_lbn=0`, `zmarker_bad=0`, `SA37 holes=0`, e todas as 3 tags, payloads e hashes verificados em 100% no log [`bgo_bloco_a_surgery_ntsc_pid0c_pid1ac_20261123.log`](../evidence/bgo_bloco_a_surgery_ntsc_pid0c_pid1ac_20261123.log).
- **Janela `BOOTTRACE 75000` & confirmação pós-boot:** no boot seguinte ([`boottrace_bloco_a_ntsc_cured_20261123.log`](../evidence/boottrace_bloco_a_ntsc_cured_20261123.log)), o double-reset `PANAMON1>` `[41]` atrasou `[STAGE2] Boot MAIN @ 0x57000000` até `+49019ms`; a captura de 75 s (`bytes=19369` UART em `75021956us`) encerrou em `+51263ms` (`System assert!! File:src/fpcmain.c Line:963 Err:0`) durante a janela de `~29.6s` de `FPC RCV TIMEOUT GATE=15`, antes de o MAIN imprimir `AVOUT_init`/`avout_tv_system`/`0x1AC`. Logo após, `POST_STATUS` (`vdd=3.27V nsw5v=1 drops=41`) e o heartbeat passivo via `pico_heartbeat_ops.py` (`ONLINE_HEALTHY`, `vdd_mv=3258`, `nsw5v=1`, `drops=41`, `POWERED / AWAKE`, `wdt_recoveries=0`, `reboot_src=CLEAN`) confirmaram término limpo do boot com o aparelho desperto.

### 5.3 Cirurgia CoW Bloco B (`NTSC` Curado + `PID 0x01C1`) & Isolamento da Falha `L1489` (`EV-20261123-028`)

- **Cirurgia CoW BGO Bloco B em `COM10` ([`bgo_bloco_b_surgery_ntsc_pid1c1_20261123.log`](../evidence/bgo_bloco_b_surgery_ntsc_pid1c1_20261123.log)):** com backup prévio de `177.664 B` salvo fora do repositório (`pre_bloco_b_bgo_surgery_20261123.bin`, SHA-256 `7576FF4AC4C41C0E0A00AFB4051DB8D19770F8F01CC7E9C9896965B4C26ECF43`), a travessia ativa encontrou `P1=SA41` (`sp=167`) e `open=SA29`:
  1. `config1[9]` (`PID 0x003E`, rotacionado em `SA37 slot 33 -> SA29 slot 58`): gravado `+0x0EC=0x01` e `+0x0F0=01000000` (`TV_NTSC`, `L2P tid=0x0003 (sp=163,s=0)->(sp=167,s=3)`).
  2. `fs4.dat[132]` (`PID 0x01C1`, órfão `0xFF` no slot compartilhado `SA37 slot 19` com `0x0F20`): restaurado em `SA29 slot 59` com payload limpo (`SHA-256 E0B56378B146CF67A03486CED4681F7A4EC30A74112D950717E67E1CDFC5A522`) mantendo intacta a entrada `SA37 slot 19` (`tag=0x0F20`) e zerando divergências L2P (`mismatches=0`, `SA29 holes=0`, `healthy=YES`).
- **Boot real subsequente em `MAIN 3410PQ` ([`boottrace_bloco_b_ntsc_cured_20261123.log`](../evidence/boottrace_bloco_b_ntsc_cured_20261123.log)):** confirmou `AVOUT_init() success.`, `avin_set_now_state() sys tvsystem = TV_NTSC(0).`, `avout_tv_system = 0`, `AVOUT_V_Mute: onoff=0` e visor VFD **`DV`**, mas na escrita ao vivo em BGO capturou exatamente na **linha 1489**:
  - `[nfrom] Status flag change error[1].` $\rightarrow$ `Status change error[-3].` $\rightarrow$ `File rewrite end error[-3].` $\rightarrow$ `Rewrite error[-3].`
  - seguido de `11x` `[nfrom] bgo_writeD(ID=0xF07/0xF01) return error(19).` (lock latched em `0x530FD6A4 == 1`) e `[SYS_CNTL]##### NFROMError Power off !!!!! #####`.

### 5.4 Diff Exaustivo vs IPL `ver0.61` `8 MiB` (`S12/S13A/S13B/S14`) & Prova `Mn103Vm` do Kit Rebuildado (`EV-20261123-029`)

1. **Causa raiz estática completa (4 sítios restantes isolados contra `eh55_nor_8mb_ic5001_repaired.bin` `@ 0x8401_E800..0x8402_6500`):** os kits anteriores (`sd_unbrick_kit_sata_bridge_stage2_3410pq` e `portao4_sd_dryrun_3410pq`) aplicavam apenas `S1..S11` (`165 B`). O diff byte a byte provou exatamente 4 lacunas remanescentes em [`main_bgo_retarget/mod.rs`](../../crates/panadiga-core/src/re/main_bgo_retarget/mod.rs) (e zero fora de `0x574A_6000..0x574B_4500`):
   - **`S12` (`MAIN3060NP: 0x574B44C0` / `MAIN3410PQ: 0x574ACA0A`):** limite `sp` de compactação `P1` `cmp 0x1F -> 0xFF, d0` via trampolim cave `0x570003C5`.
   - **`S13A` (`MAIN3060NP: 0x574AFBB8..FBBA` / `MAIN3410PQ: 0x574A8102..8104`):** `mov 0x2000, a3` substituído inline por `3x NOP` (`CB CB CB`) para que a alocação reserva em `P1` (`param_2 == 1`) preserve `a3 = 0x10000` (identicamente ao IPL nativo `@ 0x840202AD`); sem `S13A`, todo rollover de `P1` falha com `error=1`, trava o lock e dispara `bgo_writeD error(19)`.
   - **`S13B` (`MAIN3060NP: 0x574AFC18` / `MAIN3410PQ: 0x574A8162`):** `mov 8, d2` alterado para `mov 4, d2` (`0x08 -> 0x04`), alinhando o `memmove` da fila reserva `P5` (`4` entradas) e protegendo `P5[3]` (`sec_map_plus1_va + (45 + 3) * 2`).
   - **`S14` (`MAIN3060NP: 0x574B3E59` / `MAIN3410PQ: 0x574AC3A3`):** limite de formatação `P1` `cmp 0x20 -> 0x100, r5` via trampolim cave `0x570003D7` (cave expandida para **`201 B`** em `0x57000320..0x570003E9`, preservando `23 B` zero padding até `0x57000400`).
2. **Prova determinística em `Mn103Vm` (`MAIN3060NP` & `MAIN3410PQ`):**
   - **Controle negativo sem `S13A`:** falhou na `iter 0` do rollover (`sp=0xFF -> 0x100`) com `rc = -3`, erro de compactação `[1]`, `File rewrite end error[-3]`, `lock = 1`, `bgo_writeD(0xF07) error(19)` e chamada subsequente retornando `rc = -2`.
   - **Controle positivo `S1..S14 + S13A/S13B`:** `Part A` (`8` escritas `NFD_Write(22)` avançando `sp > 0x1F`) e `Part B` (`10` escritas cruzando `sp=0xFF -> 0x100` com `P5[3] == 0xFFFF`, reserva `P5` alocada e gravada, remount BGO idempotente e read-back íntegro sob `NvramOnly`) passaram nos dois binários (`3060NP` e `3410PQ`) e sobre `cow_nor` do Portão 4.
3. **Kit Stage-2 `MAIN3410PQ` rebuildado e VALIDADO EM SILÍCIO REAL (`EV-20261123-030`):** composto com `--main-bgo-retarget --sata-bridge-reset-fix --sata-bridge-stage2-boot` e aprovado com `0` falhas em `verify-sd --audit-profile hardened` (`PANA__SD.FRM` `7.181.464 B`, SHA-256 `2dc075f2c01fbb8e798d450eac806ed0db6113f2b93c1be31bef626a99a889de`; módulo `MAIN3410PQ` extraído `13.003.052 B`, SHA-256 `cda7b2cb960d878b40ae64d768b0138aa855e35f258d67d1a8befc47156f0c3a`). Na bancada `COM10` (`N4_HARDWARE`):
   - **Boot 1 (`WAITTX_FVU`, [`boottrace_sata_bridge_gate3_3410pq_s14_boot1_fvu_20261123.log`](../evidence/boottrace_sata_bridge_gate3_3410pq_s14_boot1_fvu_20261123.log)):** `triggered=YES`, `fvu program check-sum[0x21647BA]`, `File ID[0xC8] seek 0xF1800 size=0x5E7C98`, **`check_sum = 0x2f82c81a`**, `Vefify OK` × 2 (`0x00010000` e `0x1D1A5970`) e `success to update mainsoft.`.
   - **Boot 2 a frio direto do `SAMSUNG HM251JI` (`BOOTTRACE 90000 NORMAL`, [`boottrace_sata_bridge_gate3_3410pq_s14_boot2_main_20261123.log`](../evidence/boottrace_sata_bridge_gate3_3410pq_s14_boot2_main_20261123.log)):** `134.268 B` (`95.399 B` UART, `1.935` linhas, `ovf=NO`, `ferr=0`, `perr=0`, `CtrlEdges: NSW5V=0`); `[STAGE2] Boot MAIN @ 0x57000000` executou `MAIN 3410PQ` com `[SYS_BOOT]AVOUT_init`, `TV_NTSC(0)`, `avout_tv_system = 0`, `AVOUT_V_Mute: onoff=0`, completou **`:::SYS_RSV::: >> nfrom back up ok.` (L1690)** e **`:::SYS_RSV::: >> sys_rsv init success.` (L1692)**, montou `startup: HDD opened`, assumiu `SYS_CNTL_POW_ON`, `### HDD WAKEUP ###`, `sys_navi` / `sys_media READY`, `SYS_CNTL_SetSysState[EE]` e `setgui end. tsk=106` com **`0` ocorrências** de `Status flag change error`, `File omap update error`, `File rewrite end error`, `bgo_writeD error(19)`, `NFROMError Power off` ou `U99`, encerrando desperto em `ONLINE_HEALTHY` (`VDD=3.26V`, `NSW5V=1`, `NSW5V_DROPS=74` inalterado).

---

## 6. Inventário SHA-256 & Evidências Verificadas desta Série

| Artefato / Log | Tamanho (Bytes) | SHA-256 Verificado | Papel / Gate |
| :--- | :---: | :--- | :--- |
| [`boottrace_sata_bridge_gate2_20261122.log`](../evidence/boottrace_sata_bridge_gate2_20261122.log) | `7.536` | `f8c1b5699986334f1a97ef004319eaeef880b3447dc4d80d04c1db82c92d6865` | Portão 2 aprovado em HW (`IDE2SAT2`) |
| [`boottrace_sata_bridge_gate3_boot1_20261122.log`](../evidence/boottrace_sata_bridge_gate3_boot1_20261122.log) | `7.557` | `4be424ba7e96859b824537e4f6940a777d4d3540c75fa5a7753211df54c60748` | Portão 3 tentativa 1 (`src/nfd_api.c:228`) |
| [`boottrace_sata_bridge_gate3_boot2_20261122.log`](../evidence/boottrace_sata_bridge_gate3_boot2_20261122.log) | `7.402` | `4d35e08ed4ca9a0cb5e7c7e8c9846494e8966c46831bc57745a473e0935c665e` | Portão 3 tentativa 2 (boot pré-retarget) |
| [`boottrace_sata_bridge_nosd_only_20261122.log`](../evidence/boottrace_sata_bridge_nosd_only_20261122.log) | `9.021` | `3120123e29115694e99b51eb1cf120daeb346683b36fc5ad5091026b2a34ac2e` | Captura sem SD (triagem SATA `IDE2SAT2`) |
| [`boottrace_sata_bridge_nosd_warm_20261122.log`](../evidence/boottrace_sata_bridge_nosd_warm_20261122.log) | `7.536` | `15abe196a8a6a1611907b6ee25f020f47a352f025dcf7b8e8d377d6b0513faab` | Warm boot sem SD |
| [`boottrace_sata_bridge_gate3_8mb_boot1_fvu_20261123.log`](../evidence/boottrace_sata_bridge_gate3_8mb_boot1_fvu_20261123.log) | `4.906` | `455a86083636cf4bcc2c49d33ffd55be1391e6e9c12edb7ebd15791dbdacd2d3` | Portão 3 Boot 1 `FVU)` (`0x32836a32`, `Vefify OK` × 2) |
| [`boottrace_sata_bridge_gate3_8mb_boot2_main_20261123.log`](../evidence/boottrace_sata_bridge_gate3_8mb_boot2_main_20261123.log) | `55.917` | `acbee64c1710e015dc32f707bff4acd3ce25cb1224575ab38753a7b3ff95595a` | **Portão 3 Boot 2 APROVADO (`PiE-OS 3060NP`, `TM LI`)** |
| [`com10_portao4_etapa1_sd_dryrun_20261123.log`](../evidence/com10_portao4_etapa1_sd_dryrun_20261123.log) | `91.489` | `87707afa1e4a63e18eabc62b2008cf7d526e2efb2284fc8317d77bc3bb82b2e7` | **Portão 4 Etapa 1 AO VIVO (`P4ZU` `0x316E619` -> `MAIN3410PQ`, `91.312 B` UART, `0` NOR writes)** |
| [`bgo_heal_nfrom_ntsc_20261123.log`](../evidence/bgo_heal_nfrom_ntsc_20261123.log) | `76.081` | `52cd7675528e1b294f6b6098717e2a2428a05b1f89d175042bfad025dcbf2985` | Cura BGO Fase 1 (zero furos tag) + evidência do postmortem `PID 0x000C` |
| [`bgo_bloco_a_surgery_ntsc_pid0c_pid1ac_20261123.log`](../evidence/bgo_bloco_a_surgery_ntsc_pid0c_pid1ac_20261123.log) | `12.169` | `df864ea9fac42e016f3f2b532a6e26f304a10769d05d34b1e9308d39f62a98d2` | Cirurgia CoW BGO Bloco A ao vivo (0 erases): `config1[0]` (`PID 0x0044`), `config1[9]` `TV_NTSC` (`PID 0x003E`) e `fs4.dat[131]` (`PID 0x01AC`) em `SA37` slots `32..34` + read-back (`EV-20261123-027`) |
| [`boottrace_bloco_a_ntsc_cured_20261123.log`](../evidence/boottrace_bloco_a_ntsc_cured_20261123.log) | `27.588` | `113eb37723f551446ecef41539261857a19f33ed4c080de59225406d3dc1dfca` | `BOOTTRACE 75000` pós-cirurgia Bloco A (`+51263ms` em `FPC RCV TIMEOUT` após double-reset `[41]`; boot limpo confirmado por heartbeat `ONLINE_HEALTHY`) |
| [`bgo_bloco_b_surgery_ntsc_pid1c1_20261123.log`](../evidence/bgo_bloco_b_surgery_ntsc_pid1c1_20261123.log) | `10.745` | `a4347281362152ef9f32c512e287879a9b5d42be1793234f5940b288b44373d8` | Cirurgia CoW BGO Bloco B (`SA29` slots `58..59`: `config1[9]` `TV_NTSC` + `fs4.dat[132]` `PID 0x01C1`, `0` mismatches, `EV-20261123-028`) |
| [`boottrace_bloco_b_ntsc_cured_20261123.log`](../evidence/boottrace_bloco_b_ntsc_cured_20261123.log) | `92.895` | `d5497da7f15858f42d72c26c9ae5e8ac2f8bc5493e81fcb9a8718aaeb0a77685` | Boot ao vivo pós-Bloco B (`MAIN 3410PQ`, `TV_NTSC(0)`, `AVOUT_V_Mute: onoff=0`, `VFD DV`, captura `L1489` `Status flag change error[1]` / `error(19)`) |
| [`boottrace_sata_bridge_gate3_3410pq_s14_boot1_fvu_20261123.log`](../evidence/boottrace_sata_bridge_gate3_3410pq_s14_boot1_fvu_20261123.log) | `5.092` | `33b0cc171292cbd02e235609e0bc3748812867d6a1d5281eb7e55cc1a6ef7d12` | **Boot 1 AO VIVO `MAIN3410PQ` `S1..S14+S13A/S13B` (`size=0x5E7C98`, `check_sum=0x2f82c81a`, `Vefify OK` × 2, `EV-20261123-030`)** |
| [`boottrace_sata_bridge_gate3_3410pq_s14_boot2_main_20261123.log`](../evidence/boottrace_sata_bridge_gate3_3410pq_s14_boot2_main_20261123.log) | `134.268` | `5ad28a99f7b8692d93a36f27f16201340c7aaaa76a7872f9e7d8f7a748eef30a` | **Boot 2 AO VIVO `MAIN 3410PQ` (`TV_NTSC(0)`, `nfrom back up ok.`, `sys_rsv init success.`, `setgui end. tsk=106`, `0` erros BGO/`U99`, `EV-20261123-030`)** |
| `sd_kit_3410pq_bgo_complete/PANA__SD.FRM` *(scratch)* | `7.181.464` | `2dc075f2c01fbb8e798d450eac806ed0db6113f2b93c1be31bef626a99a889de` | Kit SD Stage-2 `MAIN3410PQ` `S1..S14+S13A/S13B` rebuildado e aprovado em `verify-sd --audit-profile hardened` (`EV-20261123-029`, validado em HW em `EV-20261123-030`) |
| `sd_unbrick_kit_sata_bridge/DIGADIAG.FRM` | `262.144` | `305cc2eebc09bbaa3bb5c5d75c0395eb16831bed4b9f80eafd57b9945e6a762d` | Kit SD base (`ver061` `256 KiB` raw) |
| `sd_unbrick_kit_sata_bridge/PANA__SD.FRM` | `7.563.224` | `58d501fa63e69571afdcb7cbc1e6c5644e3dcb29ee4b713d7faa4ed8e703a2b0` | Kit SD Portão 2 |
| `sd_unbrick_kit_sata_bridge_stage2/DIGADIAG.FRM` | `262.144` | `305cc2eebc09bbaa3bb5c5d75c0395eb16831bed4b9f80eafd57b9945e6a762d` | Kit SD Stage-2 Portão 3 (`ver061`) |
| `sd_unbrick_kit_sata_bridge_stage2/PANA__SD.FRM` | `7.563.304` | `42b007eb7e35caeeba19afc0d105f92a909312b9b7fd92da4e29d03076ef8523` | Kit SD Stage-2 `MAIN3060NP` `8 MiB` (`0x32836a32`) |
| `eh55_loader_ver061_256k.bin` | `262.144` | `9f5c448d8879106be5ca3da6d6ac649026d50fbc191251ae0f4a003efca63800` | `Loader ver0.61` (`SA0..SA10`) usado na VM |
| `main3060np_unpacked.bin` | `13.427.852` | `075f552ed583ba1835c0d44d1b533ff5a016b8d81d15d7166240098461ddcca7` | Base `MAIN3060NP` pré-retarget |
| `hdd_partition_firmware_0x02000000.bin` | `6.173.264` | `6ec63de059c00b287a0e8a1a4102a8007bbccf3210a552bfb45118a03b85c898` | Partição HDD `0x02000000` (dump referência europeu `M6:PAL:DVD.HDD`) |
| `eh55_main3410pq_unpacked.bin` | `13.003.052` | `c3275d39549b68f48db16bc5fd043c6f8a373d4ba416826de09be5c55dc991b1` | `MAIN3410PQ` (europeu `M6:PAL` c/ `NTSC`/`PAL` + drivers `AIOF PAL_M`) extraído |

---

## 7. Referências Cruzadas

- **Guia Hardware `StarTech IDE2SAT2` & Cabo FFC `Type B`:** [`GUIA_HARDWARE_STARTECH_IDE2SAT2_E_CABO_FFC_TYPE_B.md`](GUIA_HARDWARE_STARTECH_IDE2SAT2_E_CABO_FFC_TYPE_B.md)
- **Guia Anti-Armadilhas & Atlas Forense SSOT:** [`GUIA_ANTI_ARMADILHAS_GEOMETRIA_BGO_E_ATLAS_FORENSE_SSOT.md`](GUIA_ANTI_ARMADILHAS_GEOMETRIA_BGO_E_ATLAS_FORENSE_SSOT.md)
- **Ledger de Evidências & Status Geral:** [`ledger.json`](../evidence/ledger.json) · [`STATUS.md`](../STATUS.md) · [`debug_logs/README.md`](../debug_logs/README.md)
