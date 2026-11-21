/**
 * @file p7501_target_bus.c
 * @brief Target debug/memory transport VTable dispatcher with fail-closed safety.
 *
 * Host-testable with zero Pico SDK dependencies (`C11_HOST_SOURCES`).
 * Enforces permanent read/write isolation over the lethal AM34 register
 * (`0xC000100C..0xC000100F`) and checked error propagation (`read_mem32_checked`,
 * `read_block_checked`) so offline buses can never masquerade as `0xFFFFFFFF`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "hal/p7501_hw.h"
#include "flash/p7501_flash.h"
#include <stddef.h>
#include <string.h>

static uint32_t default_read_mem32(void *ctx, uint32_t addr) {
    (void)ctx;
    (void)addr;
    return P7501_BUS_RESP_OPEN_HIGH;
}

static void default_write_mem32(void *ctx, uint32_t addr, uint32_t data) {
    (void)ctx;
    (void)addr;
    (void)data;
}

static bool default_is_false(void *ctx) {
    (void)ctx;
    return false;
}

static uint32_t default_read_dcsr(void *ctx) {
    (void)ctx;
    return P7501_BUS_RESP_OPEN_HIGH;
}

static void default_set_pc(void *ctx, uint32_t pc_val) {
    (void)ctx;
    (void)pc_val;
}

static void default_resume_cpu(void *ctx) {
    (void)ctx;
}

static bool default_read_mem32_checked(void *ctx, uint32_t addr, uint32_t *out_word) {
    (void)ctx;
    (void)addr;
    if (out_word != NULL) {
        *out_word = P7501_BUS_RESP_OPEN_HIGH;
    }
    return false;
}

#define P7501_DEFAULT_OFFLINE_OPS_INIT         \
    {                                          \
        .read_mem32 = default_read_mem32,      \
        .write_mem32 = default_write_mem32,    \
        .is_online = default_is_false,         \
        .is_halted = default_is_false,         \
        .read_dcsr = default_read_dcsr,        \
        .set_pc = default_set_pc,              \
        .resume_cpu = default_resume_cpu,      \
        .ctx = NULL,                           \
        .read_mem32_checked = default_read_mem32_checked, \
        .read_block_checked = NULL,            \
    }

static const p7501_target_bus_ops_t k_default_offline_ops = P7501_DEFAULT_OFFLINE_OPS_INIT;

static p7501_target_bus_ops_t s_bus_ops = P7501_DEFAULT_OFFLINE_OPS_INIT;

void p7501_target_bus_register_ops(const p7501_target_bus_ops_t *ops) {
    if (ops == NULL) {
        s_bus_ops = k_default_offline_ops;
        return;
    }
    s_bus_ops.read_mem32 = (ops->read_mem32 != NULL) ? ops->read_mem32 : default_read_mem32;
    s_bus_ops.write_mem32 = (ops->write_mem32 != NULL) ? ops->write_mem32 : default_write_mem32;
    s_bus_ops.is_online = (ops->is_online != NULL) ? ops->is_online : default_is_false;
    s_bus_ops.is_halted = (ops->is_halted != NULL) ? ops->is_halted : default_is_false;
    s_bus_ops.read_dcsr = (ops->read_dcsr != NULL) ? ops->read_dcsr : default_read_dcsr;
    s_bus_ops.set_pc = (ops->set_pc != NULL) ? ops->set_pc : default_set_pc;
    s_bus_ops.resume_cpu = (ops->resume_cpu != NULL) ? ops->resume_cpu : default_resume_cpu;
    s_bus_ops.ctx = ops->ctx;
    if (ops->read_mem32_checked != NULL) {
        s_bus_ops.read_mem32_checked = ops->read_mem32_checked;
    } else if (ops->read_mem32 != NULL) {
        /* Legacy mock VTable providing only `read_mem32`: checked reads wrap it cleanly. */
        s_bus_ops.read_mem32_checked = NULL;
    } else {
        s_bus_ops.read_mem32_checked = default_read_mem32_checked;
    }
    s_bus_ops.read_block_checked = ops->read_block_checked;
}

void p7501_target_bus_reset_default_ops(void) {
    s_bus_ops = k_default_offline_ops;
}

bool p7501_hw_read_mem32_checked(uint32_t addr, uint32_t *out_word) {
    if (out_word == NULL) {
        return false;
    }
    if (p7501_bus_is_lethal_mmio_range(addr, sizeof(uint32_t))) {
        *out_word = P7501_BUS_RESP_OPEN_HIGH;
        return false;
    }
    if (s_bus_ops.read_mem32_checked != NULL) {
        return s_bus_ops.read_mem32_checked(s_bus_ops.ctx, addr, out_word);
    }
    *out_word = s_bus_ops.read_mem32(s_bus_ops.ctx, addr);
    return true;
}

bool p7501_hw_read_block_checked(uint32_t addr, uint8_t *out_buf, uint32_t len, bool allow_abort) {
    if ((out_buf == NULL) || (len == 0UL)) {
        return false;
    }
    if (p7501_bus_is_lethal_mmio_range(addr, (size_t)len)) {
        return false;
    }
    if (s_bus_ops.read_block_checked != NULL) {
        return s_bus_ops.read_block_checked(s_bus_ops.ctx, addr, out_buf, len, allow_abort);
    }
    /* Word-wise fallback using checked reads (used by host mock bus). */
    uint32_t done = 0UL;
    while (done < len) {
        const uint32_t cur_addr = addr + done;
        const uint32_t aligned = cur_addr & ~0x03UL;
        const uint32_t byte_off = cur_addr & 0x03UL;
        uint32_t word = 0UL;
        if (!p7501_hw_read_mem32_checked(aligned, &word)) {
            return false;
        }
        const uint32_t avail = 4UL - byte_off;
        const uint32_t rem = len - done;
        const uint32_t take = (avail < rem) ? avail : rem;
        for (uint32_t i = 0UL; i < take; ++i) {
            out_buf[done + i] = (uint8_t)((word >> ((byte_off + i) * 8UL)) & 0xFFUL);
        }
        done += take;
    }
    return true;
}

uint32_t p7501_hw_read_mem32(uint32_t addr) {
    uint32_t val = P7501_BUS_RESP_OPEN_HIGH;
    (void)p7501_hw_read_mem32_checked(addr, &val);
    return val;
}

void p7501_hw_write_mem32(uint32_t addr, uint32_t data) {
    if (p7501_bus_is_lethal_mmio_range(addr, sizeof(uint32_t))) {
        return;
    }
    if (p7501_flash_is_critical_boot_ipl_range(addr, sizeof(uint32_t))) {
        return;
    }
    s_bus_ops.write_mem32(s_bus_ops.ctx, addr, data);
}

bool p7501_hw_is_target_online(void) {
    return s_bus_ops.is_online(s_bus_ops.ctx);
}

bool p7501_hw_is_cpu_halted(void) {
    return s_bus_ops.is_halted(s_bus_ops.ctx);
}

uint32_t p7501_hw_read_dcsr(void) {
    return s_bus_ops.read_dcsr(s_bus_ops.ctx);
}

void p7501_hw_set_pc(uint32_t pc_val) {
    s_bus_ops.set_pc(s_bus_ops.ctx, pc_val);
}

void p7501_hw_resume_cpu(void) {
    s_bus_ops.resume_cpu(s_bus_ops.ctx);
}
