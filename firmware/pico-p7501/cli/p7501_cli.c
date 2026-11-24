/**
 * @file p7501_cli.c
 * @brief USB CDC CLI parser, safety interlocks, and modular command router for P7501.
 *
 * Routes incoming text commands cleanly to:
 *   - Hardware HAL (`hal/p7501_hw.h`)
 *   - Bus Protocol (`protocol/p7501_bus_protocol.h`)
 *   - NOR Flash & SRAM Loader (`flash/p7501_flash.h`)
 *   - Electrical & Bus Diagnostics (`diag/p7501_diag_{electrical,bus}.h`)
 *   - P7501 Service UART & FMU Probes (`uart/p7501_*.h`)
 *   - GDB Remote Serial Protocol Bridge (`gdb/gdb_rsp.h`)
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "cli/p7501_cli.h"
#include "cli/p7501_cli_help.h"
#include "cli/p7501_cli_sys.h"
#include "common/p7501_hex.h"
#include "common/p7501_push_stream.h"
#include "crypto/p7501_crypto_cli.h"
#include "diag/p7501_diag_boottrace.h"
#include "diag/p7501_diag_bus.h"
#include "diag/p7501_diag_electrical.h"
#include "diag/p7501_diag_pwrtrace.h"
#include "diag/p7501_diag_sdsniff.h"
#include "diag/p7501_gps_cli.h"
#include "diag/p7501_hotplug_map.h"
#include "diag/p7501_surg_cli.h"
#include "diag/p7501_i2c_codec.h"
#include "diag/p7501_identify.h"
#include "diag/p7501_ir_kaseikyo.h"
#include "diag/p7501_jigsweep_codec.h"
#include "diag/p7501_mmio_snapshot_codec.h"
#include "diag/p7501_piologic_codec.h"
#include "diag/p7501_pwrosc_codec.h"
#include "diag/p7501_serv_macro.h"
#include "diag/p7501_tbus_codec.h"
#include "flash/p7501_fast_stream.h"
#include "flash/p7501_flash_cli.h"
#include "flash/p7501_ftl_inspect.h"
#include "flash/p7501_job.h"
#include "flash/p7501_payloads.h"
#include "flash/p7501_qspi_storage.h"
#include "flash/p7501_sector_ops.h"
#include "flash/p7501_vault_codec.h"
#include "gdb/gdb_rsp.h"
#include "hal/p7501_ble_nus.h"
#include "hal/p7501_led.h"
#include "hal/p7501_usb_dual_cdc.h"
#include "hal/p7501_watchdog.h"
#include "uart/p7501_diag_watch.h"
#include "uart/p7501_drv_pana.h"
#include "uart/p7501_fmu_probe.h"
#include "uart/p7501_pana_rsp.h"
#include "uart/p7501_uart_baud.h"
#include "uart/p7501_uart_capture.h"
#include "uart/p7501_uart_codec.h"
#include "uart/p7501_uart_interactive.h"
#include "uart/p7501_uart_stream.h"
#include "uart/p7501_uart_tx.h"

#include <stdio.h>
#include <string.h>

static const char *s_cli_res_status = "OK";
static const char *s_cli_res_code = "0";
static char s_cli_res_extra[96] = {0};

void p7501_cli_set_result(const char *status, const char *code) {
    s_cli_res_status = (status != NULL) ? status : "OK";
    s_cli_res_code = (code != NULL) ? code : "0";
}

void p7501_cli_set_result_extra(const char *extra_kv) {
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

char *p7501_cli_trim_whitespace(char *str) {
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
static void cli_cmd_help_wrap(const char *arg) { (void)arg; p7501_cli_print_help(); }
static void cli_cmd_banner_wrap(const char *arg) { (void)arg; p7501_cli_print_banner(); }
static void cli_cmd_sniff_wrap(const char *arg) { p7501_diag_cmd_sniff(arg); }
static void cli_cmd_pincheck_wrap(const char *arg) { p7501_diag_cmd_pincheck(arg); }
static void cli_cmd_busdiag_wrap(const char *arg) { (void)arg; p7501_diag_cmd_busdiag(); }
static void cli_cmd_bootcap_wrap(const char *arg) { p7501_cmd_boot_capture(arg, true, false, "BOOTCAP"); }
static void cli_cmd_uartmon_wrap(const char *arg) { p7501_cmd_boot_capture(arg, false, false, "UARTMON"); }
static void cli_cmd_jigboot_wrap(const char *arg) { p7501_cmd_boot_capture(arg, true, true, "JIGBOOT"); }

static void cli_cmd_dump_wrap(const char *arg) {
    p7501_led_set_auto_mode(LED_MODE_BUS_ACTIVE);
    p7501_diag_cmd_dump(arg);
    p7501_led_set_auto_mode(LED_MODE_HEARTBEAT_IDLE);
}

typedef void (*p7501_cli_handler_fn)(const char *arg);

typedef struct p7501_cli_cmd_entry {
    const char *name;
    p7501_cli_handler_fn handler;
} p7501_cli_cmd_entry_t;

static const p7501_cli_cmd_entry_t k_cli_commands[] = {
    { "PING",           cli_cmd_ping },
    { "STATUS",         p7501_cli_cmd_status },
    { "VDD",            p7501_cli_cmd_status },
    { "SELFTEST",       p7501_cli_cmd_selftest },
    { "HELP",           cli_cmd_help_wrap },
    { "?",              cli_cmd_help_wrap },
    { "BANNER",         cli_cmd_banner_wrap },
    { "DUMP",           cli_cmd_dump_wrap },
    { "SNIFF",          cli_cmd_sniff_wrap },
    { "BREAK_IN",       p7501_diag_cmd_boot_profile },
    { "BOOT_PROFILE",   p7501_diag_cmd_boot_profile },
    { "PINCHECK",       cli_cmd_pincheck_wrap },
    { "BUSDIAG",        cli_cmd_busdiag_wrap },
    { "BOOTCAP",        cli_cmd_bootcap_wrap },
    { "UARTMON",        cli_cmd_uartmon_wrap },
    { "JIGBOOT",        cli_cmd_jigboot_wrap },
    { "JIGPIN",         p7501_cli_cmd_jigpin },
    { "FMUSCAN",        p7501_cmd_fmu_scan },
    { "FMUTEST",        p7501_cmd_fmu_test },
    { "FMUBATCH",       p7501_cmd_fmu_batch },
    { "UARTSEND",       p7501_cmd_uart_send },
    { "UARTPROBE",      p7501_cmd_uart_probe },
    { "GDB",            p7501_cli_cmd_gdb },
    { "ISOLATE",        p7501_cli_cmd_isolate },
    { "HIZ",            p7501_cli_cmd_isolate },
    { "ENGAGE",         p7501_cli_cmd_engage },
    { "AUTODETECT",     p7501_flash_cmd_autodetect },
    { "PROBE",          p7501_flash_cmd_autodetect },
    { "CFI",            p7501_flash_cmd_cfi },
    { "FLASH_SIZE",     p7501_flash_cmd_size },
    { "TARGET_PROFILE", p7501_flash_cmd_target_profile },
    { "RESET",          p7501_cli_cmd_reset_pulse },
    { "LED",            p7501_cli_cmd_led },
    { "BAUD",           p7501_cmd_baud },
    { "FRAMING",        p7501_cmd_framing },
    { "BAUDSCAN",       p7501_cmd_baudscan },
    { "UARTSTREAM",     p7501_cmd_uart_stream },
    { "HEXCAP",         p7501_cmd_hexcap },
    { "UARTTXHEX",      p7501_cmd_uart_tx_hex },
    { "WAITTX",         p7501_cmd_waittx },
    { "PWRTRACE",       p7501_cmd_pwrtrace },
    { "BOOTTRACE",      p7501_cmd_boottrace },
    { "PIOLOGIC",       p7501_cmd_piologic },
    { "SDSNIFF",        p7501_cmd_sdsniff },
    { "JIGSWEEP",       p7501_cmd_jigsweep },
    { "PANARSP",        p7501_cmd_pana_rsp },
    { "DIAGWATCH",      p7501_cmd_diag_watch },
    { "DRVPANA",        p7501_cmd_drv_pana },
    { "IR",             p7501_cmd_ir },
    { "IRTX",           p7501_cmd_irtx },
    { "IRRX",           p7501_cmd_irrx },
    { "SERVMACRO",      p7501_cmd_serv_macro },
    { "IDENTIFY",       p7501_cmd_identify },
    { "AUTOPROBE",      p7501_cmd_identify },
    { "JOB",            p7501_cmd_job },
    { "VAULT",          p7501_cmd_vault },
    { "BLACKBOX",       p7501_cmd_blackbox },
    { "PROBE_CFG",      p7501_cmd_probe_cfg },
    { "RECIPE",         p7501_cmd_recipe },
    { "BGO_INSPECT",    p7501_cmd_bgo_inspect },
    { "EEMUL_INSPECT",  p7501_cmd_eemul_inspect },
    { "NOR_MAP",        p7501_cmd_nor_map },
    { "FAST_READ",      p7501_cmd_fast_read },
    { "FAST_WRITE",     p7501_cmd_fast_write },
    { "FAST_ERASE",     p7501_cmd_fast_erase },
    { "MMIO_SNAP",      p7501_cmd_mmio_snap },
    { "HOTPLUG",        p7501_cmd_hotplug },
    { "LIVE_MAP",       p7501_cmd_live_map },
    { "STREAM",         p7501_cmd_stream },
    { "TRAP",           p7501_cmd_trap },
    { "PAYLOAD",        p7501_cmd_payload },
    { "SECTOR",         p7501_cmd_sector },
    { "CRYPTO",         p7501_cmd_crypto },
    { "DECOMP",         p7501_cmd_decomp },
    { "GPS",            p7501_cmd_gps },
    { "ROUTE",          p7501_cmd_route },
    { "XRAY",           p7501_cmd_xray },
    { "SURG",           p7501_cmd_surg },
    { "PWROSC",         p7501_cmd_pwrosc },
    { "TBUSSNIFF",      p7501_cmd_tbussniff },
    { "M0KEEPALIVE",    p7501_cmd_m0keepalive },
    { "I2C_SCAN",       p7501_cmd_i2c_scan },
    { "EEPROM",         p7501_cmd_eeprom },
    { "USBMODE",        p7501_cmd_usbmode },
    { "BLE",            p7501_cmd_ble },
    { "BLE_NOR_AUTH",   p7501_cmd_ble_nor_auth },
    { "BRIDGE",         p7501_cmd_bridge },
    { "WDT",            p7501_cmd_wdt_status },
    { "BOOTSEL",        p7501_cli_cmd_bootsel },
};

void p7501_cli_process_command(char *line) {
    char *trimmed = p7501_cli_trim_whitespace(line);
    if ((trimmed == NULL) || (*trimmed == '\0')) {
        return;
    }

    char raw_cmd[32];
    (void)p7501_next_token(trimmed, raw_cmd, sizeof(raw_cmd));
    char cmd[32];
    p7501_ascii_upper_copy(cmd, sizeof(cmd), raw_cmd);
    const char *arg = cli_extract_arg(trimmed);

    for (size_t i = 0u; i < (sizeof(k_cli_commands) / sizeof(k_cli_commands[0])); ++i) {
        if (strcmp(cmd, k_cli_commands[i].name) == 0) {
            p7501_cli_set_result("OK", "0");
            p7501_cli_set_result_extra(NULL);
            p7501_wdt_begin_cmd((uint8_t)i, k_cli_commands[i].name);
            k_cli_commands[i].handler(arg);
            if (!gdb_rsp_is_active()) {
                p7501_wdt_end_cmd(false);
                printf("@P7501_RES cmd=%s status=%s code=%s elapsed_us=%llu%s%s\n",
                       k_cli_commands[i].name,
                       s_cli_res_status,
                       s_cli_res_code,
                       (unsigned long long)p7501_wdt_snapshot().last_cmd_elapsed_us,
                       (s_cli_res_extra[0] != '\0') ? " " : "",
                       s_cli_res_extra);
            }
            return;
        }
    }
    printf("Unknown command: '%s'. Type HELP for commands.\n", cmd);
    printf("@P7501_RES cmd=%s status=ERR code=EINVAL elapsed_us=0\n", cmd);
}
