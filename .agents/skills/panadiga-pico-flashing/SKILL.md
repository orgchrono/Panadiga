---
name: panadiga-pico-flashing
description: >-
  Desenvolvimento modular em C11 (firmware/picofw/ em 9 pastas), arquitetura Zero-Gadget
  (256 KiB P0_FW_RO, PIO 8x, Pico W/2W BLE NUS), compilação dual-target RP2040/RP2350 com
  auto-fatiamento UF2 (INFO_UF2.TXT), gravação via BOOTSEL ou pico flash, testes host C11
  (PICO_HOST_TEST=1 + DigaOS host test) e operação da sonda PicoFW (PICO> / @PICO_*).
  Use ao modificar o firmware C11 ou gravar o Pico.
---

# Firmware C11 `PicoFW` & Operação da Sonda (`RP2040` / `RP2350` `P7501`)

Esta skill governa o desenvolvimento em C11 (`firmware/picofw/`, prompt `PICO>`), a compilação e merge de `picofw_unified.uf2` dual-target e a operação/seleção de portas da sonda `P7501` (`115200 8N1`).

## 1. Quando Usar & Handoffs
- Ative ao editar módulos C11/PIO da sonda, rodar testes host C11, compilar `picofw_unified.uf2` ou gravar o Raspberry Pi Pico / Pico 2 / Pico W.
- Para protocolos de bancada ao vivo, isolamento SMPS/ADuM, física `Q7510`/`Q7514` e `DAVIS`, use `.agents/skills/panadiga-bench-operations/SKILL.md`.
- Para automação em bancada e modo `--mock`, use `.agents/skills/panadiga-bench-automation-and-headless/SKILL.md`.
- Mapa das 9 pastas C11, 3 PIOs, comandos da sonda e testes host em [pico-c11-module-architecture.md](./references/pico-c11-module-architecture.md).

## 2. Invariantes Elétricas & de Arquitetura Pico
1. **Zero-Gadget na Sonda (`P0_FW_RO = 256 KiB`):** Nenhum display, buzzer ou botão extra conectado ao Pico; smartphone (USB-OTG/BLE NUS) e desktop atuam como interface.
2. **Pinos Reservados `P7501` (`GP0..GP5` + `GP26`):** `GP3` (`NSW5V`) sempre `Hi-Z Input` sem pulls; `GP5` (`XRESET`) sempre Open-Drain; `GP26` (`SW_UART`) retorna a `adc_gpio_init` após `ISOLATE`; nunca alimentar Pico por `P7501`; GPIO de IR livre apenas em `GP6..GP28` fora da faixa reservada (`RESERVED_P7501_MAX_GPIO = 5`).
3. **SSOT Gerada:** Não edite `firmware/picofw/generated/picofw_ssot*` à mão; regenere com `panadiga ssot export-all`.
4. **C11 Estrito (`-Wall -Wextra -Werror` / `/W4 /WX`):** Todo módulo novo deve compilar limpo tanto para host (`-DPICO_HOST_TEST=1`) quanto para ARM (`RP2040`/`RP2350`).

## 3. Compilação Dual-Target, Merge & Gravação UF2
```bash
# Compilar RP2040 + RP2350 + UF2 unificado (requer arm-none-eabi-gcc, cmake, ninja, PICO_SDK_PATH >= 2.1.1)
python scripts/build.py --pico
# ou: npm run build:pico | .\build.cmd --pico | ./build.sh --pico

# Merge manual de UF2 universal (compatível com errata RP2350-E10 via INFO_UF2.TXT):
cargo run -q -p panadiga-cli -- pico merge-uf2 --rp2040 <RP2040.uf2> --rp2350 <RP2350.uf2> -o <UNIFIED.uf2>

# Gravação automática (toca CDC 1200-baud ou usa drive RPI-RP2 / RP2350 em BOOTSEL):
cargo run -q -p panadiga-cli -- pico flash [-p COM10] [-f crates/panadiga-core/src/pico/firmware_bins/picofw_unified.uf2]
```
- **Montagem BOOTSEL por SO:** Windows `D:\..Z:\`; Linux `/media`, `/run/media/$USER/`, `/mnt`; macOS `/Volumes/RPI-RP2` ou `/Volumes/RP2350`.
- **Portas Seriais por SO:** Windows `COM*`; Linux `/dev/ttyACM*` (grupo `dialout`/`uucp`); macOS `/dev/cu.usbmodem*`.

## 4. Subcomandos CLI Reais (`panadiga pico`)
- Descoberta & saúde: `list`, `ping -p <PORT>`, `status -p <PORT>`, `dcsr -p <PORT>`
- Controle & captura: `breakin -p <PORT> -m <0..3>` (**flag `-m`/`--mode`**), `dump -p <PORT> -a <HEX> -w <1..16384> -o <BIN>`, `capture-nor -p <PORT> --slice <full|bgo|nvram|loader|custom> -o <BIN>`, `davis-probe -p <PORT> --drive <master|slave|both>`
- Depuração & automação: `gdb-bridge -p <PORT> --target-mode <ocd|panamon1|panarsp>`, `bench-wizard -p <PORT> --profile <passive|prejig|diagwatch|jig|all>`, `silicon-replay`, `silicon-snapshot`, `ab-boot`, `rsp-codec`, `validate-bench-cmd`, `parse-diag`, `decode-log`, `error-lookup`

## 5. Daqui pra Frente & Verificação
- **Expansão C11:** Mantenha novos drivers em subpastas temáticas (`diag/`, `uart/`, `flash/`, `gdb/`), sem inchamento de `cli/`, respeitando `256 KiB` em `P0_FW_RO` e adicionando testes host em `firmware/picofw/tests/`.
- **Verificação:** `python scripts/validate.py` Gate 8 compila os módulos C11 (`firmware/picofw/` e `firmware/digaos/`) e roda os executáveis de teste host + checagem ARM (quando `PICO_SDK_PATH` estiver presente).
