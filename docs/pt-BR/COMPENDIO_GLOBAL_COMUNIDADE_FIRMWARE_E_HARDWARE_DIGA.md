> 🌐 **Language / Idioma:** [English (US)](../en-US/GLOBAL_COMMUNITY_COMPENDIUM_DIGA_FIRMWARE_AND_HARDWARE.md) | **Português (BR)**
> 🧭 **Portal:** [Índice de Documentos](../INDEX.md) | [Auditoria Completa](../../AUDITORIA_COMPLETA.md) | [Auditorias Históricas](../audits/README.md) | [Pseudocódigo Ghidra](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Logs de Depuração](../debug_logs/README.md)

# Compêndio Global da Comunidade: Firmware, Hardware e Engenharia Reversa Panasonic DIGA

## Referência Canônica de Matrizes Oficiais, Bases Internacionais, Modos de Serviço e Ferramentas Forenses

**Documento:** `COMPENDIO_GLOBAL_COMUNIDADE_FIRMWARE_E_HARDWARE_DIGA.md`
**Autor:** Especialista em Inteligência Comunitária e Dossiês Técnicos Bilíngues
**Ecossistema:** Panadiga — Engenharia Reversa, Forense de Storage e Laboratório de Hardware DMR-EH55
**Data:** Novembro de 2026
**Classificação:** Referência Técnica Canônica / Inteligência Global de Engenharia Reversa

---

## Sumário Executivo

A linha de gravadores digitais de DVD/HDD **Panasonic DIGA** (abrangendo as famílias DMR-E, DMR-EH, DMR-EX, DMR-EZ e as gerações posteriores baseadas na arquitetura Matsushita/UniPhier AM34, MN103S e MN2DS) representou o ápice do design eletrônico japonês para gravação de consumo entre 2003 e 2012. No entanto, sua arquitetura fortemente protegida por DRM industrial (CPRM, C-BSA, Macrovision, assinaturas proprietárias MBR `"SH"`, criptografia Feistel de firmware em blocos de 64 bits e emaranhamento estrito entre sub-CPUs e drive óptico) transformou diagnósticos de bancada e recuperações de falhas críticas — em especial o infame código **`U99`** — em verdadeiros enigmas técnicos.

Ao longo de duas décadas, o conhecimento sobre esses gravadores desenvolveu-se de forma fragmentada em ecossistemas fechados pelo mundo:

1. **Comunidade Técnica do Japão (2channel / 5ch / fóruns ncos1):** Investigou os comandos proprietários de gravação contínua AV do padrão ATA-7, setores de teste com a assinatura `"Panasonic DIGA RTSC"` e os menus de serviço de fábrica (*Service Mode* `SERV`) acessíveis por códigos de controle remoto.
2. **Reparadores do Leste Europeu (`remont-aud.net`, `monitor.espec.ws`, `telemaster.ru`):** Documentaram esquemas elétricos, realizaram dumps físicos com programadores universais (Triton, Orange, ChipProg) de memórias Flash TSOP-48 e EEPROMs I²C (`24LC64`), e correlacionaram falhas de capacitores eletrolíticos nas fontes de alimentação chaveadas com corrupções de NVRAM e o travamento `U99`.
3. **Engenheiros e Entusiastas Poloneses e Alemães (`elektroda.pl`, `hardwarefetish.com`):** Mapearam substituições de discos rígidos, clonagem de setores de boot, descompressão de partições de firmware e as primeiras tentativas de engenharia reversa do sistema de arquivos MEIHDFS.
4. **Comunidade Open-Source Moderna de Engenharia Reversa (`unixtract`, `pana-fw`, `panasonic-rec`, `dvr-recover`, `sddl_dec`):** Identificou as 12 chaves simétricas Feistel (`PANA_DVD`), os esquemas de encapsulamento `EXTRHEAD`/`EXTRFOOT`, as variantes de compressão LZSS (Okumura) e os consoles seriais SLM (*Serial Log Menu*) em equipamentos UniPhier com Linux embarcado.

Este compêndio unifica e valida rigorosamente todos esses achados em uma única especificação canônica, dividida em cinco grandes seções técnicas (seguidas da matriz de síntese e das referências correlatas), incorporando integralmente a crônica de engenharia reversa do projeto Panadiga até o commit `475fd48`.

---

## 1. Matriz Global de Firmware Oficial (`av.jpn.support.panasonic.com` / `fdl.jpn.support.panasonic.com`)

A Panasonic manteve sua infraestrutura oficial de distribuição de firmwares em servidores CDN baseados no Japão (`fdl.jpn.support.panasonic.com` e `av.jpn.support.panasonic.com`). O ecossistema Panadiga rastreou, baixou, auditou e calculou hashes criptográficos SHA-256 de todas as 30 URLs oficiais ativas conhecidas, descompactando os contêineres executáveis auto-extraíveis (`.exe`) e extraindo os arquivos de imagem óptica (`PANA_DVD.FRM`), cartões de memória (`PANA__SD.FRM`), bootloaders em plaintext (`PROG.bin`) e núcleos de sistema operacional (`MAIN.bin`).

### 1.1 Catálogo Completo de URLs Oficiais Verificadas (30 Links Ativos — HTTP 200)

Todas as 30 URLs a seguir foram testadas e validadas com retorno `HTTP/1.1 200 OK`:

| # | Linha / Família | URL Oficial Direta (CDN Panasonic Japão)                                                              |
| :-: | :--------------- | :----------------------------------------------------------------------------------------------------- |
| 1 | `eh49`         | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/eh49/UPDATE_EH49_495.exe`                           |
| 2 | `eh49`         | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/eh59/UPDATE_EH59_595.exe`                           |
| 3 | `eh49`         | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/eh69/UPDATE_EH69_695.exe`                           |
| 4 | `ex769`        | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex769/UPDATE_DMR-EX769EF_V14.exe`                   |
| 5 | `ex769`        | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex79/Update_EX79_EX89_EX769EB_V105.exe`             |
| 6 | `ex773`        | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex773/UPDATE_DMR-EX773EC_V109.exe`                  |
| 7 | `ex773`        | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex773/UPDATE_DMR-EX773EP_V109.exe`                  |
| 8 | `ex773`        | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex86/Update_EX86_EX83_EX773_EX99V_EZ49VEB_V107.exe` |
| 9 | `ex79`         | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex79/UPDATE_DMR-EX79EF_V14.exe`                     |
| 10 | `ex79`         | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex79/Update_EX79_EX89_EX769EB_V105.exe`             |
| 11 | `ex795`        | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex795/UPDATE_EX795EP_V108.exe`                      |
| 12 | `ex80`         | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex80/UPDATE_DMR-EX72SEG_V112.exe`                   |
| 13 | `ex80`         | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex80/UPDATE_DMR-EX81S_71SEG_V112.exe`               |
| 14 | `ex80`         | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex80/Update_DMR-EX80SEG_V115.exe`                   |
| 15 | `ex83`         | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex83/UPDATE_DMR-EX83EC_V109.exe`                    |
| 16 | `ex83`         | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex83/UPDATE_DMR-EX83EP_V109.exe`                    |
| 17 | `ex83`         | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex86/Update_EX86_EX83_EX773_EX99V_EZ49VEB_V107.exe` |
| 18 | `ex84c`        | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex84/UPDATE_DMR-EX84CEG_V107303.exe`                |
| 19 | `ex86`         | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex86/Update_EX86_EX83_EX773_EX99V_EZ49VEB_V107.exe` |
| 20 | `ex89`         | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex79/Update_EX79_EX89_EX769EB_V105.exe`             |
| 21 | `ex89`         | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex89/UPDATE_DMR-EX89EF_V14.exe`                     |
| 22 | `ex93c`        | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex93/UPDATE_DMR-EX93CEG_V139324.exe`                |
| 23 | `ex97`         | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex97/UPDATE_EX97EB_V110.zip`                        |
| 24 | `ex97`         | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ex97/UPDATE_EX97SEG_EX97CEG_V105.exe`               |
| 25 | `ez49v`        | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ez49v/UPDATE_DMR-EZ49VEC_V109.exe`                  |
| 26 | `ez49v`        | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/ez49v/UPDATE_EZ49VEB_V11.exe`                       |
| 27 | `xs350`        | `https://fdl.jpn.support.panasonic.com/pc/en/bd/bs850/UPDATE_BS850_BS750_XS350EG_V16.exe`            |
| 28 | `xs385`        | `https://fdl.jpn.support.panasonic.com/pc/en/bd/bs885/UPDATE_BS880_BS780_XS380EB_V141.exe`           |
| 29 | `xs385`        | `https://fdl.jpn.support.panasonic.com/pc/en/bd/bs885/UPDATE_BS885_BS785_XS385EG_V140.exe`           |
| 30 | `xw300`        | `https://fdl.jpn.support.panasonic.com/pc/en/dvd/xw300/UPDATE_XW300GN_V17.exe`                       |

### 1.2 Pacotes Auditados, Extraídos e Hashes SHA-256

Os pacotes baixados compreendem as gerações clássicas (`DMR-EH55`, `DMR-EH50`), a linha tardia analógica/HDMI (`EH49`, `EH59`, `EH69`) e a linha com sintonizadores digitais DVB-T/DVB-S (`DMR-EX`, `DMR-EZ`).

#### Matriz DMR-EH49 / DMR-EH59 / DMR-EH69

* Chave Feistel do contêiner: `DF7215C7427FD010` (coleção `PANA_DVD_KEYONLY "Old"`).
* Estrutura de TOC: 64 bytes (`0x40`) com payload iniciando em `0x2000` (8.192 bytes).

| Modelo                     | Instalador (`.exe`) Bytes & SHA-256                                               | `PANA_DVD.FRM` Bytes & SHA-256                                                    | `MAIN.bin` Bytes / Base / SHA-256                                                                                 | `PROG.bin` SHA-256                                                 |
| :------------------------- | :---------------------------------------------------------------------------------- | :---------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------ | :------------------------------------------------------------------- |
| **DMR-EH49** / EH495 | `7,492,387` B`ff12c1d02e5bc8a10a78f0935177af4e70e188867af8c9b523e0638d39c59762` | `7,461,592` B`6d7441246fcd1d3032297cf70df90fc64fdc93035e2f7b9dc55eae1d6903a7d8` | `13,653,820` B`0x50622000` (`0x513276B8`)`594b6c5aa5872bb46b7fb117e21a1129ecc5ce520fd76dbf049f77e44aec25ce` | `3bf48f806898643a1f4fbd3bf9e7bebff0d97eab25449a604dac498d0744383b` |
| **DMR-EH59** / EH595 | `7,546,243` B`f35345fc4c0603a13c80e9bc762d64b61db72ab74ce317b7d130a0203afc46fb` | `7,515,448` B`7d7f6ad21f9c0a5b0d2e1c42e39d46389f05a1a87a6a4cb964aaf3343d8df3f8` | `13,773,740` B`0x50622000` (`0x51344B28`)`9d8a4bca9209bbfe135586ed94bbe902f0b721e56c45b32a9ad9b34afb38fec2` | `3bf48f806898643a1f4fbd3bf9e7bebff0d97eab25449a604dac498d0744383b` |
| **DMR-EH69** / EH695 | `7,512,090` B`20af7c95088ba7f1242ebd6a63dd08201665261f6121242e8eb58644cfa34da8` | `7,567,480` B`9942349c4c175c2881c3d3e3eecf5f446b012ae28341958ed8dce46889695565` | `13,886,636` B`0x50622000` (`0x51360428`)`a7182537e8859b83cbf082855844a2845fcd57303fa1113898721708cbc435b2` | `3bf48f806898643a1f4fbd3bf9e7bebff0d97eab25449a604dac498d0744383b` |

#### Matriz Séries DMR-EX e DMR-EZ (17 Subdiretórios Auditados)

| Subdiretório       | Instalador Oficial (`.exe`)                     | Bytes `.exe` | Bytes `.FRM` | SHA-256 do Arquivo `.FRM`                                          |
| :------------------ | :------------------------------------------------ | -------------: | -------------: | :------------------------------------------------------------------- |
| `ex72seg/`        | `UPDATE_DMR-EX72SEG_V112.exe`                   |  `7,321,784` |  `7,213,584` | `2abdfbeb8888ff66ac41f15aa1c7eb9a7bb587ea7de6d46e2bbf6dac7dd0e2f9` |
| `ex769ef/`        | `UPDATE_DMR-EX769EF_V14.exe`                    |  `6,365,331` |  `6,264,392` | `b1d55c1f15467341ce7becfbf3d32c50ef3564ad5a9f611dc0e859f68b29012f` |
| `ex773ec/`        | `UPDATE_DMR-EX773EC_V109.exe`                   |  `7,126,264` |  `7,018,064` | `ef8ead563883809f5d6a01aaf1fdd9de8e7a64e15f51c07abb68515a16057ff6` |
| `ex773ep/`        | `UPDATE_DMR-EX773EP_V109.exe`                   |  `7,123,800` |  `7,015,608` | `e602b452d2a981e2b3021b436d699c54158f475a3d5d1a5161d8aabafa1f0e42` |
| `ex795ep/`        | `UPDATE_EX795EP_V108.exe`                       |  `2,516,032` |  `2,297,800` | `9e0c849d7253cd5163fcc86959c52d71e8ca60ed7c3322a2545899f699f1df39` |
| `ex79_89_769eb/`  | `Update_EX79_EX89_EX769EB_V105.exe`             |  `1,795,184` |  `1,576,960` | `3b78fb68384fe4ffb26aaef28067cc3a336c7036696fda6b4ce09beff53acbfe` |
| `ex79ef/`         | `UPDATE_DMR-EX79EF_V14.exe`                     | `13,806,904` | `13,700,448` | `7ca219d3a40779674a4402bcf145d51abbc71ab17a1353af1bc334cfba758ca8` |
| `ex80seg/`        | `Update_DMR-EX80SEG_V115.exe`                   |  `7,355,464` |  `7,453,680` | `bb63c998a4f9200d5637c65ca8b1c4e5a19a473ded982ce813d242422734066b` |
| `ex81s_71seg/`    | `UPDATE_DMR-EX81S_71SEG_V112.exe`               |  `7,002,744` |  `6,894,544` | `df2c572206763c6fdd4309d02881a478b999c54e1bb157bb8c2ee5e92e3599a1` |
| `ex83ec/`         | `UPDATE_DMR-EX83EC_V109.exe`                    |  `8,215,768` |  `8,107,576` | `4d2929133a67562c4e52a5a091ecccda117f37f2e35c75fa53e1a198a40cc7c9` |
| `ex83ep/`         | `UPDATE_DMR-EX83EP_V109.exe`                    |  `7,931,720` |  `7,823,528` | `9115c7a9dd517c35376745163cbeca2d5ca73c6681683e8acff7dfee2ebbfbaf` |
| `ex84ceg/`        | `UPDATE_DMR-EX84CEG_V107303.exe`                | `10,413,976` | `10,305,272` | `216a7b72ae117be26a07b319e231bc8e19816b3fc37bfe4551c76e2723789771` |
| `ex86_83_773_eb/` | `Update_EX86_EX83_EX773_EX99V_EZ49VEB_V107.exe` |  `1,792,600` |  `1,576,960` | `8237b06fd4cce0dbedf32a9ecbf7cdcefa3ca18abff66f5ead1a2fb9b3576dac` |
| `ex89ef/`         | `UPDATE_DMR-EX89EF_V14.exe`                     |  `7,502,744` |  `7,396,288` | `e7a3368cd94c16017d985901bc34b4ac7e2bad8d64594372de7bb460fe2763a3` |
| `ex93ceg/`        | `UPDATE_DMR-EX93CEG_V139324.exe`                | `10,654,152` | `10,545,448` | `5ca06ba7a41ab9f41462b4f2b957b7d434c23cfe29a3dbc58f583392894a480e` |
| `ez49veb/`        | `UPDATE_EZ49VEB_V11.exe`                        |  `6,150,285` |  `6,049,344` | `a4c5d8b2b7fdb4e69f4081348ba73f23dc43e9f83c5068136834403e2221816c` |
| `ez49vec/`        | `UPDATE_DMR-EZ49VEC_V109.exe`                   |  `5,986,400` |  `5,878,208` | `7fb924b9c71b92ef246c1015dfed944c20c760147fe7a3de47ad32ba76b52af5` |

### 1.3 Diferenças Estruturais de TOC: Feistel 48B vs Feistel 64B vs Plaintext `PANA__SD.FRM`

A análise da Tabela de Conteúdos (TOC) presente nos primeiros setores dos arquivos de atualização revelou três variantes arquiteturais principais:

```mermaid
flowchart TD
    subgraph TOC_48B["1. TOC Feistel 48 Bytes (Legada: EH55, EX80SEG)"]
        E1_48["Entrada 0 em 0x0000 (48 Bytes decifrados)"] --> E2_48["Entrada 1 em 0x0030 (48 Bytes)"]
        E2_48 --> P1["Padding até 0x1800 (6.144 Bytes)"]
        P1 --> PAY1["Payloads Feistel a partir de 0x1800"]
    end

    subgraph TOC_64B["2. TOC Feistel 64 Bytes (Moderna: EH49..69, EX81S, EX72..93)"]
        E1_64["Entrada 0 em 0x0000 (64 Bytes decifrados)"] --> E2_64["Entrada 1 em 0x0040 (64 Bytes)"]
        E2_64 --> P2["Padding até 0x2000 (8.192 Bytes)"]
        P2 --> PAY2["Payloads Feistel a partir de 0x2000"]
    end

    subgraph TOC_SD["3. TOC Plaintext SD Card (PANA__SD.FRM)"]
        E1_SD["Entrada 0 em 0x0000 (48 B sd48 EH55 ver0.61 | 64 B sd64 ver0.63)"] --> P3["Tabela 0x1800 (sd48) / 0x2000 (sd64)"]
        P3 --> PAY3["Payload Modular Direto em Plaintext"]
    end
```

1. **TOC Feistel de 48 Bytes (`Stride = 0x30`, Payload a partir de `0x1800` / 6.144 B):**
   - Utilizada no `DMR-EH55` e no `DMR-EX80SEG`, iniciando diretamente no offset `0x0000` após descriptografia Feistel (`128` entradas potenciais de 48 bytes; os primeiros 16 bytes cifrados exibidos em inspeções brutas correspondem aos blocos iniciais cifrados da entrada 0).
   - Na arquitetura AM34 do `DMR-EH55`, cada entrada de 48 bytes contém: Nome (`0x00..0x0A`), Versão (`0x0A..0x0C` uint16 LE), Offset (`0x0C..0x10` uint32 LE), Família HW (`0x10..0x18`), OEM (`0x18`, `'M'`), Flag (`0x19`), Grupo Modelo (`0x1A..0x20`), Tamanho (`0x20..0x24`), Soma FVU (`0x24..0x28`), Metadados (`0x28..0x2E`) e Checksum aditivo `list_sum` de 16 bits (`0x2E..0x30`); em gerações Blu-ray mais recentes documentadas no `pana-fw`, o somatório final evolui para Adler-32 de 32 bits.
2. **TOC Feistel de 64 Bytes (`Stride = 0x40`, Payload a partir de `0x2000` / 8.192 B):**
   - Utilizada na série `EH49/59/69`, `EX81S/71SEG` e maioria dos modelos `EX72..EX93` (por exemplo, entrada 0 em `0x0000` com `PROG071`, entrada 1 em `0x0040` com `MAIN3100`), deslocando o início dos módulos para `0x2000`.
3. **TOC Plaintext de Cartão SD (`PANA__SD.FRM` — `48 Bytes sd48` vs `64 Bytes sd64`):**
   - Utilizada nos gravadores com leitor SD para recuperação direta sem depender da mecânica óptica ATAPI, mantendo os descritores em plaintext (sem Feistel). No hardware **`DMR-EH55` (`Loader ver0.61 [06/03/06]` + `PROG061`)**, o formato nativo é **`48 Bytes` (`sd48`, TOC `0x1800 B` = `6.144 B`, `list_sum` em `+0x2E`)**, pareado com `DIGADIAG.FRM` bruto de `262.144 B`; já no **`Loader ver0.63 [07/10/24]`** (`/BOOT.BIN` EZ485V/EH58), o formato é **`64 Bytes` (`sd64`, TOC `0x2000 B` = `8.192 B`, `list_sum` em `+0x3E`)**, pareado com `DIGADIAG.FRM` `EXTRHEAD` de `131.072 B`.

### 1.4 Endereços Base do `MAIN.bin`: `0x57000000` vs `0x50622000`

A extração completa do módulo principal (`MAIN.bin`) da CPU Matsushita AM34 comprovou duas grandes famílias de mapa de memória em SDRAM:

1. **Família `0x57000000` (Geração DMR-EH55, DMR-EX80SEG, DMR-EX81S/71SEG):**
   - No `DMR-EH55`, o `MAIN.bin` possui `13,427,852` bytes (SHA-256 `075f552ed583ba1835c0d44d1b533ff5a016b8d81d15d7166240098461ddcca7`) e roda mapeado de `0x57000000` a `0x57CCE408`.
   - No `DMR-EX80SEG` (Hardware `M6932EG` / `PZ-317`), o `MAIN.bin` possui `12,138,476` bytes com vetor de carga em `0x57000000` (`0x57B93768`).
   - No `DMR-EX81S / EX71SEG`, o `MAIN.bin` possui `12,409,484` bytes também mapeado na base `0x57000000` (`0x57BD5A08`).
   - Isso confirma o parentesco arquitetural direto entre `DMR-EX80SEG`, `DMR-EX81S` e `DMR-EH55`.
2. **Família `0x50622000` (Geração DMR-EH49/59/69 e DMR-EX72..EX93):**
   - Os gravadores desta geração migraram a base lógica da imagem descomprimida para `0x50622000` (com cabeçalho `dc44`).
   - `EH49`: `0x50622000` a `0x513276B8` (`13,653,820` B).
   - `EH59`: `0x50622000` a `0x51344B28` (`13,773,740` B).
   - `EH69`: `0x50622000` a `0x51360428` (`13,886,636` B).

### 1.5 Estruturas Internas, `<deviceinfo>`, `pre_update_firm.c` e `_dev_SERIAL`

Nos binários `MAIN.bin` extraídos em plaintext (tanto `0x57000000` quanto `0x50622000`), a análise de strings confirma o módulo de validação pré-gravação:

* Caminho do código-fonte compilado: `src/fmupre/pre_update_firm.c`.
* Cadeia exata observada de identificação de hardware antes da comparação modelo/OEM:
  ```c
  "[fmu] <deviceinfo> model = %s, oem = %c"
  ```
* Canal serial `_dev_SERIAL`: referenciado nativamente em todos os `MAIN.bin` auditados (`EH55`, `EX80SEG`, `EX81S`, `EH49/59/69`, `EX72..EX93`) para o protocolo de depuração/console e atualização.

### 1.6 O Bootloader em Plaintext `PROG.bin` (262.144 Bytes / 256 KB)

Diferente do contêiner Feistel cifrado, o módulo extraído `PROG.bin` (`262,144` bytes = `0x40000`) fornece código AM34/MN103S plano para análise direta em disassemblers como o Ghidra:

1. **Assinatura Inicial Divergente:**
   - Nos modelos `DMR-EH55` e `DMR-EX80SEG`, o arquivo inicia com o cabeçalho de 16 bytes `dc 14 00 00 00 ff ff ff dc e1 00 00 00 ff ff ff` (`0x14DC`).
   - Nos modelos `EH49/59/69`, `EX81S`, `EX72..EX93` e `EZ49V`, inicia diretamente com a sequência `00 F2 F3 FE 01 04 10 00 ...` (word LE `0xFEF3F200`).
2. **Rotinas de Calibração DDR (`[CK:DQS:L:C:H]` / `[CK:DQS]`):**
   - Presentes diretamente no `PROG.bin` de modelos como `EH49/59/69` (`[CK:DQS:L:C:H] = [`) e `EX81S` (`[CK:DQS] = [`).
3. **Motor de Descompressão Integrado (`deflate 1.2.3` / `inflate 1.2.2`):**
   - Embute strings `deflate 1.2.3 Copyright 1995-2005 Jean-loup Gailly` e `inflate 1.2.2 Copyright 1995-2004 Mark Adler`.
4. **Driver de Flash BGO (*Background Operation*) `[nfrom]`:**
   - Funções e logs observados em plaintext: `[nfrom] bgo_read_D`, `[nfrom] bgo_writeD`, `[nfrom] bgo_writeD_FF` e `"NOR STATE : Backup - Original"` / `"NOR STATE : Original - Backup"`.
5. **Strings `U99` em Plaintext:**
   - Todos os 23 arquivos `PROG.bin` extraídos contêm a assinatura literal `U99` além de mensagens como `Firm list read failed` e `Failed to startup drive.`.

---

## 2. Bases Internacionais de Reparo e Referência Cruzada de Hardware

O reparo de gravadores Panasonic DIGA atingiu seu maior nível de maturidade prática nas oficinas e bancadas do Leste Europeu (Rússia, Ucrânia, Belarus) e na Polônia, onde a escassez de peças de reposição e o alto custo de placas de circuito principais obrigaram os técnicos a realizar engenharia reversa nos chips de silício.

### 2.1 Fóruns Especializados do Leste Europeu e Repositórios Técnicos

As fontes canônicas de dados de hardware identificadas pelo projeto são:

1. **`remont-aud.net` (Rússia/CEI):** A mais qualificada e rigorosa base de dados de dumps de firmware e engenharia eletrônica do mundo.
2. **`elektroda.pl` (Polônia):** O maior portal europeu de eletrônica aplicada, com tópicos históricos detalhando recuperações de fontes de alimentação e trocas de drives e HDDs na linha DIGA.
3. **`monitor.espec.ws` (Rússia):** Fórum técnico de alta complexidade com discussões profundas sobre as sub-CPUs MN102/MN103 e mecânicas de drive RFKN.
4. **`telemaster.ru` (Orange Programmer Database):** Repositório de imagens binárias de EEPROMs e microcontroladores.
5. **`badcaps.net` (EUA/Global):** Análise especializada em envelhecimento de capacitores eletrolíticos e ruído elétrico de fontes chaveadas.

### 2.2 Análise Forense do `remont-aud.net` e Regras de Qualificação

O acesso aos dumps e documentações do `remont-aud.net` é rigidamente controlado. Para baixar arquivos ou visualizar anexos técnicos, o usuário deve ser aprovado no teste de admissão (*Тест 1*), composto por cálculos práticos de circuitos RLC, semicondutores, oscilogramas de fontes chaveadas e teoria de barramentos digitais. A plataforma proíbe expressamente a redistribuição comercial de seus anexos (*non-redistribution notice*), operando em regime de estrita preservação comunitária.

Casos canônicos investigados no `remont-aud.net`:

```mermaid
flowchart LR
    RA["remont-aud.net Dumps & Casos"] --> ES10["DMR-ES10E\nChassi VEP79104\nCPU MN2DS0012-H\nFlash R27V802 (Triton)"]
    RA --> EH57["DMR-EH57 / EH67\nDump DMDE LBA 0 'SH'\nPrimeiros 10 MB MEIHFS"]
    RA --> ES35["DMR-ES35V\nMecânica RFKNES35VEE\nErros U61 e U99"]
    RA --> ES15["DMR-ES15\nCPU MN2DS00151DB\nFlash S29JL064H\nEEPROM 24LC64"]
    RA --> EH58["DMR-EH58 / EH65\nEmparelhamento de Drive\nFalha de NVRAM"]
```

* **DMR-ES10E (`VEP79104`, Dump #31642):**
  - URL: `https://remont-aud.net/dump/dvd/panasonic/panasonic_dmr_es10e_shassi_main_board_vep79104/205-1-0-31642`
  - Placa principal: `VEP79104`.
  - Processador central: Matsushita `MN2DS0012-H`.
  - Memória de programa: Flash/Mask ROM Oki/Spansion `R27V802` lida fisicamente através de programador universal Triton com soquete de bancada.
* **DMR-EH57 / DMR-EH67 (Dump #5696):**
  - URL: `https://remont-aud.net/dump/dvd/panasonic/dmr_eh57_67/205-1-0-5696`
  - Contém o dump de imagem de disco rígido bruto capturado via utilitário DMDE (*DM Disk Editor*).
  - Confirma que o setor LBA 0 inicia com a assinatura de partição proprietária `"SH"`, seguido pelo setor espelho e pela estrutura de journal idêntica à do DMR-EH55 nos primeiros 10 MB.
* **DMR-ES35V (Combo DVD/VHS, Dump #21200 — Baixado e Auditado em `firmware-pack-v0.1.0`):**
  - URL original: `https://remont-aud.net/dump/dvd/panasonic/dmr_es35v/205-1-0-21200` (leitura física Triton v5.7 — pacote **`dmr-es35v-8mb-nor-flash-and-eeprom-triton-dump.zip`**, SHA-256 `800f3874d4a3ed2bacf90780a7f6f666cb9f8b5a3b2e6bf9dfd9529f2b50cb32`, incluído na Release oficial!).
  - **Conteúdo Extraído e Descompactado:**
    - `flash.bin` (`8.388.608 B`, CRC32 `B246C174`, SHA-256 `0969f2580ad30b98027271a8e3947f5a6ae1c02dec06462b87ce1057a7a1bcf9`): dump físico integral de 8 MiB da Flash NOR em plaintext.
    - `eeprom.bin` (`8.192 B`, CRC32 `66A845DB`, SHA-256 `17bb5383c287dc90a493aa5c67f729dbb54b587224b267940e1e7133aa2ce32c`): EEPROM I²C 24C64 com tabelas de calibração/estratégia de mídia (`GS176031`, `@RITEK`, `@MBIPG101R05`).
    - **Mapa Físico Verificado da NOR de 8 MiB (`flash.bin`):**
      * `0x000000..0x03FFFF` (`256 KB`): Bootloader XIP em plaintext **`Loader ver0.18 [06/03/13]`** (apenas 7 dias posterior ao `Loader ver0.61 [06/03/06]` do DMR-EH55, com as mesmas strings `MATSHITA DVD-RAM RECORDER`, `serial boot mode`, `Read fvu program from Nor-FROM`, `RV NG U99`).
      * `0x080000..0x12C0C0`: contêiner `EXTRHEAD` `"DRV "` (Zlib `algo=0`) $\rightarrow$ descomprime para **`DRV_0x080000.bin`** (`1.441.792 B`, `dst=0x90202000`, SHA-256 `f6655390b3769bf278571c561b976e59489a2b075eab218d3b2095ef8f9800c7`), contendo referências diretas a **`DMR-EH55`**, `DMR-EH55S159`, `DMR-E500F`.
      * `0x130000..0x40FFFF`: código e dados `MAIN` executados in-place (XIP) na NOR (`_dev_SERIAL`, `src/fmupre/pre_update_firm.c`, `[fmu] <deviceinfo> model = %s, oem = %c`).
      * `0x430000..0x6D42F0`: contêiner `EXTRHEAD` `"MAIN"` (LZMA `algo=3`, build **`3080GR`**) $\rightarrow$ descomprime para **`MAIN_3080GR_0x430000.bin`** (`6.739.752 B`, `dst=0x52DF6000`, SHA-256 `841a5419506e995a31d0a3ef6583051a4f46500a5dda834c70edabbfd07732c1`).
      * `0x700000..0x7EFFFF` (`15 × 64 KB`): sistema de arquivos Flash **BGO** com marcadores `FF FF FF FF <id> F2 FF FF` (`01f2`..`0df2`).
      * `0x7F0000..0x7FFFFF` (últimos `64 KB`): setores **EEMUL / NVRAM** com cabeçalhos `FF FF FF FF <id> F1 FF FF` em `0x7F2000` (`02f1`), `0x7F4000` (`01f1`) e `0x7F8000` (`03f1`) — prova física direta da região sanitizada para cura do `U99`.
  - Veja o catálogo detalhado em [`firmware_and_images/dmr-es35v/README.md`](../../firmware_and_images/dmr-es35v/README.md).
* **DMR-ES15 (`MN2DS00151DB`, Fórum #103187):**
  - URL: `https://remont-aud.net/forum/52-103187-1`
  - Placa `A-3 863ZES MP-1`, CPU `MN2DS00151DB`.
  - Utiliza exatamente a mesma memória Flash Spansion `S29JL064H70TFI00` (8 MB TSOP-48) do DMR-EH55, acompanhada de uma EEPROM externa Microchip `24LC64`. O autor comprova que uma corrupção de bits na `24LC64` ou nos setores superiores da Spansion causa travamento fatal em `U99`.
* **DMR-EH58 (Fórum #126813) e DMR-EH65 (Fórum #33721):**
  - Discussões detalhadas sobre recuperação de setores com bad blocks em HDDs IDE/SATA e a regeneração de partições MEIHFS via dump de setores hexadecimais puros.

### 2.3 Comunidade Polonesa de Engenharia de Hardware (`elektroda.pl`)

O portal `elektroda.pl` preserva discussões e anexos técnicos relevantes da família DIGA:

1. **Tópico `topic2896624.html` (`dmr-ex75-ex85.zip`):**
   - Disponibiliza o pacote de inicialização para substituição de HDD, erros `HDD Error`, loop de ejeção de bandeja e falhas `U81`.
2. **Tópico `topic1598940.html` e `topic1320603.html` (`DMR-EH55`):**
   - Discussões sobre variantes regionais europeias/norte-americanas de firmware, protocolos de recuperação em Flash e serviço de motor do drive.
3. **Tópicos `topic2323763.html`, `topic3378979.html`, `topic3082439.html`:**
   - Procedimentos de substituição e clonagem 1:1 de HDD nos modelos `EX75`, `EX77` e `EX85` (incluindo comandos de inicialização em Service Mode).
4. **Tópico `topic3500975.html` (`DMR-EH59`) e `topic1800763.html` (`DMR-EH67`):**
   - Discussões de armazenamento/arquitetura no `DMR-EH59` e diagnósticos de erros `U61`/`U88` e loops de boot no `DMR-EH67`.

### 2.4 Fóruns Técnicos Russos (`monitor.espec.ws` e `telemaster.ru`)

1. **`monitor.espec.ws` — Tópico `topic100315.html` (`DMR-ES15` com erro `U61`):**
   - Analisa a mecânica óptica `RFKNES15PL`, o circuito de alimentação `MR4020` e recalibração do drive associada ao código `U61`.
2. **`monitor.espec.ws` — Tópico `topic121264.html` e `/section24/topic323568.html` (`DMR-ES35V` e Erro `U99` / `U61`):**
   - Correlaciona falhas de NVRAM/Flash no conjunto digital `RFKNES35VEE` ao código `U99` e documenta guias de recuperação de `U61`.
3. **`monitor.espec.ws` — Tópico `topic203970.html` (`DMR-EH58` — Pareamento e `U61`/`U88`):**
   - Discussão detalhada de pareamento entre placa digital / drive óptico na família `DMR-EH58` e falhas `U61` / `U88`.
4. **`monitor.espec.ws` — Tópico `topic270939.html` e `topic165151.html` (`DMR-E85H`):**
   - Discussões técnicas dedicadas a reparo de fonte de alimentação e falha de boot no `DMR-E85H`.
5. **`telemaster.ru` (Base de Dados Orange Programmer):**
   - Repositório localizado em `http://www.telemaster.ru/orange/proshivki4.html` contendo imagens binárias de EEPROMs Panasonic (`24C32`, `24C64`, `93C46`) e discussões históricas sobre desbloqueio multi-região via EEPROM no fórum `cnclab`.

### 2.5 Comunidades Ocidentais e Acervos de Preservação

1. **`badcaps.net` (Tópico #75610 e #3468935 — `DMR-ES30V`):**
   - Analisa falhas de fonte de alimentação em gravadores `DMR-ES30V` por degradação de capacitores eletrolíticos.
2. **Wayback Machine & Softpedia:**
   - Indexação histórica de atualizações norte-americanas e utilitários como `DMR-EH55VS` (`DMR-EH55.exe`, `IR-01`), `DMR-EH75VS`, `DMR-ES45VS`, `DMR-ES20S` e snapshots em `web.archive.org/web/*/av.jpn.support.panasonic.com/support/dvd/download/*`.
3. **Archive.org:**
   - Preservação de Manuais de Serviço Oficiais em PDF (como `DMR-EH50`, `DMR-EH55`, `DMR-EH575`, `DMR-EX98V`), sem contêineres binários `.FRM` de DMR indexados diretamente até o momento.

---

## 3. Modo de Serviço Oficial (Manual `DSD0603003CE`), Sequência JDM (`[RED]`) e Física Elétrica do Standby / `TW7501` vs. `P7501`

Os gravadores Panasonic DIGA incorporam rotinas de diagnóstico de fábrica documentadas oficialmente nos Manuais de Serviço (como o **`ORDER NO. DSD0603003CE`** do DMR-EH55EC/EP/EH56EG) além de atalhos IR estendidos utilizados na geração japonesa/Blu-ray.

### 3.1 O que é o botão `[RED]` (`[VERMELHO]`) e diferença entre DMR-EH55 e Linha JDM/BD

1. **Identificação física do `[RED]` no Controle Remoto (Manual do Usuário `RQT8415`, p. 12 item `#10`):**
   - No controle remoto original da família DIGA (`EUR7659Y60` / `EUR7659YC0`), logo **abaixo** do anel direcional (`ENTER` / `FUNCTIONS` / `DIRECT NAVIGATOR` / `RETURN` / `SUB MENU`), existe uma fileira com **4 botões coloridos de Teletexto/Funções (`Colour buttons`)**: **Vermelho (`[RED]`)**, **Verde (`[GREEN]`)**, **Amarelo (`[YELLOW]`)** e **Azul (`[BLUE]`)**.
2. **Importante — JDM/BD vs. DMR-EH55 Global:**
   - A combinação **`[5] + [9] + [RED]`** por 5 segundos (exibindo **`70 RET`** $\rightarrow$ seta direita **`80 SRV`** $\rightarrow$ `[ENTER]`) pertence aos modelos domésticos japoneses (JDM) e gravadores DIGA/Blu-ray posteriores.
   - No **Panasonic DMR-EH55 (Europa/Global — Manual de Serviço Oficial `ORDER NO. DSD0603003CE`, Seções `8.1.2` e `8.1.3`, páginas 18–26)**, **NÃO se usa `[5]+[9]+[RED]`**: tanto o **Modo de Serviço (`SERV`)** quanto o **TEST Mode (`TM AV1`)** são acionados pelas **teclas do painel frontal com o aparelho em Standby (Power-Off)**, e os subcomandos internos são digitados em 2 dígitos numéricos (`[0][0]`..`[9][9]`) no controle remoto!

### 3.2 Modos Especiais Oficiais no Painel Frontal (DMR-EH55 `DSD0603003CE`, pp. 17–21)

| Modo Especial                     |      Display FL      | Operação exata (Manual de Serviço `DSD0603003CE`, pp. 17–21)                                                                                                        | Efeito / Uso de Bancada                                                                                                 |
| :-------------------------------- | :-------------------: | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | :---------------------------------------------------------------------------------------------------------------------- |
| **Service Mode (`SERV`)** |       `SERV`       | Com a energia **desligada (Standby)**, manter pressionadas simultaneamente **`[CH UP] + [OPEN/CLOSE] + [REC]`** no painel frontal por **5 segundos**. | Libera o menu de engenharia interno (`[0][0]`..`[9][9]`) via controle remoto.                                       |
| **TEST Mode (`TM AV1`)**  |      `TM AV1`      | Com a energia **desligada (Standby)**, manter pressionadas **`[STOP] + [CH UP] + [OPEN/CLOSE]`** por **5 segundos**.                                  | **NÃO é Service Mode**: reinicializa todos os parâmetros do aparelho (sintonizador/preferências de usuário). |
| **Forced Disc Eject**       |    *(anterior)*    | Com a energia **desligada**, manter **`[STOP] + [CH UP]`** por **5 segundos**.                                                                        | Ejeção forçada da bandeja (inativo sob Child/Demo Lock).                                                             |
| **Forced Power-Off**        |      *(P-off)*      | Pressionar **`[POWER]`** no painel frontal por **mais de 10 segundos**.                                                                                     | Desligamento forçado da unidade principal quando travada.                                                              |
| **First Time Setup Reset**  |       `HELLO`       | *(Firmware Guide p. 7–8)* Com a unidade sintonizada, manter **`[CH UP] + [CH DOWN]`** por **10–15 segundos** até desligar.                             | Força tela de configuração inicial no próximo boot.                                                                 |
| **NTSC / PAL Switch**       |    *(anterior)*    | Com o aparelho **ligado (E-E mode)**, manter **`[STOP] + [OPEN/CLOSE]`** por **5 segundos**.                                                          | Alterna saída entre sistema PAL e NTSC.                                                                                |
| **Rating Password Reset**   |    *(nível 8)*    | Abrir bandeja e manter **`[REC] + [PLAY]`** por **5 segundos**.                                                                                             | Reseta senha de controle parental para Nível 8.                                                                        |
| **Progressive Init**        |    *(anterior)*    | Ligado (E-E): **`[STOP] + [PLAY]`** por **5 segundos**.                                                                                                     | Retorna saída de vídeo para Interlace.                                                                                |
| **Demo Lock / Unlock**      | `LOCK` / `UNLOCK` | Ligado: **`[STOP] + [POWER]`** por **5 segundos**.                                                                                                          | Bloqueia/desbloqueia ejeção da bandeja (persiste após Factory Init).                                                 |
| **Child Lock (`X-HOLD`)** |      `X-HOLD`      | No controle remoto: manter **`[ENTER] + [RETURN]`** simultaneamente até exibir `[X-HOLD]`.                                                                     | Bloqueia comandos frontais e remotos (não aceita 10 pulsos de `ENTER`).                                              |

### 3.3 Tabela Completa de Comandos Remotos do Modo `SERV` (DMR-EH55 `DSD0603003CE`, pp. 21–26)

Uma vez exibido **`SERV`** no display fluorescente (VFD), o técnico digita os **dois dígitos no controle remoto**:

|                    Código                    | Nome Oficial (Manual `DSD0603003CE`, pp. 21–26)                | Descrição & Diagnóstico Verificado                                                                                                                                                                                                                                                                          |
| :--------------------------------------------: | :---------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **`[0][0]`** ou **`[RETURN]`** | **Release Items**                                           | Cancela o item atual do Service Mode e retorna para `SERV` (não é teste do VFD; o teste de VFD é `[5][1]`).                                                                                                                                                                                             |
|              **`[0][1]`**              | **Error Code Display**                                      | Exibe o **último código de erro (`U**`/`H**`/`F**`) retido pelo Timer `IC7501`** (`F00` se vazio).                                                                                                                                                                                           |
|              **`[0][2]`**              | **ROM Version Display**                                     | Sequência (5s cada): **1. Region code** $\rightarrow$ **2. Main firm** $\rightarrow$ **3. Timer firm** $\rightarrow$ **4. Drive firm** $\rightarrow$ **5. ROM correction**.                                                                                             |
|       **`[1][1]` / `[1][2]`**       | **White / Magenta Picture Output**                          | Gera padrão interno 100% branco ou magenta no AV Decoder (componente).                                                                                                                                                                                                                                        |
|       **`[1][3]` / `[1][4]`**       | **RTSC Return in XP / I-P Switch**                          | Codifica e decodifica AV1 em tempo real (`XP`, 48kHz) sem gravar em disco; `[1][4]` alterna Interlace/Progressive.                                                                                                                                                                                         |
|       **`[2][1]` / `[2][2]`**       | **Audio Mute (`XTMUTE` / `XDMUTE`)**                    | Testa mute controlado pelo Timer (`T MUTE`) ou pela PCB Digital (`D MUTE`).                                                                                                                                                                                                                                |
|       **`[2][3]` / `[2][4]`**       | **Audio Pattern Output**                                    | Emite tons internos (`L=1kHz/-18dB`, `R=400Hz/-18dB`); `[2][4]` alterna 48kHz / 44.1kHz.                                                                                                                                                                                                                 |
|              **`[3][1]`**              | **HDD READ inspection**                                     | Varredura de leitura completa do HDD (**`HDD OK`** ou código de defeito; sair com `[POWER] > 10s`).                                                                                                                                                                                                 |
|       **`[4][1]` / `[9][5]`**       | **Laser Used Time / Delete Laser Time**                     | Exibe horas acumuladas de uso do laser óptico (`*****`); **`[9][5]`** zera o contador (`CLR`, destrutivo).                                                                                                                                                                                        |
|       **`[4][2]` / `[9][6]`**       | **RAM Drive Last Error (`NO **`) / Delete**               | Detalha número, timestamp (`DD/hh/mm`), tipo de mídia e fabricante; `[0][1]`..`[1][9]` mostra últimos 19 erros (`[9][6]` limpa).                                                                                                                                                                    |
|              **`[4][4]`**              | **Laser Power Confirmation**                                | Compara calibração atual do laser contra o valor de fábrica com DVD-RAM inserido (`CHK *` / `NO DISC` / `ERROR`).                                                                                                                                                                                     |
|       **`[5][1]` / `[5][4]`**       | **All FL/LEDs / Front Key Inspection**                      | `[5][1]` acende todos segmentos do VFD/LEDs; `[5][4]` testa cada tecla do painel frontal incrementalmente.                                                                                                                                                                                                 |
|       **`[6][1]` / `[6][4]`**       | **Production Date / Accumulated Working Time**              | `[6][1]` exibe data de fabricação (`YYMMDD`); `[6][4]` exibe tempo total ligado em segundos.                                                                                                                                                                                                           |
|       **`[6][5]` / `[9][7]`**       | **Display Error History (`[0][1]`..`[1][9]`) / Delete** | Lista os últimos **19 eventos de erro** com código de causa (5s): **`01`/`05`** = Digital PCB (`AV DEC / MAIN CPU`), **`02`** = RAM Drive, **`03`** = Disco, **`04`** = Digital PCB / Erro de Comunicação, **`06`** = Defeito de HDD (`[9][7]` limpa). |
|              **`[7][4]`**              | **SD Card WRITE check**                                     | Inserir cartão SD em Standby (`[CARD SD]`): testa escrita (`SD OK` / `SD NG`; **apaga imagens no cartão!**).                                                                                                                                                                                     |
|        **`[8][0]`..`[8][3]`**        | **AV1/AV2/AV4 Video I/O Routing**                           | Comuta roteamento analógico Y/C, V e RGB para aferição de entrada/saída (testes de passthrough SCART nos modelos europeus, ausentes nos modelos NA).                                                                                                                                                       |
|       **`[8][4]` / `[8][5]`**       | **`P50(H)` / `P50(L)` Timer Output Check**              | Testa saída `IC7501-76` passando pelo inversor até AV1 pino 10 (`~0V` vs `~4.4V DC`).                                                                                                                                                                                                                  |
|              **`[9][1]`**              | **Tray OPEN/CLOSE Test**                                    | Abre/fecha a bandeja continuamente contando ciclos (liberar segurando `[POWER]` no controle `> 10s`).                                                                                                                                                                                                      |
|              **`[9][8]`**              | **Error Code Initialization**                               | Limpa o último código de erro retido pelo Timer `IC7501` $\rightarrow$ grava `F00` (`CLR`).                                                                                                                                                                                                          |
|              **`[9][9]`**              | **Initialize Service (`CLR SERV`)**                       | Inicializa simultaneamente **Last Drive Error**, **Error History** e **Error Codes** para padrão de fábrica (`CLR`).                                                                                                                                                                     |
|             **`[POWER]`**             | **Finishing Service Mode**                                  | Sai do modo `SERV` retornando ao modo `STOP (E-E)`.                                                                                                                                                                                                                                                        |

> [!NOTE]
> Em modelos japoneses (JDM) ou geração Blu-ray posteriores, códigos adicionais de controle remoto incluem `38 + [ENTER]` (`HDDCHK`), `03 VL`, `04 PRG`, `06 FTO` e `08 FIN`.

### 3.4 Por que `PLEASE WAIT (~2s) -> U99`, `U99` direto ou `HELLO (~2s) -> U99` não entram em `SERV` / `ND2` / `Functions`

Conforme o Manual de Serviço `DSD0603003CE` (p. 18 tabela `8.1.1` e p. 40 *Display Flow*), os esquemáticos do Timer `IC7501` ([`panasonic_dmr_eh55ec_service_manual_vol2_schematics.pdf`](../../manuals_and_schematics/dmr-eh55/service/panasonic_dmr_eh55ec_service_manual_vol2_schematics.pdf) pp. 121–122; [`panasonic_dmr_eh55ec_service_manual_vol1_diagnostics.pdf`](../../manuals_and_schematics/dmr-eh55/service/panasonic_dmr_eh55ec_service_manual_vol1_diagnostics.pdf) pp. 22–25, 40, 63–68; [`panasonic_dmr_eh55p9_eh55pc9_service_manual_north_america.pdf`](../../manuals_and_schematics/dmr-eh55/service/panasonic_dmr_eh55p9_eh55pc9_service_manual_north_america.pdf) pp. 21–24, 62–64) e a arquitetura científica detalhada em [`ARQUITETURA_STANDBY_IC7501_TBUS_TW7501_E_MODOS_BOOT.md`](./ARQUITETURA_STANDBY_IC7501_TBUS_TW7501_E_MODOS_BOOT.md), o erro **`U99` ("Hang-up")** sinaliza falha de comunicação/handshake `TBUS` entre o microcontrolador Timer (`IC7501`) e o SoC principal (`AM34` / `MN2DS0009`):

1. **Diferença Crítica entre `TEST Mode`, `Service Mode (`SERV`)` e os Dois Perfis Empíricos no VFD (`PLEASE WAIT` vs `HELLO`):**
   - Na inicialização normal na tomada ou em loop pós-glitch (`IC7501` saindo de recuperação AC / `TW7501`), o VFD **NÃO** mostra `HELLO -> PLEASE WAIT`: ele exibe **`PLEASE WAIT` (~2s) -> `U99`**, ou vai **diretamente para `U99`** sem exibir `PLEASE WAIT`.
   - Somente após estabilizar o aparelho em **Standby real (`0:00` / `P-OFF`)** via `[POWER]` off $\rightarrow$ salto rápido em `TW7501` energizado $\rightarrow$ segurar `[POWER]` (`KEY3_POWER`), quando o aparelho é **acordado do Standby** (pelo botão `[POWER]` ou pelas combinações frontais `[CH UP] + [OPEN/CLOSE] + [REC]`/`[STOP]`), o VFD exibe **`HELLO`** (passo 11 da p. 40 do manual `EC`: banner de acordamento normal) e então, **~2s depois**, cai em **`U99`** antes de concluir o startup!
   - A combinação **`[STOP] + [CH UP] + [OPEN/CLOSE]`** por **5s** aciona o **`TEST Mode` (`TM AV1`)**, que reinicializa preferências de sintonizador/usuário — ela **não** entra no Service Mode. Já o **`SERV`** oficial (`[CH UP] + [OPEN/CLOSE] + [REC]`), `Functions` e os atalhos `ND2` (`[5]+[9]+[RED]`) dependem todos da subida completa do **`MAIN` (`SYS_SERV`)**; quando `U99` aborta o boot em ~2s, nenhum desses menus chega a existir em memória.
2. **Por que tanto `PLEASE WAIT (~2s) -> U99` (Tomada/Retry) quanto `HELLO (~2s) -> U99` (Acordar de Standby) falham antes do `BOOT.BIN`:**
   - Na inicialização normal da Flash NOR, o `BOOT.BIN` (`PROG061`) executa `wait_1_sec_timer_sync` (`0x4400b34d`: `delay_ms(1000)` $\rightarrow$ lê `startup reason` via `FUN_4400592c(2, ...)` $\rightarrow$ envia `FUN_440056b1(0x81, 1)` $\rightarrow$ `FUN_44005cd0()` `[0x8A, 0x02, ..., 0xFF]`), estendendo o watchdog de hang-up do Timer `IC7501` para **`255s`** (`"Main hung-up time changed 255[s]."` sobre `TBUS`).
   - Quando o aparelho cai em **`U99` em ~2s** (seja via **`PLEASE WAIT -> U99`**, **`U99` direto**, ou **`HELLO -> U99`** ao acordar de Standby estável), a falha ocorre **ANTES** que o AM34 complete `FUN_44005cd0()`. Isso aponta para três classes de defeito pré-handshake:
     1. Falha precoce de barramento de potência ou supervisão de reset (`X_SW+3.8V`, `X_SW+5.8V`, `X_SW+12V`, `ANA+3.3V`, `X_SW+3.3V` ou queda em `PFAIL[L]` — falha que dispara proteção imediata, nunca forçar tensões acima de `3.3V` nos pinos LVTTL);
     2. Congelamento prematuro em **`SA0..SA3`** (`0x80000000..0x8003FFFF`) ou falha na **SDRAM** (`DDR CHECK`) antes do handshake `TBUS`; ou
     3. Falha física no próprio barramento **`TBUS`** (`TBUS_CLK`, `TBUS_TXD`, `TBUS_RXD`, `XMPREQ`, `XINTM`).
   - Em contraste, a mensagem **`BGO driver init failed[1].`** ocorre em um estágio posterior, **depois** que o `BOOT.BIN` já inicializou a SDRAM e começou a imprimir mensagens pela saída serial do AM34 (**`UARTP2M_T`**, `P7501` Pino 6 $\rightarrow$ Pico `GP2` RX; enquanto `P7501` Pino 3 **`UARTM2P_T`** é a entrada Pico `GP4` TX $\rightarrow$ AM34 via `Q7514`).
3. **Rota Real de Recuperação:**
   - Combinações de botões frontais não conseguem corrigir falhas de rail, `TBUS`, SDRAM ou Flash corrompida. A rota de bancada é a **triagem passiva UART em `P7501` Pino 6 (`UARTP2M_T`)** associada à sequência **`SW_UART` (`P7501` Pino 1 / `GP26`) `LOW` + `XRESET` (`P7501` Pino 2 / `GP5`) Open-Drain** (`***** serial boot mode *****` $\rightarrow$ `DIGADIAG.FRM`) ou recuperação direta por gravador **TSOP-48** (preservando `SA0..SA3` imutáveis em leitura estrita, tratando Upper NOR / NVRAM `SA134..SA141`), mantendo **`NSW5V` (`GP3`) sempre em `Hi-Z Input` sem pulls (nunca dirigir push-pull!)** e **nunca alimentando o Pico pelo `P7501`**.

### 3.5 Física Elétrica do Standby (`P-OFF`), Distinção `TW7501` vs. `P7501` Pino 1 (`SW_UART`), `IC3001` e Mapas de Acordamento

A verificação cruzada dos esquemáticos oficiais ([`panasonic_dmr_eh55ec_service_manual_vol2_schematics.pdf`](../../manuals_and_schematics/dmr-eh55/service/panasonic_dmr_eh55ec_service_manual_vol2_schematics.pdf) pp. 121–122, [`panasonic_dmr_eh55ec_service_manual_vol1_diagnostics.pdf`](../../manuals_and_schematics/dmr-eh55/service/panasonic_dmr_eh55ec_service_manual_vol1_diagnostics.pdf) pp. 22–25, 67–68, [`panasonic_dmr_eh55p9_eh55pc9_service_manual_north_america.pdf`](../../manuals_and_schematics/dmr-eh55/service/panasonic_dmr_eh55p9_eh55pc9_service_manual_north_america.pdf) pp. 21–24, 62–63 e [`panasonic_dmr_es10_service_manual_north_america.pdf`](../../manuals_and_schematics/dmr-es10/service/panasonic_dmr_es10_service_manual_north_america.pdf) p. 27) corrige uma confusão histórica de nomenclatura e consolida a arquitetura elétrica real:

1. **Correção Crítica — `TW7501` (`VWJ0795=5`) NÃO é o Pino 1 (`SW_UART`) do `P7501`:**
   - **`TW7501` (`VWJ0795=5`):** Situa-se no **trilho de alimentação de 5V / backup de memória** (`D7509`, `C7578`, `R7623`, supercapacitor `C7584` `0.047F` nos modelos `EC` / bateria `B7501` `CR2354` nos modelos `NA`) que alimenta o **Microprocessador Timer `IC7501`** (`C2CBJG000713` no `EC`; `MN67788RA` + `IC8001` no `NA`) e o detector de reset **`IC7502`** (`C0EBE0000504`) $\rightarrow$ `XRESET` (`IC7501` Pino 12 e `P7501` Pino 2). Curto-circuitar `TW7501` reseta o estado do Timer MCU e limpa dados voláteis de standby (assim como no `DMR-ES10` `NA p. 27`, onde `TW7501` envia `RESET (L)` para `XRESET`).
   - **`P7501` Pino 1 (`SW_UART/IR_BUSY`):** É uma **linha separada de strap de boot serial do AM34 (`MN2DS0009`)**, ligada ao estágio `Q7510` / `QR7503` e controlada em software pelo pino **`GP26`** do Pico via comandos `JIGPIN LOW` / `JIGPIN HIZ`.
   - **Nota sobre nomenclatura legada:** Strings antigas de firmware/UI (ex.: `"TW7501_STRAP"` no comando `JIGPIN` em `p7501_cli.c`) apelidaram historicamente o `JIGPIN` de `"TW7501"`; eletricamente, porém, `TW7501` (rail de backup/`XRESET` do Timer MCU) e `P7501` Pino 1 (`SW_UART/IR_BUSY` do AM34) são **redes distintas**.
2. **Papel Real de `IC3001` e Ausência de EEPROM Externa no `DMR-EH55`:**
   - **`IC3001` (`C1AB00002379`)** é o **CI matriz de comutação A/V de 100 pinos** na placa Main (controlado via barramento I²C pelo `IC7501`), **NÃO** o Microcontrolador Timer (`IC7501`).
   - O **DMR-EH55 NÃO possui EEPROM externa** conectada ao Timer MCU: nos esquemáticos (`EC vol2 p. 122`, `EC vol1 p. 68`), o pino **`(EEPROM_W_C)` do `IC7501` está explicitamente ligado como `NC(OPEN)`**. Toda persistência não-volátil do sistema reside na Flash NOR Spansion `IC5001` (`SA134..SA141`).
3. **Por que `STATUS` / `VDD` lê `0.03V (LOW / UNPOWERED)` em Standby sem jumper:**
   - Os reguladores da placa (`IC1502` `X SW +5.2V`, `IC1504` `JC +5V`, `IC1506` `DR +5V`, `IC1507` `ANA +3.3V`) e a alimentação da placa Digital (`D3R3V`) são **comutados** por `P_SAVE_L` / `DI_P_ON[H]` sob comando do `IC7501` (`NA pp. 55–56, 62`).
   - Como `NSW5V` (`P7501` Pino 5) e os pull-ups de `SW_UART/IR_BUSY` (Pino 1), `UARTM2P_T` (Pino 3) e `UARTP2M_T` (Pino 6) derivam desses barramentos comutados, durante o **Standby (`P-OFF` / relógio `0:00`) todos os pinos do `P7501` caem a ~`0.00V`** (`VDD: 0.03V`, `RXD=0, NSW5V=0, TXD=0, XRESET=0`). Assim que `DI_P_ON[H]` sobe (mesmo num pulso de 1–2s), `NSW5V` e `SW_UART` sobem a `~3.25V`.
4. **A sonda ligada em `P7501` pode ligar o aparelho diretamente do Standby?**
   - **Não pelos 6 fios atuais do `P7501`**: o botão `[POWER]` do painel (`S7002` `KEY3_POWER`, `EC p. 65`) vai diretamente para `IC7501` Pino 88 (`KEYIN3`) e o receptor IR (`IR7501`) vai para o Pino 65 (`IR`) — nenhum desses dois sinais passa pelo conector `P7501`.
   - **Alternativas para acordar via Pico sem tocar no aparelho:**
     * **LED IR Kaseikyo 38 kHz (`IRTX`, não invasivo, zero solda):** um LED emissor infravermelho (+ resistor `330 Ω`) em GPIO livre seguro (`GP6..9` ou `GP16..22`) apontado para `IR7501` envia códigos Panasonic Kaseikyo (`Power Toggle`, dígitos `[0][1]`..`[9][9]` do `SERV`).
     * **Fio opcional `Open-Drain` em `S7002` (`KEYIN3`):** permite simular o clique físico do `[POWER]`.
5. **Como a sonda Pico mapeia o Standby e aciona `PREJIG` + `WAKE` sem pulsar `XRESET`:**
   - **Mapeamento Contínuo de Transições (`BRIDGE 115200 PASSIVE CTRL`, `PWRTRACE`, `BOOTTRACE ... WAKE`, `BAUDSCAN ... WAKE`):** em modo passivo durante Standby, o Pico aguarda `NSW5V (GP3) == 1 || VDD >= 2.0V` e registra com carimbo de tempo a borda de subida/queda de **`NSW5V` (`GP3`, estritamente `Hi-Z Input`), `TXD` (`GP4`) e `XRESET` (`GP5`)** + cada byte em `115200 8N1` por `UARTP2M_T` (`GP2`).
   - **Strap em Standby (`JIGPIN LOW` / `PREJIG WAKE` sem `XRESET`):** Nos esquemáticos `EC` (`europe.pdf` pp. 47, 50, 54, 63–65), o conector `P59001` leva `XFWE` (`Pino 32` $\rightarrow$ `Q7401`/`QR7403`/`QR7404`) e `P_OFF_L` (`Pino 30`), porém **não** propaga `XRESET` (ficando `IC7502` $\rightarrow$ `IC7501 Pino 12` na Main e `P7501 Pino 2` local na Digital `VEP79138`). O modo **`PREJIG WAKE`** mantém `SW_UART = LOW` ainda em Standby **sem pulsar `XRESET`**, disparando na borda exata de subida de `NSW5V (GP3)` para capturar `***** serial boot mode *****` desde `t = 0 ms` ao acordar via `[POWER]`, IR ou combos `SERV`/`TEST`.
6. **Física da Sequência `TW7501` + Segurar `[POWER]` Que Encerra o Loop de Auto-Religamento de 2s (`C7584` + `R7623` + `KEYIN3`):**
   - **Por que descarregar `TW7501` inicia loop de religamento:** Em `EC p. 63`, `TW7501` (`VWJ0795=5`) fica em paralelo com o supercapacitor `C7584` (`0.047F`), `C7578` (`47uF`), `R7621` (`100K`) e o Pino 1 de `D7509` (alimentado por `X_SW_5R2V` via `R7623 180Ω`). Zerar `TW7501` fora da tomada apaga a RAM de backup do `IC7501` (armando `SELF CHECK`); ao ligar na tomada, `IC7501` sobe `DI_P_ON[H]` sozinho e, se travar antes do foco do leitor ODD, 1 toque em `[POWER]` derruba `NSW5V` temporariamente mas a flag volátil de retentativa religa ~2s depois.
   - **Por que `1x [POWER]` $\rightarrow$ pulso curto em `TW7501` energizado $\rightarrow$ segurar `[POWER]` estabiliza o Standby (`VDD=0.03V`):** O curto momentâneo em `TW7501` com a placa energizada (limitado a `~29 mA` por `R7623 180Ω` em `X_SW_5R2V`) reinicia/zera a flag volátil de 2s via `IC7502`/`IC7505` enquanto `S7002` (`KEY3_POWER` $\rightarrow$ `IC7501 Pino 88 KEYIN3`; `EC p. 20` *Forced power-off* `>10s`) trava `IC7501` diretamente em **`P-OFF` Standby estável**, permitindo armar `panadiga pico bench-wizard --port COM10` sem pressa antes de qualquer tipo de boot.

```mermaid
flowchart TD
    subgraph P7501_PINOUT["Conector P7501 (Placa Principal VEP79138 — Barramento Comutado por DI_P_ON)"]
        P1["Pino 1: SW_UART/IR_BUSY (AM34 Serial Boot Strap) -> Pico GP26 (JIGPIN LOW/HIZ)"]
        P2["Pino 2: XRESET (Open-Drain via QR7503 / IC7502) -> Pico GP5"]
        P3["Pino 3: UARTM2P_T (TX Pico -> Q7514 -> AM34 RX) -> Pico GP4"]
        P4["Pino 4: GND (Referência 0.0 Ω ao Chassi)"]
        P5["Pino 5: NSW5V (Comutado ~5.15V upstream / ~3.29V ON; 0V em Standby) -> Pico GP3 Hi-Z"]
        P6["Pino 6: UARTP2M_T (AM34 TX 115200 8N1 -> RX Pico) -> Pico GP2"]
    end

    subgraph BACKUP_RAIL["Circuito Separado: TW7501 (VWJ0795=5) — Backup 5V & Reset IC7501/IC7502"]
        TW["TW7501 (D7509, C7578, R7623, C7584 0.047F / B7501 CR2354)"] -->|"Alimenta"| MCU["Timer MCU IC7501 (C2CBJG000713 / MN67788RA+IC8001) + IC7502"]
        MCU -->|"Pin 12 XRESET"| P2
    end

    subgraph STANDBY_STATES["Mapeamento Elétrico do P7501 (Standby vs Acordado vs Auto-Retry AC)"]
        ST_AC["Tomada AC / Auto-Retry\nVFD: PLEASE WAIT (~2s) -> U99\n(ou U99 direto)"] -->|"1x [POWER] + TW7501 energizado + segurar [POWER]"| ST_OFF["Standby Estável P-OFF ('0:00')\nDI_P_ON=L -> NSW5V=0, VDD=0.03V"]
        ST_OFF -->|"[POWER] ou Combo via KEYIN3 / IR"| ST_WAKE["Wakeup Digital PCB (VFD: HELLO)\nNSW5V=1, XRESET=1, SW_UART~3.25V"]
        ST_WAKE -->|"SW_UART=Hi-Z"| BOOT_NORM["Boot NOR Flash: HELLO (~2s) -> U99\n(antes de FUN_44005cd0)"]
        ST_WAKE -->|"SW_UART=LOW (PREJIG WAKE)"| BOOT_SER["Loader: ***** serial boot mode *****"]
    end
```

---

## 4. Ferramentas Open-Source de Criptografia e Armazenamento Forense (`unixtract`, `sddl_dec`, `pana-fw`, `panasonic-rec`, `dvr-recover`)

A reconstrução de dados, descompactação de firmware e extração de vídeo nos gravadores Panasonic DIGA culminou no desenvolvimento de um conjunto de utilitários de código aberto mantidos por engenheiros e pesquisadores de segurança da computação.

### 4.1 A Suíte Completa das 12 Chaves Feistel `PANA_DVD` e Contêineres Híbridos AES-128-CBC

A ferramenta de extração de firmware **`unixtract`** (desenvolvida em Rust) e o utilitário **`pana-fw`** catalogaram as 12 chaves simétricas de 8 bytes (64 bits) utilizadas no algoritmo proprietário de cifra Feistel (`FUN_4400a666` no pseudocódigo Ghidra):

```rust
// Matriz Canônica das 12 Chaves Feistel PANA_DVD (Fonte: unixtract / keys.ukf)
// 10 Chaves Simples de 64 bits + 2 Pares Híbridos com Encriptação AES-128-CBC

// 1. Coleção PANA_DVD_KEYONLY (Contêineres Feistel Puros)
"Old" (EH55, EX80SEG, EH49/59/69) : 0xDF7215C7427FD010
"<=2011"                          : 0x08E03D859AF9F3EE
"2011"                            : 0xAF4D5B16C298C16D
"2012"                            : 0x9407311B7561F97E
"2012_2"                          : 0x5D88719745A4EFFF
"2013"                            : 0x38BB0C17D362701F
"2013_2"                          : 0xEC238CD791456646
"2013 china"                      : 0x0A1EE6C5BDC4933F
"2014"                            : 0x8F8CA88482E43D9B
"2014_2"                          : 0x21FCA17361E66B4E

// 2. Coleção PANA_DVD_AESPAIR (Contêineres Modernos Híbridos AES-128-CBC + Feistel)
"2016":
    AES Key : 0xA692605194CF7F4917E50521C1DF0BBA
    AES IV  : 0xF80A6BCAD57AA67C3C60E3BE755E05DF
    Feistel : 0xFDFE0BEE4C90FA2C

"2018":
    AES Key : 0x62A39E1C5594AE09244EB326EF7938FA
    AES IV  : 0x46D03C52B1A398FA03276FDE7CC4EA26
    Feistel : 0x06C943F3B997F7E0
```

Nos contêineres pós-2015 (`PANA_DVD_AESPAIR`), o arquivo óptico sofre uma primeira camada de encriptação em bloco AES-128 no modo CBC sem preenchimento (*nopad*). Uma vez descriptografado com o par de Chave/IV correspondente, os dados internos revelam a estrutura legada `PANASONIC\x00\x00\x00`, a qual é processada pela cifra Feistel tradicional de 64 bits utilizando a chave correspondente.

### 4.2 Arquitetura Criptográfica `SDBOOT` e `SDDL.SEC` (`sddl_dec`)

Para atualizações distribuídas através de cartões Secure Digital (SD) ou downloads de rede em gravadores UniPhier com interface Ethernet, o utilitário **`sddl_dec`** documentou duas arquiteturas criptográficas de segurança:

1. **Chaves `SDBOOT` (Tabela `tbl_sdboot` decifrada com AES):**
   - Par 0: Chave `2E2A336233E55ABAF5FFEC54F8AB7125`, IV `2CA4B47AFFCB1AE8E1EA2D9EF512629A`.
   - Par 1: Chave `245E8DE8F499B0F96EC155B608E242F3`, IV `3E8F29D4BAE776A518A7B63C42CA1B43`.
2. **Chaves `SDDL.SEC` (Arquitetura Pré-2011 vs Pós-2011):**
   - **Família Pré-2011:** Utiliza tabelas de chaves (`tbl2009`, `tbl2010`, `tbl_sdboot`) que são decodificadas em tempo de execução através de uma chave mestra tripla DES (3DES):
     ```text
     Master DES Key: C2A421F6ADEB44BE B0FDA68C234BB3C5 E98EC68C326FD395 07C8D75EF1B1B142
     ```

     As chaves intermediárias obtidas descriptografam o conteúdo via AES ou DES.
   - **Família Pós-2011 (`SDDL_SEC_NEW_KEY`):**
     Extraída do binário do cliente `/usr/local/customer_dl/crypto_key`:
     - Chave AES-128: `26E096D3EF8A8FBBAA5E516F7726C22C`
     - Vetor IV: `3E4AE23A69DB8154CD8838C4B90C7666`

### 4.3 Estrutura de Contêineres e Cabeçalhos `EXTRHEAD` e `EXTRFOOT`

O cabeçalho comprimido `EXTRHEAD` (exatamente 64 bytes) gerencia a integridade e descompressão dos dados de firmware de cada módulo:

|  Offset  | Tamanho | Campo                   | Descrição e Validação                                                                  |
| :------: | :-----: | :---------------------- | :----------------------------------------------------------------------------------------- |
| `0x00` |   8 B   | `Magic`               | Caracteres fixos ASCII: `"EXTRHEAD"`                                                     |
| `0x08` |   4 B   | `Subtype`             | Identificador de subsistema (geralmente ASCII `"DRV "` ou `"MAIN"`)                    |
| `0x0C` | uint16 | `Compression Flag`    | Flag de compressão: deve ser `0x0001` (senão erro `"Error! Not compress"`)           |
| `0x0E` | uint16 | `Compression Type`    | Algoritmo: `0` = Sem compressão, `1` = GZIP, `2` = LZSS, `3` = Zlib, `4` = LZMA |
| `0x10` | uint32 | `Destination Size`    | Tamanho exato dos dados após descompressão                                               |
| `0x14` | uint32 | `Destination Address` | Endereço de destino em RAM (não utilizado, tipicamente 0)                                |
| `0x18` | uint32 | `Source Size`         | Tamanho dos dados comprimidos no contêiner                                                |
| `0x1C` | uint32 | `Source Address`      | Contador de bloco / endereço de origem de transferência                                  |
| `0x20` | uint32 | `Footer Offset`       | Deslocamento relativo até o cabeçalho de rodapé `"EXTRFOOT"`                          |
| `0x24` |   4 B   | `Reserved`            | Uso interno de sincronização                                                             |
| `0x28` | uint32 | `Sum`                 | Soma Adler32 dos dados descomprimidos calculada a cada salto                               |
| `0x2C` | uint32 | `Sum Skip`            | Passo em bytes utilizado no cálculo da soma de verificação                              |
| `0x30` |  16 B  | `Padding`             | Preenchimento nulo (`0x00`) para alinhamento de 64 bytes                                 |

O rodapé **`EXTRFOOT`** (8 bytes) alinha os dados na fronteira de 8 bytes requerida pela cifra Feistel em bloco de 64 bits.

### 4.4 Mecânica de Descriptografia de Segmentos: `0x2800` vs `0x1400`

Uma das mais refinadas otimizações implementadas pela Matsushita foi a descriptografia seletiva dos segmentos de firmware do `MAIN.bin`. Em vez de processar todo o arquivo de 13 MB com a rotina de decifração em tempo real, o código divide o módulo através da estrutura **`MainListHeader`**:

```mermaid
flowchart TD
    MLH["MainListHeader (20 Bytes)"] --> CNT["Quantidade de Entradas = (List Size - 20) / 8"]
    MLH --> CHK{"Tamanho da Parte Descomprimida\n(decompressed_part_size)"}
  
    CHK -- "== 0x2000000 (Legado)" --> LEG["LegacyMainSegmentEncryptedSize = 0x2800 (10.240 Bytes)\nDescriptografa apenas os PRIMEIROS 10 KB\ne os ÚLTIMOS 10 KB do segmento"]
    CHK -- "!= 0x2000000 (Moderno)" --> MOD["ModernMainSegmentEncryptedSize = 0x1400 (5.120 Bytes)\nDescriptografa apenas os PRIMEIROS 5 KB\ne os ÚLTIMOS 5 KB do segmento"]

    LEG --> DEC["Dados Centrais Permanecem Inalterados\nDescompressor (deflate / LZSS) Concatena no MAIN.bin"]
    MOD --> DEC
```

* **Estrutura do `MainListHeader` (20 bytes):**
  - `List Sum` (uint32): Adler32 de toda a lista excluindo este campo.
  - `Version` (uint32): Versão do formato (sempre 1).
  - `List Size` (uint32): Tamanho total do cabeçalho mais as entradas de segmento.
  - `Decompressed Size` (uint32): Tamanho final descomprimido da parte do sistema.
  - `Compression Type` (uint32): Algoritmo de compressão do segmento.
* Se o segmento for menor do que o limite (`decrypt_size`), o bloco é integralmente descriptografado. Se for maior, **apenas as extremidades (cabeçalho de compressão e rodapé) são decifradas**, economizando preciosos ciclos de clock da CPU AM34 na inicialização.

### 4.5 Algoritmo LZSS de Haruhiko Okumura (Janela Circular em `0xFEE`)

Quando o cabeçalho indica compressão LZSS (`Compression Type = 2`), o firmware utiliza a implementação canônica formulada pelo pesquisador japonês **Haruhiko Okumura** (1989):

* Tamanho do anel circular (*sliding window*): `4,096` bytes (`0x1000`).
* **Offset inicial de pré-carga do buffer:** exatamente **`0xFEE`** (4.078 em decimal), preenchido com espaços em branco (`0x20`) ou nulos (`0x00`).
* Formato dos ponteiros de retrovisor (*backreference*): 2 bytes (`b1`, `b2`):

  $$
  \text{Offset} = b_1 \mid ((b_2 \ \& \ \text{0xF0}) \ll 4)
  $$

  $$
  \text{Comprimento} = (b_2 \ \& \ \text{0x0F}) + 3
  $$

  Permitindo cópias entre 3 e 18 bytes repetidos a uma distância de até 4.095 bytes atrás no fluxo de vídeo/código.

### 4.6 Terminal Serial SLM (*Serial Log Menu*) em Equipamentos UniPhier com Linux

Nos gravadores Panasonic de gerações posteriores baseados em processadores UniPhier multicore com sistema operacional Linux embarcado, o utilitário **`pana-fw`** identificou o protocolo de console de serviço **SLM** (*Serial Log Menu*):

1. **Conector Físico de Diagnóstico (`K1KA06AA0083`):**
   - Conector miniatura de 6 pinos operando em UART assíncrona com nível TTL 3.3V e taxa de transmissão de **`115200 bps 8N1`**:
     - Pino 1: `XDIAG` (Interrupção de diagnóstico / entrada SLM)
     - Pino 2: `XRESET` (Reset geral do sistema)
     - Pino 3: `SBI` (Serial Bus Input — RX da placa)
     - Pino 4: `GND` (Terra elétrico de referência)
     - Pino 5: `D3R3V` (Alimentação lógica 3.3V)
     - Pino 6: `SBO` (Serial Bus Output — TX da placa)
2. **Métodos de Disparo do Console:**
   - **Método Hardware:** Aterrar o pino 1 (`XDIAG`) durante o boot por aproximadamente 1 segundo.
   - **Método Sequência Escape:** Enviar diretamente no pino RX (`SBI`) a sequência de bytes:
     ```text
     0x0C, 0x0F, 0x07
     ```
3. **Autenticação e Senha de Fábrica:**
   - O processo `/usr/dtvrec/bin/slm` solicita senha de autenticação. A senha canônica de fábrica decifrada a partir do hash MD5 crypt (`$1$UmYifOe7$u8VBq8WANl5zvis7uJw.v.`) é:
     ```text
     pm6030p
     ```
4. **Credenciais do `/etc/passwd` Identificadas:**
   - Usuário `media` : senha `media`
   - Usuário `vup`   : senha `pcinosanap` (a palavra *"panasonic"* invertida!)
   - Usuário `vupw`  : senha `pcinosanap`

### 4.7 Arquitetura Forense do Sistema de Arquivos MEIHDFS-V2.0 (`meihdfs1.h`)

O sistema de arquivos proprietário gravado nos discos rígidos Panasonic DIGA denomina-se **MEIHDFS** (*Matsushita Electric Industrial Hard Disk File System*). A engenharia reversa das estruturas do **MEIHDFS-V2.0** e **V2.1** foi pioneiramente realizada por **Honza Malý** (`hkmaly@matfyz.cz`, 2012), estendida por **Leecher** (`leecher@dose.0wnz.at`, 2015), e enriquecida por análises forenses nos portais *hardwarefetish.com* (artigo #601) e *0xcaffee.blog*:

```c
// Definições Estruturais Fundamentais do MEIHDFS-V2.0 (meihdfs1.h)
#define ISIZE        0x1000     // Tamanho do Inode em bytes (4.096 B / 4 KB)
#define BSIZE        0x800      // Tamanho do bloco de stream de vídeo MPEG (2.048 B / 2 KB)
#define BCNT         0x180      // Fator de bloco de alocação (384 blocos)
#define ASIZE        (BSIZE * BCNT) // Unidade Fundamental de Alocação = 786.432 Bytes (768 KB)
#define GSIZE        0x10000    // Fator de Repetição do Superbloco: A cada 50.331.648 KB (48 GB)

#define ITBL_START    (ASIZE + 0x6000)  // Offset da Tabela de Inodes no MEIHDFS-V2.0
#define ITBL_START_21 (ASIZE + 0xB000)  // Offset da Tabela de Inodes no MEIHDFS-V2.1

#define TIME_OFFSET  315532800  // Época Base: 1980-01-01 00:00:00 UTC (Segundos Unix)
#define MPEG_MAGIC   0xBA010000 // Magic Packet Header MPEG-2 PS (Big-Endian 0x000001BA)

#define INODE_MAGIC       0x81C00001  // Magic padrão de Inode de Arquivo
#define INODE_MAGIC_21    0x81800001  // Magic de Inode V2.1
#define DIRECTORY_MAGIC   0x41C20001  // Magic padrão de Diretório
#define ROOTDIR_MAGIC     0x41FF0001  // Magic do Diretório Raiz
```

#### Anatomia do Inode MEIHDFS e Cadeia de Blocos (*Extents*)

Cada arquivo de vídeo gravado no disco rígido possui um Inode de exatamente `4,096` bytes (`0x1000`). Diferente do ext4 ou NTFS, o MEIHDFS organiza a localização dos clusters de vídeo através de um vetor de **`0x140` (320) extensões de bloco contíguo** (`struct block_run runs[320]`):

```c
typedef struct block_run {
    uint32 start;   // Bloco LBA inicial do fragmento de vídeo
    uint32 offset;  // Deslocamento relativo dentro do arquivo
    uint32 len;     // Comprimento da extensão em blocos de alocação
} block_run;
```

Essa arquitetura explica o comportamento de bancada do DMR-EH55:

1. **Zero Fragmentação em Arquivos Grandes:** Como a alocação mínima de vídeo opera em blocos de `768 KB` (`ASIZE`), o sistema garante taxa de transferência contínua sem jitter mecânico na cabeça do disco.
2. **Registro de Data/Hora:** Todas as gravações utilizam carimbos de data/hora em segundos transcorridos desde `01/01/1980 00:00:00 UTC` (`TIME_OFFSET = 315532800`).
3. **Mascaramento e Criptografia Forense:** Conforme catalogado no projeto Panadiga, os metadados do Inode nos primeiros modelos utilizam operadores de inversão de bits (`XOR 0xFF`), o que impediu durante décadas que utilitários comerciais de recuperação de dados em PC reconhecessem a estrutura dos vídeos gravados.

### 4.8 Compatibilidade OEM Multi-Marca (O que Compartilha Chassi `.FRM` vs. Plataformas Incompatíveis)

Nem todo gravador de DVD com marca europeia ou etiqueta parecida usa eletrônica Panasonic. O cruzamento de placas (`VEP`), SoCs (`MN2DS` / `MN103S`), assinaturas de partição (`MEIHFS`) e cabeçalhos criptografados (`PANA_DVD.FRM`) separa claramente o que é compatível do que pertence a outro fabricante:

| Plataforma / Marca                                                                           | Evidência de Hardware / Cifra                                                                                                                                                      |        Compatibilidade Prática com Panadiga        | Regra de Bancada e Fontes Localizadas                                                                                                                                                                                                                                                                                                   |
| :------------------------------------------------------------------------------------------- | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :-------------------------------------------------: | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Loewe Centros `1102` / `1172` / `2102 HD` / `2172 HD` & `ViewVision DR+`** | Projeto OEM Matsushita/Panasonic (`MN2DS` SoC, placas série `VEP`, convenções de atualização/serviço Panasonic)                                                           |     **Mesma Arquitetura (OEM Genuíno)**     | Compartilha ferramentas de análise (`MEIHFS` / estrutura `.FRM`); exige conferência exata do `DEV.BIN` antes de qualquer gravação cruzada. Manuais de serviço no Elektrotanya; versões SW `090410L`/`070627a` citadas no `die-oberklasse.de` (sem links públicos diretos sem autenticação para `.FRM`/`.bin`). |
| **Hualu `BDR-9800`**                                                                 | Catálogo `PANA_DVD_KEY_CATALOG` com chave Feistel de 8 bytes `0x0A1EE6C5BDC4933F` (`bfhualu.com`)                                                                            | **Confirmada (Mesma Cifra Feistel `.FRM`)** | Aceita descriptografia/empacotamento `.FRM` pela mesma engine matemática usada no DMR-EH55 (com sua chave própria de 8 bytes).                                                                                                                                                                                                      |
| **Subsistema `SW-9585` / SoCs `MN2DS0003`..`0018` & Submarcas Matsushita**       | Unidade IDE DVD-RAM `SW-9585` e SoCs `MN2DS0003`/`0009`/`0016`/`0018` em unidades selecionadas JVC, Pioneer, Marantz, Cambridge Audio e Nintendo RVT-R; Quasar / National |  **Parcial (Peças / Driver / Subsistema)**  | Útil para mapeamento de firmware de drive óptico ou reaproveitamento mecânico/SoC, **não** intercambiável diretamente por imagem `.FRM` inteira sem checar placa e modelo.                                                                                                                                                 |
| **Tevion / Medion / Funai / maioria Philips**                                          | Plataforma Funai/MediaTek (`FUNAI_UPG` em `unixtract/src/keys.ukf`)                                                                                                             |               **Incompatível**               | Arquitetura de CPU, formato de pacote e cifra totalmente distintos — **nunca** tentar gravar binários Panadiga/DIGA nessas placas.                                                                                                                                                                                              |

### 4.9 Mapa Prático de IR Kaseikyo: Controle `EUR7659Y70`, `AUDIO [A]` vs. `[RED]`, Android CIR e `IRTX` no Pico

#### 1. Preciso do botão `[RED]` no controle `EUR7659Y70`? Existe substituto?

* **SIM — no DMR-EH55 você PODE usar o `EUR7659Y70` sem o botão `[RED]`:** no **Panasonic DMR-EH55** (Manual de Serviço `DSD0603003CE`), a entrada no **Modo de Serviço (`SERV`)** é feita pelos **botões do painel frontal** em Standby (**`[CH UP] + [OPEN/CLOSE] + [REC]` por 5s**) — **não** pela sequência JDM `[5]+[9]+[RED]`. Depois que o visor exibe `SERV`, todos os códigos internos (`00`, `01`, `02`, `31`, `41`, `42`, `65`, `80`, `81`, `91`, `98`, `99`) usam **exclusivamente as teclas numéricas `0`..`9`**, que já existem normalmente no seu **`EUR7659Y70`**.
* **Nenhum botão do `EUR7659Y70` ou dos controles combo norte-americanos substitui o `[RED]` (`0x141`) nem `ND2` (`0x182`):** nos controles norte-americanos (`DVD/VHS POWER`, `AUDIO [A]`, `DISPLAY [B]`, `SUB MENU [S]`), a fileira de 4 botões coloridos de Teletexto foi removida de fábrica e substituída pelos atalhos de Guide/VCR+ `[A]` e `[B]`. Decodificação cruzada (`Flipper-IRDB`, `pana-fw` e o perfil real `Panasonic_DMR_EH55_Commander.irplus` em `android-remote-panasonic`):

| Botão / Comando                      |       Kaseikyo       | Bytes Wire LSB-first              | Origem Verificada                   | Função Real                                                           |
| :------------------------------------ | :-------------------: | :-------------------------------- | :---------------------------------- | :---------------------------------------------------------------------- |
| **`POWER`**                   |       `0x03D`       | `02 20 B0 00 3D 8D`             | `.irplus` (`0000 0070...`)      | Liga / Standby                                                          |
| **`AUDIO [A]`**               |       `0x033`       | `02 20 B0 00 33 83`             | `.irplus` + `Flipper-IRDB`      | Faixa de áudio / softkey `[A]` (**NÃO** é `[RED]`)         |
| **`DISPLAY [B]`**             |       `0x092`       | `02 20 B0 00 92 22`             | `.irplus` + `Flipper-IRDB`      | Informações na tela (OSD) / softkey `[B]`                           |
| **`FUNCTIONS` / `ENTER`**   | `0x057` / `0x082` | `02 20 B0 00 57 E7` / `82 32` | `.irplus`                         | Menus OSD e confirmação (e destrave `SET 2`/`SET 3`)              |
| **`1`..`9`, `0`**         | `0x010`..`0x019` | `02 20 B0 00 10 A0`..`19 A9`  | `.irplus`                         | Dígitos do modo `SERV` (`00`..`99`) no `EUR7659Y70`            |
| **`MEDIA SERVER`**            |       `0x004`       | `02 20 B0 00 04 B4`             | `.irplus` (`MULTI_REGION_HACK`) | Pulso discreto final após `1..0 -> AUDIO` no desbloqueio Region Free |
| **Teletexto `[RED]`**         |       `0x141`       | `02 20 B0 01 41 F0`             | `pana-fw` (`41 B0 01 41`)       | Apenas controles EU/JDM (`EUR7659Y60`) ou emissor programável        |
| **`ND2` (`[5]+[9]+[RED]`)** |       `0x182`       | `02 20 B0 01 82 33`             | `pana-fw` (`40 B0 01 82`)       | Sequência Service Non-Disclosure mode 2 em modelos JDM/Blu-ray         |

#### 2. Preciso ligar um transmissor IR no Raspberry Pi Pico ou posso usar controle normal / celular Android?

| Cenário / Ferramenta na Bancada                                                                                               |      Precisa de LED IR no Pico?      | O que você consegue fazer na prática                                                                                                                                                                                                                                                                                                                                            |
| :----------------------------------------------------------------------------------------------------------------------------- | :-----------------------------------: | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **1. Controle Físico Comum (`EUR7659Y70` / `EUR7729KG0`)**                                                          |            **NÃO**            | Suficiente para ligar/desligar e rodar **todos os códigos `00`..`99` do modo `SERV`** do DMR-EH55 após acionar o painel frontal (`[CH UP] + [OPEN/CLOSE] + [REC]`). Não possui `[RED]` (`0x141`) nem `ND2` (`0x182`).                                                                                                                                    |
| **2. Tablet / Celular Android com IR Blaster nativo (`SM-T805M` + perfis `.irplus` / `android-remote-panasonic`)** |            **NÃO**            | Usa o emissor IR infravermelho **embutido no aparelho Android** (`ConsumerIrManager` / `/sys/class/sec/sec_ir/ir_send` @ `37.010 kHz`). Envia qualquer código sem fios extras: modos `SET 1`/`2`/`3` (subdevices `0x00`/`0x10`/`0x20`), deck VCR `0x90`, macros `AUTO_RESTORE`, Region Free (`1..0 -> AUDIO -> MEDIA SERVER`) e `[RED]`/`ND2`. |
| **3. Sonda Pico UART `P7501` (`STATUS`, `CDDIAG`, `DUMP`, `ERASE_NVRAM`, `GDB`, `DIAGWATCH WAKE`)**        |            **NÃO**            | Toda a depuração serial/JTAG/NVRAM ocorre eletricamente pelo conector `P7501`. Os novos modos **`DIAGWATCH WAKE`** e **`DIAGWATCH WAKE:<ms>`** monitoram a subida de tensão do trilho (`VDD >= P7501_VDD_WAKE_THRESHOLD_V`, limiar `2.0f`) ao sair do Standby antes de capturar via PIO, avisando estado Standby quando `rx=0`.                        |
| **4. Comando `IRTX` do Pico (`firmware/pico-p7501/`)**                                                               | **SIM (LED IR 940 nm externo)** | `IRTX <RED                                                                                                                                                                                                                                                                                                                                                                        |

O catálogo de referência em `crates/panadiga-core/src/crypto/pana_fw_go.rs` (`PANA_KASEIKYO_SERVICE_COMMANDS`) espelha `"Audio / Subtitles [A]"`, `"Display [B]"`, `"Teletext Red [RED]"` e `"Power On/Off"` para decodificação uniforme em desktop e terminal serial.

### 4.10 Validação E2E em Emulador `Mn103Vm` + Spansion NOR Simulada (`panadiga re verify-patches`)

O módulo [`crates/panadiga-core/src/re/emu_patch_suite.rs`](../../crates/panadiga-core/src/re/emu_patch_suite.rs) e o comando CLI `panadiga re verify-patches` executam uma validação integrada combinando o emulador **`Mn103Vm`**, uma imagem simulada de **Flash NOR Spansion `S29JL064H` (8 MiB)** com proteção estrita de IPL (`0x80000000..0x807EFFFF` $\rightarrow$ erro `E05`) e ciclos reais de empacotamento/retarguetamento/desempacotamento `.FRM` em seis cenários:

1. `spansion_nor_cfi_and_ipl_airgap` — Validação de identificação CFI, bloqueio read-only da região bootloader/IPL e saneamento isolado da NVRAM (`0x807F0000..0x807FFFFF`).
2. `frm_pack_retarget_and_flash` — Empacotamento, ajuste de modelo/região no TOC (`3410PQ`/`3060NP` + `DEV.BIN`) e gravação na imagem Flash simulada.
3. `bgo_bypass_and_panamon1_commands` — Execução em `Mn103Vm` do bypass BGO e dos comandos do monitor `PANAMON1>` (`B/R/W/D/F/J/E/U/Q`).
4. `tbc_ee_passthrough_routines` — Verificação em máquina virtual das rotinas de passthrough TBC/E-E.
5. `region_model_compat_unlock` — Desbloqueio determinístico de região/modelo com re-empacotamento `.FRM`.
6. `hdd_resize_over_200gb_lba48` — Execução real da rotina de limite LBA em `src/hdd_cmd.c` (`[MaxProductLBA]` em `0x5311_B9AC`, instrução `cmp d0, d1; bls/bra +12` em `site_va + 0x4E..0x6F`): o opcode de fábrica `0xC6` trunca um disco de 500 GB (`0x0E8D4A51`) para `0x05D21DB9` (`200 GB - 1`), ao passo que o opcode patchado `0xCA` preserva integralmente `0x0E8D4A50` (`500 GB - 1`).

---

## 5. Crônica Integral da Pesquisa Panadiga: Da Hipótese `DWIRE32A` à Engenharia Reversa Completa do DMR-EH55

Ao longo da investigação no **Panasonic DMR-EH55 (`VEP79138`)**, o projeto Panadiga evoluiu de hipóteses herdadas de literatura fechada até provas em hardware real (`N4_HARDWARE`), validação em emulador (`N2_EMULATED`) e triangulação estática em Ghidra (`N1_STATIC`) registradas no [ledger verificável de evidências](../evidence/ledger.json) (`EV-20261119-001` .. `EV-20261120-018`) e nas **22 capturas** catalogadas em [`docs/debug_logs/README.md`](../debug_logs/README.md).

```mermaid
flowchart LR
    F1["2026-11-06..12\nHipotese DWIRE32A e\nMock 99,999% 0xFF"] --> F2["2026-11-12..13\nRefutacao via 3 Manuais\nP7501 = UART 115200 8N1"]
    F2 --> F3["2026-11-13..15\nEsquemas VEP79138, S29JL064H,\nPINCHECK, BUSDIAG e PIO 8x"]
    F3 --> F4["2026-11-19\nDump 8 MiB, BGO 45 Setores\nCura U99 em 2 Ciclos e 250 GB"]
    F4 --> F5["Commit 475fd48\nZero-Undef, SSOT 89 Modelos,\nEH55_M6930LB_8MB e FVU SD"]
```

### 5.1 Sondagem Inicial & Refutação da Hipótese `DWIRE32A`

Na fase inicial do repositório (`3843833` / `04894f8` / `ffbb8bc`), assumiu-se por analogia com outros chassis DIGA (`MN2DS` / `MN2WS` / `PanaX NEO` / `PX-ADPDWA` / conector `CN55001`) que a porta de 6 pinos do DMR-EH55 operava o depurador síncrono On-Chip Debug **`DWire32A`** (`EXTRG` / `SCLK` / `SDATA` / `nRESET`). Essa hipótese caiu diante de quatro constatações empíricas entre `2026-11-12` e `2026-11-13`:

1. **Auditoria Forense do Mock Sintético (`panasonic_dmr-eh55_flash_dump.bin`):** A inspeção estatística de entropia do primeiro dump de `8.388.608 B` provou que `8.388.509 B` (`99,999%`) eram preenchimento `0xFF` acompanhados por apenas `99` bytes sintéticos de fixture de teste (`PANASONIC DMR-EH55 AM34 IPL BOOTLOADER VEP79138`). A engenharia reversa estática foi redirecionada integralmente para binários reais: [`PANA_DVD.FRM`](../../firmware_and_images/README.md) (`7.531.744 B`) e `dmr-eh58/DVDRAM.7z` (`BOOT.BIN` + `DRV.BIN`).
2. **Falha de Conversores Seriais Passivos e `DCSR: 0xFFFFFFFF` em `DWire32A`:** Adaptadores passivos de 3 fios (`FT232RL`, Arduino sem ATmega) não controlavam o pino de modo (`SW_UART`) nem o pulso de `XRESET` necessários para chavear `XFWE`. Simultaneamente, o comando `HANDSHAKE` do protocolo síncrono `DWire32A` retornava invariavelmente `DCSR: 0xFFFFFFFF` em `P7501`: no chicote conectado, `GP3` ligava-se ao rail estático `Pin 5 (NSW5V)` (não `SCLK`), e `GP4` ligava-se ao pull-up `R7570` em `Pin 3 (UARTM2P_T)` (não `SDATA` bidirecional).
3. **Captura do Ponto de Inflexão em `COM10` (`captura_live_20261112_*.log`):** As primeiras capturas preservadas em [`docs/debug_logs/`](../debug_logs/README.md) documentam literalmente essa transição histórica no firmware do Pico em `2026-11-12`:
   * [`captura_live_20261112_143843.log`](../debug_logs/captura_live_20261112_143843.log) (`126 B`, SHA-256 `5F8A1712...`) e [`captura_live_20261112_151352.log`](../debug_logs/captura_live_20261112_151352.log) (`126 B`, `FC82AFBC...`) ainda imprimem o banner legado **`DWIRE32A>`** ao acionar `UARTMON 30000`.
   * Na execução imediatamente seguinte, [`captura_live_20261112_144101.log`](../debug_logs/captura_live_20261112_144101.log) (`324.209 B`, SHA-256 `C7775A5C...`), o mesmo prompt `DWIRE32A> UARTMON 30000` decodificou em `GP2` o fluxo assíncrono **`115200 8N1`** vivo do SoC AM34: `Loader ver0.61 [06/03/06]`, `cd_diag`, `startup reason = [70]`, erros de cartão `SD -91006/-90002/-90001` e `1.361` frames `SENSE` em `X(80020401)` $\rightarrow$ `[RTSD] === DRIVE ACCESS ERROR === ( 80040904 )` (`0x83A7`).
4. **Refutação Documental Cruzada nos 3 Manuais Oficiais do DMR-EH55:** A varredura integral dos manuais **`CHM0605028CE`** (`panasonic_dmr_eh55p9_eh55pc9_service_manual_north_america.pdf`) e **`DSD0603003CE` Vol. 1 / Vol. 2** (`europe.pdf`, `vol1_diagnostics.pdf`, `vol2_schematics.pdf`) retornou **zero ocorrências** para `MN2DS`, `MN2WS`, `CN55001`, `P55001`, `DWire` ou `IC55007`. Ficou provado que **`P7501` (`JST B6B-ZR-3R4`, legenda `"TO JIGU"` / `"TO WRITE JIGU"`)** na placa `VEP79138` é uma porta **UART assíncrona `115200 8N1`** com controle discreto de boot/reset — motivando a migração arquitetural do firmware e da documentação para `P7501 Async UART` (commit `d0740f6`).

### 5.2 Diagnóstico Eletrônico via Datasheet & Esquemáticos (`VEP79138`)

A correlação vetorial dos esquemáticos (`NA pp. 55, 62–64`, `EC Vol. 1 p. 68`, `Vol. 2 pp. 113, 122, 151, 153`) com datasheets oficiais e medições em DC esclareceu todos os aparentes paradoxos de bancada:

1. **Alimentação `IC1506` / `D7510` $\rightarrow$ `NSW5V` (`P7501` Pino 5 / `GP3`):**
   * O regulador **`IC1506` (`C0DAEYH00002` / `SI-3011ZFE`)** gera `~5.15V` nominais (`X_SW_5R2V` via divisor `R1510 1.5 kΩ`, `R1509 39 kΩ`, `R1511 11 kΩ`) e alimenta o nó `NSW5V` através do diodo duplo **`D7510` (`B0JDCE000002`)** rumo aos pull-ups **`R7570` / `R7572` (`10 kΩ`)** e ao `VDD` do **`IC7501`** (em wired-OR com a bateria `B7501` `CR2354`). Embora o multímetro leia **`~3.29V` em repouso** no Pino 5 de `P7501` devido à queda/divisão no barramento, o potencial upstream de `~5.15V` impõe o invariante absoluto: **`GP3` (`NSW5V`) deve permanecer SEMPRE como `Hi-Z Input` sem pull-up/pull-down internos — NUNCA dirigir em push-pull**.
2. **Estágio Transistorizado Discreto (`Q7510`, `Q7514`, `QR7503`) & Acoplamento `SW_UART` $\leftrightarrow$ `TX`:**
   * **`Q7510` (`2SD0601A0L` NPN):** tem o Coletor ligado ao **`P7501` Pino 1 (`SW_UART`)** e ao `IC7501` Pino 80, Base polarizada em `D3R3V (3.30V)` por `R7562 (4.7 kΩ)`, e Emissor ligado a **`XFWE`**, ao pull-up `R7559 (10 kΩ)` (`3.30V`) e, via **`R7528 (4.7 kΩ)`**, à Base de **`Q7514` (`2SD0601A0L`)**.
   * Esse circuito explicou simultaneamente dois fenômenos medidos na bancada: (a) sob pull-down interno do Pico (`~50–80 kΩ`), a junção Base-Coletor de `Q7510` conduz e grampeia `GP26` exatamente em **`2.68–2.69V`** ($3.30\text{ V} - V_{BC}$, vs **`3.24–3.25V`** em `Hi-Z`), protegendo o ADC0; e (b) forçar **`SW_UART = LOW`** drena `XFWE` para `0V` e **coloca `Q7514` em corte via `R7528` (`4.7 kΩ`)**, bloqueando fisicamente `Pin 3 (UARTM2P_T)` (`R7570 10 kΩ`) de alcançar `UARTM2P` no SoC (`R7527 4.7 kΩ`) até que `SW_UART` seja liberado para `Hi-Z` (provando por que tentativas precoces de `UARTSEND` com `SW_UART = LOW` obtiveram `0` bytes, somado ao fato de `uart_gets` em `0x4400488a` ter `0` chamadores em `BOOT.BIN`).
   * **`QR7503` (`UNR511300L` PNP digital)** e **`XRESET` (`Pin 2` / `GP5`):** `QR7503` acopla `SW_UART` ao pino 15 (`MMOD`, `R7513 4.7 kΩ`) do `IC7501` e compõe a malha `XRESET`, cujo controle em **`GP5`** deve ser estritamente **Open-Drain** (`0V` ou `Hi-Z`, subida medida de **`~20 µs`**). Em Standby real (`P-OFF`, `DI_P_ON=0V`), o domínio comutado de `P7501` zera (`VDD = 0.03V`), enquanto o domínio Always-On mantém `IC7501` / `TW7501` / `C7584 (0.047F)` / `R7623 (180 Ω)` / `IC7502`.
3. **Ausência de EEPROM Externa no Timer MCU (`IC7501` `EEPROM_W_C = NC (OPEN)`):**
   * Os esquemáticos (`EC Vol. 1 p. 68`, `Vol. 2 p. 122`) mostram o pino **`EEPROM_W_C` de `IC7501` explicitamente como `NC (OPEN)`**, confirmando que o **DMR-EH55 não possui EEPROM serial externa no Timer MCU** (`IC3001` `C1AB00002379`/`1979` é a matriz I²C de A/V nos endereços 7-bit `0x4A` / 8-bit `0x94`). Toda persistência de configuração, BGO e NVRAM reside na Flash NOR `IC5001`.
4. **Datasheet da Flash NOR Spansion `S29JL064H70TFI00` (`IC5001`, Pub. `S29JL064H_00` Rev. B, Dez/2005):**
   * Memória **8 MiB (`8.388.608 B`)** TSOP-48 Top-Boot 3.0V com **142 setores (`SA0..SA141`: `8×8 KiB` + `126×64 KiB` + `8×8 KiB`)**.
   * **Pino 14 (`WP#/ACC`):** quando aterrado (`0V` / $V_{IL}$), protege por hardware os dois micro-setores superiores **`SA140` (`0x007FC000`) e `SA141` (`0x007FE000`)**, exigindo nível lógico HIGH (`3.3V`) antes de qualquer apagamento in-circuit de NVRAM.
   * **Temporização real de apagamento (~`700 ms`):** o erase físico de setor de `64 KiB` leva ~`700 ms` (superando laços limitados de `DQ7` de `0x00100000` iterações em `jedec_emit.rs`), impondo polling externo de leitura **`0xFF × 32`** após `OK:ERASE` antes de programar bytes.
5. **Pinagem Invertida `P58001` & Polaridade Assimétrica `DAVIS` (`EV-20261119-005`/`006`, `N4_HARDWARE`):**
   * O esquemático `vol2_schematics.pdf` p. 113 revelou que o conector FFC de 40 vias do HDD **`P58001`** inverte integralmente a ordem ATA segundo $\text{Pino}_{P58001}(k) \equiv \text{ATA}(41 - k)$ ao longo do conjunto `VEK0K00` $\rightarrow$ placa adaptadora `N5EZZ0000003` $\rightarrow$ ponte `StarTech IDE2SAT2`. Testes ao vivo confirmaram que girar `IDE2SAT2` para **`180°`** com cabo Tipo A lia `0x00002004` em todos os registradores (`DD13|DD2`, expondo `IORDY`/`CSEL` `3.3V`), enquanto **`0°`** ([`probe_ch1_live_v2_0deg_20261119.log`](../debug_logs/probe_ch1_live_v2_0deg_20261119.log)) eliminou o travamento de `R 89000D58,4` retornando `0x00000000`, com tensões DC verificadas na `IDE2SAT2` (`Pin 27 IORDY = 3,34 V`, `Pins 1/23/25/37/38 = 2,45–2,47 V`, sem curto-circuito).
   * A sonda ao vivo `J 504156AD` ([`probe_fe_release_com10_20261119.log`](../debug_logs/probe_fe_release_com10_20261119.log), [`probe_ch1_full_diag_20261119.log`](../debug_logs/probe_ch1_full_diag_20261119.log)) provou a polaridade assimétrica de reset do controlador **`DAVIS`**: **`CH0` (ODD) é ativo-BAIXO** (`0x504F81D0 = 2`, liberado por `0x89000420 = 0x02/0x03`, respondendo `STATUS=0x50` / `0xEB14` ATAPI), enquanto **`CH1` (HDD) é ativo-ALTO** (`0x504F81D4 = 1`, liberado por `0x89000C20 = 0x00` por inversor externo).

### 5.3 Disparo de Pulsos Controlados pela Sonda Pico (`RP2040`/`RP2350`) e Captura da UART

O firmware C11 modular em `firmware/pico-p7501/` consolidou uma sonda **Zero-Gadget** dual-target (`RP2040` e `RP2350`, com BLE NUS opcional em `Pico W`/`Pico 2W` via CYW43439) contida integralmente em `P0_FW_RO` (`256 KiB`), empacotada no UF2 universal `panasonic_p7501_uart_unified.uf2` (auto-fatiado por `INFO_UF2.TXT`) e coberta por **38 suítes/funções de teste host C11** em `4` módulos:

1. **Varredura Elétrica `PINCHECK` & `BUSDIAG` ([`p7501_diag_electrical.c`](../../firmware/pico-p7501/diag/p7501_diag_electrical.c), [`p7501_diag_busdiag.c`](../../firmware/pico-p7501/diag/p7501_diag_busdiag.c)):**
   * `PINCHECK` mede os 6 pinos de `P7501` em Hi-Z, pull-down e pull-up (exceto `GP3/NSW5V`, mantido sem pulls) e amostra `GP26/ADC0` (**`~3.25V`** Hi-Z / **`~2.69V`** pull-down / **`~3.27V`** pull-up), aplica guarda anti-backpower quando `VDD` está desligado e cronometra a subida `~20 µs` de `XRESET` em Open-Drain. (Corrigiu-se o bug em `ISOLATE` que deixava `GP26` como saída SIO `LOW` lendo `0.02V`, restaurando `adc_gpio_init(GP26)`.)
   * `BUSDIAG` varre a força de drive (`2mA`, `4mA`, `8mA`, `12mA`) em `GP4` através dos 3 estados de `SW_UART`/`XRESET` e contabiliza transições nas janelas `Phase A (0..1300ms)` e `Phase B (1300..3500ms)`, comprovando `GP2 = 872 / 2124 / 3308` transições frente a `0` em `GP3` e `GP4`.
2. **Máquina de Estados PIO 8x & Amostradores Lógicos ([`p7501_uart_duplex.pio`](../../firmware/pico-p7501/p7501_uart_duplex.pio), [`p7501_logic_sampler.pio`](../../firmware/pico-p7501/p7501_logic_sampler.pio), [`p7501_sds_sampler.pio`](../../firmware/pico-p7501/p7501_sds_sampler.pio)):**
   * `p7501_uart_rx_fast` (`PIO1 SM0` em `GP2`) executa **oversampling 8x** (`921.600 Hz` @ `115200 8N1`), aguarda `1,5` bit (`[10]` + ciclos de setup) até o centro de `D0`, amostra cada bit a cada `8` ciclos e verifica o Stop bit via `jmp pin` sinalizando erro de enquadramento em `irq 4` sem jamais tocar `GP3`. `p7501_uart_tx_fast` (`PIO1 SM1`) transmite em `GP4` via `Q7514`. Em paralelo, `PIO0` opera o amostrador lógico de 4 canais (`PIOLOGIC`) e o sniffer SD passivo (`SDSNIFF`, `GP10..15`).
3. **Orquestração `UARTPROBE`, `JIGBOOT`, `BOOTTRACE`, `UARTSTREAM` & `DIAGWATCH` ([`p7501_diag_watch.c`](../../firmware/pico-p7501/uart/p7501_diag_watch.c)):**
   * Coordenando o pulso Open-Drain em `XRESET` (`GP5`) com retenção cronometrada de `SW_UART = LOW` (`JIGBOOT` / `JIG:2500` ou disparo automático por padrão `"serial boot mode"` / `"DDR CHECK: OK"`), a sonda capturou `<00>DDR CHECK: OK`, `Chip select = 1`, `***** Loader ver0.61 [06/03/06] *****`, `***** serial boot mode *****`, `startup reason = [70]`, `CD diag boot mode`, o boot SD 4-bit ao vivo em [`sddiag_ver061_live_20261119.log`](../debug_logs/sddiag_ver061_live_20261119.log) (`0x3c4000[LBA]`, `25000 Kbps`, `641 B`, `ferr=0` $\rightarrow$ `***** Fvu ver0.61 [05/11/01] *****`), detecção automática de acordamento `PREJIG WAKE` (`VDD >= 2.0V`) e injeção de comandos no monitor **`PANAMON1>`**.
4. **Ponte Transparente `BRIDGE 115200 PASSIVE` ([`p7501_uart_tx.c`](../../firmware/pico-p7501/uart/p7501_uart_tx.c), `crates/panadiga-core/src/pico/panamon1/`):**
   * Estabelece passthrough full-duplex USB-CDC $\leftrightarrow$ `SC0` com guarda Hayes de `50 ms` para saída `~~~`, telemetria de bordas `[CTRL]`, sonda segura `?\r` (evitando `\r` nu ou frames GDB `#B1` que disparariam `'B'` = `OK:BYPASS`) e espaçamento determinístico de `300–350 µs` entre bytes para respeitar o registrador `SC0RXB` (1 byte) do MN103S durante dumps de `8 MiB`, reparos BGO e sondas `DAVIS`.

### 5.4 Avanços Subsequentes até a Arquitetura Atual (`475fd48`)

A posse de acesso vivo ao monitor `PANAMON1>` em `COM10` impulsionou quatro saltos sucessivos até o estado consolidado no commit **`475fd48`**:

1. **Dump Ouro de `8 MiB` (`IC5001`) & Descoberta da Geometria BGO Real de `45` Setores (`0x440337A4`):**
   * Via `PANAMON1>` + `PANADMP1` na janela *uncached* `0x84000000..0x847FFFFF`, capturou-se o dump integral [`eh55_nor_8mb_ic5001_dump.bin`](../../firmware_and_images/README.md) (`8.388.608 B`, SHA-256 `c1faee798f60545853eefdba0d15f23678b471466dc2bc84f522bc972f15a02d`), além dos recortes `eh55_loader_ver061_256k.bin` (`9f5c448d...`) e `eh55_nvram_64k_raw.bin` (`803d1465...`).
   * A desmontagem cruzada de `Loader ver0.61` provou que o DMR-EH55 usa a tabela **`0x440337A4`** com **`45 × 64 KiB` setores em `SA26..SA70` (`0x84130000..0x843FFFFF`, `40` blocos lógicos `0xF2xx`, `D_ID 0x0F01..0x0F24`)** — refutando a assunção prévia baseada na tabela de **`35` setores em `0x84200000..0x843FFFFF` (`28` blocos, `D_ID 0x0A01`)** do `Loader ver0.63` (`/BOOT.BIN`) e do `PROG061` stock (`0x50436E30`, EH50 `M6930`). Também comprovou que `Loader ver0.61` exige `DIGADIAG.FRM` **bruto de `262.144 B` (`0x40000`)** em `0x90400000` (`EV-20261119-001`) e TOC plaintext **`sd48`** (`0x1800 B`, stride 48 B) em `PANA__SD.FRM`.
2. **Cura Cirúrgica BGO/`U99` em 2 Ciclos (`SA54`/`SA41`/`SA29`/`SA39`) & Porta #1 Sem JIG (`EV-20261119-002`/`003`/`004`/`007`):**
   * **Ciclo 1** ([`apply_bgo_repair_com10_20261119.log`](../debug_logs/apply_bgo_repair_com10_20261119.log)): sob `U CONFIRM_UPPER_NOR` / `U LOCK` (preservando `SA0..SA3` bloqueados em `E05`), formatou `SA70` (`256` zeros) e aplicou `1.629` escritas puras `1 -> 0` em `13` setores *spare* (`SA42..SA66`, restaurando `ifodev2` blk2 `0x0055` e páginas FAT `0x0F0A`/`0x0F19`, equivalente offline `adb21d95...`).
   * **Diagnóstico da Recaída & Ciclo 2** ([`repair_bgo_sa39_sa54_com10_20261119.log`](../debug_logs/repair_bgo_sa39_sa54_com10_20261119.log), [`verify_bgo_restored_boot_20261119.log`](../debug_logs/verify_bgo_restored_boot_20261119.log)): explicou-se o rollback `+0x14` em `SA41 -> SA29` (`FUN_4401F536`), a exaustão de `0xF20A` em `5` ciclos `"ATIHSTAM"` (`SA26 -> SA29`) e o apagamento de `SA54`/`SA39..SA69` + aborto em `SA70` (`0x843FE100`) causado pelo `Fvu` stock `0x2182DC9`. O Ciclo 2 restaurou `SA54`, `SA41` slot 9, `SA29` (`0x51 -> 0x64`) e `SA39` (`E 84200000`), compactando para **`1` `0xF101` (`SA29`) + `40` `0xF2xx` + `4` spares limpos (`SA41, 67, 68, 70`)** e `19/19` cadeias FAT íntegras. O boot normal passou a executar `Loader ver0.61` $\rightarrow$ `normal mode` $\rightarrow$ `*** RTSC Initialize ***` **sem `U99`**, gerando a captura pós-reparo ao vivo [`eh55_nor_8mb_ic5001_repaired.bin`](../../firmware_and_images/README.md) (`8.388.608 B`, SHA-256 `bd089ed1a062c48756c60407e6555463e8646cde094b0328b88a253e4f1d4121`, `317,8 s`, `ferr=0`), o descritor brasileiro `eh55_ifodev2_m6930lb_2048b.bin` (`M6930LB`, `f4f22dd0...`) e a prova da **Porta #1** de entrada direta em `PANAMON1>` sem pino JIG quando `CH1` falha ([`probe_ch1_full_diag_20261119.log`](../debug_logs/probe_ch1_full_diag_20261119.log)).
3. **Pipeline Ghidra Zero-Undef (`0 ??`) & Camada Relacional SSOT (`profiles/`):**
   * O patch [`am33_panadiga_zero_undef.patch`](../../tools/ghidra/sleigh_patches/am33_panadiga_zero_undef.patch) (corrigindo `12` defeitos SLEIGH em `am33.sinc` e `mn103.sinc` sobre o clone isolado `Panadiga_MN103S_ZeroUndef`) zerou os bytes indefinidos e `mn_todo()` em todos os `6` estágios binários: **`3.626.323` instruções e `45.034` funções com `0` `??`** ([`GHIDRA_UNDEF_AUDIT_REPORT.md`](../pseudocode/GHIDRA_UNDEF_AUDIT_REPORT.md)), unificados em SSOT Rust/JSON (`ghidra_maps_ssot.json` $\rightarrow$ `ghidra_ssot_maps.json`).
   * O crate `panadiga-profiles` e `profiles/` consolidaram **89 modelos DIGA**, **12 JSON Schemas** Draft 2020-12, **796 checagens relacionais** e o **T0..T4 Capability Gate** (restringindo escrita ao vivo ao `DMR-EH55` `T4`).
4. **Desbloqueio de HDD `5 GB..2 TB`, Preset `250 GB` RTSD (`0x07466528`) & Gravação em `PhysicalDrive1` (`EV-20261119-008`):**
   * [`crates/panadiga-core/src/hdd/capacity_patch/specs.rs`](../../crates/panadiga-core/src/hdd/capacity_patch/specs.rs) passou a cobrir de **`5 GB` (`0x0024C388`, `SAMSUNG SV0511D`) até `2 TB` (`DYNAMIC_UNCLAMPED_MAX_BLOCKS_2KB = 0x3FFF_FFFF`, mais teto explícito `4 TiB`)** via bypass `0xC6 -> 0xCA` em `src/hdd_cmd.c` (`3410PQ @ 0x574DD1FC`, `3060NP @ 0x574E499C`) e os 9 presets RTSD (`#3 250GB = 0x07466528` = `122.053.928` blocos 2 KiB = `488.215.712` LBAs). Em [`hdd_unbrick_flash_250gb_20261119.log`](../debug_logs/hdd_unbrick_flash_250gb_20261119.log), o disco `\\.\PhysicalDrive1` (`250 GB`) recebeu backup CoW (`354d38bd...`), imagem `panasonic_dmr_eh55_hdd_250gb_patched.img` (`4fde753c...`) e espelho P1 no `LBA 488266096` (`d932692c...`) validados por SHA-256 read-back.
5. **Estado Pós-`475fd48` (SSOT Consolidada) — Retarget Declarativo `BGO_45SEC_8MB_0X84130000`, `PANA__SD.FRM` FVU Desacoplado (`MAIN3060NP` + `250 GB`) & Seletor RTSD:**
   * [`crates/panadiga-core/src/re/bgo_unbrick/retarget.rs`](../../crates/panadiga-core/src/re/bgo_unbrick/retarget.rs) (evoluído do módulo inicial do commit `475fd48`) traduz declarativamente via `profiles/firmware_trains/` o `PROG061` de fábrica (`BGO_35SEC_4MB_0X84200000`) para **`BGO_45SEC_8MB_0X84130000`** (`0x84130000..0x84400000`, `45` setores físicos / `40` lógicos, `D_ID 0x0F01..0x0F24`), transplantando `nfrom_drv.o` de `Loader ver0.61` (`29.770 B`, [`profiles/blobs/loader_ver061_nfrom_drv.bin.gz`](../../profiles/blobs/loader_ver061_nfrom_drv.bin.gz)), relocando `211` ponteiros RAM para `0x504FA000..0x504FE000`, ajustando `.bss` (`0x504FE000`) e aplicando `34` patches de geometria/`D_ID` — sem nomes de produto hardcoded no código Rust.
   * [`crates/panadiga-core/src/re/unbrick_sd_kit/compose/builders.rs`](../../crates/panadiga-core/src/re/unbrick_sd_kit/compose/builders.rs) e [`firmware_and_images/sd_unbrick_kit/SHA256SUMS.txt`](../../firmware_and_images/sd_unbrick_kit/SHA256SUMS.txt) desacoplaram os dois modos do kit SD: **`DIGADIAG.FRM`** (`305cc2ee...`, `262.144 B`) carrega `UART_MONITOR_PANAMON1` + `BGO_45SEC_8MB_0X84130000` para JIG `P7501`, enquanto **`PANA__SD.FRM`** (`2d3d62a7...`, `7.563.224 B`, `sd48`, `6` slots lidos do SSOT) embute `PROG061` em **`BYPASS_CHECKS_ONLY`** + `BGO_45SEC_8MB_0X84130000` para instalar flexivelmente `MAIN3060NP` (padrão do kit com HDD **`250 GB`** `0x07466528`) ou `MAIN3410PQ` em HDDs zerados via `"SD version up mode."`, controlado na UI desktop por [`SdKitBuilderCard.tsx`](../../apps/desktop/src/components/unbrick/SdKitBuilderCard.tsx) e [`SdKitProfileControls.tsx`](../../apps/desktop/src/components/unbrick/SdKitProfileControls.tsx) (`Auto`, `250 GB` RTSD, `400..80 GB`).
   * **Fronteira Aberta Documentada:** o firmware `MAIN` aguarda a conclusão do handshake `IDENTIFY DEVICE (0xEC)` no canal `CH1` (`P58001`–`VEK0K00`–`N5EZZ0000003`–`IDE2SAT2`, com tensões DC validadas `IORDY = 3,34 V` e `2,45–2,47 V` nas linhas de controle) para bootar pelo HDD em hardware real, junto aos ensaios de Estágio 0 `PAL-M` / TBC (`AIOF` `0x8800_0000`, `AVDD` `0x8980_0000`, keepalive `M0` `TBUS`).

### 5.5 Ecossistema Open-Source & Repositórios Citados (Emulação EEPROM/Flash & Ferramentas Panasonic)

O desenvolvimento do Panadiga apoiou-se na análise comparativa entre arquiteturas modernas abertas de emulação EEPROM/drivers NOR e os utilitários comunitários de engenharia reversa Panasonic:

#### Emulação EEPROM em Flash Spansion / Cypress / Infineon vs. `EEMUL` & `BGO` Matsushita

| Projeto / Fonte Aberta                                   | URL Canônica                                                                | Papel Técnico & Contraste com a `S29JL064H` (`IC5001`) do DMR-EH55                                                                                                                                                                                                                                                                                                                                                                                            |
| :------------------------------------------------------- | :--------------------------------------------------------------------------- | :----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **`Infineon/emeeprom`**                          | https://github.com/Infineon/emeeprom                                         | Middleware canônico Emulated EEPROM da ModusToolbox (PSoC / XMC / Traveo): gerencia *wear-leveling*, filas redundantes e checksums em Flash interna. Contrasta diretamente com o motor proprietário **`EEMUL`** (`"1284"`, cabeçalho `0x0FF00000`, `1.158` tags em `SA71..SA141`, cauda `0ff00000002c` em `SA134`) que substitui EEPROM I²C externa ausente no EH55 (`EEPROM_W_C = NC`).                                               |
| **`Infineon/mtb-xmc-emeeprom`**                  | https://github.com/Infineon/mtb-xmc-emeeprom                                 | Biblioteca ModusToolbox que adapta o algoritmo `emeeprom` sobre o driver Flash `XMCLib` (famílias industriais XMC1xxx/XMC4xxx), ilustrando paginação lógica sobre setores físicos grandes — paralelo conceitual útil ao particionamento de slots `512 B` dentro de setores `64 KiB` da FTL BGO `[nfrom]` (`SA26..SA70`).                                                                                                                        |
| **`Infineon/mtb-example-psoc6-emulated-eeprom`** | https://github.com/Infineon/mtb-example-psoc6-emulated-eeprom                | Aplicação de referência demonstrando persistência não-volátil em Flash auxiliar/interna de MCUs PSoC 6 com `emeeprom`, evidenciando a distância arquitetural entre esquemas modernos documentados por linha/checksum e a ausência de documentação pública para os headers dual-bitmap (`+0x14` rollback / `+0x5C` forward) da Matsushita.                                                                                                         |
| **Linux MTD `drivers/mtd/spi-nor/spansion.c`**   | https://github.com/torvalds/linux/blob/master/drivers/mtd/spi-nor/spansion.c | Driver mainline do kernel Linux para Flash NOR Spansion/Cypress/Infineon (ID fabricante `0x01`, tabelas SFDP, quirks de geometria multi-die e proteção de setores). Diferente de dispositivos SPI-NOR modernos atendidos por `spansion.c`, a **`S29JL064H70TFI00`** do EH55 é **NOR paralela 16-bit TSOP-48** (barramento CFI/JEDEC `0xAA`/`0x55`/`0xA0`, `8×8K + 126×64K + 8×8K`, `WP#/ACC` no Pino 14 e ~`700 ms` de erase). |

#### Decompiladores, Desempacotadores, Recompiladores, Forense & IR Panasonic

| Repositório / Projeto                        | URL Canônica                                                               | Linguagem & Função no Ecossistema / Integração Panadiga                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| :-------------------------------------------- | :-------------------------------------------------------------------------- | :--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **`WeebDataHoarder/pana-fw`**         | https://git.gammaspectra.live/WeebDataHoarder/pana-fw                       | **Go:** referência estrutural para linha Blu-ray/UniPhier: `MainListHeader` (`20 B`, Feistel parcial `0x1400`/`0x2800`), `DRV` (`32 B` + trailer `-48`), skip `VGD`, divisão e reconstrução `fmaX` (`RomFS`/`CramFS`), `findKeySlow`, `libdownloadlib.so` `NetworkKeyIV`, `AesHeader` (`2016`/`2018` AES-128-CBC + Feistel), console `SLM` UniPhier e códigos IR Kaseikyo `[RED]`/`ND2` — totalmente integrado em Rust puro (`panadiga frm unpack-all` / `repack`) em [`crates/panadiga-core/src/crypto/`](../../crates/panadiga-core/src/crypto/universal_frm/mod.rs). |
| **`theubusu/unixtract`**              | https://github.com/theubusu/unixtract                                       | **Rust:** desempacotador universal (`PANA_DVD.FRM` / `PANA_ESD.FRM` / `PANAEUSB.FRM`, `12` chaves Feistel, pares AES `2016`/`2018`, `keys.ukf`, `EXTRHEAD`/`EXTRFOOT`, Okumura LZSS / GZIP / LZMA) — unificado com o motor legado DIGA HDD/SD em [`crates/panadiga-core/src/crypto/aes128.rs`](../../crates/panadiga-core/src/crypto/aes128.rs) (`--key-file keys.ukf`) e [`scripts/re/README.md`](../../scripts/re/README.md). |
| **`theubusu/sddl_dec`**               | https://github.com/theubusu/sddl_dec                                        | **Decodificador `SDBOOT` / `SDDL.SEC`:** decifra pacotes de atualização SD e de rede Panasonic (`tbl2009`/`tbl2010`/3DES e chaves AES-128-CBC pós-2011 `SDDL_SEC_NEW_KEY`).                                                                                                                                                                                                                                                                                                                         |
| **`txt231/GhidraMn10300`**            | https://github.com/txt231/GhidraMn10300                                     | **Ghidra SLEIGH (`mn10300:LE:32:default`):** especificação de processador para a família Matsushita `MN103`/`MN10300`/`AM33`; serviu de base sobre a qual o Panadiga desenvolveu [`am33_panadiga_zero_undef.patch`](../../tools/ghidra/sleigh_patches/am33_panadiga_zero_undef.patch) corrigindo `12` defeitos de decodificação/semântica até atingir `0 ??`.                                                                                                                                |
| **`leecher1337/panasonic-rec`**       | https://github.com/leecher1337/panasonic-rec                                | **C (`extract_meihdfs` / `meihdfs1.h`):** projeto pioneiro na engenharia reversa do **`MEIHDFS-V2.0` / `V2.1`**, mapeando superblocos, `itbl`, unidade de alocação `ASIZE` de `768 KiB`, `320` extents por inode e época `1980-01-01` (`TIME_OFFSET = 315532800`).                                                                                                                                                                                                                  |
| **`haliner/dvr-recover`**             | https://github.com/haliner/dvr-recover                                      | **Python:** ferramenta forense de varredura de superfície bruta de discos DVR/CCTV por assinaturas de stream de vídeo, citada como referência comparativa de recuperação de mídia sem metadados intactos.                                                                                                                                                                                                                                                                                                  |
| **`lukasz-gratkowski/diga-archive`**  | https://github.com/lukasz-gratkowski/diga-archive                           | **Windows / Rede:** utilitário para conectar-se via rede local a gravadores Panasonic DIGA, navegar por títulos gravados e arquivá-los localmente ou em nuvem sem recodificação.                                                                                                                                                                                                                                                                                                                            |
| **`Flipper-IRDB` (`emptythevoid`)** | https://github.com/Lucaslhm/Flipper-IRDB · https://github.com/emptythevoid | **Base Comunitária IR:** forneceu o arquivo verificado `DVD_Players/Panasonic/Panasonic_DVD_TV.ir` (`4.943 B`, SHA-256 `82acb0a7...`, `50` sinais Kaseikyo: `6` TV `0x80` + `44` DVD/DIGA `0xB0`), cruzado com o controle físico `EUR7659Y70` e perfis Android `.irplus`.                                                                                                                                                                                                                  |

---

## 6. Matriz de Síntese e Diretrizes de Engenharia para o Ecossistema Panadiga

A consolidação de todos os dados da comunidade internacional estabelece as seguintes diretrizes para o ecossistema Panadiga:

1. **Validação da Cifra Feistel:** A compatibilidade da chave `DF7215C7427FD010` abrange desde o `DMR-EH55` até a série `EH49/59/69` e `EX72..EX93`, confirmando que o algoritmo `FUN_4400a666` é o padrão canônico da Matsushita para todas as imagens `.FRM` da era SD/DVD analógica e DVB inicial.
2. **Regra de Ouro da Tensão do Conector P7501:** O pino 5 (`NSW5V`), originado em `IC1506` com potencial upstream de ~5.15V, deve **SEMPRE permanecer como entrada em alta impedância (`Hi-Z Input`) sem resistores de pull no microcontrolador Raspberry Pi Pico**, garantindo proteção física absoluta contra a queima do silício do SoC Matsushita AM34.
3. **Cura do Erro `U99`:** O código `U99` não é um erro de defeito de componente único, mas o resultado determinístico do travamento do bootloader `PROG.bin` quando a verificação dos últimos 64 KB da NVRAM Spansion falha — ou, no DMR-EH55 real (`Loader ver0.61`), da corrupção do FTL BGO `[nfrom]` em `SA26..SA70` (`0x84130000..0x843FFFFF`). Sua resolução definitiva exige preservação integral do bootloader IPL (`SA0..SA10`, `256 KiB` em `E05`), sanitização precisa da faixa NVRAM `0x807F0000..0x807FFFFF` quando aplicável e reconstrução BGO verificada por SHA-256 com retarget declarativo `BGO_45SEC_8MB_0X84130000`.

---

## 7. Referências Técnicas e Documentos Correlatos

* [Auditoria Completa do Ecossistema Panadiga](../../AUDITORIA_COMPLETA.md)
* [Relatório 100% Fiel da Sessão de Engenharia Reversa (`DMR-EH55` / `COM10` / `P7501`)](RELATORIO_SESSAO_ENGENHARIA_REVERSA_EH55_100_FIEL.md)
* [Dossiê Histórico e Forense EH55: BGO (`SA39`/`SA54`), `PANAMON1>`, `DAVIS`, `P58001` &amp; `PAL-M`](HISTORICO_UNBRICK_EH55_BGO_CH1_DAVIS_E_REGIAO.md)
* [Blueprint de Engenharia de Hardware e Circuitos de Bancada (`VEP79138` / `S29JL064H`)](BLUEPRINT_ESQUEMA_ELETRICO_BANCADA.md)
* [Guia do Firmware Pico `P7501` (`RP2040` / `RP2350`)](FIRMWARE_PICO_P7501_GUIA.md)
* [Dossiê Técnico Forense do Bootloader SDDIAG (`Loader ver0.61` vs `ver0.63`)](DOSSIE_FORENSE_BOOTLOADER_SDDIAG_EH55.md)
* [Armadilhas MN103S e Resolução Ghidra Zero-Undef](ARMADILHAS_MN103S_E_RESOLUCAO_GHIDRA.md)
* [Catálogo e Auditoria de Perfis de Hardware DIGA (89 Modelos)](CATALOGO_E_AUDITORIA_PERFIS_HARDWARE_LINHA_DIGA.md)
* [Dossiê Técnico de Criptografia e Firmware PANA_DVD](DOSSIE_CRIPTOGRAFIA_E_FIRMWARE_PANA_DVD.md)
* [Teoria e Prática de Unbrick U99 e Criptografia Matsushita](TEORIA_E_PRATICA_UNBRICK_U99_E_CRIPTOGRAFIA_MATSUSHITA.md)
* [Arquitetura de Storage MEIHDFS e Partições](ARQUITETURA_STORAGE_MEIHDFS_PARTICOES.md)
* [Especificação da Interface Assíncrona P7501 UART](PANASONIC_P7501_UART_RECONSTRUCTION.md)
* [Índice de Evidência de Pseudocódigo Ghidra MN103S](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md)
* [Catálogo de Imagens e Firmware Oficial Panadiga](../../firmware_and_images/dmr-ex_ez_series/README.md)
* [Catálogo Oficial das Séries DMR-EH49 / EH59 / EH69](../../firmware_and_images/dmr-eh49_59_69/README.md)
