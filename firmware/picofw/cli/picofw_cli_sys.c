/**
 * @file picofw_cli_sys.c
 * @brief System, telemetry, GPIO, LED, SELFTEST (+LOOPBACK), GDB, and BOOTSEL CLI handlers.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "cli/picofw_cli_sys.h"
#include "cli/picofw_cli.h"
#include "common/picofw_hex.h"
#include "common/picofw_push_stream.h"
#include "diag/picofw_tbus_codec.h"
#include "flash/picofw_flash.h"
#include "flash/picofw_qspi_storage.h"
#include "gdb/gdb_rsp.h"
#include "hal/picofw_hw.h"
#include "hal/picofw_led.h"
#include "hal/picofw_multicore.h"
#include "hal/picofw_watchdog.h"
#include "picofw_types.h"
#include "uart/picofw_uart_codec.h"
#include "uart/picofw_uart_pio_engine.h"

#include "hardware/gpio.h"
#include "pico/bootrom.h"
#include "pico/stdlib.h"
#include "pico/time.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static picofw_ps_t s_probe_state = PICOFW_PS_ISOLATED;

picofw_ps_t picofw_probe_get_state(void) {
    return s_probe_state;
}

bool picofw_probe_dispatch_event(picofw_ev_t ev, bool active_vdd_sample) {
    picofw_rail_snap_t snap;
    picofw_hw_rail_snapshot(&snap, active_vdd_sample);
    const picofw_step_res_t res = picofw_probe_step(s_probe_state, ev, &snap);
    s_probe_state = res.next;

    if ((res.effects & PICOFW_EFF_UPPER_NOR_LOCK) != 0u) {
        picofw_flash_set_upper_nor_unlocked(false);
    }
    if ((res.effects & PICOFW_EFF_KEEPALIVE_OFF) != 0u) {
        picofw_tbus_keepalive_stop();
    }
    if ((res.effects & PICOFW_EFF_PIO_END) != 0u) {
        picofw_pio_end_session();
    }
    if ((res.effects & PICOFW_EFF_HIZ_ALL) != 0u) {
        picofw_hw_isolate_bus_hiz();
    }
    if ((res.effects & PICOFW_EFF_ACK_FAULT) != 0u) {
        (void)picofw_hw_ack_fault();
    }
    if ((res.effects & PICOFW_EFF_ENGAGE_TX) != 0u) {
        picofw_hw_engage_uart_bus();
    }
    if ((res.effects & PICOFW_EFF_EMIT_FAULT) != 0u) {
        if (!gdb_rsp_is_active()) {
            picofw_led_set_auto_mode(LED_MODE_BUS_FAULT);
        }
        (void)picofw_push_emit_str("FAULT", "nsw5v_dropout=1");
    }
    return res.allowed;
}

void picofw_cli_cmd_status(const char *arg) {
    (void)arg;
    picofw_bus_pin_sample_t pins;
    picofw_hw_sample_bus_pins(&pins);
    const bool jig_latched = pins.jig_latched;
    const uint32_t vdd_mv = pins.vdd_mv;
    const bool nsw_h = pins.nsw5v_high;
    const uint32_t sw_pu_mv = pins.sw_pu_mv;
    const bool rxd_h = pins.rxd_high;
    const bool txd_h = pins.txd_high;
    const bool xrst_h = pins.xreset_high;
    char s_vdd[8], s_pu[8];
    const char *state = "LOW / UNPOWERED";
    pico_led_mode_t next_led = LED_MODE_BUS_FAULT;

    if (vdd_mv >= PICOFW_SSOT_VDD_FLOATING_CEIL_MV) {
        state = "FLOATING / NO TARGET";
    } else if (vdd_mv >= PICOFW_SSOT_VDD_MIN_MV) {
        state = "OK";
        next_led = LED_MODE_HEARTBEAT_IDLE;
    }

    if (!gdb_rsp_is_active()) {
        picofw_led_set_auto_mode(next_led);
    }
    printf("VDD: %sV (%s)\n", picofw_fmt_mv_volts(vdd_mv, s_vdd), state);
    printf("[STANDBY] RXD=%d NSW5V=%d TXD=%d XRESET=%d SW_PU=%sV NSW5V_DROPS=%lu fault_latch=%u recoveries=%lu | State=%s\n",
           rxd_h ? 1 : 0, nsw_h ? 1 : 0, txd_h ? 1 : 0, xrst_h ? 1 : 0,
           picofw_fmt_mv_volts(sw_pu_mv, s_pu),
           (unsigned long)picofw_hw_get_nsw5v_dropouts(),
           picofw_hw_is_nsw5v_fault_latched() ? 1u : 0u,
           (unsigned long)picofw_hw_get_nsw5v_recoveries(),
           picofw_classify_standby_state_mv(vdd_mv, sw_pu_mv, nsw_h, txd_h, jig_latched));
    picofw_wdt_print_summary_line();
}

void picofw_cli_cmd_jigpin(const char *arg) {
    picofw_jigpin_mode_t mode = PICOFW_JIGPIN_STATUS;
    if (!picofw_parse_jigpin_arg(arg, &mode)) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("[JIGPIN] ERROR: Use JIGPIN LOW, JIGPIN HIZ, or JIGPIN STATUS.\n");
        return;
    }
    if (mode == PICOFW_JIGPIN_LOW) {
        picofw_hw_sw_uart_set_low(true);
        printf("JIGPIN: SW_UART(GP26)=LOW (TW7501_STRAP; XRESET untouched)\n");
        return;
    }
    if (mode == PICOFW_JIGPIN_HIZ) {
        picofw_hw_sw_uart_set_low(false);
    }
    const bool latched = gpio_is_dir_out(P7501_PIN_SW_UART_ADC);
    const uint32_t mv_hiz = latched ? 0u : picofw_hw_sample_sw_uart_mv(false, false);
    char s_hiz[8];
    printf("JIGPIN: SW_UART(GP26)=%s (%sV, NSW5V=%d)\n",
           latched ? "LOW (TW7501_STRAP)" : "Hi-Z ADC0",
           picofw_fmt_mv_volts(mv_hiz, s_hiz),
           gpio_get(P7501_PIN_NSW5V) ? 1 : 0);
}

void picofw_cli_cmd_led(const char *arg) {
    pico_led_mode_t req_mode = LED_MODE_HEARTBEAT_IDLE;
    bool is_auto = false;
    bool is_query = false;

    if (!picofw_led_parse_arg(arg, &req_mode, &is_auto, &is_query)) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("[LED] ERROR: Invalid mode '%s'. Use IDLE, ACTIVE, GDB, FAULT, OFF, or AUTO.\n",
               arg ? arg : "");
        return;
    }

    if (is_auto) {
        const uint32_t vdd_mv = picofw_hw_read_vdd_mv();
        const bool vdd_ok = (vdd_mv >= PICOFW_SSOT_VDD_MIN_MV) &&
                            (vdd_mv < PICOFW_SSOT_VDD_FLOATING_CEIL_MV);
        picofw_led_unlock_auto(vdd_ok ? LED_MODE_HEARTBEAT_IDLE : LED_MODE_BUS_FAULT);
    } else if (!is_query) {
        picofw_led_set_mode(req_mode);
    }

    const picofw_led_state_t st = picofw_led_get_state();
    printf("[LED] Pico %s | mode=%s | lock=%s | pin=%d\n",
           picofw_led_backend_label(),
           picofw_led_mode_name(st.mode),
           st.manual_lock ? "MANUAL" : "AUTO",
           st.current_on ? 1 : 0);
    printf("LED: OK (%s, %s)\n",
           picofw_led_mode_name(st.mode),
           st.manual_lock ? "MANUAL_LOCK" : "AUTO");
}

static void run_selftest_loopback(void) {
    static const uint8_t k_pattern[8] = { 0x55u, 0xAAu, 0xA5u, 0x5Au, 'P', 'A', 'N', 'A' };
    if (picofw_hw_is_nsw5v_high()) {
        picofw_cli_set_result("ERR", "EBUSY");
        printf("[SELFTEST LOOPBACK] ERROR EBUSY: NSW5V is HIGH (disconnect P7501 target before wiring GP4<->GP2 jumper).\n");
        return;
    }

    picofw_pio_begin_rx_session(PICOFW_BOOT_ACTION_PASSIVE);
    picofw_pio_drain_rx();
    picofw_pio_tx_bytes_duplex(k_pattern, sizeof(k_pattern), false, true, NULL, NULL);

    uint8_t rx[8];
    size_t got = 0u;
    const uint64_t deadline = time_us_64() + 25000ULL;
    while (got < sizeof(rx) && time_us_64() < deadline) {
        uint8_t b = 0u;
        if (picofw_pio_pop_rx_byte(&b)) {
            rx[got++] = b;
        }
        picofw_wdt_kick();
    }
    picofw_pio_end_session();

    const bool match = (got == sizeof(k_pattern)) && (memcmp(rx, k_pattern, sizeof(k_pattern)) == 0);
    printf("[SELFTEST LOOPBACK] tx=%u rx=%u match=%s\n",
           (unsigned)sizeof(k_pattern), (unsigned)got, match ? "YES" : "NO");
    if (!match) {
        picofw_cli_set_result("ERR", "EIO");
        printf("SELFTEST: ERROR (loopback mismatch; jumper GP4 TXD <-> GP2 RXD)\n");
    } else {
        printf("SELFTEST: OK (LOOPBACK)\n");
    }
}

void picofw_cli_cmd_selftest(const char *arg) {
    picofw_hw_enforce_nsw5v_hiz();
    if ((arg != NULL) && (*arg != '\0')) {
        char tok[24];
        if ((picofw_next_token(arg, tok, sizeof(tok)) != NULL) && picofw_ascii_ieq(tok, "LOOPBACK")) {
            run_selftest_loopback();
            return;
        }
        picofw_cli_set_result("ERR", "EINVAL");
        printf("[SELFTEST] ERROR: Usage SELFTEST [LOOPBACK]\n");
        return;
    }

    const bool nsw5v_hiz_safe = !gpio_is_dir_out(P7501_PIN_NSW5V);
    const bool xreset_hiz = !gpio_is_dir_out(P7501_PIN_XRESET);
    const bool txd_hiz = !gpio_is_dir_out(P7501_PIN_TXD);
    const uint32_t vdd_mv = picofw_hw_read_vdd_mv();
    const uint32_t drops = picofw_hw_get_nsw5v_dropouts();
    const bool dma_ok = picofw_pio_is_rx_dma_active();
    const bool hw_ok = picofw_hw_is_initialized();
    uint32_t c1_ticks = 0u;
    uint32_t c1_drop = 0u;
    picofw_multicore_stats(&c1_ticks, &c1_drop);
    uint32_t fw_len = 0u;
    const uint32_t fw_crc = picofw_qspi_firmware_crc32(&fw_len);

    printf("[SELFTEST] hal_init=%s nsw5v_hiz=%s xreset_od_hiz=%s txd_hiz=%s dma_ring=%s vdd=%lumV nsw5v_dropouts=%lu fault_latch=%u recoveries=%lu wdt_ms=%u core1_ticks=%lu core1_drop=%lu fw_bytes=%lu fw_crc32=0x%08lX\n",
           hw_ok ? "OK" : "ERR",
           nsw5v_hiz_safe ? "SAFE_IN" : "FAULT",
           xreset_hiz ? "SAFE_IN" : "LOW",
           txd_hiz ? "SAFE_IN" : "OUT",
           dma_ok ? "ARMED_16K" : "ON_DEMAND",
           (unsigned long)vdd_mv,
           (unsigned long)drops,
           picofw_hw_is_nsw5v_fault_latched() ? 1u : 0u,
           (unsigned long)picofw_hw_get_nsw5v_recoveries(),
           (unsigned)PICOFW_WDT_TIMEOUT_MS,
           (unsigned long)c1_ticks,
           (unsigned long)c1_drop,
           (unsigned long)fw_len,
           (unsigned long)fw_crc);
    if (!hw_ok || !nsw5v_hiz_safe) {
        picofw_cli_set_result("ERR", "EHW");
        printf("SELFTEST: ERROR\n");
    } else {
        printf("SELFTEST: OK\n");
    }
}

void picofw_cli_cmd_gdb(const char *arg) {
    (void)arg;
    if (!picofw_probe_dispatch_event(PICOFW_EV_GDB_ENTER, true)) {
        picofw_cli_set_result("ERR", picofw_hw_is_nsw5v_fault_latched() ? "EDROPOUT" : "EOFFLINE");
        printf("[GDB] ERROR: Target rail not healthy (NSW5V/VDD/fault_latch); refusing GDB.\n");
        return;
    }
    printf("Switching to GDB Remote Serial Protocol (RSP) Bridge Mode...\n");
    printf("[GDB] WDT armed (STAGE=GDB_RSP). Send '$D#44' to detach cleanly.\n");
    fflush(stdout);
    picofw_wdt_set_stage(PICOFW_WDT_STAGE_GDB_RSP);
    gdb_rsp_set_active(true);
    picofw_led_set_auto_mode(LED_MODE_GDB_RSP);
}

void picofw_cli_cmd_isolate(const char *arg) {
    if ((arg != NULL) && (picofw_ascii_ieq(arg, "CLEAR") || picofw_ascii_ieq(arg, "CLEAR_FAULT"))) {
        (void)picofw_probe_dispatch_event(PICOFW_EV_FAULT_ACK, true);
    }
    (void)picofw_probe_dispatch_event(PICOFW_EV_ISOLATE, false);
    printf("[BUS] P7501 pins placed in High-Impedance Isolation (NSW5V/XRESET Hi-Z, GP26 ADC0).\n");
}

void picofw_cli_cmd_faultack(const char *arg) {
    (void)arg;
    if (picofw_probe_dispatch_event(PICOFW_EV_FAULT_ACK, true)) {
        printf("FAULTACK: OK (bus isolated Hi-Z)\n");
        return;
    }
    picofw_cli_set_result("ERR", "EOFFLINE");
    printf("FAULTACK: ERROR code=EOFFLINE reason=RAIL_UNHEALTHY\n");
}

void picofw_cli_cmd_engage(const char *arg) {
    (void)arg;
    if (!picofw_probe_dispatch_event(PICOFW_EV_ENGAGE, true)) {
        picofw_cli_set_result("ERR", picofw_hw_is_nsw5v_fault_latched() ? "EDROPOUT" : "EOFFLINE");
        printf("[BUS] ERROR: Target rail not healthy (NSW5V/VDD/fault_latch); refusing ENGAGE.\n");
        return;
    }
    printf("[BUS] P7501 UART TX (GP4) engaged idle HIGH; NSW5V (GP3) kept Hi-Z Input.\n");
}

void picofw_cli_cmd_reset_pulse(const char *arg) {
    (void)arg;
    picofw_hw_reset_assert();
    picofw_wdt_sleep_ms(PICOFW_RESET_ASSERT_MS);
    picofw_hw_reset_release();
    printf("[RESET] Open-drain XRESET pulse complete.\n");
}

void picofw_cli_cmd_bootsel(const char *arg) {
    (void)arg;
    printf("[BOOTSEL] Rebooting into USB UF2 Mass-Storage Bootloader...\n");
    fflush(stdout);
    picofw_wdt_kick();
    picofw_wdt_sleep_ms(50U);
    picofw_wdt_prepare_bootsel_reset();
    reset_usb_boot(0U, 0U);
}
