# Arquitetura Modular C11 `PicoFW` (`firmware/picofw/`) & Comandos da Sonda

## 1. Mapa das Pastas Modulares (`firmware/picofw/`)
| Pasta | Responsabilidade |
| :--- | :--- |
| `cli/` | Parser de comandos da sonda (`PICO>` / `@PICO_*`), dispatch de console USB-CDC/BLE |
| `common/` | Utilitários puros, ring buffers, CRC32 nibble-LUT, tipos compartilhados |
| `diag/` | `PINCHECK`, `BUSDIAG`, `DIAGWATCH`, `PREJIG`, `PWRTRACE`, `BOOTTRACE`, `BAUDSCAN`, `PIOLOGIC`, `SDSNIFF`, captura IR (`IRRX`) |
| `flash/` | CoW vault interno A/B (`VAULT VERIFY <1..3>`), metadados de boot, persistência RP2040/RP2350 |
| `gdb/` | Servidor GDB Remote Serial Protocol (RSP), codec/guardas `E05` para `SA0..SA10` e `PANAMON1>` |
| `generated/` | `picofw_ssot.h` & `picofw_ssot_tables.c` — **100% gerado** por `panadiga ssot export-all` (nunca editar manualmente) |
| `hal/` | Abstração RP2040 / RP2350 / CYW43439 BLE NUS (`Pico W` / `Pico 2W`) / stubs de host test |
| `uart/` | `PANAMON1>` / `ndb>` bridge transparente (`BRIDGE 115200 PASSIVE`), espaçamento `300–350 µs`, `PANADMP`, protocolo `JEDEC`/`RSP`, `IRTX` |
| `tests/` | Suítes host (`test_uart_codec.c`, `test_pana_logic.c`, `test_pana_diag_drv.c`, `test_gdb_rsp_host.c`, `test_picofw_hw_host.c`, `test_pico_autonomous.c`) compiladas com `-Wall -Wextra -Werror -DPICO_HOST_TEST=1` |

## 2. Os 3 Programas PIO (`firmware/picofw/*.pio`)
- **`picofw_uart_duplex.pio`:** `picofw_uart_rx_fast` (`PIO1 SM0` em `GP2`) com **oversampling 8x** (`921.600 Hz` @ `115200 8N1`), centro em 1,5 bit, amostragem a cada 8 ciclos, check de Stop bit com `irq 4` sem tocar `GP3`; `picofw_uart_tx_fast` (`PIO1 SM1` em `GP4` via `Q7514`).
- **`picofw_logic_sampler.pio`:** Amostrador lógico de 4 canais (`PIOLOGIC`) em `PIO0`.
- **`picofw_sds_sampler.pio`:** Sniffer passivo de barramento SD (`SDSNIFF`, `GP10..GP15`).

## 3. Comandos Principais da Sonda via Terminal / Bridge
- **Diagnóstico Elétrico & Boot:** `STATUS`, `PINCHECK` (`ISOLATE` restaura `adc_gpio_init(GP26)`), `BUSDIAG`, `JIGPIN <LOW|HIZ>`, `JIGBOOT`, `DIAGWATCH WAKE[:ms]`, `PREJIG WAKE`, `BOOTTRACE <ms> [NORMAL|WAKE]`, `PWRTRACE`, `BAUDSCAN`, `PIOLOGIC`, `SDSNIFF`, `ISOLATE`.
- **Ponte & Captura:** `BRIDGE 115200 PASSIVE` (saída pela guarda Hayes `~~~` com silêncio de `50 ms`), `DUMP <addr> <words>`, `CDDIAG`, `DUMPNOR ff_blocks`.
- **Zero-Gadget / Vault / IR:** `VAULT VERIFY <1..3>`, `IR VERIFY`, `IRTX <code...>`, `IRRX`.
- **Allowlist Segura (`scripts/bench/protocol.py`):** Comandos fora de `SAFE_BENCH_COMMANDS` são recusados com `BenchSafetyError` no runner automatizado.
