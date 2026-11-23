# Física Elétrica `VEP79138`, Perigos Multi-Modelo, Standby `IC7501` & IR `SERV`

## 1. Perigos Cross-Model em Conectores de Bancada (Medir Sempre!)

| Modelo / Chassi | Conector / Protocolo Real | Risco Crítico & Regra |
| :--- | :--- | :--- |
| **`DMR-EH55` (`VEP79138`)** | **`P7501` (`B6B-ZR-3R4`, 6 pinos) UART `115200 8N1` 3.3V** | `GP3` (`NSW5V`) SEMPRE `Hi-Z` (`~5.15V` upstream); `GP5` Open-Drain |
| **`DMR-EH58 / EH59 / EH68` (`VEP79183`)** | **`P7501` é header HDMI 8 pinos (`+5.8V_AV`) — NÃO UART!** | **PERIGO:** Pluggar chicote `P7501` do EH55 pode injetar `+5.8V` direto no Pico (`p7501_header_present: false`) |
| **`DMR-EX98V` (`RFKB71134A`)** | UART dedicada **`P9705`** | Confirmar pinout no esquemático antes de ligar |
| **`DMR-ES10` (`VEP79104A`) / `EH80V`** | Sem conector `P7501` | Captura direta TSOP-48 ou pontos de teste do manual |
| **UniPhier Linux (`BW/BWT/XW/BS`)** | **`K1KA06AA0083` 6 pinos (`SLM` `115200 8N1`)** | Pino 1 `XDIAG` aterrado ~1s ou sequência `0x0C, 0x0F, 0x07` -> senha **`pm6030p`** (`media/media`, `vup/pcinosanap`) |

## 2. Pinagem Verificada `DMR-EH55 P7501` (`VEP79138`) & Física Discreta
- **Pin 1 (`SW_UART/IR_BUSY`, `GP26/ADC0`):** Coletor NPN `Q7510` (`2SD0601A0L`, base `D3R3V`, emissor=`XFWE` via `R7559 10kΩ`) + base PNP `QR7503`. Em `Hi-Z`: `~3.25V`; com pull-down interno do Pico, junção B-C de `Q7510` grampeia em **`2.68–2.69V`**. **NUNCA alimentar Pico aqui (`<10 mA`).**
- **Pin 2 (`XRESET`, `GP5`):** **Open-Drain apenas** (`0V` / `Hi-Z`, subida `~20 µs` via `QR7503` / `IC7502`). Não atravessa `P59001` nas placas `EC`.
- **Pin 3 (`UARTM2P_T`, `GP4` TX Pico):** Entra no AM34 via **`Q7514`** (`R7528 4.7kΩ` ligado a `XFWE`, pull-up `R7570 10kΩ`). **Se `SW_UART=LOW`, `Q7514` entra em corte e bloqueia `100%` do TX para o AM34!**
- **Pin 4 (`GND`):** Referência comum `0.0Ω` obrigatória (atrás de transformador isolador AC ou `ADuM3160`/`ADuM4160`).
- **Pin 5 (`NSW5V`, `GP3`):** De `IC1506` (`SI-3011ZFE ~5.15V`) via `D7510` (`~3.29V` idle). **SEMPRE `Hi-Z Input` sem pulls (nunca push-pull).**
- **Pin 6 (`UARTP2M_T`, `GP2` RX Pico, pad quadrado):** Saída TX do AM34 `115200 8N1` (`R7572 10kΩ`).

## 3. Física do Standby (`IC7501`), `TW7501` & VFD (`PLEASE WAIT` vs `HELLO`)
- **`TW7501` (`VWJ0795=5`) ≠ `P7501` Pino 1:** `TW7501` está no trilho Always-On de backup `IC7501` (`C7584 0.047F`, `R7623 180Ω`, `IC7502`). Em Standby real (`P-OFF` / `0:00`), `DI_P_ON=0` derruba todos os pinos de `P7501` (`VDD = 0.03V`).
- **Sequência para travar Standby estável (encerra auto-religamento de 2s pós-`TW7501`):** `1x [POWER]` -> pulso curto em `TW7501` com placa energizada -> segurar `[POWER]` (`KEY3_POWER` `IC7501 Pin 88`).
- **Interpretação de VFD:** `PLEASE WAIT (~2s) -> U99` (ou `U99` direto) ocorre no boot de tomada / auto-retry; `HELLO (~2s) -> U99` ocorre ao acordar de Standby estável. Ambos abortam **antes** do `BOOT.BIN` estender o watchdog `TBUS` para `255s` (`FUN_44005cd0()`).

## 4. Modo de Serviço `SERV` (`DSD0603003CE`) & IR Kaseikyo
- **Combos Frontais (Standby):** **`[CH UP] + [OPEN/CLOSE] + [REC]` (5s)** = `SERV`; `[STOP] + [CH UP] + [OPEN/CLOSE]` (5s) = `TM AV1` (`TEST Mode` — reinicializa preferências, **não** é `SERV`); `[STOP] + [CH UP]` (5s) = ejeção forçada; `[POWER] > 10s` = desligamento forçado.
- **Códigos `SERV` (`EUR7659Y70` `00..99`):** `00` libera item; `01` último erro `IC7501` (`F00`); `02` versões Region/Main/Timer/Drive/ROM; `31` `HDD READ`; `41`/`95` tempo laser/reset; `42`/`96` erros ODD; `65`/`97` histórico 19 erros; `74` SD write check (**apaga imagens no SD!**); `98`/`99` `CLR SERV`.
- **Kaseikyo (`0x02 0x20 0xB0`):** `POWER=0x03D`, `AUDIO [A]=0x033` (não é `[RED]`), `DISPLAY [B]=0x092`, `0..9=0x010..0x019`, `MEDIA SERVER=0x004`, `[RED]=0x141`, `ND2=0x182` (JDM/BD `[5]+[9]+[RED]` não existe no EH55). Use controle `EUR7659Y70`, Android CIR (`--mode android`) ou Pico `IRTX` (LED IR 940 nm + `330 Ω` em GPIO livre `GP6..GP22`).
