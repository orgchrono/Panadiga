> 🌐 **Idioma / Language:** [English (US)](../en-US/PANASONIC_DVD_TV_IR_CAPTURE_REFERENCE.md) | **Português (BR)**
> 🧭 **Portal:** [Índice da Documentação](../INDEX.md) | [Auditoria Completa](../../AUDITORIA_COMPLETA.md) | [Auditorias](../audits/README.md) | [Pseudocódigo Ghidra](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Logs de Debug](../debug_logs/README.md)

# Referência Técnica de Captura IR: Panasonic DVD/TV (`Panasonic_DVD_TV.ir`)
## Decodificação Kaseikyo 48-bit LSB-first, Inventário Integral dos 50 Sinais e Comparação com `EUR7659Y70`, Teletexto e Android CIR

**Documento:** `REFERENCIA_CAPTURA_IR_PANASONIC_DVD_TV.md`  
**Artefato Fonte:** [`firmware_and_images/Panasonic_DVD_TV.ir`](../../firmware_and_images/Panasonic_DVD_TV.ir) (`4.943 bytes`, `305` linhas, `50` sinais parseados)  
**SHA-256 Verificado:** `82acb0a7611c89ea56799094bf3c71b9e366f38c0301b457cf5c8266279a9b99`  
**Origem / Crédito:** `Flipper-IRDB` (`DVD_Players/Panasonic/Panasonic_DVD_TV.ir`, crédito `emptythevoid` — `https://github.com/emptythevoid`)  
**Data:** Novembro de 2026  

---

## 1. Sumário Executivo & Metadados do Artefato

O arquivo [`firmware_and_images/Panasonic_DVD_TV.ir`](../../firmware_and_images/Panasonic_DVD_TV.ir) é uma captura real em formato **Flipper Zero IR (`Filetype: IR signals file`, `Version: 1`, `protocol: Kaseikyo`)** extraída de um controle remoto combo norte-americano Panasonic DVD/HDD + TV. Apesar do nome genérico `Panasonic_DVD_TV`, o conjunto completo de sinais inclui comandos dedicados aos gravadores Panasonic **DIGA** com HDD/DVD-RAM (`Time_slip`, `Cm_skip`, `Direct_navigator`, `Create_chapter`, `Rec_mode`, `F_rec`), distribuídos em **50 sinais parseados** (`linhas 7–305`):

* **Subsistema TV (`dev = 0x80`):** **6 sinais** (`Tv_power`, `Tv_video`, `Tv_chan_up`, `Tv_chan_down`, `Tv_vol_up`, `Tv_vol_down`).
* **Subsistema DVD / Gravador DIGA HDD (`dev = 0xB0`):** **44 sinais** (energia, teclado numérico `0`–`9`, transporte, navegação, programação e funções específicas de gravação).

---

## 2. Fórmula de Decodificação Kaseikyo do Formato Flipper `.ir`

Cada entrada `type: parsed` / `protocol: Kaseikyo` em `Panasonic_DVD_TV.ir` apresenta exatamente dois vetores de 4 bytes em ordem *little-endian*:

```text
address: <dev> 02 20 <hi>
command: <c0> <c1> 00 00
```

### 2.1 Anatomia dos Campos Flipper vs. Quadro Físico de 48 bits (6 Bytes)

O protocolo infravermelho **Panasonic Kaseikyo** (portadora `38 kHz`, unidade temporal `T ≈ 432 µs`, preâmbulo `8T` mark / `4T` space) transmite **48 bits (6 bytes) em ordem LSB-first**:

```text
Byte 0 = 0x02                 (Vendor ID Panasonic LSB: 0x2002)
Byte 1 = 0x20                 (Vendor ID Panasonic MSB: 0x2002)
Byte 2 = <dev>                (Device / Genre: 0xB0 = DVD/DIGA HDD | 0x80 = TV)
Byte 3 = 0x00                 (Subdevice = 0x00 / SET 1 em todos os 50 sinais do arquivo)
Byte 4 = <cmd_byte>           (Código de comando de 8 bits)
Byte 5 = <dev> ^ <cmd_byte>   (Checksum XOR: dev ^ subdevice ^ cmd_byte, com subdevice = 0x00)
```

No registro compactado do Flipper Zero:
1. **`address[0]` (`<dev>`)** guarda o byte de dispositivo (`0xB0` ou `0x80`, cujo nibble inferior `0x0` também coincide com a paridade dos nibbles do Vendor ID `0^2^2^0 = 0x0`).
2. **`address[1..2]` (`02 20`)** preservam os dois bytes do Vendor ID Matsushita (`0x2002`).
3. **`address[3]` (`<hi>`)** varia entre `0x00` e `0x03`, armazenando os **2 bits mais significativos `[7:6]`** de `<cmd_byte>`.
4. **`command[0]` (`<c0>`)** apresenta sempre nibble inferior `0x0` (refletindo `subdevice = 0x00`), enquanto seu nibble superior (`c0 >> 4`) armazena os **4 bits menos significativos `[3:0]`** de `<cmd_byte>`.
5. **`command[1]` (`<c1>`)** varia entre `0x00` e `0x03`, armazenando os **2 bits intermediários `[5:4]`** de `<cmd_byte>`, enquanto `command[2..3]` permanecem `00 00`.

### 2.2 Equação Exata de Reconstrução

A extração determinística do byte de comando e do quadro completo de 6 bytes na linha IR (*LSB-first*) segue a relação exata:

```text
cmd_byte = ((hi & 0x03) << 6) | ((c1 & 0x0F) << 4) | (c0 >> 4)

Quadro Wire 48-bit (6 bytes, LSB-first):
  02 20 <dev> 00 <cmd_byte> <dev ^ cmd_byte>
```

#### Exemplos Verificados Diretamente do Arquivo
* **`Dvd_power` (linhas 7–11):** `address: B0 02 20 00`, `command: D0 03 00 00`  
  `cmd_byte = ((0x00 & 0x03) << 6) | ((0x03 & 0x0F) << 4) | (0xD0 >> 4) = 0x00 | 0x30 | 0x0D = 0x3D`  
  Checksum: `0xB0 ^ 0x3D = 0x8D` → Quadro Wire: **`02 20 B0 00 3D 8D`**.
* **`Audio` (`AUDIO [A]`, linhas 259–263):** `address: B0 02 20 00`, `command: 30 03 00 00`  
  `cmd_byte = ((0x00 & 0x03) << 6) | ((0x03 & 0x0F) << 4) | (0x30 >> 4) = 0x00 | 0x30 | 0x03 = 0x33`  
  Checksum: `0xB0 ^ 0x33 = 0x83` → Quadro Wire: **`02 20 B0 00 33 83`**.
* **`Display` (`DISPLAY [B]`, linhas 265–269):** `address: B0 02 20 02`, `command: 20 01 00 00`  
  `cmd_byte = ((0x02 & 0x03) << 6) | ((0x01 & 0x0F) << 4) | (0x20 >> 4) = 0x80 | 0x10 | 0x02 = 0x92`  
  Checksum: `0xB0 ^ 0x92 = 0x22` → Quadro Wire: **`02 20 B0 00 92 22`**.
* **`F_rec` (linhas 301–305):** `address: B0 02 20 03`, `command: 70 00 00 00`  
  `cmd_byte = ((0x03 & 0x03) << 6) | ((0x00 & 0x0F) << 4) | (0x70 >> 4) = 0xC0 | 0x00 | 0x07 = 0xC7`  
  Checksum: `0xB0 ^ 0xC7 = 0x77` → Quadro Wire: **`02 20 B0 00 C7 77`**.

---

## 3. Catálogo Integral dos 50 Comandos em `Panasonic_DVD_TV.ir`

### 3.1 Subsistema Televisor Panasonic (`dev = 0x80` — 6 Sinais)

Observe que os botões compartilhados (`Tv_power`, `Tv_chan_up`, `Tv_chan_down`) reutilizam exatamente os mesmos códigos de comando (`0x3D`, `0x34`, `0x35`) do subsistema DVD/DIGA, alterando apenas o byte de dispositivo de `0xB0` para `0x80` (e o byte XOR final correspondente).

| # | Linhas | `Name` | Flipper `Address` | Flipper `Command` | `Cmd` | Hex Wire 48-bit (6 bytes) | Função Prática (TV / Combo) |
| :-: | :---: | :--- | :--- | :--- | :-: | :--- | :--- |
| 2 | 13–17 | `Tv_power` | `80 02 20 00` | `D0 03 00 00` | `0x3D` | `02 20 80 00 3D BD` | Liga / Standby do televisor Panasonic (`TV POWER`) |
| 3 | 19–23 | `Tv_video` | `80 02 20 00` | `50 00 00 00` | `0x05` | `02 20 80 00 05 85` | Alterna entrada de vídeo do televisor (`TV/VIDEO`) |
| 4 | 25–29 | `Tv_chan_up` | `80 02 20 00` | `40 03 00 00` | `0x34` | `02 20 80 00 34 B4` | Canal seguinte no televisor (`TV CH +`) |
| 5 | 31–35 | `Tv_chan_down` | `80 02 20 00` | `50 03 00 00` | `0x35` | `02 20 80 00 35 B5` | Canal anterior no televisor (`TV CH -`) |
| 6 | 37–41 | `Tv_vol_up` | `80 02 20 00` | `00 02 00 00` | `0x20` | `02 20 80 00 20 A0` | Aumenta volume do televisor (`VOL +`) |
| 7 | 43–47 | `Tv_vol_down` | `80 02 20 00` | `10 02 00 00` | `0x21` | `02 20 80 00 21 A1` | Diminui volume do televisor (`VOL -`) |

---

### 3.2 Subsistema DVD / Gravador DIGA HDD (`dev = 0xB0` — 44 Sinais)

Todos os 44 sinais abaixo operam com `dev = 0xB0` e `subdevice = 0x00` (`SET 1` padrão de fábrica do Panasonic DMR-EH55):

| # | Linhas | `Name` | Flipper `Address` | Flipper `Command` | `Cmd` | Hex Wire 48-bit (6 bytes) | Função Prática no DMR-EH55 / Combos |
| :-: | :---: | :--- | :--- | :--- | :-: | :--- | :--- |
| 1 | 7–11 | `Dvd_power` | `B0 02 20 00` | `D0 03 00 00` | `0x3D` | `02 20 B0 00 3D 8D` | Liga / Standby do gravador DVD/HDD (`POWER`) |
| 8 | 49–53 | `1` | `B0 02 20 00` | `00 01 00 00` | `0x10` | `02 20 B0 00 10 A0` | Dígito `1` (sintonizador e códigos `SERV` `00..99`) |
| 9 | 55–59 | `2` | `B0 02 20 00` | `10 01 00 00` | `0x11` | `02 20 B0 00 11 A1` | Dígito `2` (sintonizador e códigos `SERV` `00..99`) |
| 10 | 61–65 | `3` | `B0 02 20 00` | `20 01 00 00` | `0x12` | `02 20 B0 00 12 A2` | Dígito `3` (sintonizador e códigos `SERV` `00..99`) |
| 11 | 67–71 | `4` | `B0 02 20 00` | `30 01 00 00` | `0x13` | `02 20 B0 00 13 A3` | Dígito `4` (sintonizador e códigos `SERV` `00..99`) |
| 12 | 73–77 | `5` | `B0 02 20 00` | `40 01 00 00` | `0x14` | `02 20 B0 00 14 A4` | Dígito `5` (sintonizador e códigos `SERV` `00..99`) |
| 13 | 79–83 | `6` | `B0 02 20 00` | `50 01 00 00` | `0x15` | `02 20 B0 00 15 A5` | Dígito `6` (sintonizador e códigos `SERV` `00..99`) |
| 14 | 85–89 | `7` | `B0 02 20 00` | `60 01 00 00` | `0x16` | `02 20 B0 00 16 A6` | Dígito `7` (sintonizador e códigos `SERV` `00..99`) |
| 15 | 91–95 | `8` | `B0 02 20 00` | `70 01 00 00` | `0x17` | `02 20 B0 00 17 A7` | Dígito `8` (sintonizador e códigos `SERV` `00..99`) |
| 16 | 97–101 | `9` | `B0 02 20 00` | `80 01 00 00` | `0x18` | `02 20 B0 00 18 A8` | Dígito `9` (sintonizador e códigos `SERV` `00..99`) |
| 17 | 103–107 | `0` | `B0 02 20 00` | `90 01 00 00` | `0x19` | `02 20 B0 00 19 A9` | Dígito `0` (sintonizador e códigos `SERV` `00..99`) |
| 18 | 109–113 | `Ch_up` | `B0 02 20 00` | `40 03 00 00` | `0x34` | `02 20 B0 00 34 84` | Canal seguinte / página acima em listas (`CH +`) |
| 19 | 115–119 | `Ch_down` | `B0 02 20 00` | `50 03 00 00` | `0x35` | `02 20 B0 00 35 85` | Canal anterior / página abaixo em listas (`CH -`) |
| 20 | 121–125 | `Add_del_ch` | `B0 02 20 00` | `60 03 00 00` | `0x36` | `02 20 B0 00 36 86` | Adiciona/remove canal no sintonizador (`ADD/DELETE`) |
| 21 | 127–131 | `Delete` | `B0 02 20 01` | `40 00 00 00` | `0x44` | `02 20 B0 00 44 F4` | Apaga título/capítulo/item selecionado (`ERASE` / `DELETE`) |
| 22 | 133–137 | `Cancel_asterisk` | `B0 02 20 02` | `30 00 00 00` | `0x83` | `02 20 B0 00 83 33` | Cancela ação/entrada numérica (`CANCEL` / `*`) |
| 23 | 139–143 | `Input_select` | `B0 02 20 01` | `00 01 00 00` | `0x50` | `02 20 B0 00 50 E0` | Alterna entrada AV (`AV1..AV4` / `DV` / Tuner; útil no bypass TBC) |
| 24 | 145–149 | `Skip_back` | `B0 02 20 01` | `90 00 00 00` | `0x49` | `02 20 B0 00 49 F9` | Salta para capítulo/faixa anterior (`\|<< SKIP`) |
| 25 | 151–155 | `Skip_forward` | `B0 02 20 01` | `A0 00 00 00` | `0x4A` | `02 20 B0 00 4A FA` | Salta para próximo capítulo/faixa (`>>\| SKIP`) |
| 26 | 157–161 | `Slow_back` | `B0 02 20 00` | `40 00 00 00` | `0x04` | `02 20 B0 00 04 B4` | Busca reversa / câmera lenta reversa (`<< SLOW/SEARCH`) |
| 27 | 163–167 | `Slow_forward` | `B0 02 20 00` | `50 00 00 00` | `0x05` | `02 20 B0 00 05 B5` | Busca rápida / câmera lenta avante (`>> SLOW/SEARCH`) |
| 28 | 169–173 | `Stop` | `B0 02 20 00` | `00 00 00 00` | `0x00` | `02 20 B0 00 00 B0` | Interrompe reprodução ou gravação (`STOP`) |
| 29 | 175–179 | `Pause` | `B0 02 20 00` | `60 00 00 00` | `0x06` | `02 20 B0 00 06 B6` | Pausa reprodução ou gravação (`PAUSE`) |
| 30 | 181–185 | `Play` | `B0 02 20 00` | `A0 00 00 00` | `0x0A` | `02 20 B0 00 0A BA` | Inicia reprodução (`PLAY`) |
| 31 | 187–191 | `Time_slip` | `B0 02 20 01` | `B0 00 00 00` | `0x4B` | `02 20 B0 00 4B FB` | Salto temporal durante reprodução/gravação (`TIME SLIP`) |
| 32 | 193–197 | `Cm_skip` | `B0 02 20 01` | `C0 00 00 00` | `0x4C` | `02 20 B0 00 4C FC` | Avanço rápido +60s / pulo de comercial (`CM SKIP`) |
| 33 | 199–203 | `Direct_navigator` | `B0 02 20 02` | `B0 01 00 00` | `0x9B` | `02 20 B0 00 9B 2B` | Abre lista de títulos gravados no HDD/DVD (`DIRECT NAVIGATOR`) |
| 34 | 205–209 | `Schedule` | `B0 02 20 01` | `C0 03 00 00` | `0x7C` | `02 20 B0 00 7C CC` | Abre agenda de gravações temporizadas (`PROG/CHECK`) |
| 35 | 211–215 | `Functions` | `B0 02 20 01` | `70 01 00 00` | `0x57` | `02 20 B0 00 57 E7` | Menu principal `FUNCTIONS` (Setup, Dubbing, Disc Mgmt, `SET 2/3`) |
| 36 | 217–221 | `Up` | `B0 02 20 02` | `50 00 00 00` | `0x85` | `02 20 B0 00 85 35` | Navegação direcional Cima (`UP`) |
| 37 | 223–227 | `Left` | `B0 02 20 02` | `70 00 00 00` | `0x87` | `02 20 B0 00 87 37` | Navegação direcional Esquerda (`LEFT`) |
| 38 | 229–233 | `Right` | `B0 02 20 02` | `80 00 00 00` | `0x88` | `02 20 B0 00 88 38` | Navegação direcional Direita (`RIGHT`) |
| 39 | 235–239 | `Down` | `B0 02 20 02` | `60 00 00 00` | `0x86` | `02 20 B0 00 86 36` | Navegação direcional Baixo (`DOWN`) |
| 40 | 241–245 | `Enter` | `B0 02 20 02` | `20 00 00 00` | `0x82` | `02 20 B0 00 82 32` | Confirma seleção em menus (`ENTER` / `OK`) |
| 41 | 247–251 | `Sub_menu` | `B0 02 20 02` | `00 00 00 00` | `0x80` | `02 20 B0 00 80 30` | Menu contextual / softkey `SUB MENU [S]` |
| 42 | 253–257 | `Return` | `B0 02 20 02` | `10 00 00 00` | `0x81` | `02 20 B0 00 81 31` | Retorna à tela ou menu anterior (`RETURN`) |
| 43 | 259–263 | `Audio` | `B0 02 20 00` | `30 03 00 00` | `0x33` | `02 20 B0 00 33 83` | Seleção de trilha de áudio / softkey `AUDIO [A]` (**não** é `[RED]`) |
| 44 | 265–269 | `Display` | `B0 02 20 02` | `20 01 00 00` | `0x92` | `02 20 B0 00 92 22` | Exibe status OSD na tela / softkey `DISPLAY [B]` |
| 45 | 271–275 | `Create_chapter` | `B0 02 20 01` | `80 01 00 00` | `0x58` | `02 20 B0 00 58 E8` | Cria marcação manual de capítulo (`CREATE CHAPTER`) |
| 46 | 277–281 | `Setup` | `B0 02 20 02` | `40 01 00 00` | `0x94` | `02 20 B0 00 94 24` | Atalho direto ao menu de configuração (`SETUP`) |
| 47 | 283–287 | `Rec` | `B0 02 20 00` | `80 00 00 00` | `0x08` | `02 20 B0 00 08 B8` | Inicia gravação imediata (`REC`) |
| 48 | 289–293 | `Rec_mode` | `B0 02 20 01` | `10 01 00 00` | `0x51` | `02 20 B0 00 51 E1` | Seleciona taxa/qualidade de gravação `XP/SP/LP/EP` (`REC MODE`) |
| 49 | 295–299 | `Status` | `B0 02 20 01` | `F0 01 00 00` | `0x5F` | `02 20 B0 00 5F EF` | Exibe informações estendidas de disco/sinal (`STATUS`) |
| 50 | 301–305 | `F_rec` | `B0 02 20 03` | `70 00 00 00` | `0xC7` | `02 20 B0 00 C7 77` | Gravação flexível ajustada ao espaço livre (`F REC`) |

---

## 4. Nota Comparativa: Controles Norte-Americanos vs. Teletexto `[RED]`/`ND2`, `EUR7659Y70` e Android CIR

A captura `Panasonic_DVD_TV.ir` resolve de forma definitiva uma dúvida recorrente de bancada sobre o mapeamento de teclas entre diferentes regiões de controles Panasonic DIGA:

1. **Controles Combo Norte-Americanos (`Panasonic_DVD_TV.ir`) vs. Fileira Teletexto EU/JDM (`[RED]` / `ND2`):**
   * Nos controles norte-americanos, a fileira inferior de **4 botões coloridos de Teletexto** presente nos modelos europeus/japoneses (`EUR7659Y60` / `EUR7659YC0`) foi suprimida de fábrica e substituída pelos atalhos de guia/menu marcados como **`AUDIO [A]`**, **`DISPLAY [B]`** e **`SUB MENU [S]`**.
   * Conforme comprovado diretamente nos sinais #43 (`Audio`), #44 (`Display`) e #41 (`Sub_menu`) de `Panasonic_DVD_TV.ir`:
     * **`AUDIO [A]`** transmite **`cmd = 0x33`** (`sub = 0x00`, wire `02 20 B0 00 33 83`).
     * **`DISPLAY [B]`** transmite **`cmd = 0x92`** (`sub = 0x00`, wire `02 20 B0 00 92 22`).
     * **`SUB MENU [S]`** transmite **`cmd = 0x80`** (`sub = 0x00`, wire `02 20 B0 00 80 30`).
   * **Nenhum desses botões equivale aos códigos de Teletexto ou serviço estendido**, que operam com **`subdevice = 0x01`** (ausente em todos os 50 registros de `Panasonic_DVD_TV.ir`):
     * **Teletexto `[RED]`:** código **`0x141`** (`sub = 0x01`, `cmd = 0x41`, wire `02 20 B0 01 41 F0`; `[BLUE]` = `0x140`, `[GREEN]` = `0x142`, `[YELLOW]` = `0x143`).
     * **`ND2` (`[5]+[9]+[RED]` Service Non-Disclosure Mode 2):** código **`0x182`** (`sub = 0x01`, `cmd = 0x82`, wire `02 20 B0 01 82 33`).
2. **Relação com o Controle Físico `EUR7659Y70` (DMR-EH55):**
   * O controle original `EUR7659Y70` (região América/Brasil) tampouco possui `[RED]` (`0x141`) ou `ND2` (`0x182`), compartilhando os mesmos códigos `sub = 0x00` de `Panasonic_DVD_TV.ir` para dígitos `0..9` (`0x10..0x19`), energia (`0x3D`), navegação, `FUNCTIONS` (`0x57`), `DIRECT NAVIGATOR` (`0x9B`), `AUDIO` (`0x33`) e `DISPLAY` (`0x92`). Além disso, traz teclas físicas dedicadas ausentes nesta captura combo (como `OPEN/CLOSE` = `0x01` e `DRIVE SELECT` `DVD/HDD`).
   * No **Panasonic DMR-EH55** (Manual de Serviço `DSD0603003CE`), a ausência de `[RED]` **não** impede o diagnóstico: o modo de serviço **`SERV`** é ativado diretamente no painel frontal em Standby (**`[CH UP] + [OPEN/CLOSE] + [REC]` por 5 s**), e todos os testes de serviço (`00`..`99`) utilizam apenas as teclas `0..9` (`0x10..0x19`).
3. **Emissores Programáveis (Android CIR / `IrCommanderView` & Pico `IRTX`):**
   * Quando há necessidade de emitir códigos `sub = 0x01` (`[RED]` `0x141`, `ND2` `0x182`), alternar conjuntos de endereço `SET 1/2/3` (`sub = 0x00 / 0x10 / 0x20`) ou executar macros declarativas verificadas, deve-se recorrer ao módulo unificado **DIGA IR Commander** do APK Android ([`IrCommanderView.tsx`](../../apps/desktop/src/views/IrCommanderView.tsx) + SSOT em [`crates/panadiga-core/src/ir/`](../../crates/panadiga-core/src/ir/mod.rs), via `/sys/class/sec/sec_ir/ir_send` ou `ConsumerIrManager`) ou ao sintetizador PWM 38 kHz **`IRTX`** do firmware Pico ([`p7501_ir_codec.c`](../../firmware/pico-p7501/diag/p7501_ir_codec.c), limitado aos GPIOs livres seguros `GP6..9` / `GP16..22`).

---

## 5. Macros de Serviço DIGA Verificadas Declarativamente (SSOT)

Todas as sequências automatizáveis de serviço do **Panasonic DMR-EH55** foram consolidadas de forma declarativa em [`ir_macros_ssot.json`](../../crates/panadiga-core/src/ir/ir_macros_ssot.json) e expostas via [`verified_service_macros()`](../../crates/panadiga-core/src/ir/macros.rs) (`panadiga-core::ir::macros`), após validação cruzada entre a captura `Panasonic_DVD_TV.ir`, os manuais de serviço **EV/EC** ([`panasonic_dmr_eh55ec_service_manual_vol1_diagnostics.pdf`](../../manuals_and_schematics/dmr-eh55/service/panasonic_dmr_eh55ec_service_manual_vol1_diagnostics.pdf) pp. 22–30), o manual **NA** ([`panasonic_dmr_eh55p9_eh55pc9_service_manual_north_america.pdf`](../../manuals_and_schematics/dmr-eh55/service/panasonic_dmr_eh55p9_eh55pc9_service_manual_north_america.pdf) pp. 21–29) e os *handlers* `SYS_SERV` desmontados de `main3060np`.

> [!IMPORTANT]
> **Pré-requisito de Entrada no Modo de Serviço (`SERV`):**
> A execução das macros `[x][y]` abaixo exige que o gravador esteja em **Service Mode (`SERV`)**, acionado no painel frontal com o aparelho **desligado (Standby)** pressionando simultaneamente **`[CH UP] + [OPEN/CLOSE] + [REC]` por ~5 s**. Caso o aparelho trave com `U99` antes de inicializar o `MAIN` / `SYS_SERV`, a interface IR não responde ao modo `SERV` e o diagnóstico deve ocorrer via `P7501`.

### 5.1 Catálogo Auditado das 14 Macros Verificadas

| Nível | ID na SSOT (`ir_macros_ssot.json`) | Sequência IR | Função & Comportamento Verificado |
| :--- | :--- | :--- | :--- |
| **SAFE** | `eh55_service_release` | `[0][0]` | **`Release Items`** — sai do item de diagnóstico atual sem alterar NVRAM/contadores. |
| **SAFE** | `eh55_error_history_latest` | `[0][1]` | Exibe o **código de erro mais recente** armazenado no histórico de serviço. |
| **SAFE** | `eh55_rom_version_region` | `[0][2]` | Exibe **versão da ROM / firmware e código de região** no VFD/OSD. |
| **SAFE** | `eh55_av_pattern_white` | `[1][1]` | Gera padrão interno de teste A/V **White** (tela branca 100%). |
| **SAFE** | `eh55_av_pattern_magenta` | `[1][2]` | Gera padrão interno de teste A/V **Magenta**. |
| **SAFE** | `eh55_laser_hours` | `[4][1]` | Exibe o horímetro acumulado do leitor/gravador óptico (**Laser Hours**). |
| **SAFE** | `eh55_ram_drive_last_error` | `[4][2]` | Exibe o último erro do drive óptico (`Drive Last Error`); navegue pelos 19 slots com `[0][1]..[1][9]`. |
| **SAFE** | `eh55_vfd_all_segments` | `[5][1]` | Acende **todos os segmentos do VFD e LEDs** frontais para inspeção visual. |
| **SAFE** | `eh55_error_history_browse` | `[6][5]` → `[0][1]`..`[1][9]` | Entra no navegador de **19 slots de histórico de erros** (`[6][5]` abre `[0][1]`, avançando até `[1][9]`). |
| **MODERATE** | `eh55_hdd_read_inspection` | `[3][1]` | Executa **inspeção de leitura do HDD** (*HDD READ inspection*); para abortar/sair, segure **`[POWER] > 10s`**. |
| **MODERATE** | `eh55_tray_endurance_test` | `[9][1]` | Inicia loop contínuo de abertura/fechamento da bandeja (**Tray OPEN/CLOSE loop**); **`STOP` NÃO encerra** — saia segurando **`[POWER] > 10s`**. |
| **MODERATE** | `eh55_clear_last_error_f00` | `[9][8]` | Executa limpeza pontual do erro **`F00`** / último código registrado (`Clear Last Error F00`). |
| **DESTRUCTIVE** | `eh55_clear_laser_hours` | `[9][5]` | **Destrutivo:** zera o contador permanente de horas do laser (**Clear Laser Hours**). |
| **DESTRUCTIVE** | `eh55_service_initialize_clr` | `[9][9]` | **Destrutivo (`CLR SERV`):** limpa *Last Drive Error*, histórico de 19 slots e códigos de erro; **NÃO** apaga o HDD nem zera *Laser Hours*. |

### 5.2 Mitos de Comunidade Rejeitados & Correções Auditadas

Os testes unitários em [`crates/panadiga-core/src/ir/tests.rs`](../../crates/panadiga-core/src/ir/tests.rs) bloqueiam explicitamente IDs legados/não verificados (`multi_region`, `child_lock`, `video_io_test_80`, `auto_restore_set2`) e corrigem confusões recorrentes da comunidade:

1. **`[0][0]` vs. `[5][1]` (Teste de VFD):** `[0][0]` executa **`Release Items`** (saída de item de serviço), **NÃO** o teste do visor VFD; a sequência verificada para acender todos os segmentos VFD/LEDs é **`[5][1]`**.
2. **`[8][0]` / `[8][1]` (SCART EU) vs. `[1][1]` / `[1][2]` (Padrões Internos):** `[8][0]` e `[8][1]` correspondem a testes de *passthrough* de E/S **SCART europeu**, ausentes em modelos norte-americanos (**NA**). Os padrões internos de vídeo disponíveis em serviço são **`[1][1]`** (*White*) e **`[1][2]`** (*Magenta*).
3. **Desbloqueio de Child Lock:** A abertura do *Child Lock* exige segurar simultaneamente **`[ENTER] + [RETURN]`**, e **não** enviar 10 pulsos sequenciais de `ENTER`.
4. **Macro `SET 2/3 -> SET 1` & Navegação do Menu `Setup`:** O menu `Setup` possui **7 abas** de configuração e a seleção do painel direito exige **`[RIGHT]`**; macros cegas baseadas em contagem incorreta de profundidade (`auto_restore_set2`) foram removidas da SSOT.
5. **Macro Não Verificada `MULTI_REGION` (`OBC 0x04 = REWIND`):** A sequência divulgada online (`1..0 -> AUDIO -> MEDIA SERVER`) apoia-se em `OBC 0x04`, que na tabela real `curated_diga_eh55_eur7659` e em `Panasonic_DVD_TV.ir` mapeia para **`REWIND`** (`MediaServer = B0 00 04`, idêntico a `Rewind = B0 00 04`). Por falta de comprovação determinística em firmware, `multi_region` foi excluída da SSOT verificada.
6. **Divergências Comprovadas em `Panasonic_DVD_TV.ir` & `curated_diga_eh55_eur7659`:**
   * **`AUDIO [A]`** (`0x33` / `B0 00 33`) **≠ `[RED]`** (`0x141`, `sub = 0x01` / `B0 01 41`) — usar `[RED]` no lugar de `AUDIO` quebra comandos de áudio/macros.
   * Códigos confirmados na matriz principal (`sub = 0x00`, exceto onde indicado): **`FUNCTIONS = 0x57`** (`B0 00 57`), **`STATUS = 0x5F`** (`B0 00 5F`), **`SCHEDULE = 0x7C`** (`B0 00 7C`), **`GUIDE = 0x60`** (`B0 00 60`) e **`EXIT = 0x51 (sub = 0x01)`** (`B0 01 51`).

---

## 6. Documentos Relacionados

* [`firmware_and_images/README.md`](../../firmware_and_images/README.md) — Catálogo de artefatos binários e capturas de bancada (Seção #6).
* [`COMPENDIO_GLOBAL_COMUNIDADE_FIRMWARE_E_HARDWARE_DIGA.md`](COMPENDIO_GLOBAL_COMUNIDADE_FIRMWARE_E_HARDWARE_DIGA.md) — Seção `4.9` (Mapa prático `EUR7659Y70`, `AUDIO [A]` vs. `[RED]`, Android CIR e `IRTX`).
* [`ARQUITETURA_MOBILE_ANDROID_E_BLUETOOTH.md`](ARQUITETURA_MOBILE_ANDROID_E_BLUETOOTH.md) — Pipeline Android APK (`IrCommanderView`, shims Kotlin e `panadiga-core::ir::adapter`).
* [`TEORIA_E_PRATICA_UNBRICK_U99_E_CRIPTOGRAFIA_MATSUSHITA.md`](TEORIA_E_PRATICA_UNBRICK_U99_E_CRIPTOGRAFIA_MATSUSHITA.md) — Diagnóstico `U99`, mapa Spansion NOR e rotina de recuperação.
