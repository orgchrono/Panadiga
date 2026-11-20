---
name: panadiga-pico-flashing
description: >-
  Compilação e gravação do firmware C11 P7501 Async UART (115200 8N1) no Raspberry Pi Pico (RP2040/RP2350) para bancada do Panasonic DMR-EH55.
  Use esta skill para construir UF2 dual-target, gravar em modo BOOTSEL, escolher portas seriais por SO ou operar comandos da sonda P7501.
---

# Firmware & Operação da Sonda Raspberry Pi Pico (`P7501` Async UART)

Esta skill orienta a compilação do firmware C11 modular (`firmware/pico-p7501/`), a gravação `.uf2` em **RP2040** / **RP2350** e a operação segura na porta de serviço **`P7501`** (`115200 8N1`) nos sistemas **Windows**, **Linux** e **macOS**.

## 1. Invariantes Elétricas Obrigatórias (`P7501` / Placa `VEP79138`)
- **NUNCA exceda 3.3V LVTTL** nos pinos do SoC AM34.
- **`GP3` (`P7501` Pino 5 `NSW5V`, ~`5.15V` upstream via `D7510`)**: SEMPRE `Hi-Z Input` sem pulls — **NUNCA dirigir push-pull**.
- **`GP5` (`P7501` Pino 2 `XRESET`)**: estritamente **Open-Drain** (`0V` ou `Hi-Z`).
- **`GP26` (`P7501` Pino 1 `SW_UART`)**: sinal/sense (`< 10 mA`). **NUNCA alimente o Pico pelo conector `P7501`**; use USB ou `VSYS`.
- **Bloqueio Letal DAVIS**: jamais leia ou escreva no registrador `0xC000_100C`.

## 2. Compilação Dual-Target (`RP2040` + `RP2350` + Unified UF2)
Requer `arm-none-eabi-gcc`, `cmake`, `ninja` e `PICO_SDK_PATH` (2.1.1+):
```bash
# Cross-platform via Python DAG (Windows, Linux, macOS):
python scripts/build.py --pico
# ou via launchers nativos:
# Windows: .\build.cmd --pico
# Linux / macOS: ./build.sh --pico

# Merge manual para UF2 universal RP2350-E10 compatível:
cargo run -p panadiga-cli -- pico merge-uf2 --help
```
Binários pré-compilados ficam em `crates/panadiga-core/src/pico/firmware_bins/panasonic_p7501_uart_unified.uf2` (e variantes `_rp2040.uf2` / `_rp2350.uf2`).

## 3. Gravação Automática ou BOOTSEL por Sistema Operacional
```bash
cargo run -p panadiga-cli -- pico flash
```
Caso não haja firmware anterior rodando CDC 1200-baud, segure o botão **BOOTSEL** ao conectar o USB (`RPI-RP2` / `RP2350`) e copie o `.uf2`:
- **Windows**: detectado automaticamente de `D:\` a `Z:\` (`Copy-Item "...\panasonic_p7501_uart_unified.uf2" "E:\"`).
- **Linux**: detectado em `/media`, `/run/media`, `/mnt`, `$XDG_RUNTIME_DIR/media` e `/proc/mounts` (`cp .../panasonic_p7501_uart_unified.uf2 /run/media/$USER/RPI-RP2/`).
- **macOS**: detectado em `/Volumes/RPI-RP2` ou `/Volumes/RP2350` (`cp .../panasonic_p7501_uart_unified.uf2 /Volumes/RPI-RP2/`).

## 4. Portas Seriais & Comandos de Bancada por SO
- **Identificador de Porta**:
  - **Windows**: `COM*` (ex.: `COM5`, `COM10`)
  - **Linux**: `/dev/ttyACM*` (requer usuário no grupo `dialout` ou `uucp`)
  - **macOS**: `/dev/cu.usbmodem*` (ex.: `/dev/cu.usbmodem101`)
- **Principais Subcomandos CLI (`panadiga pico`)**:
  - `list`, `ping --port <PORT>`, `status --port <PORT>`, `dcsr --port <PORT>`
  - `breakin --port <PORT> --profile <0..3>`, `dump`, `gdb-bridge`
  - `capture-nor --port <PORT> --mode <full|bgo|nvram|loader|custom>` (via `PANAMON1>`)
  - `davis-probe --port <PORT>` (probe seguro CH0/CH1/0xEC sem tocar `0xC000_100C`)
  - `decode-log`, `parse-diag`, `error-lookup <QUERY>`, `bench-wizard`
