# Invariantes de Segurança de Hardware e Bancada (Panadiga)

## 1. Isolamento Galvânico SMPS, Verificação Multi-Modelo & `P7501` (`VEP79138`)
- **Isolamento SMPS Obrigatório:** Capacitores Y de fontes chaveadas acoplam `~60–110 VAC` no chassi em circuito aberto. Alimente a placa via **transformador isolador AC/AC** ou use **isolador USB galvânico (`ADuM3160` / `ADuM4160`)**; descarregue o primário (`~170–340 VDC`) antes de tocar.
- **Perigo Cross-Model (`P7501`):** Nunca assuma que `P7501` é UART sem conferir o esquemático — em **`DMR-EH58 / EH59 / EH68` (`VEP79183`)**, `P7501` é header HDMI de 8 pinos com **`+5.8V_AV`** (`p7501_header_present: false` na SSOT).
- **Tensão Limite:** `3.3V LVTTL` nos pinos lógicos do SoC `AM34` / `MN103S` (medir sempre com multímetro em modelos novos).
- **Alimentação da Sonda:** **NUNCA alimente o Raspberry Pi Pico pelo pino 1 (`SW_UART`, `< 10 mA`)**. Use USB isolado ou `VSYS`.
- **Bloqueio Letal DAVIS:** **JAMAIS leia ou escreva o registrador AM34 `0xC000_100C`**.
- **Pinagem Verificada `DMR-EH55 P7501` (`B6B-ZR-3R4` 6 pinos — UART 115200 8N1):**
  - Pino 1: `SW_UART` (`GP26/ADC0`) — `LOW` força `serial boot` (`XFWE`, mas **corta `Q7514`** bloqueando TX); `Hi-Z` habilita TX UART normal e medição VDD.
  - Pino 2: `XRESET` (`GP5`) — Reset em dreno aberto (**Open-Drain apenas**, nunca Push-Pull).
  - Pino 3: `UARTM2P_T` (`GP4`) — TX Pico -> RX AM34 via transistor `Q7514` (`R7570 10kΩ` pull-up; requer `SW_UART=Hi-Z` para conduzir).
  - Pino 4: `GND` (`Pin 38`) — Referência comum `0.0Ω` obrigatória.
  - Pino 5: `NSW5V` (`GP3`) — **SEMPRE `Hi-Z Input` sem pulls (NUNCA dirigir push-pull!)** — originado em `IC1506` (`~5.15V` upstream via `D7510`, ~`3.29V` estático em placa).
  - Pino 6: `UARTP2M_T` (`GP2`) — RX Pico <- TX AM34 `115200 8N1` (`R7572 10kΩ`).

## 2. Política de 3 Zonas — Flash Spansion `S29JL064H` (`IC5001` TSOP-48)
- **Capacidade:** `8 MiB` (`64 Mbit`, `8.388.608 bytes`, `142` setores `SA0..SA141`: `8×8K + 126×64K + 8×8K`, base `0x80000000..0x807FFFFF` cached / `0x84000000..0x847FFFFF` uncached).
- **Zona 1 — IPL Crítico (`SA0..SA10`, `256 KiB`, `0x80000000..0x8003FFFF`):** **LEITURA ESTRITA PERMANENTE (`E05`)** na sonda Pico (`CapabilityGate::C5` sempre negada).
- **Zona 2 — Upper NOR App & BGO FTL (`SA11..SA133`, `0x80040000..0x807EFFFF`):** Protegida por padrão (`E05`); inclui os **45 setores BGO (`SA26..SA70`, `0x84130000..0x843FFFFF`)**. Desbloqueável offline via CoW (`nvram patch-upper-nor` / `unbrick bgo-repair-plan`) ou live em modelo `T4` via `--backup-dir` + `--confirm-upper-nor` (`U CONFIRM_UPPER_NOR` / `U LOCK`).
- **Zona 3 — NVRAM / EEMUL (`SA134..SA141`, últimos `64 KiB`, `0x807F0000..0x807FFFFF`):** Escrita autorizada para reset cirúrgico do erro `U99` preservando os primeiros `8.323.072` bytes.
