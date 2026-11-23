---
name: panadiga-bench-operations
description: >-
  Gestão operacional e segurança em bancada de hardware ao vivo: isolamento galvânico SMPS
  (capacitores Y 60–110 VAC e ADuM3160), perigo cross-model em P7501 (EH58/EH59 é +5.8V_AV),
  física VEP79138 (Q7510/Q7514/NSW5V/TW7501), estabilização de Standby IC7501, Service Mode
  SERV (DSD0603003CE) e IR Kaseikyo, protocolo PANAMON1> (Porta #1, ?\r, 350 µs), DAVIS
  CH0/CH1, P58001 e roteiro pós-2026-11-22. Use antes de conectar sondas ou operar hardware real.
---

# Operações de Bancada, Segurança Elétrica, Standby & `DAVIS`

Esta skill concentra tudo o que foi aprendido na bancada física do `DMR-EH55` (`VEP79138`) e outros modelos DIGA para operar hardware real sem queimar portas USB, SoCs ou placas digitais.

## 1. Quando Usar & Handoffs
- Ative sempre que for ligar cabos, calibrar alimentação, estabilizar Standby, entrar em `SERV`, operar `PANAMON1>` ou depurar `DAVIS CH0/CH1` ao vivo.
- Para automação repetível, headless e fluxos E2E de teste/dump/unbrick, combine com `.agents/skills/panadiga-bench-automation-and-headless/SKILL.md`.
- Para gravação do Pico UF2 -> `.agents/skills/panadiga-pico-flashing/SKILL.md`; para cura BGO/NVRAM -> `.agents/skills/panadiga-nvram-unbrick/SKILL.md`.
- Física elétrica `VEP79138`, pinagens multi-modelo, `TW7501`, `SERV` e Kaseikyo em [bench-electrical-and-standby.md](./references/bench-electrical-and-standby.md); `PANAMON1>`, `DAVIS`, `P58001` e fronteiras `2026-11-22` em [panamon1-davis-and-frontiers.md](./references/panamon1-davis-and-frontiers.md).

## 2. Invariantes Elétricas Obrigatórias (Pré-Energização)
1. **Isolamento Galvânico SMPS:** Capacitores Y da fonte chaveada acoplam **`~60–110 VAC`** no chassi em circuito aberto. Use **transformador isolador AC/AC** ou **isolador USB galvânico (`ADuM3160` / `ADuM4160`)** entre o PC e o Pico.
2. **Descarga & Medição Prévia:** Com cabo AC fora da tomada, confirme descarga do primário (`~170–340 VDC`) e meça com multímetro a tensão lógica do conector alvo (`3.3V`/`1.8V`/`5V`) antes de ligar a sonda.
3. **Perigo Cross-Model `P7501`:** Em **`DMR-EH58 / EH59 / EH68` (`VEP79183`)**, `P7501` virou header HDMI de 8 pinos com **`+5.8V_AV`** (`p7501_header_present: false` na SSOT)! Em `DMR-EX98V`, a UART é **`P9705`**; `ES10`/`EH80V` não possuem `P7501`.
4. **Pinos `P7501` (`DMR-EH55`) & Bloqueio Letal:** `GP3` (`NSW5V`) sempre `Hi-Z Input` sem pulls; `GP5` (`XRESET`) sempre Open-Drain; nunca alimentar Pico pelo Pino 1 (`SW_UART < 10 mA`); **jamais ler/escrever `0xC000_100C`**.

## 3. Workflow Canônico em Bancada (CLI & Python)
```bash
# 1. Conferir catálogo elétrico e status passivo sem acionar reset ativo
cargo run -q -p panadiga-cli -- pico status -p COM10
python -m scripts.bench.runner health --port COM10

# 2. Captura passiva / diagnóstico de wake ou bateria guiada em bancada
cargo run -q -p panadiga-cli -- pico bench-wizard -p COM10 --profile diagwatch --window-ms 12000

# 3. Captura ao vivo da Spansion IC5001 via PANAMON1> (ATENÇÃO: flag --slice)
cargo run -q -p panadiga-cli -- pico capture-nor -p COM10 --slice full -o tmp/eh55_nor_live.bin -j -

# 4. Diagnóstico seguro DAVIS CH0/CH1 & 0xEC IDENTIFY DEVICE (bloqueia 0xC000_100C)
cargo run -q -p panadiga-cli -- pico davis-probe -p COM10 --drive master --out-ec tmp/ch1_ec.bin -j -

# 5. Traduzir códigos de erro DRV/VFD/SENSE (ex.: 80040904, 80000404, 0x83A7)
cargo run -q -p panadiga-cli -- pico error-lookup 80000404 --json
```
- **MCP:** `panadiga_safety_check`, `panadiga_pico_catalog`, `panadiga_davis_probe`.

## 4. Armadilhas Descobertas na Bancada Real
- **`SW_UART = LOW` Corta TX (`Q7514`):** Manter `SW_UART` baixo coloca `Q7514` em corte via `R7528`; só envie comandos UART ao alvo após liberar `SW_UART` para `Hi-Z`.
- **Standby Travado (`P-OFF` `VDD=0.03V`):** `XRESET` não cruza `P59001` nas placas `EC`. Para sair do auto-religamento de 2s pós-`TW7501`: `1x [POWER]` -> curto momentâneo em `TW7501` energizado -> segurar `[POWER]`.
- **`SERV` no EH55 (`DSD0603003CE`):** Aciona-se em Standby segurando **`[CH UP] + [OPEN/CLOSE] + [REC]` por 5s** no painel frontal e digitando `00..99` no controle `EUR7659Y70` (não usa `[5]+[9]+[RED]` de modelos JDM).

## 5. Daqui pra Frente (Pós-`2026-11-22`) & Verificação
- **Prioridade #1:** Em `2026-11-22` (`hdmi_flat`), o canal `CH1` respondeu `status=0x50` e `0xEC IDENTIFY` real no `SAMSUNG HM251JI` (`488.397.168` LBA48), mas `FUN_5041CD4D(1)` retornou `is_ata_m=0` e o boot frio `RTSC` ainda reportou `80000404` — o próximo passo é isolar essa diferença de estado/timing antes de promover `MAIN` para `N4_HARDWARE`.
- **Verificação:** Toda sessão ao vivo deve encerrar com `ISOLATE` no Pico, log salvo e SHA-256 checado antes de desconectar.
