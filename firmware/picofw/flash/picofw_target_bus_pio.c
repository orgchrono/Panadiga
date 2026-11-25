/**
 * @file picofw_target_bus_pio.c
 * @brief Real RP2040/RP2350 asynchronous P7501 PIO1 target-bus transport backend.
 *
 * Uses persistent PIO1 sessions and 64-byte `$m` block transfers with strict
 * checked error returns, Ctrl+C abort, and zero per-word `GP26` pull-down glitches.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "hal/picofw_hw.h"
#include "hal/picofw_watchdog.h"
#include "common/picofw_hex.h"
#include "flash/picofw_target_bus_panamon.h"
#include "uart/picofw_pana_rsp.h"
#include "uart/picofw_uart_pio_engine.h"

#include "pico/stdlib.h"
#include "pico/time.h"

#include <stdio.h>
#include <string.h>

#define PIO_BUS_TX_TIMEOUT_MS 120u

static uint32_t s_pending_pc = 0u;
static bool s_has_pending_pc = false;
static bool s_panamon1_mode = false;

static bool pio_rsp_exchange_open(
    const char *frame,
    size_t flen,
    bool allow_abort,
    char *out_reply,
    size_t out_cap
) {
    if ((frame == NULL) || (flen == 0u) || picofw_hw_is_nsw5v_fault_latched()) {
        return false;
    }

    picofw_pio_drain_rx();
    picofw_pio_tx_bytes((const uint8_t *)frame, flen, false);

    uint8_t rx[192];
    size_t rx_len = 0u;
    const uint64_t t0 = time_us_64();
    const uint64_t deadline = t0 + ((uint64_t)PIO_BUS_TX_TIMEOUT_MS * 1000ULL);
    picofw_rsp_frame_info_t info = {0};
    bool aborted = false;

    while (picofw_wdt_loop_step(t0, deadline, allow_abort, &aborted)) {
        if (picofw_hw_is_nsw5v_fault_latched()) {
            return false;
        }
        rx_len = picofw_pio_pop_rx_bounded(rx, sizeof(rx), rx_len, false);
        if ((rx_len > 0u) && picofw_rsp_inspect_frame(rx, rx_len, &info) &&
            info.found && info.checksum_ok) {
            break;
        }
        if (picofw_bus_panamon1_has_prompt(rx, rx_len)) {
            break;
        }
    }

    if (!aborted && !info.found && picofw_bus_panamon1_has_prompt(rx, rx_len)) {
        s_panamon1_mode = true;
        picofw_bus_panamon1_drain_tail(25u);
        return false;
    }
    if (aborted || !info.found || !info.checksum_ok) {
        return false;
    }
    s_panamon1_mode = false;
    if (out_reply != NULL) {
        if (info.payload_len >= out_cap) {
            return false;
        }
        memcpy(out_reply, &rx[info.payload_offset], info.payload_len);
        out_reply[info.payload_len] = '\0';
    }
    return true;
}

static bool pio_rsp_transact_frame(const char *frame, size_t flen, char *out_reply, size_t out_cap) {
    if ((frame == NULL) || (flen == 0u) || !picofw_hw_is_nsw5v_high() || picofw_hw_is_nsw5v_fault_latched()) {
        return false;
    }
    picofw_pio_begin_rx_session(PICOFW_BOOT_ACTION_PASSIVE);
    const bool ok = pio_rsp_exchange_open(frame, flen, true, out_reply, out_cap);
    picofw_pio_end_session();
    return ok;
}

static bool pio_rsp_transact(const char *payload, char *out_reply, size_t out_cap) {
    if (payload == NULL) {
        return false;
    }
    char frame[96];
    const size_t flen = picofw_rsp_build_frame(payload, frame, sizeof(frame));
    return pio_rsp_transact_frame(frame, flen, out_reply, out_cap);
}

static bool pio_read_block_checked(
    void *ctx,
    uint32_t addr,
    uint8_t *out_buf,
    uint32_t len,
    bool allow_abort
) {
    (void)ctx;
    if ((out_buf == NULL) || (len == 0UL)) {
        return false;
    }
    /* One passive NSW5V rail check up front: zero GP26 pull-down glitches during bulk reads. */
    if (!picofw_hw_is_nsw5v_high() || picofw_hw_is_nsw5v_fault_latched()) {
        s_panamon1_mode = false;
        return false;
    }

    picofw_pio_begin_rx_session(PICOFW_BOOT_ACTION_PASSIVE);
    /* Probe PANAMON1> (`?\r`) before emitting raw GDB RSP `$m...#cs` frames whose hex/checksum bytes can trigger `B` (BYPASS) or `D` (DUMP). */
    if (!s_panamon1_mode) {
        (void)picofw_bus_panamon1_probe_open(allow_abort, &s_panamon1_mode);
    }
    uint32_t done = 0UL;
    bool ok = true;
    char req[32];
    char frame[48];
    char reply[(PICOFW_PIO_BUS_BLOCK_CHUNK_MAX * 2u) + 8u];

    while (done < len) {
        picofw_wdt_kick();
        if (allow_abort && picofw_wdt_poll_abort()) {
            ok = false;
            break;
        }
        const uint32_t rem = len - done;
        const uint32_t chunk = (rem > PICOFW_PIO_BUS_BLOCK_CHUNK_MAX) ? PICOFW_PIO_BUS_BLOCK_CHUNK_MAX : rem;
        if (s_panamon1_mode) {
            if (!picofw_bus_panamon1_read_chunk_open(
                    addr + done, &out_buf[done], chunk, allow_abort, &s_panamon1_mode
                )) {
                ok = false;
                break;
            }
            done += chunk;
            continue;
        }
        snprintf(req, sizeof(req), "m%08lX,%lX", (unsigned long)(addr + done), (unsigned long)chunk);
        const size_t flen = picofw_rsp_build_frame(req, frame, sizeof(frame));
        if ((flen == 0u) ||
            !pio_rsp_exchange_open(frame, flen, allow_abort, reply, sizeof(reply))) {
            if (s_panamon1_mode &&
                picofw_bus_panamon1_read_chunk_open(
                    addr + done, &out_buf[done], chunk, allow_abort, &s_panamon1_mode
                )) {
                done += chunk;
                continue;
            }
            ok = false;
            break;
        }
        /* Reject RSP error replies (`E01`, `E05`, etc.) or truncated hex payloads. */
        if ((reply[0] == 'E') && (strlen(reply) == 3u)) {
            ok = false;
            break;
        }
        size_t blen = 0u;
        if (!picofw_decode_hex_bytes(reply, &out_buf[done], chunk, &blen) || (blen != chunk)) {
            ok = false;
            break;
        }
        done += chunk;
    }

    picofw_pio_end_session();
    return ok;
}

static bool pio_read_mem32_checked(void *ctx, uint32_t addr, uint32_t *out_word) {
    uint8_t bytes[4] = {0};
    if (!pio_read_block_checked(ctx, addr & ~0x03UL, bytes, 4UL, true)) {
        if (out_word != NULL) {
            *out_word = PICOFW_BUS_RESP_OPEN_HIGH;
        }
        return false;
    }
    if (out_word != NULL) {
        *out_word = (uint32_t)bytes[0] |
                    ((uint32_t)bytes[1] << 8u) |
                    ((uint32_t)bytes[2] << 16u) |
                    ((uint32_t)bytes[3] << 24u);
    }
    return true;
}

static uint32_t pio_read_mem32(void *ctx, uint32_t addr) {
    uint32_t val = PICOFW_BUS_RESP_OPEN_HIGH;
    (void)pio_read_mem32_checked(ctx, addr, &val);
    return val;
}

static void pio_write_mem32(void *ctx, uint32_t addr, uint32_t data) {
    (void)ctx;
    const uint32_t aligned = addr & ~0x03UL;
    if (picofw_bus_check_access(
            picofw_session_default_ctx(),
            picofw_flash_get_active_size(),
            aligned,
            sizeof(uint32_t),
            true,
            false,
            picofw_flash_is_upper_nor_unlocked()) != PICOFW_BUS_ALLOW) {
        return;
    }
    if (!picofw_hw_is_nsw5v_high() || picofw_hw_is_nsw5v_fault_latched()) {
        return;
    }
    if (!s_panamon1_mode) {
        picofw_pio_begin_rx_session(PICOFW_BOOT_ACTION_PASSIVE);
        (void)picofw_bus_panamon1_probe_open(true, &s_panamon1_mode);
        picofw_pio_end_session();
    }
    if (s_panamon1_mode) {
        (void)picofw_bus_panamon1_write_u32(aligned, data, &s_panamon1_mode);
        return;
    }
    char frame[96];
    char reply[32];
    const size_t flen = picofw_rsp_build_write_u32(aligned, data, frame, sizeof(frame));
    if (flen == 0u) {
        return;
    }
    (void)(pio_rsp_transact_frame(frame, flen, reply, sizeof(reply)) && (strcmp(reply, "OK") == 0));
}

static bool pio_probe_target_mode(bool *out_halted) {
    if (out_halted != NULL) {
        *out_halted = false;
    }
    if (!picofw_hw_is_nsw5v_high() || picofw_hw_is_nsw5v_fault_latched()) {
        s_panamon1_mode = false;
        return false;
    }
    picofw_pio_begin_rx_session(PICOFW_BOOT_ACTION_PASSIVE);
    const bool panamon_ok = picofw_bus_panamon1_probe_open(true, &s_panamon1_mode);
    picofw_pio_end_session();
    if (panamon_ok) {
        if (out_halted != NULL) {
            *out_halted = true;
        }
        return true;
    }
    char reply[32];
    if (pio_rsp_transact("?", reply, sizeof(reply))) {
        if (out_halted != NULL) {
            *out_halted = (reply[0] == 'S') || (reply[0] == 'T');
        }
        return true;
    }
    if (s_panamon1_mode) {
        if (out_halted != NULL) {
            *out_halted = true;
        }
        return true;
    }
    return false;
}

static bool pio_is_online(void *ctx) {
    (void)ctx;
    return pio_probe_target_mode(NULL);
}

static bool pio_is_halted(void *ctx) {
    (void)ctx;
    bool halted = false;
    return pio_probe_target_mode(&halted) && halted;
}

static uint32_t pio_read_dcsr(void *ctx) {
    (void)ctx;
    bool halted = false;
    if (!pio_probe_target_mode(&halted)) {
        return PICOFW_BUS_RESP_OPEN_HIGH;
    }
    return halted ? 0x00000001UL : 0x00000000UL;
}

static void pio_set_pc(void *ctx, uint32_t pc_val) {
    (void)ctx;
    s_pending_pc = pc_val;
    s_has_pending_pc = true;
}

static void pio_resume_cpu(void *ctx) {
    (void)ctx;
    if (s_has_pending_pc) {
        char req[24];
        snprintf(req, sizeof(req), "c%08lX", (unsigned long)s_pending_pc);
        s_has_pending_pc = false;
        (void)pio_rsp_transact(req, NULL, 0u);
    } else {
        (void)pio_rsp_transact("c", NULL, 0u);
    }
}

void picofw_target_bus_init_pio(void) {
    s_has_pending_pc = false;
    s_panamon1_mode = false;
    static const picofw_target_bus_ops_t k_pio_bus_ops = {
        .read_mem32 = pio_read_mem32,
        .write_mem32 = pio_write_mem32,
        .is_online = pio_is_online,
        .is_halted = pio_is_halted,
        .read_dcsr = pio_read_dcsr,
        .set_pc = pio_set_pc,
        .resume_cpu = pio_resume_cpu,
        .ctx = NULL,
        .read_mem32_checked = pio_read_mem32_checked,
        .read_block_checked = pio_read_block_checked,
    };
    picofw_target_bus_register_ops(&k_pio_bus_ops);
}
