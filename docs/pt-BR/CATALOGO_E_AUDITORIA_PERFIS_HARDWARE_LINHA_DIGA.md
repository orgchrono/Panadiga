> 🌐 **Idioma / Language:** [English (US)](../en-US/DIGA_LINEUP_HARDWARE_PROFILES_CATALOG_AND_AUDIT.md) | **Português (BR)**
> 🧭 **Portal:** [Índice da Documentação](../INDEX.md) | [Auditoria Completa](../../AUDITORIA_COMPLETA.md) | [Pseudocódigo Ghidra](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Logs de Debug](../debug_logs/README.md)

# Catálogo e Auditoria de Perfis de Hardware v2 — Linha Panasonic DIGA (89 Modelos, `T0` -> `T4`)

**Documento:** `CATALOGO_E_AUDITORIA_PERFIS_HARDWARE_LINHA_DIGA.md`  
**Escopo:** Auditoria integral multi-eixo da linha Panasonic DIGA (séries **DMR-E**, **DMR-ES**, **DMR-EH**, **DMR-EX**, **DMR-EZ**, **DMR-XW**, **DMR-BS/XS**, **DMR-BW/BWT/BST** e **DMR-UBC/UBS/UBT**), elevação epistemológica dos esquemas `profiles/` (`EvidenceTier`), arquivamento de novos pacotes oficiais do CDN Panasonic, correções forenses de dumps e `DRV J147`, e plano de melhorias futuras.

> **Regra Epistêmica Obrigatória (`CapabilityGate` & `EvidenceTier`):** Nenhum modelo fora da bancada verificada do **`DMR-EH55` (`VEP79138`)** pode receber autorização de escrita ao vivo em NVRAM/NOR (`nvram_write_live` / `upper_nor_write_live`). Todas as elevações de tier seguem estritamente a evidência observada:
> - **`T0` (`community_unverified`)**: relatos de fóruns (`remont-aud`, `elektroda`), part numbers sem esquema oficial/dump ou conflitos abertos.
> - **`T1` (`service_manual_only`)**: Order Nos., códigos `VEP`/`REP`/`RFKN`/`RFKB`, EEPROMs e datasheets verificados diretamente em Manuais de Serviço Panasonic ou datasheets PDF oficiais.
> - **`T2` (`official_frm_verified`)**: containers `.FRM` / `.7z` oficiais descriptografados/validados criptograficamente por SHA-256 e TOC/payload reais.
> - **`T3` (`physical_dump_verified`)**: dumps físicos chip-off integrais validados por SHA-256 e offsets reais (`DMR-ES35V` `8 MiB` Triton, `DMR-EZ485V` `4 MiB` Magnum/LSI).
> - **`T4` (`bench_verified`)**: exclusivo do **`DMR-EH55` (`VEP79138`)**, validado via UART `P7501` (`COM10`) em silício real.

---

## 1. Resumo Quantitativo da Expansão (`profiles/` & Acervos)

Após inspeção sistemática em 6 frentes (30 PDFs locais, 24 contêineres `.FRM` + `DVDRAM.7z` + dumps `DMR-ES35V`/`DMR-EZ485V`, além de pesquisas em Manuais de Serviço e datasheets das séries `DMR-E`, `DMR-ES`, `DMR-EH`, `DMR-EX`, `DMR-EZ` e HD/Blu-ray UniPhier), o registro relacional `profiles/` foi expandido e validado com **796 checagens relacionais** (`panadiga profiles validate` aprovado com `0` erros):

| Domínio (`profiles/`) | Antes | Depois | Principais Entidades Adicionadas / Elevadas |
| :--- | :---: | :---: | :--- |
| **`soc_cores/`** | `3` | **`5`** | `+ arm_cortex_a9_uniphier`, `+ arm64_cortex_a53_uniphier` (`3300 mV`) |
| **`socs/`** | `6` | **`12`** | `+ mn2ds0010h`, `mn2ds0019` (`T2` em `MAIN.BIN` EH58/EX), `mn2ws0030`, `mn2ws0038`, `mn2ws0230`, `uniphier_ld20_pxs3`; `lsi_domino_zebra64` atualizado para `DM8635-CHZ` |
| **`flash_chips/`** | `8` | **`18`** | Elevados `s29jl032h` (`002-01186` `0x0001/[0x227E,0x220A,0x2201]`) & `oki_mr27v802_1mb_otp` (`FEDR27V802F`) para `T1`; `+10` Flash NOR paralelas `T1` (`AM29LV640D`, `S29GL064N` Top/Uniform, `S29GL256P`, `MX29LV160/640`, `MBM29LV160TE`, `M29W160ET/320ET/640GT`) |
| **`eeprom_chips/`** | `4` | **`6`** | Elevados `rohm_br24l16_2kb` & `microchip_24lc64_optical_cal` para `T1` com PDFs locais; `+ panasonic_c3ebjc000055_dif` (`IC37502` EH80V) e `+ panasonic_rfkfm4016kt_dif` (`IC9705` EX98V / `IC7404` EH58) |
| **`firmware_trains/`** | `8` | **`18`** | Corrigidos `eh50_ecb64_frm` (`le2011_08e0`), `loader_ver063_bootbin` (`drv=0x9020_2000`, `main=0x5700_0000`) & `toc*`; `+10` trains `T2` (`prog061_mini`, `prog201` 4M/8M, `prog202`, `prog205`, `xw300`, `bs850`, `bs880`, `bs885`, `ex97 PANAEUSB`) |
| **`nor_layouts/`** | `3` | **`3`** | Corrigido `ez485v_vxworks_on_mx29lv320b` (`ipl` `0x0000_0000..0x0035_FFFF`, `vendor_nvram` `0x0036_0000..0x003F_FFFF`); enriquecido `es35v_ver018_on_8mb` |
| **`fixtures/`** | `3` | **`3`** | Corrigidos 100% dos probes/SHA-256 em `dmr_eh55`, `dmr_es35v` e `dmr_ez485v` contra os dumps reais em disco |
| **`boards/`** | `6` | **`31`** | Corrigidas as 6 placas originais (`vep79104` `P7501=false`, `vep79108e`, `vep79195` Main vs D-IF `RFKB71134A`, `es35v_digital_board` `VEPV0057A/RFKNES35V*`, `ez485v_lsi_board` `VEP79180/MSC-RFKB79180C`); `+25` placas `T1` |
| **`models/`** | `18` | **`89`** | Todos os 18 modelos existentes auditados + **`71` novos modelos** nas séries DMR-E (11), DMR-ES (5), DMR-EH (8), DMR-EX (18), DMR-EZ (9) e HD/Blu-ray DIGA (20) |

---

## 2. Descobertas Críticas de Segurança Física e Arquitetura

### 2.1 Armadilha Elétrica no Conector `P7501` (`DMR-EH58` / `DMR-EH59`) e Pinagem `EX98V`
- **Alerta Crítico (`T1` em `DSD0802009CE` / `CHM0903007CE`):** Nas placas Main **`VEP79183`** (`DMR-EH58`, `DMR-EH59`, `DMR-EH68`), o designador **`P7501`** foi **reaproveitado para um conector de 8 pinos (`K1KA08A00427`)** transportando controle/potência de HDMI (**`+5.8V_AV`** / `XSW+5.8V_AV` para `IC56105`/`P56101`) — **NÃO é a porta UART `115200 8N1`!** Conectar uma sonda RP2040/RP2350 de `3.3V` em `P7501` desses modelos pode causar dano permanente. Os perfis `rfkb79183_eh58` e `rfkb79183aa_eh59` marcam `p7501_header_present: false`.
- **`DMR-ES10` (`VEP79104A`):** Varredura completa nas 66 páginas de [`panasonic_dmr_es10_service_manual_north_america.pdf`](../../manuals_and_schematics/dmr-es10/service/panasonic_dmr_es10_service_manual_north_america.pdf) (`DSD0502037C1`) confirma ausência física de conector `P7501` (apenas `PP7401`/`P9001` 88 pinos) e corrige atribuição equivocada a `VEP79118` (que pertence à geração `EX75`/`EX95V`/`EH75V`).
- **`DMR-EH80V` (`MAD0510009CE`):** Não há `P7501` (`PP7501` é FFC de 10 pinos para painel FL `K1KA10B00176`); sinais UART roteiam pelos pinos `39/40/15` de `IC37501` (`C2CBKH000211`) na placa Digital I/F `VEP09136G` até `P39702`.
- **`DMR-EX98V` (`DSD0806033CE`):** `VEP79195A/B/C` é a placa principal **Analógica/VHS** (`MAIN P.C.B.`), enquanto a placa Digital (`RFKNEX98V*`) é módulo blindado omitido no Vol. 1 e a placa `DIG.INTERFACE` **`RFKB71134A/AT`** contém `IC9705` (`RFKFM4016K/KT`) e o conector UART JST de 6 pinos **`P9705`** (`K1KA06AA0188`: `DGND`, `UARTP2M`, `UARTM2P`, `SW UART`, `XRESET`, `D3R3V`).

### 2.2 Bifurcação Arquitetural da Linha `DMR-EZ` (Magnum SPARC V8 vs. UniPhier AM34)
- **Sub-linha Norte-Americana ATSC/NTSC (`DMR-EZ28`, `DMR-EZ48VP`, `DMR-EZ485VP`):** Placa **`VEP79180`** (`MSC-RFKB79180C`), SoC **`DM8635-CHZ`** (`LSI DoMiNo / Zebra 64` SPARC V8 big-endian, VxWorks 1.3), Flash `IC8651` (`MX29LV320` 4 MiB bottom-boot) com região NV real iniciando em **`0x0036_0000`** (`CLsiPMABS` + `CIncrementalABS` em `0x3D0000`/`0x3F0000`), conforme corrigido em [`ez485v_vxworks_on_mx29lv320b.json`](../../profiles/nor_layouts/ez485v_vxworks_on_mx29lv320b.json).
- **Sub-linha Europeia DVB-T/PAL (`DMR-EZ49VEB/VEC/VEG`, `DMR-EZ48VEB`):** Opera em **Matsushita UniPhier AM34 (`MN103S`)** — comprovado via `DSD1002003CE` (`VEP79250A`), `DSD0806025CE` e contêineres Feistel64 `PANA_DVD.FRM` locais (`ez49veb`/`ez49vec` + pacote combinado `V107` compartilhado com `EX86`/`EX83`/`EX773`/`EX99V`), com `PROG201 (8MB)` / `PROG202` distinto do `PROG201 (4MB)`.

### 2.3 Transição NOR -> NAND nas Séries HD (`DMR-XW300`) e Blu-ray (`DMR-BS/XS`)
- Os 4 pacotes oficiais recém-baixados e arquivados em [`dmr-xw_bd_series/README.md`](../../firmware_and_images/dmr-xw_bd_series/README.md) (`UPDATE_XW300GN_V17.exe`, `UPDATE_BS850_BS750_XS350EG_V16.exe`, `UPDATE_BS880_BS780_XS380EB_V141.exe` e `UPDATE_BS885_BS785_XS385EG_V140.exe`) decifram sua tabela TOC pela chave **`le2011_08e0`** (`08E03D859AF9F3EE`).
- O pacote do `BS785/BS885/XS385EG` adiciona um cabeçalho externo plaintext de 48 bytes **`PANASONIC\0\0\0`** antes da TOC cifrada em `0x30`, mantendo offsets de módulos relativos ao byte `0` do arquivo (`0x2000` e `0x2A000`).
- Todos os `PROG0.00` decifrados iniciam com o vetor de reset **MN103S (`0xCA 0x10 ... 0xFC 0xDC`)**, mas executam `[SWUP] NAND FLASH` e `nand_base.c` — marcando a transição definitiva de NOR paralela TSOP-48 para **NAND Flash** (com `zlib.adler32` validando `PROG0.00` nos modelos BS/XS).
- Os dois pacotes USB da série **`DMR-EX97`** (`UPDATE_EX97EB_V110.zip` e `UPDATE_EX97SEG_EX97CEG_V105.exe`, contêineres `PANAEUSB.FRM` documentados em [`dmr-ex_ez_series/README.md`](../../firmware_and_images/dmr-ex_ez_series/README.md)) utilizam criptografia não-Feistel (sem correspondência nas 12 chaves `PANA_DVD` conhecidas) e recebem classificação honesta `T2` no nível de contêiner/hash.

### 2.4 Por que SoC/Flash Permanecem Ocultos em Manuais Vol. 1 (`DMR-E85H`, `ES15`, `EH65`, `EX`)
- A busca direta nos 30 PDFs locais confirmou **0 ocorrências** de `MN2DS*`, `S29*`, `MR27V802` ou `C3FB*` nos esquemáticos: a Panasonic trata o módulo `RAM/Digital P.C.B.` (`RFKN*`) como peça de troca selada na oficina autorizada. Por isso, modelos como `DMR-E85H` (conflito aberto entre `2 MB` vs `4 MB` na comunidade), `DMR-EH56` (`DSD0603003CE`) e `DMR-EH58`/`EX98V` recebem **`T1` em `board`/`eeprom`**, **`T2` em `train`** (quando há `.FRM` oficial), mantendo **`flash_chip`/`nor_layout` em `T0` (`community_unverified`)** até captura física ou aquisição de manuais **Vol. 2**.

---

## 3. Novos Arquivos Arquivados com Verificação SHA-256

### 3.1 Firmware Oficial Baixado e Extraído (`firmware_and_images/`)
Todos os hashes abaixo estão travados em [`dmr-xw_bd_series/SHA256SUMS.txt`](../../firmware_and_images/dmr-xw_bd_series/SHA256SUMS.txt) e [`dmr-ex_ez_series/README.md`](../../firmware_and_images/dmr-ex_ez_series/README.md):
- `dmr-xw_bd_series/xw300gn/PANA_DVD.FRM` (`33,290,728 B`, `193fd4377a1312b4a46aabd60e2449c7349b18c1ed08b1fee228b7befa1c2633`)
- `dmr-xw_bd_series/bs850_bs750_xs350eg/PANA_DVD.FRM` (`45,049,232 B`, `a23cc1288222b2816fb1328c24613686ec890b775c9b44daa91085d56d8f913d`)
- `dmr-xw_bd_series/bs880_bs780_xs380eb/PANA_DVD.FRM` (`43,836,424 B`, `747b607f61dc20a180beac78969cbf2c32743e7636e269110d27a6033f351468`)
- `dmr-xw_bd_series/bs885_bs785_xs385eg/PANA_DVD.FRM` (`40,562,688 B`, `222f213c234c5ec17ba1e026ee6fe9575eade3b8557f257a73b80740d9b30b21`)
- `dmr-ex_ez_series/ex97eb/PANAEUSB.FRM` (`107,417,648 B`, `f62bc64672fe3931a1987ff8d76098ce755e1b4647ad6f3983874c154e892fb2`)
- `dmr-ex_ez_series/ex97seg_ceg/PANAEUSB.FRM` (`112,152,576 B`, `25131e36cd0e1912d7be4d3ce26c222b0670cacb289b197ce60119c0943a65f8`)

### 3.2 Datasheets Oficiais de Silício (`manuals_and_schematics/datasheets/` — 32 PDFs no acervo)
Manifesto versionado em [`datasheets/SHA256SUMS.txt`](../../manuals_and_schematics/datasheets/SHA256SUMS.txt) e catalogado em [`manuals_and_schematics/README.md`](../../manuals_and_schematics/README.md):
- [`rohm_br24l16_datasheet.pdf`](../../manuals_and_schematics/datasheets/rohm_br24l16_datasheet.pdf) (`693,706 B`, `5811033bbd71708dfe2067878945da59ba8c19fa9ba9d1b0ba6fb72d6b3a149b`) — eleva `rohm_br24l16_2kb` para `T1`.
- [`microchip_24lc64_datasheet.pdf`](../../manuals_and_schematics/datasheets/microchip_24lc64_datasheet.pdf) (`602,424 B`, `af7781c3d84bdf0fac62ed55db5711c55368e821262bd6e49b5be9be6497329e`) — eleva `microchip_24lc64_optical_cal` para `T1`.

---

## 4. Correções Aplicadas em Código e Fixtures Físicas

1. **Correção do offset `DRV J147` (`DMR-EH50` `le2011_08e0`):** Em `dmr-eh50/PANA_DVD.FRM`, o slot 0 (`DRV J147`) grava o offset `0x1800` no byte `+0x08` em vez de `+0x0C` (`0xCDF58588`). O parser em [`parse.rs`](../../crates/panadiga-core/src/crypto/toc/parse.rs) foi corrigido para normalizar o slot 0 quando `+0x08 == 0x1800` dentro dos limites do arquivo, coberto pelo teste `eh50_drv_j147_slot0_offset_quirk_resolves_to_0x1800`.
2. **Alinhamento das 3 fixtures aos dumps reais (`profiles/fixtures/`):**
   - [`dmr_eh55.fixture.json`](../../profiles/fixtures/dmr_eh55.fixture.json): offsets exatos verificados em `eh55_nor_8mb_ic5001_dump.bin` (`0x0002_FFE7`, `0x0003_0000`, `0x0003_FFE0`, `0x0013_0000`, `0x007F_0000`).
   - [`dmr_es35v.fixture.json`](../../profiles/fixtures/dmr_es35v.fixture.json): corrigidos 3 offsets obsoletos (`0x0002_F6F0`, `0x0003_46AE` `DAVIS`, `0x0070_0000` `0AF2`, `0x0072_0604` `M6720EE`, `0x007F_4000` `01F1`).
   - [`dmr_ez485v.fixture.json`](../../profiles/fixtures/dmr_ez485v.fixture.json): adicionado `expected_sha256` (`6ebb31f2...`) e verificados `CLsiPMABS` (`0x0036_0010`) e `CIncrementalABS` (`0x003D_0010` / `0x003F_0010`).

---

## 5. Melhorias Propostas para Próximos Ciclos

1. **Suporte no Parser `.FRM` ao Cabeçalho Externo `PANASONIC\0\0\0` (48 B) e Checksum `Adler-32`:** Estender `panadiga-core::crypto` para detectar automaticamente o cabeçalho de 48 B observado no `BS785/BS885/XS385EG` e validar `Adler-32` nas séries UniPhier Blu-ray/HD.
2. **Extensão do Schema v2.1 para Memórias NAND Flash e eMMC (`flash_chip`):** Os gravadores 2009–2019 (`XW300`, `BS/XS`, `BW/BWT`, `UBC/UBS`) usam SLC NAND / eMMC em vez de NOR paralela com setores top/bottom uniformes. Estender `flash_chip.schema.json` (OOB/page/erase-block/eMMC RPMB) permitirá modelar mídia não-NOR preservando `writable: false` no barramento de bancada paralelo.
3. **Pesquisa Criptográfica do Contêiner `PANAEUSB.FRM` (`DMR-EX97`):** Analisar binários `libfmupre.so` / carregadores USB UniPhier ARMv7 para identificar a cifra AES/envelope usada em `PANAEUSB.FRM`.
4. **Aquisição Prioritária de Manuais Vol. 2 / Esquemas Faltantes:** Baixar e indexar em `manuals_and_schematics/` os manuais com maior potencial de expor silício Digital `MN2DS*` / `C3FB*` / `MN2WS*`: `DSD1006036CE` (`DMR-EX93CEG` Vol. 1 & Vol. 2), `panasonic_dmr-ex80seg.pdf` (`DMR-EX80SEG`), `CHM0802005CE` (`DMR-EZ48VP/EZ485VP`), `DSD1002003CE` (`DMR-EZ49V`) e `DSD0806025CE` (`DMR-EZ48VEB`).
