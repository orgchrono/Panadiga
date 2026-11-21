# Panasonic DMR-EX / DMR-EZ Official Firmware Matrix (19 Extracted Packages + 32 Live CDN URLs)

Coleção de pacotes oficiais baixados diretamente de `fdl.jpn.support.panasonic.com`, extraídos, verificados por SHA-256 e redistribuídos em `dmr-ex-ez-series-17models-official-pack.zip` (`262,322,776` B, SHA-256 `7f0910e63dacbf332951330e7015b82330198733e77d3a9c1197a2b0dcd51d02`) — além do pacote de binários plaintext desempacotados `dmr-am34-unpacked-plaintext-bootloaders-and-kernels.7z` (`34,003,390` B, SHA-256 `b4b25f02e16fa425fc146d3486507385dfad03ae4b94f24237abcb8c5299d747`) — na Release [`firmware-pack-v0.1.0`](https://github.com/orgchrono/Panadiga/releases/tag/firmware-pack-v0.1.0).

> **Nota:** Conforme [`.gitignore`](../../.gitignore), binários proprietários (`*.exe`, `*.FRM`, `*.bin`) não são versionados diretamente no Git e são distribuídos via asset empacotado com manifesto SHA-256.

## 1. Pacotes Baixados, Extraídos e Auditados (19 Subdiretórios)

| Subdir | Instalador Oficial (`.exe`) | `.exe` Bytes | `PANA_DVD.FRM` Bytes | `PANA_DVD.FRM` SHA-256 | Fonte Oficial |
| :--- | :--- | ---: | ---: | :--- | :--- |
| `ex72seg/` | `UPDATE_DMR-EX72SEG_V112.exe` | `7,321,784` | `7,213,584` | `2abdfbeb8888ff66ac41f15aa1c7eb9a7bb587ea7de6d46e2bbf6dac7dd0e2f9` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex80/UPDATE_DMR-EX72SEG_V112.exe` |
| `ex769ef/` | `UPDATE_DMR-EX769EF_V14.exe` | `6,365,331` | `6,264,392` | `b1d55c1f15467341ce7becfbf3d32c50ef3564ad5a9f611dc0e859f68b29012f` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex769/UPDATE_DMR-EX769EF_V14.exe` |
| `ex773ec/` | `UPDATE_DMR-EX773EC_V109.exe` | `7,126,264` | `7,018,064` | `ef8ead563883809f5d6a01aaf1fdd9de8e7a64e15f51c07abb68515a16057ff6` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex773/UPDATE_DMR-EX773EC_V109.exe` |
| `ex773ep/` | `UPDATE_DMR-EX773EP_V109.exe` | `7,123,800` | `7,015,608` | `e602b452d2a981e2b3021b436d699c54158f475a3d5d1a5161d8aabafa1f0e42` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex773/UPDATE_DMR-EX773EP_V109.exe` |
| `ex78en/` | `UPDATE_DMR-EX78EN_V15.exe` | `6,877,384` | `6,770,928` | `11f72af33eef53c16d2841bc4a8011fd6b2e054dabbff25a3c4936dc05b238f8` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex78/UPDATE_DMR-EX78EN_V15.exe` |
| `ex795ep/` | `UPDATE_EX795EP_V108.exe` | `2,516,032` | `2,297,800` | `9e0c849d7253cd5163fcc86959c52d71e8ca60ed7c3322a2545899f699f1df39` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex795/UPDATE_EX795EP_V108.exe` |
| `ex79_89_769eb/` | `Update_EX79_EX89_EX769EB_V105.exe` | `1,795,184` | `1,576,960` | `3b78fb68384fe4ffb26aaef28067cc3a336c7036696fda6b4ce09beff53acbfe` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex79/Update_EX79_EX89_EX769EB_V105.exe` |
| `ex79ef/` | `UPDATE_DMR-EX79EF_V14.exe` | `13,806,904` | `13,700,448` | `7ca219d3a40779674a4402bcf145d51abbc71ab17a1353af1bc334cfba758ca8` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex79/UPDATE_DMR-EX79EF_V14.exe` |
| `ex80seg/` | `Update_DMR-EX80SEG_V115.exe` | `7,355,464` | `7,453,680` | `bb63c998a4f9200d5637c65ca8b1c4e5a19a473ded982ce813d242422734066b` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex80/Update_DMR-EX80SEG_V115.exe` |
| `ex81s_71seg/` | `UPDATE_DMR-EX81S_71SEG_V112.exe` | `7,002,744` | `6,894,544` | `df2c572206763c6fdd4309d02881a478b999c54e1bb157bb8c2ee5e92e3599a1` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex80/UPDATE_DMR-EX81S_71SEG_V112.exe` |
| `ex83ec/` | `UPDATE_DMR-EX83EC_V109.exe` | `8,215,768` | `8,107,576` | `4d2929133a67562c4e52a5a091ecccda117f37f2e35c75fa53e1a198a40cc7c9` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex83/UPDATE_DMR-EX83EC_V109.exe` |
| `ex83ep/` | `UPDATE_DMR-EX83EP_V109.exe` | `7,931,720` | `7,823,528` | `9115c7a9dd517c35376745163cbeca2d5ca73c6681683e8acff7dfee2ebbfbaf` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex83/UPDATE_DMR-EX83EP_V109.exe` |
| `ex84ceg/` | `UPDATE_DMR-EX84CEG_V107303.exe` | `10,413,976` | `10,305,272` | `216a7b72ae117be26a07b319e231bc8e19816b3fc37bfe4551c76e2723789771` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex84/UPDATE_DMR-EX84CEG_V107303.exe` |
| `ex86_83_773_eb/` | `Update_EX86_EX83_EX773_EX99V_EZ49VEB_V107.exe` | `1,792,600` | `1,576,960` | `8237b06fd4cce0dbedf32a9ecbf7cdcefa3ca18abff66f5ead1a2fb9b3576dac` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex86/Update_EX86_EX83_EX773_EX99V_EZ49VEB_V107.exe` |
| `ex88en/` | `UPDATE_DMR-EX88EN_V15.exe` | `6,946,168` | `6,839,712` | `9598c3722caac7476a920cbbfa91d98d1650707120073c67b85396cf78df4eae` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex88/UPDATE_DMR-EX88EN_V15.exe` |
| `ex89ef/` | `UPDATE_DMR-EX89EF_V14.exe` | `7,502,744` | `7,396,288` | `e7a3368cd94c16017d985901bc34b4ac7e2bad8d64594372de7bb460fe2763a3` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex89/UPDATE_DMR-EX89EF_V14.exe` |
| `ex93ceg/` | `UPDATE_DMR-EX93CEG_V139324.exe` | `10,654,152` | `10,545,448` | `5ca06ba7a41ab9f41462b4f2b957b7d434c23cfe29a3dbc58f583392894a480e` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex93/UPDATE_DMR-EX93CEG_V139324.exe` |
| `ez49veb/` | `UPDATE_EZ49VEB_V11.exe` | `6,150,285` | `6,049,344` | `a4c5d8b2b7fdb4e69f4081348ba73f23dc43e9f83c5068136834403e2221816c` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ez49v/UPDATE_EZ49VEB_V11.exe` |
| `ez49vec/` | `UPDATE_DMR-EZ49VEC_V109.exe` | `5,986,400` | `5,878,208` | `7fb924b9c71b92ef246c1015dfed944c20c760147fe7a3de47ad32ba76b52af5` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ez49v/UPDATE_DMR-EZ49VEC_V109.exe` |

## 2. Descobertas Estruturais & Parentesco Direto com `DMR-EH55` (`0x57000000`)

- **`ex80seg/` (`DMR-EX80SEG`, HW `M6932EG` / `PZ-317`):** compartilha exatamente o layout **TOC Feistel de 48 bytes (`0x1800`)**, `PROG.bin` com cabeçalho `0x14DC` (`262,144` B) e **`MAIN.bin` (`12,138,476` B) com endereço base `0x57000000` (`0x57B93768`)** — idêntico ao mapa do `DMR-EH55` (`0x57000000..0x57CCE408`).
- **`ex81s_71seg/` (`DMR-EX81S` / `EX71SEG`):** `MAIN.bin` (`12,409,484` B) também usa base `0x57000000` (`0x57BD5A08`), combinado com TOC Feistel 64B e bootloader `PROG.bin` (`0x00F2F3FE`, `[CK:DQS]`).
- **`ex72seg`, `ex769ef`, `ex773ec/ep`, `ex83ec/ep`, `ex84ceg`, `ex89ef`, `ex93ceg`:** utilizam TOC Feistel 64B + chave `DF7215C7427FD010` com `MAIN.bin` plano em `0x50622000` contendo `<deviceinfo>` / `pre_update_firm` / `_dev_SERIAL` + bootloader `PROG.bin` plaintext (`0x00F2F3FE`, `[nfrom]` / `bgo_read_D`).
- **`ex795ep`, `ex79_89_769eb`, `ex86_83_773_eb`:** pacotes focados em sintonizador DVB (`STB.bin` plaintext com header `CRC:... / size:...` + `PROG.bin` bootloader).

## 3. Catálogo Completo de URLs Oficiais Verificadas (`av.jpn.support.panasonic.com` / `fdl.jpn.support.panasonic.com`)

| Linha | URL Oficial Direta (HTTP 200) |
| :--- | :--- |
| `eh49` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/eh49/UPDATE_EH49_495.exe` |
| `eh49` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/eh59/UPDATE_EH59_595.exe` |
| `eh49` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/eh69/UPDATE_EH69_695.exe` |
| `ex769` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex769/UPDATE_DMR-EX769EF_V14.exe` |
| `ex769` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex79/Update_EX79_EX89_EX769EB_V105.exe` |
| `ex773` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex773/UPDATE_DMR-EX773EC_V109.exe` |
| `ex773` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex773/UPDATE_DMR-EX773EP_V109.exe` |
| `ex773` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex86/Update_EX86_EX83_EX773_EX99V_EZ49VEB_V107.exe` |
| `ex78` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex78/UPDATE_DMR-EX78EN_V15.exe` |
| `ex79` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex79/UPDATE_DMR-EX79EF_V14.exe` |
| `ex79` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex79/Update_EX79_EX89_EX769EB_V105.exe` |
| `ex795` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex795/UPDATE_EX795EP_V108.exe` |
| `ex80` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex80/UPDATE_DMR-EX72SEG_V112.exe` |
| `ex80` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex80/UPDATE_DMR-EX81S_71SEG_V112.exe` |
| `ex80` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex80/Update_DMR-EX80SEG_V115.exe` |
| `ex83` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex83/UPDATE_DMR-EX83EC_V109.exe` |
| `ex83` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex83/UPDATE_DMR-EX83EP_V109.exe` |
| `ex83` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex86/Update_EX86_EX83_EX773_EX99V_EZ49VEB_V107.exe` |
| `ex84c` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex84/UPDATE_DMR-EX84CEG_V107303.exe` |
| `ex86` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex86/Update_EX86_EX83_EX773_EX99V_EZ49VEB_V107.exe` |
| `ex88` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex88/UPDATE_DMR-EX88EN_V15.exe` |
| `ex89` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex79/Update_EX79_EX89_EX769EB_V105.exe` |
| `ex89` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex89/UPDATE_DMR-EX89EF_V14.exe` |
| `ex93c` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex93/UPDATE_DMR-EX93CEG_V139324.exe` |
| `ex97` (UK/EB) | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex97/UPDATE_EX97EB_V110.zip` — **Baixado/Extraído** (`ex97eb/`): `.zip` `107,549,814 B` (`fc444e8d06fc710611664fb771726e9bbe9cb767b7aee483b501dd7c4d6e1ee2`), `PANAEUSB.FRM` `107,417,648 B` (`f62bc64672fe3931a1987ff8d76098ce755e1b4647ad6f3983874c154e892fb2`) — container USB não-Feistel (cifra desconhecida) |
| `ex97` (SEG/CEG) | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex97/UPDATE_EX97SEG_EX97CEG_V105.exe` — **Baixado/Extraído** (`ex97seg_ceg/`): `.exe` `112,370,424 B` (`dcee8c2c85cfd0f8198551d096062a3d9a100ac7b2e4ba622cd303c1c97853e5`), `PANAEUSB.FRM` `112,152,576 B` (`25131e36cd0e1912d7be4d3ce26c222b0670cacb289b197ce60119c0943a65f8`) — container USB não-Feistel (cifra desconhecida) |
| `ez49v` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ez49v/UPDATE_DMR-EZ49VEC_V109.exe` |
| `ez49v` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ez49v/UPDATE_EZ49VEB_V11.exe` |
| `xs350` | `https://fdl.jpn.support.panasonic.com/pc/en/bd/bs850/UPDATE_BS850_BS750_XS350EG_V16.exe` |
| `xs385` | `https://fdl.jpn.support.panasonic.com/pc/en/bd/bs885/UPDATE_BS880_BS780_XS380EB_V141.exe` |
| `xs385` | `https://fdl.jpn.support.panasonic.com/pc/en/bd/bs885/UPDATE_BS885_BS785_XS385EG_V140.exe` |
| `xw300` | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/xw300/UPDATE_XW300GN_V17.exe` |
