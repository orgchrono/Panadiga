# Firmware Dumps and Binary Images (`firmware_and_images/`)

Este diretório armazena os dumps brutos de memória e imagens de armazenamento do Panasonic DMR-EH55 utilizados pelas ferramentas de análise forense e engenharia reversa do `panadiga`.

> [!TIP]
> **Download Direto de Todas as Imagens Verificadas (`firmware-pack-v0.1.0`):** Como os dumps brutos de 100 MiB excedem o limite por arquivo do Git (e mantêm o `git clone` leve), todos os pacotes validados (`DMR-EH55`, `DMR-EH58` `DVDRAM.7z`, `DMR-EH50`, `DMR-EX98V`, `DMR-EZ485V` 4MB NOR, **`DMR-ES35V` 8MB NOR + EEPROM Triton dump**, `dmr-am34-unpacked-plaintext-bootloaders-and-kernels.7z`, partições extraídas e imagens HDD `.bin.gz`/`.7z` + `FIRMWARE_SHA256SUMS.txt`) estão disponíveis diretamente na Release oficial:  
> **https://github.com/orgchrono/Panadiga/releases/tag/firmware-pack-v0.1.0**
>
> Ao trabalhar localmente em bancada, baixe ou coloque seus dumps nesta pasta com os nomes descritos abaixo.

## 🧭 Guia Rápido por Função (DMR-EH55 / DMR-EH55LB-S)

| Categoria | Arquivo na Pasta (`firmware_and_images/`) | Tamanho | Função / Uso na Bancada |
| :--- | :--- | ---: | :--- |
| **Cartão SD (`LD FVU`)** | [`sd_unbrick_kit/PANA__SD.FRM`](sd_unbrick_kit/PANA__SD.FRM) | `7.181.224 B` | Contêiner `sd48` dual (`M6930LB` + `M6930`): `DRV S211`, `PROG061` `UART_MONITOR_PANAMON1` e `MAIN3410PQ` `AV_PASSTHROUGH` + HDD 250 GB |
| **Cartão SD (`JIG UART`)** | [`sd_unbrick_kit/DIGADIAG.FRM`](sd_unbrick_kit/DIGADIAG.FRM) | `262.144 B` | Binário bruto `ver0.61` (`0x90400000`) que sobe o monitor serial interativo `PANAMON1>` via sonda Pico `P7501` (`COM10`) |
| **HDD Unbrick (250 GB)** | `panasonic_dmr_eh55_hdd_250gb_patched.img` | `104.857.600 B` | Imagem `100 MiB` pronta para gravar em HDD 250 GB: cabeçalho `"SH"` + `MAIN3410PQ` desbloqueado para `249.97 GB` (`0xCA` LBA48) |
| **HDD Stock (250 GB ODD)** | `panasonic_dmr_eh55_hdd_250gb_odd_stock.img` | `104.857.600 B` | Variante `hdd_100.bin` ajustada para geometria de capacidade stock em 250 GB sem alterações extras no `MAIN` |
| **HDD Referência (160 GB)** | `panasonic DMR EH-55 Firmware hdd_100.bin` | `104.857.600 B` | Dump bruto original de referência (Internet Archive, `MAIN` travado em 160 GB) |
| **Flash NOR (`IC5001` Pré)** | `eh55_nor_8mb_ic5001_dump.bin` | `8.388.608 B` | Captura física original `S29JL064H` pré-reparo (`Loader ver0.61` + FTL BGO com `U99` + NVRAM original) |
| **Flash NOR (`IC5001` Pós)** | `eh55_nor_8mb_ic5001_repaired.bin` | `8.388.608 B` | Imagem consolidada pós-cirurgia BGO validada (`U99` eliminado; boot avança até `normal mode -> RTSC Initialize`) |
| **Bootloader + NVRAM NOR** | `eh55_loader_ver061_256k.bin` / `eh55_nvram_64k_raw.bin` | `256 KiB` / `64 KiB` | Recortes exatos do estágio inicial (`0x84000000`) e da região EEMUL (`0x847F0000`) da NOR real |
| **CD Óptico (`PANA_DVD`)** | `PANA_DVD.FRM` / `PANA_DVD_PATCHED.FRM` | `7,53 MB` / `7,18 MB` | Pacotes Feistel 48 B para gravação de CD-R (`PANA_DVD.FRM` stock `3060NP` vs versão patched `3410PQ`) |
| **Partições & Dumps Ref.** | `extracted_partitions/`, `dmr-es35v/`, `dmr-eh58/`, etc. | — | Módulos `DRV`/`PROG061`/`MAIN` extraídos + dumps comparativos de outros modelos DIGA |

---

## 📁 Arquivos Esperados

### 1. Dump de Referência do HDD — Internet Archive (`panasonic DMR EH-55 Firmware hdd_100.bin`)
* **Descrição:** Dump bruto dos primeiros 100 MB de um HDD de Panasonic DMR-EH55 obtido do **Internet Archive** (**não** foi extraído do HDD físico da bancada). Contém o cabeçalho `MEIHDFS1.0`, tabelas espelho, journal transacional e duas partições de firmware cifradas via Feistel 64-bit ECB:
  * `extracted_partitions/hdd_partition_firmware_0x02000000.bin` (`6.173.264 bytes`, SHA-256 `6ec63de059c00b287a0e8a1a4102a8007bbccf3210a552bfb45118a03b85c898`) → descompacta (`--mode main`) para **`EH55_MAIN_3410PQ.bin`** (`13.003.052 bytes`, SHA-256 `c3275d39549b68f48db16bc5fd043c6f8a373d4ba416826de09be5c55dc991b1`, `tag='MAIN'`, `dst=0x57000000`).
  * `extracted_partitions/hdd_partition_firmware_0x04000000.bin` (`720.896 bytes`, SHA-256 `487d5951d4aa7bc45e5f634c4ca079f369db06d5cb62e41a173575cecd68dbae`) → descompacta (`--mode drv`) para **`EH55_DRV_S211.bin`** (`1.441.792 bytes`, SHA-256 `6f2b61ede80ea9e3e9185d3d677813593f3d361446678e437c905a0037060243`, `tag='DRV '`, `dst=0x90202000`).
* **Tamanho exato:** `104.857.600 bytes` (204.800 setores LBA de 512 bytes).
* **Hash MD5:** `ef4453fc8651903b0a4b0c1873d950d4`
* **Hash SHA-256:** `9d292d5b46f190775f359e30af11d074021fb321f040bf701d0ecf0f2056db00`

### 2. Contêiner Óptico de Atualização (`PANA_DVD.FRM`)
* **Descrição:** Pacote oficial de atualização de firmware via CD para o Panasonic DMR-EH55 (extraído de `UPDATE_EH55.zip`, modelo/variante `NP-309`), contendo 3 módulos cifrados na TOC de 48 bytes (`extracted_partitions/frm_unpacked/`):
  * `drv_s211_unpacked.bin` = **`EH55_DRV_S211.bin`** (`1.441.792 bytes`, SHA-256 `6f2b61ede80ea9e3e9185d3d677813593f3d361446678e437c905a0037060243`, idêntico byte a byte ao módulo da partição `0x04000000`).
  * `prog061_dec.bin` = **`eh55_frm_PROG061_BOOT.bin`** (`262.144 bytes`, SHA-256 `c7da9121174fcf4c54f35f5bb1f7b23a3dc0c93ab227158ad4fdd90a1ea24a81`, base VA `0x50400000`).
  * `main3060np_unpacked.bin` (`13.427.852 bytes`, SHA-256 `075f552ed583ba1835c0d44d1b533ff5a016b8d81d15d7166240098461ddcca7`, base VA `0x57000000`).
* **Tamanho exato:** `7.531.744 bytes`.
* **Hash SHA-256:** `a596d52a8220a9113777c9caf271ac4d518202d0639d6118a4d0bc03b9a6367f`

### 3. Dumps Ouro da Flash NOR `IC5001` do DMR-EH55 Real (Spansion `S29JL064H70TFI00`, 8 MiB — Captura `COM10` / `PANADMP1`)
* **Descrição:** Leitura física integral (somente-leitura, por fatias via `PANAMON1>` + `PANADMP1` na janela *uncached* `0x84000000..0x847FFFFF`, ajustando os limites `MOV` em SDRAM *uncached* `0x9043950E` $\rightarrow$ `J 904394FA`) da NOR Flash TSOP-48 real da placa `VEP79138` em bancada:
  * **`eh55_nor_8mb_ic5001_dump.bin`** (`8.388.608 bytes` / 8 MiB, **`SHA-256: c1faee798f60545853eefdba0d15f23678b471466dc2bc84f522bc972f15a02d`**): Dump completo pré-reparo (`142` setores `SA0..SA141`: `8×8 KiB` + `126×64 KiB` + `8×8 KiB`), mapeando:
    * `SA0..SA10` (`0x000000..0x03FFFF` / `0x84000000..0x8403FFFF`, `256 KiB`): `Loader ver0.61 [06/03/06]` em plaintext + tabela BGO em `0x440337A4`.
    * `SA11..SA14` (`0x040000..0x07FFFF`): Erased `0xFF`.
    * `SA15..SA25` (`0x080000..0x12FFFF` / `0x84080000..0x8412FFFF`): Contêiner `EXTRHEAD "DRV "` (`src=0x84080040`, `len=0xACC01` $\rightarrow$ `dst=0x90202000`, `len=0x160000`).
    * `SA26..SA70` (`0x130000..0x3FFFFF` / `0x84130000..0x843FFFFF`, **`45 × 64 KiB`**): FTL **BGO (`[nfrom]`)** real do EH55 (`1` bloco P1 `0xF101`, `40`/`0x28` blocos P2 `0xF201..0xF228`, `4` spares `0xFFFF`, `max_pid=0x0F24`, controle `0x0F01`, dir `0x0F02..0x0F04`, FAT primária `0x0F07..0x0F15`, FAT backup `0x0F16..0x0F24`) — distinto de `Loader ver0.63` / `PROG061` (`0x84200000..0x843FFFFF`).
    * `SA71..SA141` (`0x400000..0x7FFFFF` / `0x84400000..0x847FFFFF`, `63×64 KiB + 8×8 KiB`): Região **Upper EEMUL / NOR Flash FS** (`0x0FF00000` `"1284"` em `SA71..SA134`; `SA134` em `0x7F0000` com cabeçalho ativo de 6 bytes `0ff00000002c` e `SA135..SA141` em `0xFF`).
  * **`eh55_loader_ver061_256k.bin`** (`262.144 bytes` / 256 KiB, `0x84000000..0x8403FFFF`, **`SHA-256: 9f5c448d8879106be5ca3da6d6ac649026d50fbc191251ae0f4a003efca63800`**): Recorte exato dos primeiros `256 KiB` (`SA0..SA10`, `Loader ver0.61 [06/03/06]`).
  * **`eh55_nvram_64k_raw.bin`** (`65.536 bytes` / 64 KiB, `0x847F0000..0x847FFFFF`, **`SHA-256: 803d1465f48fb32e17d3e478e02b2a9a7e707bbb675bb424b02c5c1df92f3643`**): Recorte exato dos últimos `64 KiB` (`SA134..SA141`).
* **Estado Pós-Reparo Verificado em Bancada (`U99` Eliminado):** **`eh55_nor_8mb_ic5001_repaired.bin`** (`8.388.608 bytes` / 8 MiB, **`SHA-256: adb21d955ed96121249e7abeec58e1ce3951f511a595636d70ebb07a35ca4c9c`**) — imagem consolidada da cirurgia no FTL BGO (`SA70` formatado + `1.629` escritas puras `1 -> 0` em 13 spares `SA42..SA66` + `6` bytes em `SA41+0x14`), que eliminou `BGO driver init failed[1].` (`U99`) e restabeleceu o boot normal até `***** normal mode *****` $\rightarrow$ `*** RTSC Initialize ***`.

### 4. Pacote de Serviço `DMR-EH58` (`dmr-eh58/DVDRAM.7z`)
* **Descrição:** Arquivo de serviço comparativo da mesma família MN103S DIGA (`BOOT_VER,0.63`, `DEV_VER,3110PC`):
  * `BOOT.BIN` → **`BOOT.BIN`** (`262.144 bytes`, SHA-256 `7c13c72a54c77387f3f7df0ddc213e4800e20b27eecbfd225cbc05a8fc73e377`, base VA `0x44000000`, `Loader ver0.63 [07/10/24]` em plaintext).
  * `DRV.BIN` (`720.896 bytes`, SHA-256 `3a9699dd576e96ac89d163094786f2c292bb68826bac344837ec3c9f9a0eccbf`) → descompactado (`--mode plain`) para **`DRV_decomp.bin`** (`1.441.792 bytes`, SHA-256 `694a4ab8aa29fcc45350dc0712047663602bdee7e4bff313b8ae62b057efd052`, base VA `0x40000000`, `MATSHITA R6-MV4 A0 S415`).

### 5. Dump Físico `DMR-ES35V` (`dmr-es35v/` — 8 MiB Plaintext NOR + 8 KiB EEPROM)
* **Descrição:** Dump chip-off real (Triton v5.7) publicado na Release `firmware-pack-v0.1.0` como **`dmr-es35v-8mb-nor-flash-and-eeprom-triton-dump.zip`** (`13.599.723 bytes`, SHA-256 `800f3874d4a3ed2bacf90780a7f6f666cb9f8b5a3b2e6bf9dfd9529f2b50cb32`), com `flash.bin` (`8.388.608 bytes`, bootloader XIP `0.18 [06/03/13]`, `EXTRHEAD DRV/MAIN`, FS BGO `0x700000..0x7EFFFF` e EEMUL NVRAM `0x7F0000..0x7FFFFF`) e `eeprom.bin` (`8.192 bytes`, 24C64). Veja [`dmr-es35v/README.md`](dmr-es35v/README.md).

### 6. Captura IR Flipper Zero — Controle Combo Panasonic DVD/TV (`Panasonic_DVD_TV.ir`)
* **Descrição:** Arquivo de captura infravermelho **Kaseikyo** (`Filetype: IR signals file`, `Version: 1`, `305` linhas, **`50` sinais parseados**: `6` comandos TV `0x80` + `44` comandos DVD/HDD `0xB0`) de controle combo DVD/TV norte-americano (com botões `AUDIO [A]`, `DISPLAY [B]`, `SUB MENU [S]`, `F REC`, `CM SKIP`, `TIME SLIP` e `DIRECT NAVIGATOR`, sem fileira de 4 botões de Teletexto).
* **Origem / Crédito:** `Flipper-IRDB` (`DVD_Players/Panasonic/Panasonic_DVD_TV.ir`, crédito `emptythevoid` — `https://github.com/emptythevoid`).
* **Tamanho exato:** `4.943 bytes`.
* **Hash exato:** `82acb0a7611c89ea56799094bf3c71b9e366f38c0301b457cf5c8266279a9b99`.
* **Referência Técnica Bilíngue:** [`docs/pt-BR/REFERENCIA_CAPTURA_IR_PANASONIC_DVD_TV.md`](../docs/pt-BR/REFERENCIA_CAPTURA_IR_PANASONIC_DVD_TV.md) · [`docs/en-US/PANASONIC_DVD_TV_IR_CAPTURE_REFERENCE.md`](../docs/en-US/PANASONIC_DVD_TV_IR_CAPTURE_REFERENCE.md)

### 7. Kit Canônico de Recuperação via Cartão SD (`sd_unbrick_kit/`)
* **Descrição:** Diretório pronto para copiar direto para a raiz de cartão SDSC (`<= 2 GiB`, FAT16) no DMR-EH55 / DMR-EH55LB-S (`Loader ver0.61 [06/03/06]`):
  * **`DIGADIAG.FRM`** (`262.144 bytes`, SHA-256 `de72e2640ee48a7628a674e3dad731cff71548fc9ea3403ca616f3e5f7e392c0`): `PROG061` bruto descomprimido para `0x90400000` (`JMP 0x50400014`) com salto incondicional em `0x5040A2BC` (`ca 11 cb cb cb cb cb`, `BRA 0x5040A2CD` após `FUN_50404592`) e monitor interativo `PANAMON1` (`0x50439494`, usado no modo JIG via `P7501`).
  * **`PANA__SD.FRM`** (`7.181.224 bytes`, SHA-256 `cdcc2cdf4617c4f08d8af3f617db369cfd360d8db2601b33c84bae0e6f995d90`): contêiner plaintext **`sd48`** (`0x1800 B` TOC, **6 entradas** cobrindo tanto **`M6930LB`** quanto **`M6930`** com `DRV S211`, `PROG061 ` `UART_MONITOR_PANAMON1` (`DIGADIAG.FRM`, evitando corrupção BGO `0x84200000` do `[05/11/01]` no EH55) e `MAIN3410PQ` `AV_PASSTHROUGH` + HDD LBA48 dinâmico).
  * **`prepare_report.json`**, **`SHA256SUMS.txt`** & **`LEIA_ME_CARTAO_SD.txt`**: manifesto e guia verificáveis via `panadiga unbrick verify-sd firmware_and_images/sd_unbrick_kit`. *(Consulte [`docs/pt-BR/DOSSIE_FORENSE_BOOTLOADER_SDDIAG_EH55.md`](../docs/pt-BR/DOSSIE_FORENSE_BOOTLOADER_SDDIAG_EH55.md).)*
