/**
 * @file p7501_cli_sys.c
 * @brief System, telemetry, GPIO, LED, SELFTEST (+LOOPBACK), GDB, and BOOTSEL CLI handlers.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "cli/p7501_cli_sys.h"
#include "cli/p7501_cli.h"
#include "common/p7501_hex.h"
#include "flash/p7501_qspi_storage.h"
#include "gdb/gdb_rsp.h"
#include "hal/p7501_hw.h"
#include "hal/p7501_led.h"
#include "hal/p7501_multicore.h"
#include "hal/p7501_watchdog.h"
#include "p7501_types.h"
#include "uart/p7501_uart_codec.h"
#include "uart/p7501_uart_pio_engine.h"

#include "hardware/gpio.h"
#include "pico/bootrom.h"
#include "pico/stdlib.h"
#include "pico/time.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

void p7501_cli_cmd_status(const char *arg) {
    (void)arg;
    const bool jig_latched = gpio_is_dir_out(P7501_PIN_SW_UART_ADC);
    const uint32_t vdd_mv = jig_latched ? 0u : p7501_hw_read_vdd_mv();
    const bool nsw_h = gpio_get(P7501_PIN_NSW5V);
    const bool powered = nsw_h || (vdd_mv >= P7501_SSOT_VDD_MIN_MV);
    /* Avoid engaging GP26 internal pull-up into an unpowered board (anti-backpower). */
    const uint32_t sw_pu_mv = (jig_latched || !powered) ? vdd_mv : p7501_hw_sample_sw_uart_mv(false, true);
    char s_vdd[8], s_pu[8];
    const bool rxd_h = gpio_get(P7501_PIN_RXD);
    const bool txd_h = gpio_get(P7501_PIN_TXD);
    const bool xrst_h = gpio_get(P7501_PIN_XRESET);
    const char *state = "LOW / UNPOWERED";
    pico_led_mode_t next_led = LED_MODE_BUS_FAULT;

    if (vdd_mv >= P7501_SSOT_VDD_FLOATING_CEIL_MV) {
        state = "FLOATING / NO TARGET";
    } else if (vdd_mv >= P7501_SSOT_VDD_MIN_MV) {
        state = "OK";
        next_led = LED_MODE_HEARTBEAT_IDLE;
    }

    if (!gdb_rsp_is_active()) {
        p7501_led_set_auto_mode(next_led);
    }
    printf("VDD: %sV (%s)\n", p7501_fmt_mv_volts(vdd_mv, s_vdd), state);
    printf("[STANDBY] RXD=%d NSW5V=%d TXD=%d XRESET=%d SW_PU=%sV NSW5V_DROPS=%lu | State=%s\n",
           rxd_h ? 1 : 0, nsw_h ? 1 : 0, txd_h ? 1 : 0, xrst_h ? 1 : 0,
           p7501_fmt_mv_volts(sw_pu_mv, s_pu),
           (unsigned long)p7501_hw_get_nsw5v_dropouts(),
           p7501_classify_standby_state_mv(vdd_mv, sw_pu_mv, nsw_h, txd_h, jig_latched));
    p7501_wdt_print_summary_line();
}

void p7501_cli_cmd_jigpin(const char *arg) {
    p7501_jigpin_mode_t mode = P7501_JIGPIN_STATUS;
    if (!p7501_parse_jigpin_arg(arg, &mode)) {
        p7501_cli_set_result("ERR", "EINVAL");
        printf("[JIGPIN] ERROR: Use JIGPIN LOW, JIGPIN HIZ, or JIGPIN STATUS.\n");
        return;
    }
    if (mode == P7501_JIGPIN_LOW) {
        p7501_hw_sw_uart_set_low(true);
        printf("JIGPIN: SW_UART(GP26)=LOW (TW7501_STRAP; XRESET untouched)\n");
        return;
    }
    if (mode == P7501_JIGPIN_HIZ) {
        p7501_hw_sw_uart_set_low(false);
    }
    const bool latched = gpio_is_dir_out(P7501_PIN_SW_UART_ADC);
    const uint32_t mv_hiz = latched ? 0u : p7501_hw_sample_sw_uart_mv(false, false);
    char s_hiz[8];
    printf("JIGPIN: SW_UART(GP26)=%s (%sV, NSW5V=%d)\n",
           latched ? "LOW (TW7501_STRAP)" : "Hi-Z ADC0",
           p7501_fmt_mv_volts(mv_hiz, s_hiz),
           gpio_get(P7501_PIN_NSW5V) ? 1 : 0);
}

void p7501_cli_cmd_led(const char *arg) {
    pico_led_mode_t req_mode = LED_MODE_HEARTBEAT_IDLE;
    bool is_auto = false;
    bool is_query = false;

    if (!p7501_led_parse_arg(arg, &req_mode, &is_auto, &is_query)) {
        p7501_cli_set_result("ERR", "EINVAL");
        printf("[LED] ERROR: Invalid mode '%s'. Use IDLE, ACTIVE, GDB, FAULT, OFF, or AUTO.\n",
               arg ? arg : "");
        return;
    }

    if (is_auto) {
        const uint32_t vdd_mv = p7501_hw_read_vdd_mv();
        const bool vdd_ok = (vdd_mv >= P7501_SSOT_VDD_MIN_MV) &&
                            (vdd_mv < P7501_SSOT_VDD_FLOATING_CEIL_MV);
        p7501_led_unlock_auto(vdd_ok ? LED_MODE_HEARTBEAT_IDLE : LED_MODE_BUS_FAULT);
    } else if (!is_query) {
        p7501_led_set_mode(req_mode);
    }

    const p7501_led_state_t st = p7501_led_get_state();
    printf("[LED] Pico %s | mode=%s | lock=%s | pin=%d\n",
           p7501_led_backend_label(),
           p7501_led_mode_name(st.mode),
           st.manual_lock ? "MANUAL" : "AUTO",
           st.current_on ? 1 : 0);
    printf("LED: OK (%s, %s)\n",
           p7501_led_mode_name(st.mode),
           st.manual_lock ? "MANUAL_LOCK" : "AUTO");
}

static void run_selftest_loopback(void) {
    static const uint8_t k_pattern[8] = { 0x55u, 0xAAu, 0xA5u, 0x5Au, 'P', 'A', 'N', 'A' };
    if (p7501_hw_is_nsw5v_high()) {
        p7501_cli_set_result("ERR", "EBUSY");
        printf("[SELFTEST LOOPBACK] ERROR EBUSY: NSW5V is HIGH (disconnect P7501 target before wiring GP4<->GP2 jumper).\n");
        return;
    }

    p7501_pio_begin_rx_session(P7501_BOOT_ACTION_PASSIVE);
    p7501_pio_drain_rx();
    p7501_pio_tx_bytes_duplex(k_pattern, sizeof(k_pattern), false, true, NULL, NULL);

    uint8_t rx[8];
    size_t got = 0u;
    const uint64_t deadline = time_us_64() + 25000ULL;
    while (got < sizeof(rx) && time_us_64() < deadline) {
        uint8_t b = 0u;
        if (p7501_pio_pop_rx_byte(&b)) {
            rx[got++] = b;
        }
        p7501_wdt_kick();
    }
    p7501_pio_end_session();

    const bool match = (got == sizeof(k_pattern)) && (memcmp(rx, k_pattern, sizeof(k_pattern)) == 0);
    printf("[SELFTEST LOOPBACK] tx=%u rx=%u match=%s\n",
           (unsigned)sizeof(k_pattern), (unsigned)got, match ? "YES" : "NO");
    if (!match) {
        p7501_cli_set_result("ERR", "EIO");
        printf("SELFTEST: ERROR (loopback mismatch; jumper GP4 TXD <-> GP2 RXD)\n");
    } else {
        printf("SELFTEST: OK (LOOPBACK)\n");
    }
}

void p7501_cli_cmd_selftest(const char *arg) {
    p7501_hw_enforce_nsw5v_hiz();
    if ((arg != NULL) && (*arg != '\0')) {
        char tok[24];
        if ((p7501_next_token(arg, tok, sizeof(tok)) != NULL) && p7501_ascii_ieq(tok, "LOOPBACK")) {
            run_selftest_loopback();
            return;
        }
        p7501_cli_set_result("ERR", "EINVAL");
        printf("[SELFTEST] ERROR: Usage SELFTEST [LOOPBACK]\n");
        return;
    }

    const bool nsw5v_hiz_safe = !gpio_is_dir_out(P7501_PIN_NSW5V);
    const bool xreset_hiz = !gpio_is_dir_out(P7501_PIN_XRESET);
    const bool txd_hiz = !gpio_is_dir_out(P7501_PIN_TXD);
    const uint32_t vdd_mv = p7501_hw_read_vdd_mv();
    const uint32_t drops = p7501_hw_get_nsw5v_dropouts();
    const bool dma_ok = p7501_pio_is_rx_dma_active();
    const bool hw_ok = p7501_hw_is_initialized();
    uint32_t c1_ticks = 0u;
    uint32_t c1_drop = 0u;
    p7501_multicore_stats(&c1_ticks, &c1_drop);
    uint32_t fw_len = 0u;
    const uint32_t fw_crc = p7501_qspi_firmware_crc32(&fw_len);

    printf("[SELFTEST] hal_init=%s nsw5v_hiz=%s xreset_od_hiz=%s txd_hiz=%s dma_ring=%s vdd=%lumV nsw5v_dropouts=%lu wdt_ms=%u core1_ticks=%lu core1_drop=%lu fw_bytes=%lu fw_crc32=0x%08lX\n",
           hw_ok ? "OK" : "ERR",
           nsw5v_hiz_safe ? "SAFE_IN" : "FAULT",
           xreset_hiz ? "SAFE_IN" : "LOW",
           txd_hiz ? "SAFE_IN" : "OUT",
           dma_ok ? "ARMED_16K" : "ON_DEMAND",
           (unsigned long)vdd_mv,
           (unsigned long)drops,
           (unsigned)P7501_WDT_TIMEOUT_MS,
           (unsigned long)c1_ticks,
           (unsigned long)c1_drop,
           (unsigned long)fw_len,
           (unsigned long)fw_crc);
    if (!hw_ok || !nsw5v_hiz_safe) {
        p7501_cli_set_result("ERR", "EHW");
        printf("SELFTEST: ERROR\n");
    } else {
        printf("SELFTEST: OK\n");
    }
}

void p7501_cli_cmd_gdb(const char *arg) {
    (void)arg;
    printf("Switching to GDB Remote Serial Protocol (RSP) Bridge Mode...\n");
    printf("[GDB] WDT armed (STAGE=GDB_RSP). Send '$D#44' to detach cleanly.\n");
    fflush(stdout);
    p7501_wdt_set_stage(P7501_WDT_STAGE_GDB_RSP);
    gdb_rsp_set_active(true);
    p7501_led_set_auto_mode(LED_MODE_GDB_RSP);
}

void p7501_cli_cmd_isolate(const char *arg) {
    (void)arg;
    p7501_hw_isolate_bus_hiz();
    printf("[BUS] P7501 pins placed in High-Impedance Isolation (NSW5V/XRESET Hi-Z, GP26 ADC0).\n");
}

void p7501_cli_cmd_engage(const char *arg) {
    (void)arg;
    p7501_hw_engage_uart_bus();
    printf("[BUS] P7501 UART TX (GP4) engaged idle HIGH; NSW5V (GP3) kept Hi-Z Input.\n");
}

void p7501_cli_cmd_reset_pulse(const char *arg) {
    (void)arg;
    p7501_hw_reset_assert();
    p7501_wdt_sleep_ms(P7501_RESET_ASSERT_MS);
    p7501_hw_reset_release();
    printf("[RESET] Open-drain XRESET pulse complete.\n");
}

void p7501_cli_cmd_bootsel(const char *arg) {
    (void)arg;
    printf("[BOOTSEL] Rebooting into USB UF2 Mass-Storage Bootloader...\n");
    fflush(stdout);
    p7501_wdt_kick();
    p7501_wdt_sleep_ms(50U);
    p7501_wdt_prepare_bootsel_reset();
    reset_usb_boot(0U, 0U);
}
