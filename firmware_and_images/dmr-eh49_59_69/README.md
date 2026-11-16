# Panasonic DMR-EH49 / DMR-EH59 / DMR-EH69 Official Firmware Assets

Pacotes oficiais baixados diretamente do portal Panasonic Global (`av.jpn.support.panasonic.com/support/global/cs/dvd/download/eh49/`), extraídos, decifrados em plaintext AM34 e empacotados em `dmr-eh49-eh59-eh69-official-pack.zip` na Release [`firmware-pack-v0.1.0`](https://github.com/orgchrono/Panadiga/releases/tag/firmware-pack-v0.1.0).

| Diretório | Modelos Alvo | Versão | Fonte Oficial | Arquivo(s) & Tamanhos | SHA-256 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `eh49/` | DMR-EH49 / EH495 (Euro) | Ver 1.1 (2009/08/04) (MAIN 3100, PROG 071, DRV S514) | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/eh49/UPDATE_EH49_495.exe` | `UPDATE_EH49_495.exe` (7,492,387 B) · `PANA_DVD.FRM` (7,461,592 B) | `ff12c1d02e5bc8a10a78f0935177af4e70e188867af8c9b523e0638d39c59762` (`.exe`) / `6d7441246fcd1d3032297cf70df90fc64fdc93035e2f7b9dc55eae1d6903a7d8` (`.FRM`) |
| `eh59/` | DMR-EH59 / EH595 (Euro) | Ver 1.2 (2009/08/04) (MAIN 3200, PROG 071, DRV S514) | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/eh59/UPDATE_EH59_595.exe` | `UPDATE_EH59_595.exe` (7,546,243 B) · `PANA_DVD.FRM` (7,515,448 B) | `f35345fc4c0603a13c80e9bc762d64b61db72ab74ce317b7d130a0203afc46fb` (`.exe`) / `7d7f6ad21f9c0a5b0d2e1c42e39d46389f05a1a87a6a4cb964aaf3343d8df3f8` (`.FRM`) |
| `eh69/` | DMR-EH69 / EH695 (Euro) | Ver 1.1 (2009/08/04) (MAIN 3100, PROG 071, DRV L104) | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/eh69/UPDATE_EH69_695.exe` | `UPDATE_EH69_695.exe` (7,512,090 B) · `PANA_DVD.FRM` (7,567,480 B) | `20af7c95088ba7f1242ebd6a63dd08201665261f6121242e8eb58644cfa34da8` (`.exe`) / `9942349c4c175c2881c3d3e3eecf5f446b012ae28341958ed8dce46889695565` (`.FRM`) |

## Análise Forense & Plaintext AM34 (Chave Feistel `DF7215C7427FD010`)

- **Layout do Container:** TOC periódica Feistel com entradas de **64 bytes (`0x40`)** iniciando payload em `0x2000` (`8192`), usando a mesma S-Box (`FUN_4400a666` / `0x44035D84`) documentada no `DMR-EH55` (`unixtract` collection `PANA_DVD_KEYONLY "Old" = DF7215C7427FD010`).
- **`MAIN.bin` Unpacked (Plaintext AM34 RTOS):** todos os 3 pacotes descompactam para imagens planas de `13,653,820` B (`eh49`), `13,773,740` B (`eh59`) e `13,886,636` B (`eh69`) com cabeçalho `dc44` / base lógica `0x50622000`, contendo `src/fmupre/pre_update_firm.c` (`<deviceinfo> model = %s, oem = %c`) e driver serial `_dev_SERIAL`.
- **`PROG.bin` Unpacked (`262,144` B Plaintext Bootloader + DDR Calibration):** vetor inicial `0x00F2F3FE`, strings `[CK:DQS:L:C:H]`, `deflate 1.2.3`, mensagens `[nfrom]` / `bgo_read_D` / `NOR STATE : Original / Backup` e strings `U99` em plaintext.

| Modelo | `MAIN.bin` Bytes | `MAIN.bin` SHA-256 | Base Header (`w2`, `w3`) | `PROG.bin` SHA-256 |
| :--- | ---: | :--- | :--- | :--- |
| `EH49` | `13,653,820` | `594b6c5aa5872bb46b7fb117e21a1129ecc5ce520fd76dbf049f77e44aec25ce` | `0x513276B8` / `0x50622000` | `3bf48f806898643a1f4fbd3bf9e7bebff0d97eab25449a604dac498d0744383b` |
| `EH59` | `13,773,740` | `9d8a4bca9209bbfe135586ed94bbe902f0b721e56c45b32a9ad9b34afb38fec2` | `0x51344B28` / `0x50622000` | `3bf48f806898643a1f4fbd3bf9e7bebff0d97eab25449a604dac498d0744383b` |
| `EH69` | `13,886,636` | `a7182537e8859b83cbf082855844a2845fcd57303fa1113898721708cbc435b2` | `0x51360428` / `0x50622000` | `3bf48f806898643a1f4fbd3bf9e7bebff0d97eab25449a604dac498d0744383b` |
