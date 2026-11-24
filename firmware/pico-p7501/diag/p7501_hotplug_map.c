/**
 * @file p7501_hotplug_map.c
 * @brief Non-intrusive hotplug mapper & push controller (`HOTPLUG`, `LIVE_MAP`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_hotplug_map.h"

#include "cli/p7501_cli.h"
#include "common/p7501_cmd_arena.h"
#include "common/p7501_hex.h"
#include "common/p7501_push_stream.h"
#include "diag/p7501_boottrace_codec.h"
#include "diag/p7501_diag_boottrace.h"
#include "diag/p7501_hotplug_codec.h"
#include "diag/p7501_i2c_codec.h"
#include "diag/p7501_mmio_snapshot_codec.h"
#include "flash/p7501_fast_stream_codec.h"
#include "generated/p7501_ssot.h"
#include "hal/p7501_hw.h"
#include "hal/p7501_led.h"
#include "hal/p7501_multicore.h"
#include "p7501_types.h"
#include "uart/p7501_pio_loop.h"

#include "hardware/gpio.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

typedef enum live_map_mode {
    LIVE_MODE_QUICK = 0,
    LIVE_MODE_FULL = 1,
    LIVE_MODE_VIDEO = 2
} live_map_mode_t;

typedef struct live_map_uart_ctx {
    char line[160];
    size_t line_len;
    p7501_boot_classifier_t classifier;
} live_map_uart_ctx_t;

static const char *live_mode_name(live_map_mode_t m) {
    if (m == LIVE_MODE_QUICK) {
        return "QUICK";
    }
    if (m == LIVE_MODE_VIDEO) {
        return "VIDEO";
    }
    return "FULL";
}

static void live_flush_uart_line(live_map_uart_ctx_t *ctx) {
    if (ctx->line_len == 0u) {
        return;
    }
    ctx->line[ctx->line_len] = '\0';
    p7501_boot_classifier_feed_line(&ctx->classifier, ctx->line);
    ctx->line_len = 0u;
}

static void live_on_rx_byte(void *raw_ctx, uint8_t byte, uint32_t rel_us, bool *out_stop) {
    (void)rel_us;
    (void)out_stop;
    live_map_uart_ctx_t *const ctx = (live_map_uart_ctx_t *)raw_ctx;
    if (p7501_line_accum_push_byte(ctx->line, &ctx->line_len, sizeof(ctx->line), byte)) {
        live_flush_uart_line(ctx);
    }
}

void p7501_cmd_hotplug(const char *arg) {
    p7501_hotplug_fsm_t *const fsm = p7501_hw_hotplug_fsm();
    const bool has_arg = (arg != NULL) && (*arg != '\0');
    bool ack_req = false;
    if (has_arg && p7501_ascii_ieq(arg, "ON")) {
        fsm->push_enabled = true;
    } else if (has_arg && p7501_ascii_ieq(arg, "OFF")) {
        fsm->push_enabled = false;
    } else if (has_arg && p7501_ascii_ieq(arg, "ACK")) {
        ack_req = true;
    } else if (has_arg && !p7501_ascii_ieq(arg, "STATUS")) {
        p7501_cli_set_result("ERR", "EINVAL");
        printf("HOTPLUG: ERROR (Usage: HOTPLUG [ON|OFF|ACK|STATUS])\n");
        return;
    }

    p7501_hw_enforce_nsw5v_hiz();
    const uint32_t vdd_mv = p7501_hw_sample_sw_uart_mv(false, false);
    const bool nsw_h = gpio_get(P7501_PIN_NSW5V);
    bool changed = false;
    bool clear_fault = false;
    if (ack_req) {
        (void)p7501_hw_ack_fault();
    }
    p7501_hotplug_fsm_step(fsm, vdd_mv, nsw_h, p7501_hw_is_nsw5v_fault_latched(), &changed, &clear_fault);
    if (clear_fault) {
        (void)p7501_hw_ack_fault();
    }

    char kv[160];
    if (p7501_hotplug_format_kv(fsm, p7501_hw_is_nsw5v_fault_latched(),
                                p7501_hw_get_nsw5v_dropouts(), kv, sizeof(kv)) > 0u) {
        printf("@P7501_HOTPLUG %s\n", kv);
        (void)p7501_push_emit_str("HOTPLUG", kv);
        (void)p7501_push_flush_global();
    }
}

static bool parse_live_map_args(const char *arg, live_map_mode_t *out_mode, uint32_t *out_ms) {
    typedef struct kw_mode {
        const char *kw;
        live_map_mode_t mode;
    } kw_mode_t;
    static const kw_mode_t k_modes[3] = {
        { "QUICK", LIVE_MODE_QUICK },
        { "FULL",  LIVE_MODE_FULL },
        { "VIDEO", LIVE_MODE_VIDEO },
    };

    *out_mode = LIVE_MODE_FULL;
    *out_ms = 2000u;

    char copy[64];
    char *toks[2] = { NULL, NULL };
    const size_t n = p7501_copy_and_split_ws(arg, copy, sizeof(copy), toks, 2u);
    for (size_t idx = 0u; idx < n; ++idx) {
        bool matched = false;
        for (size_t k = 0u; k < 3u; ++k) {
            if (p7501_ascii_ieq(toks[idx], k_modes[k].kw)) {
                *out_mode = k_modes[k].mode;
                matched = true;
                break;
            }
        }
        if (matched) {
            continue;
        }
        uint32_t ms = 0u;
        if (!p7501_parse_u32_auto(toks[idx], &ms)) {
            return false;
        }
        *out_ms = (ms < 200u) ? 200u : ((ms > 30000u) ? 30000u : ms);
    }
    return true;
}

static void push_live_nor_headers(void) {
    typedef struct hdr_probe {
        const char *label;
        uint32_t va;
    } hdr_probe_t;
    static const hdr_probe_t k_probes[4] = {
        { "SA0_IPL",   0x84000000UL },
        { "SA15_FE",   0x84080000UL },
        { "SA26_BGO",  0x84130000UL },
        { "SA134_NVR", 0x847F0000UL },
    };
    uint8_t raw16[16];
    char hex32[40];
    char msg[112];
    for (size_t i = 0u; i < 4u; ++i) {
        if (p7501_hw_read_block_checked(k_probes[i].va, raw16, sizeof(raw16), true)) {
            (void)p7501_fast_format_page_hex(raw16, sizeof(raw16), hex32, sizeof(hex32));
            (void)snprintf(msg, sizeof(msg), "sa=%s va=0x%08" PRIX32 " hex=%s",
                           k_probes[i].label, k_probes[i].va, hex32);
            (void)p7501_push_emit_str("NOR_HDR", msg);
        }
    }
}

void p7501_cmd_live_map(const char *arg) {
    live_map_mode_t mode = LIVE_MODE_FULL;
    uint32_t window_ms = 2000u;
    if (!parse_live_map_args(arg, &mode, &window_ms)) {
        p7501_cli_set_result("ERR", "EINVAL");
        printf("LIVE_MAP: ERROR (Usage: LIVE_MAP [QUICK|FULL|VIDEO] [ms])\n");
        return;
    }

    p7501_bus_pin_sample_t pins;
    p7501_hw_sample_bus_pins(&pins);

    char *const elec_msg = (char *)&g_p7501_cmd_arena.raw[8192];
    (void)snprintf(
        elec_msg, 224u,
        "vdd_mv=%" PRIu32 " sw_pu_mv=%" PRIu32 " nsw5v=%u rxd=%u txd=%u xreset=%u "
        "dropouts=%" PRIu32 " fault_latch=%u recoveries=%" PRIu32 " core1_edges=%" PRIu32,
        pins.vdd_mv, pins.sw_pu_mv, pins.nsw5v_high ? 1u : 0u, pins.rxd_high ? 1u : 0u,
        pins.txd_high ? 1u : 0u, pins.xreset_high ? 1u : 0u,
        p7501_hw_get_nsw5v_dropouts(), p7501_hw_is_nsw5v_fault_latched() ? 1u : 0u,
        p7501_hw_get_nsw5v_recoveries(), p7501_multicore_edge_count()
    );
    (void)p7501_push_emit_str("ELEC", elec_msg);

    static const uint8_t k_i2c_addrs[4] = {
        P7501_SSOT_IC3001_BR_NA_ADDR7_L,
        P7501_SSOT_IC3001_BR_NA_ADDR7_H,
        P7501_SSOT_IC3001_EC_ADDR7,
        0x50u,
    };
    const uint8_t i2c_mask = p7501_i2c_probe_addrs_safe(k_i2c_addrs, 4u);
    char *const ic_msg = (char *)&g_p7501_cmd_arena.raw[8448];
    (void)snprintf(
        ic_msg, 128u,
        "ic48=%u ic49=%u ic4a_ec=%u ee50=%u mask=0x%02X",
        (i2c_mask & 1u) ? 1u : 0u,
        (i2c_mask & 2u) ? 1u : 0u,
        (i2c_mask & 4u) ? 1u : 0u,
        (i2c_mask & 8u) ? 1u : 0u,
        (unsigned)i2c_mask
    );
    (void)p7501_push_emit_str("IC3001", ic_msg);

    const bool mon_online = p7501_hw_is_target_online();
    uint32_t mmio_valid = 0u;
    uint32_t uart_bytes = 0u;
    const char *bus_tag = mon_online ? "PANAMON1" : "PASSIVE_UART";

    if (mon_online) {
        const uint8_t dom = (mode == LIVE_MODE_VIDEO)
            ? P7501_SSOT_MMIO_DOM_VIDEO
            : ((mode == LIVE_MODE_QUICK) ? P7501_SSOT_MMIO_DOM_CORE : P7501_SSOT_MMIO_DOM_ALL);
        p7501_mmio_snapshot_t *const snap =
            (p7501_mmio_snapshot_t *)(void *)&g_p7501_cmd_arena.raw[8704];
        (void)p7501_mmio_snap_domain_hw(dom, snap, &mmio_valid);
        if (mode == LIVE_MODE_FULL) {
            push_live_nor_headers();
        }
    } else if (pins.nsw5v_high || (pins.vdd_mv >= P7501_SSOT_VDD_WAKE_MIN_MV)) {
        _Static_assert(sizeof(live_map_uart_ctx_t) <= sizeof(g_p7501_cmd_arena.raw),
                       "live_map_uart_ctx_t must fit inside cmd arena");
        live_map_uart_ctx_t *const ctx = (live_map_uart_ctx_t *)(void *)g_p7501_cmd_arena.raw;
        memset(ctx, 0, sizeof(*ctx));
        p7501_boot_classifier_init(&ctx->classifier);

        const p7501_pio_loop_cfg_t cfg = {
            .window_ms = window_ms,
            .boot_action = P7501_BOOT_ACTION_PASSIVE,
            .jig_hold_ms = 0u,
            .manage_session = true,
            .no_xreset = true,
            .auto_release_serial_boot = true,
            .allow_cdc_abort = true,
            .on_rx_byte = live_on_rx_byte,
            .on_tick = NULL,
            .on_sw_rel = NULL,
            .ctx = ctx,
        };
        p7501_pio_loop_metrics_t m;
        p7501_pio_run_session(&cfg, &m);
        live_flush_uart_line(ctx);
        uart_bytes = m.rx_bytes;
        p7501_boottrace_emit_reports(&ctx->classifier);
    } else {
        bus_tag = "UNPOWERED";
    }

    (void)p7501_push_flush_global();
    const p7501_push_stats_t st = p7501_push_global()->stats;
    printf("@P7501_LIVE_MAP mode=%s bus=%s vdd_mv=%" PRIu32 " nsw5v=%u ic3001_mask=0x%02X "
           "mmio_valid=%" PRIu32 " uart_bytes=%" PRIu32 " frames=%" PRIu32 " flushes=%" PRIu32
           " drops=%" PRIu32 "\n",
           live_mode_name(mode),
           bus_tag,
           pins.vdd_mv,
           pins.nsw5v_high ? 1u : 0u,
           (unsigned)i2c_mask,
           mmio_valid,
           uart_bytes,
           st.frames_staged,
           st.flushes,
           st.drops);
}
