# DMR-ES35V (AM34 / Spansion 8 MiB NOR + 24C64 EEPROM Reference Dump)

> [!TIP]
> **Download Direto do Pacote Verificado (`firmware-pack-v0.1.0`):**  
> [`dmr-es35v-8mb-nor-flash-and-eeprom-triton-dump.zip`](https://github.com/orgchrono/Panadiga/releases/download/firmware-pack-v0.1.0/dmr-es35v-8mb-nor-flash-and-eeprom-triton-dump.zip) (`13.599.723 bytes`, SHA-256 `800f3874d4a3ed2bacf90780a7f6f666cb9f8b5a3b2e6bf9dfd9529f2b50cb32`)

Dump físico completo lido diretamente dos chips da placa (programador **Triton v5.7**, arquivo fonte `21200_dmr-es25v.rar`) contendo a imagem **8 MiB plaintext da NOR Flash** (mesma família de arquitetura AM34/Spansion do DMR-EH55) e **8 KiB da EEPROM I²C 24C64**, mais os módulos `DRV` e `MAIN` previamente extraídos/descomprimidos para análise direta no Ghidra/IDA.

---

## 1. Inventário e Hashes Verificados

| Arquivo no Pacote ZIP | Tamanho (Bytes) | CRC32 | SHA-256 | Descrição |
| :--- | ---: | :---: | :--- | :--- |
| **`flash.bin`** | `8.388.608` | `B246C174` | `0969f2580ad30b98027271a8e3947f5a6ae1c02dec06462b87ce1057a7a1bcf9` | Dump físico completo de 8 MiB da Flash NOR em plaintext (`Loader ver0.18 [06/03/13]`) |
| **`eeprom.bin`** | `8.192` | `66A845DB` | `17bb5383c287dc90a493aa5c67f729dbb54b587224b267940e1e7133aa2ce32c` | Dump da EEPROM I²C 8 KiB (registros `GS176031`, `@RITEK`, `@MBIPG101R05`) |
| **`unpacked/DRV_0x080000.bin`** | `1.441.792` | — | `f6655390b3769bf278571c561b976e59489a2b075eab218d3b2095ef8f9800c7` | Subsistema óptico descompactado (`Zlib` `0x080000`, `dst=0x90202000`; referencia `DMR-EH55`) |
| **`unpacked/MAIN_3080GR_0x430000.bin`** | `6.739.752` | — | `841a5419506e995a31d0a3ef6583051a4f46500a5dda834c70edabbfd07732c1` | Kernel `MAIN` build `3080GR` descompactado (`LZMA` `0x430000`, `dst=0x52DF6000`) |
| **`21200_dmr-es25v.rar`** | `4.599.982` | `7C91A6E7` | `ffc3540b93144eb6e75c82076a4f0e48bf537f030800156548e9d1bde59678d4` | Arquivo `.rar` original preservado integralmente |
| **`dmr-es35v.txt`** | `55` | `2A1061D5` | `d1a84f7e0406a17beee2d40078ce5ddc96f5293836c600062e6188cfcd743372` | Nota original do autor (`Panasonic dmr-es35v / Eeprom / Flash / Прошивка лечит U61!`) |

---

## 2. Mapa Físico da NOR Flash (`flash.bin` — `0x000000` a `0x7FFFFF`)

| Faixa de Endereço | Tamanho | Tipo / Algoritmo | Conteúdo e Observações Técnicas |
| :--- | ---: | :--- | :--- |
| `0x000000..0x03FFFF` | `256 KB` | Plaintext AM34 XIP | **`Loader ver0.18 [06/03/13]`** (`MATSHITA DVD-RAM RECORDER`, `serial boot mode`, `Read fvu program from Nor-FROM`, `RV NG U99`) |
| `0x040000..0x07FFFF` | `256 KB` | Erased (`0xFF`) | Espaço reservado após o bootloader |
| `0x080000..0x12C0C0` | `~688 KB` | `EXTRHEAD` `"DRV "` (`algo=0` Zlib) | Firmware do drive óptico $\rightarrow$ descomprime para `1.441.792` B em `0x90202000` (contém strings `DMR-EH55` / `DMR-EH55S159`) |
| `0x130000..0x40FFFF` | `~2.88 MB` | Plaintext AM34 XIP `MAIN` | Núcleo residente executando direto da NOR (`_dev_SERIAL`, `src/fmupre/pre_update_firm.c`, `[fmu] <deviceinfo> model = %s, oem = %c`) |
| `0x410000..0x42FFFF` | `128 KB` | Erased (`0xFF`) | Alinhamento de setor de Flash |
| `0x430000..0x6D42F0` | `~2.64 MB` | `EXTRHEAD` `"MAIN"` (`algo=3` LZMA) | Build **`3080GR`** $\rightarrow$ descomprime para `6.739.752` B em `0x52DF6000` |
| `0x6E0000..0x6FFFFF` | `128 KB` | Erased (`0xFF`) | Área livre anterior ao FS BGO |
| `0x700000..0x7EFFFF` | `960 KB` | **BGO Flash FS** (`15 × 64 KB`) | Sistema de arquivos com balanceamento de desgaste marcado por `FF FF FF FF <id> F2 FF FF` (`01f2`..`0df2`) |
| `0x7F0000..0x7FFFFF` | `64 KB` | **EEMUL / NVRAM** (`8 × 8 KB`) | Blocos ativos `02f1` (`0x7F2000`), `01f1` (`0x7F4000`), `03f1` (`0x7F8000`) — confirma exatamente a janela alvo de sanitização para `U99` |
