/**
 * @file picofw_cli.c
 * @brief USB CDC CLI parser, safety interlocks, and modular command router for P7501.
 *
 * Routes incoming text commands cleanly to:
 *   - Hardware HAL (`hal/picofw_hw.h`)
 *   - Bus Protocol (`protocol/picofw_bus_protocol.h`)
 *   - NOR Flash & SRAM Loader (`flash/picofw_flash.h`)
 *   - Electrical & Bus Diagnostics (`diag/picofw_diag_{electrical,bus}.h`)
 *   - P7501 Service UART & FMU Probes (`uart/p7501_*.h`)
 *   - GDB Remote Serial Protocol Bridge (`gdb/gdb_rsp.h`)
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "cli/picofw_cli.h"
#include "cli/picofw_cli_help.h"
#include "cli/picofw_cli_sys.h"
#include "generated/picofw_ssot.h"
#include "common/picofw_hex.h"
#include "common/picofw_push_stream.h"
#include "crypto/picofw_crypto_cli.h"
#include "diag/picofw_diag_boottrace.h"
#include "diag/picofw_diag_bus.h"
#include "diag/picofw_diag_electrical.h"
#include "diag/picofw_diag_pwrtrace.h"
#include "diag/picofw_diag_sdsniff.h"
#include "diag/picofw_gps_cli.h"
#include "diag/picofw_hotplug_map.h"
#include "diag/picofw_surg_cli.h"
#include "diag/picofw_i2c_codec.h"
#include "diag/picofw_identify.h"
#include "diag/picofw_ir_kaseikyo.h"
#include "diag/picofw_jigsweep_codec.h"
#include "diag/picofw_mmio_snapshot_codec.h"
#include "diag/picofw_piologic_codec.h"
#include "diag/picofw_pwrosc_codec.h"
#include "diag/picofw_serv_macro.h"
#include "diag/picofw_tbus_codec.h"
#include "flash/picofw_fast_stream.h"
#include "flash/picofw_flash_cli.h"
#include "flash/picofw_ftl_inspect.h"
#include "flash/picofw_job.h"
#include "flash/picofw_payloads.h"
#include "flash/picofw_qspi_storage.h"
#include "flash/picofw_sector_ops.h"
#include "flash/picofw_vault_codec.h"
#include "gdb/gdb_rsp.h"
#include "hal/picofw_ble_nus.h"
#include "hal/picofw_led.h"
#include "hal/picofw_usb_dual_cdc.h"
#include "hal/picofw_watchdog.h"
#include "uart/picofw_diag_watch.h"
#include "uart/picofw_drv_pana.h"
#include "uart/picofw_fmu_probe.h"
#include "uart/picofw_pana_rsp.h"
#include "uart/picofw_uart_baud.h"
#include "uart/picofw_uart_capture.h"
#include "uart/picofw_uart_codec.h"
#include "uart/picofw_uart_interactive.h"
#include "uart/picofw_uart_stream.h"
#include "uart/picofw_uart_tx.h"

#include <stdio.h>
#include <string.h>

#ifndef PICOFW_ENABLE_LAB_DIAG
#define PICOFW_ENABLE_LAB_DIAG 1
#endif

static const char *s_cli_res_status = "OK";
static const char *s_cli_res_code = "0";
static char s_cli_res_extra[96] = {0};

void picofw_cli_set_result(const char *status, const char *code) {
    s_cli_res_status = (status != NULL) ? status : "OK";
    s_cli_res_code = (code != NULL) ? code : "0";
}

void picofw_cli_set_result_extra(const char *extra_kv) {
    if ((extra_kv == NULL) || (extra_kv[0] == '\0')) {
        s_cli_res_extra[0] = '\0';
        return;
    }
    (void)snprintf(s_cli_res_extra, sizeof(s_cli_res_extra), "%s", extra_kv);
}

static bool cli_is_ws(char c) {
    return (c == ' ') || (c == '\t') || (c == '\r') || (c == '\n');
}

static const char *cli_extract_arg(const char *trimmed_line) {
    const char *p = trimmed_line;
    while ((*p != '\0') && (!cli_is_ws(*p))) {
        p++;
    }
    while ((*p != '\0') && cli_is_ws(*p)) {
        p++;
    }
    return p;
}

char *picofw_cli_trim_whitespace(char *str) {
    if (str == NULL) {
        return NULL;
    }
    while ((*str != '\0') && cli_is_ws(*str)) {
        str++;
    }
    size_t len = strlen(str);
    while ((len > 0U) && cli_is_ws(str[len - 1U])) {
        str[len - 1U] = '\0';
        len--;
    }
    return str;
}

static void cli_cmd_ping(const char *arg) { (void)arg; printf("PONG\n"); }
static void cli_cmd_help_wrap(const char *arg) { (void)arg; picofw_cli_print_help(); }
static void cli_cmd_banner_wrap(const char *arg) { (void)arg; picofw_cli_print_banner(); }
static void cli_cmd_sniff_wrap(const char *arg) { picofw_diag_cmd_sniff(arg); }
static void cli_cmd_pincheck_wrap(const char *arg) { picofw_diag_cmd_pincheck(arg); }
static void cli_cmd_busdiag_wrap(const char *arg) { (void)arg; picofw_diag_cmd_busdiag(); }
static void cli_cmd_bootcap_wrap(const char *arg) { picofw_cmd_boot_capture(arg, true, false, "BOOTCAP"); }
static void cli_cmd_uartmon_wrap(const char *arg) { picofw_cmd_boot_capture(arg, false, false, "UARTMON"); }
static void cli_cmd_jigboot_wrap(const char *arg) { picofw_cmd_boot_capture(arg, true, true, "JIGBOOT"); }

static void cli_cmd_dump_wrap(const char *arg) {
    picofw_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    picofw_diag_cmd_dump(arg);
    picofw_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);
}

typedef void (*picofw_cli_handler_fn)(const char *arg);

typedef struct picofw_cli_cmd_entry {
    const char *name;
    picofw_cli_handler_fn handler;
} picofw_cli_cmd_entry_t;

static const picofw_cli_cmd_entry_t k_cli_commands[] = {
    { "PING",           cli_cmd_ping },
    { "STATUS",         picofw_cli_cmd_status },
    { "VDD",            picofw_cli_cmd_status },
    { "SELFTEST",       picofw_cli_cmd_selftest },
    { "HELP",           cli_cmd_help_wrap },
    { "?",              cli_cmd_help_wrap },
    { "BANNER",         cli_cmd_banner_wrap },
    { "DUMP",           cli_cmd_dump_wrap },
    { "SNIFF",          cli_cmd_sniff_wrap },
    { "BREAK_IN",       picofw_diag_cmd_boot_profile },
    { "BOOT_PROFILE",   picofw_diag_cmd_boot_profile },
    { "PINCHECK",       cli_cmd_pincheck_wrap },
    { "BUSDIAG",        cli_cmd_busdiag_wrap },
    { "BOOTCAP",        cli_cmd_bootcap_wrap },
    { "UARTMON",        cli_cmd_uartmon_wrap },
    { "JIGBOOT",        cli_cmd_jigboot_wrap },
    { "JIGPIN",         picofw_cli_cmd_jigpin },
    { "FMUSCAN",        picofw_cmd_fmu_scan },
    { "FMUTEST",        picofw_cmd_fmu_test },
    { "FMUBATCH",       picofw_cmd_fmu_batch },
    { "UARTSEND",       picofw_cmd_uart_send },
    { "UARTPROBE",      picofw_cmd_uart_probe },
    { "GDB",            picofw_cli_cmd_gdb },
    { "ISOLATE",        picofw_cli_cmd_isolate },
    { "HIZ",            picofw_cli_cmd_isolate },
    { "FAULTACK",       picofw_cli_cmd_faultack },
    { "ENGAGE",         picofw_cli_cmd_engage },
    { "AUTODETECT",     picofw_flash_cmd_autodetect },
    { "PROBE",          picofw_flash_cmd_autodetect },
    { "CFI",            picofw_flash_cmd_cfi },
    { "FLASH_SIZE",     picofw_flash_cmd_size },
    { "TARGET_PROFILE", picofw_flash_cmd_target_profile },
    { "RESET",          picofw_cli_cmd_reset_pulse },
    { "LED",            picofw_cli_cmd_led },
    { "BAUD",           picofw_cmd_baud },
    { "FRAMING",        picofw_cmd_framing },
    { "BAUDSCAN",       picofw_cmd_baudscan },
    { "UARTSTREAM",     picofw_cmd_uart_stream },
    { "HEXCAP",         picofw_cmd_hexcap },
    { "UARTTXHEX",      picofw_cmd_uart_tx_hex },
    { "WAITTX",         picofw_cmd_waittx },
    { "PWRTRACE",       picofw_cmd_pwrtrace },
    { "BOOTTRACE",      picofw_cmd_boottrace },
    { "BOOTFOLLOW",     picofw_cmd_bootfollow },
    { "PIOLOGIC",       picofw_cmd_piologic },
    { "SDSNIFF",        picofw_cmd_sdsniff },
    { "JIGSWEEP",       picofw_cmd_jigsweep },
    { "PANARSP",        picofw_cmd_pana_rsp },
    { "DIAGWATCH",      picofw_cmd_diag_watch },
    { "DRVPANA",        picofw_cmd_drv_pana },
    { "IR",             picofw_cmd_ir },
    { "IRTX",           picofw_cmd_irtx },
    { "IRRX",           picofw_cmd_irrx },
    { "SERVMACRO",      picofw_cmd_serv_macro },
    { "IDENTIFY",       picofw_cmd_identify },
    { "AUTOPROBE",      picofw_cmd_identify },
    { "JOB",            picofw_cmd_job },
    { "VAULT",          picofw_cmd_vault },
#if PICOFW_ENABLE_LAB_DIAG
    { "BLACKBOX",       picofw_cmd_blackbox },
    { "PROBE_CFG",      picofw_cmd_probe_cfg },
    { "RECIPE",         picofw_cmd_recipe },
#endif
    { "BGO_INSPECT",    picofw_cmd_bgo_inspect },
    { "EEMUL_INSPECT",  picofw_cmd_eemul_inspect },
    { "NOR_MAP",        picofw_cmd_nor_map },
    { "FAST_READ",      picofw_cmd_fast_read },
    { "FAST_WRITE",     picofw_cmd_fast_write },
    { "FAST_ERASE",     picofw_cmd_fast_erase },
    { "MMIO_SNAP",      picofw_cmd_mmio_snap },
    { "HOTPLUG",        picofw_cmd_hotplug },
    { "LIVE_MAP",       picofw_cmd_live_map },
    { "STREAM",         picofw_cmd_stream },
    { "TRAP",           picofw_cmd_trap },
    { "PAYLOAD",        picofw_cmd_payload },
    { "SECTOR",         picofw_cmd_sector },
#if PICOFW_ENABLE_LAB_DIAG
    { "CRYPTO",         picofw_cmd_crypto },
    { "DECOMP",         picofw_cmd_decomp },
    { "GPS",            picofw_cmd_gps },
    { "ROUTE",          picofw_cmd_route },
    { "XRAY",           picofw_cmd_xray },
    { "SURG",           picofw_cmd_surg },
    { "PWROSC",         picofw_cmd_pwrosc },
#endif
    { "TBUSSNIFF",      picofw_cmd_tbussniff },
    { "M0KEEPALIVE",    picofw_cmd_m0keepalive },
    { "I2C_SCAN",       picofw_cmd_i2c_scan },
    { "EEPROM",         picofw_cmd_eeprom },
    { "USBMODE",        picofw_cmd_usbmode },
    { "BLE",            picofw_cmd_ble },
    { "BLE_NOR_AUTH",   picofw_cmd_ble_nor_auth },
    { "BRIDGE",         picofw_cmd_bridge },
    { "WDT",            picofw_cmd_wdt_status },
    { "BOOTSEL",        picofw_cli_cmd_bootsel },
};

void picofw_cli_process_command(char *line) {
    char *trimmed = picofw_cli_trim_whitespace(line);
    if ((trimmed == NULL) || (*trimmed == '\0')) {
        return;
    }

    char raw_cmd[32];
    (void)picofw_next_token(trimmed, raw_cmd, sizeof(raw_cmd));
    char cmd[32];
    picofw_ascii_upper_copy(cmd, sizeof(cmd), raw_cmd);
    const char *arg = cli_extract_arg(trimmed);

    for (size_t i = 0u; i < (sizeof(k_cli_commands) / sizeof(k_cli_commands[0])); ++i) {
        if (strcmp(cmd, k_cli_commands[i].name) == 0) {
            picofw_cli_set_result("OK", PICOFW_RES_CODE_OK);
            picofw_cli_set_result_extra(NULL);
            picofw_wdt_begin_cmd((uint8_t)i, k_cli_commands[i].name);
            k_cli_commands[i].handler(arg);
            if (!gdb_rsp_is_active()) {
                picofw_wdt_end_cmd(false);
                printf(PICOFW_WIRE_TAG_RES " cmd=%s status=%s code=%s elapsed_us=%llu%s%s\n",
                       k_cli_commands[i].name,
                       s_cli_res_status,
                       s_cli_res_code,
                       (unsigned long long)picofw_wdt_snapshot().last_cmd_elapsed_us,
                       (s_cli_res_extra[0] != '\0') ? " " : "",
                       s_cli_res_extra);
            }
            return;
        }
    }
    printf("Unknown command: '%s'. Type HELP for commands.\n", cmd);
    printf(PICOFW_WIRE_TAG_RES " cmd=%s status=ERR code=%s elapsed_us=0\n", cmd, PICOFW_RES_CODE_EINVAL);
}
