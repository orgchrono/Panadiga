/**
 * @file p7501_cli_help.c
 * @brief Banner and help reference rendering for the Panadiga P7501 bench CLI.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "cli/p7501_cli_help.h"

#include "gdb/gdb_rsp.h"
#include "hal/p7501_watchdog.h"
#include "p7501_types.h"
#include "uart/p7501_uart_codec.h"
#include "uart/p7501_uart_pio_engine.h"

#include <inttypes.h>
#include <stdio.h>

static const char *cli_mcu_name(void) {
#if defined(PICO_RP2350) && (PICO_RP2350 != 0)
    return "RP2350 (Pico 2 / ARM Cortex-M33 Dual @ 150MHz)";
#elif defined(PANADIGA_BLE_ENABLED) && (PANADIGA_BLE_ENABLED != 0)
    return "RP2040-W (Pico W + CYW43 BLE / ARM Cortex-M0+)";
#else
    return "RP2040 (Pico / ARM Cortex-M0+)";
#endif
}

void p7501_cli_print_banner(void) {
    printf("\n====================================================================\n");
    printf("   PANADIGA: Panasonic DMR-EH55 P7501 Async UART & Boot Bench Probe\n");
    printf("   MCU Host : %s\n", cli_mcu_name());
    printf("   Bus      : P7501 (B6B-ZR-3R4) %" PRIu32 " %s (PIO1) + SW_UART / XRESET / NSW5V(Hi-Z)\n",
           p7501_pio_get_baud(),
           p7501_framing_name(p7501_pio_get_framing()));
    printf("   %s\n", gdb_rsp_get_status_info());
    printf("   ");
    p7501_wdt_print_summary_line();
    printf("   Type HELP for available P7501 UART, PIO1, FMU, diag, Flash, LED, WDT & GDB commands.\n");
    printf("====================================================================\n");
}

void p7501_cli_print_help(void) {
    printf("\n--- Panadiga P7501 Bench CLI Commands ---\n");
    printf("  PING / STATUS / SELFTEST / WDT       - Heartbeat, ADC0 sense, bench self-test & WDT\n");
    printf("  BOOTCAP / UARTMON / JIGBOOT          - PIO1 boot/passive/smart-JIG RX capture\n");
    printf("  JIGPIN [LOW|HIZ|STATUS]              - Software TW7501 (SW_UART GP26) open-drain strap\n");
    printf("  UARTSEND / UARTPROBE                 - PIO1 P7501 TX (Q7514 Hi-Z default) & probe\n");
    printf("  BAUD [bps] / FRAMING [8N1|8E1|8O1]   - PIO1 baudrate & hardware parity framing\n");
    printf("  BAUDSCAN [ms] [boot] [APPLY] [WAKE]  - GP2 cluster autobaud detector & Standby wake\n");
    printf("  UARTSTREAM [ms] [NORMAL|JIG[:ms]|NOPULL] - Live non-blocking PIO1 RX stream\n");
    printf("  HEXCAP [ms] [NORMAL|JIG[:ms]|NOPULL]     - Microsecond-delta hex+ASCII RX capture\n");
    printf("  UARTTXHEX [LOW:]<hex|BRK> [rx_ms]    - PIO1 arbitrary hex / 120ms BREAK inject\n");
    printf("  WAITTX <ms> <boot> \"<trig>\" <hex|BRK> [post] - Triggered TX inject on match\n");
    printf("  BRIDGE [baud] [boot] [CTRL]          - Raw USB-CDC <-> SC0 passthrough + [CTRL] edges\n");
    printf("  PWRTRACE [ms] [boot]                 - Boot ADC0 rail & NSW5V/RXD/TXD glitch logger\n");
    printf("  BOOTTRACE [ms] [boot|PREJIG] [WAKE]  - Correlated PIO1 UART + GP3..5 + ADC0 profiler\n");
    printf("  PIOLOGIC [ms] [boot] [khz] [CTRL]    - PIO0 4-ch (GP2..5) edge & FMU timing sampler\n");
    printf("  SDSNIFF [ms] [boot]                  - Passive Hi-Z PIO0 SD sniffer (GP10..15, CRC16/lane)\n");
    printf("  JIGSWEEP [HOLD|DELAY|PREBOOT] [ms..] - SW_UART latch window & pre-DDR Bootrom sweep\n");
    printf("  PANARSP [ms] [boot] [WAIT:x] <act>   - MAIN _dev_SERIAL RSP (REGS/MEM/WMEM/DUMPNOR/CALL)\n");
    printf("  DIAGWATCH [ms] [boot] [act] [WAKE]   - Autonomous SC0 watchdog, wake-trigger & PANAMON1\n");
    printf("  DRVPANA [ms] [boot] [NOPASS] <DIAG|cmd> - Matshita R6-MV4 Drive Factory CLI (auto PASS)\n");
    printf("  IRTX <preset|sub:cmd> / IRRX [ms]    - 38 kHz Kaseikyo IR TX & TSOP38238 RX decoder\n");
    printf("  SERVMACRO <SERVICE_MODE|SELF_CHECK|OPTICAL_CAL|ERR_HISTORY|RELEASE> [GPn]\n");
    printf("  IDENTIFY / AUTOPROBE / JOB <act>     - 4-stage @P7501_IDENT & atomic NVRAM/BGO/VAULT jobs\n");
    printf("  VAULT / BLACKBOX / PROBE_CFG / RECIPE - 2 MiB QSPI P2..P5 forensic storage & ADC0 cal\n");
    printf("  BGO_INSPECT / EEMUL_INSPECT / MMIO_SNAP / PWROSC / TBUSSNIFF / M0KEEPALIVE\n");
    printf("  I2C_SCAN / EEPROM / USBMODE / BLE / BLE_NOR_AUTH\n");
    printf("  FMUSCAN / FMUTEST / FMUBATCH         - Bootrom FMU handshake sweep & batch probes\n");
    printf("  PINCHECK / BUSDIAG / SNIFF           - 6-pin electrical matrix & NSW5V Hi-Z guard\n");
    printf("  ISOLATE / ENGAGE / RESET             - Hi-Z isolate / TX idle HIGH / XRESET pulse\n");
    printf("  BREAK_IN [0..3] / DUMP               - Boot profile trigger & async monitor dump\n");
    printf("  AUTODETECT / CFI / FLASH_SIZE / TARGET_PROFILE - NOR Flash geometry & DIGA profile\n");
    printf("  GDB / LED <IDLE|ACTIVE|GDB|FAULT|OFF|AUTO> / BOOTSEL\n\n");
}
