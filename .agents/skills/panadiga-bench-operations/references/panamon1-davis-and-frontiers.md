# Protocolo `PANAMON1>`, `DAVIS CH0/CH1`, `P58001` & Fronteiras Pós-`2026-11-22`

## 1. Porta #1 (No-JIG) & Protocolo Seguro `PANAMON1>`
- **Porta #1 (`EV-20261119-004`):** Quando `HDD_MAX_LBA` falha em `normal mode` (`NOJIG`), o `Loader ver0.61` monta o cartão SDSC FAT16 (`0x3c4000[LBA]`, 4-bit `0x89001850`), carrega o slot `PROG` (`0x0B1800 -> 0x90400000`) de `PANA__SD.FRM` (`sd48`, verificado **apenas por soma 32-bit** `func_0x4400c172`, sem RSA!) e sobe `PANAMON1>` (`0x50439494`) ou `Fvu` retargetado.
- **Regras de Envio em `PANAMON1>`:**
  - Faça pre-probe com **`?\r`** antes de iniciar automações ou GDB.
  - **NUNCA** envie frames GDB brutos como `$m...#B1` para `PANAMON1>` — o caractere `'B'` (`0x42`) dispara o comando `'B'` (`OK:BYPASS`)!
  - Espace transmissões em **`300–350 µs/byte`** drenando o eco em seguida (o registrador `SC0RXB` tem apenas 1 byte).
  - Comandos suportados: `B`, `R`, `W`, `D`, `F` (bloqueado fail-closed!), `J`, `E`, `U CONFIRM_UPPER_NOR` / `U LOCK`, `Q`.

## 2. Controlador `DAVIS` & Pinagem Invertida `P58001`
- **Polaridade Assimétrica (`EV-20261119-005`):** Após `J 504156AD` (`rtsd_hw_init`):
  - **`CH0` (ODD):** reset ativo-BAIXO (`0x504F81D0 = 2`), liberado por `0x89000420 = 0x02/0x03`, responde `STATUS=0x50` / `0xEB14` ATAPI.
  - **`CH1` (HDD):** reset ativo-ALTO (`0x504F81D4 = 1`), liberado por `0x89000C20 = 0x00` (inversor externo em `RESET-`).
- **Registradores Proibidos:** **`0xC000_100C` PROIBIDO** (trava barramento/WDT); evitar também `0xDC0F0000` (travou em sondas iniciais).
- **Lei Física `P58001` (`vol2_schematics.pdf` p. 113):** `Pin_P58001(k) = ATA(41 - k)`. Combinar FFC **`Type B`** + adaptador `N5EZZ0000003` girado `180°` restaura o mapeamento 1:1 sem torcer o cabo, exigindo jumper **`MASTER`** na `StarTech IDE2SAT2`.
- **Checks de Alimentação `CH1` (`P1503`):** Medir `Pin 4 DR5V = +5.1V` e `Pin 1 DR12V = +12.3V` antes de culpar o firmware.

## 3. Diagnóstico Pós-`2026-11-22` (`hdmi_flat` Breakthrough & Mistério Aberto)
| Fato Observado em `2026-11-22` (`docs/evidence/*hdmi_flat*`) | Status | Detalhe Técnico |
| :--- | :---: | :--- |
| Liberação FE (`C0001008: 0x007F -> 0x807F`) + `CH1 raw` | **OK** | `status=0x50`, `altstat=0x50`, `devsel=0xA0`, scratchpad `0x55`/`0xAA` |
| **`0xEC IDENTIFY DEVICE` ao vivo (`ch1_identify_ec_hdmi_flat_20261122.bin`)** | **OK** | `SAMSUNG HM251JI` (`488.397.168` LBA48, `MWDMA2`, `mirror_main_lba=488266096`) |
| **`FUN_5041CD4D(1)` (`DAVIS_Connect_IDE(1)`)** | **FALHA** | Retorna `rc=0x00000000`, `is_ata_m=0` mesmo com `0xEC` funcionando logo antes! |
| **Boot frio `RTSC` (`boottrace_*_20261122.log`)** | **FALHA** | `0` `U99`, valida `MAIN3060NP`/`3410PQ`, falha com `[RTSD] 80000404` / `E8000000` `Reg[00..00]` |

## 4. As 6 Fronteiras Abertas do Projeto (Daqui pra Frente)
1. Isolar a divergência entre `0xEC` vivo funcional vs `FUN_5041CD4D(1)` (`is_ata_m=0`) / `RTSC` frio (`80000404`) — desbloqueando o primeiro boot completo de `MAIN` por HDD em silício real.
2. Validar em hardware real `PAL-M` (`AIOF`/`IC3001`) + `Stage15DigadiagTbc` (`M0` keepalive `TBUS`) após `CH1`.
3. Implementar parser completo de Upper `EEMUL` (`SA71..SA133`, tags `"1284"`).
4. Extrair chave `PANAEUSB.FRM` do `DMR-EX97` (`libfmupre.so` / ARMv7).
5. Estender `profiles/schema/` v2.1 para SLC NAND / eMMC (`XW300`, `BS/XS`, `BW/BWT`).
6. Reproduzir todo o fluxo end-to-end em uma 2ª unidade física (`N5_REPRODUCED`) e obter esquemáticos Vol. 2 faltantes.
