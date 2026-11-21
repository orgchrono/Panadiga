/**
 * @file gdb_rsp_cpu.c
 * @brief Explicit-context GDB RSP AM34 / MN103S register file, execution control, and query dispatcher.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "gdb/gdb_rsp_internal.h"
#include "flash/p7501_flash.h"
#include "hal/p7501_hw.h"
#include <string.h>

static uint32_t reg_file_get(const am34_reg_file_t *regs, size_t idx) {
    switch (idx) {
        case AM34_REG_D0:  return regs->d[0];
        case AM34_REG_D1:  return regs->d[1];
        case AM34_REG_D2:  return regs->d[2];
        case AM34_REG_D3:  return regs->d[3];
        case AM34_REG_A0:  return regs->a[0];
        case AM34_REG_A1:  return regs->a[1];
        case AM34_REG_A2:  return regs->a[2];
        case AM34_REG_A3:  return regs->a[3];
        case AM34_REG_MDR: return regs->mdr;
        case AM34_REG_PSW: return regs->psw;
        case AM34_REG_PC:  return regs->pc;
        case AM34_REG_SP:  return regs->sp;
        default:           return 0U;
    }
}

static void reg_file_set(am34_reg_file_t *regs, size_t idx, uint32_t val) {
    switch (idx) {
        case AM34_REG_D0:  regs->d[0] = val; break;
        case AM34_REG_D1:  regs->d[1] = val; break;
        case AM34_REG_D2:  regs->d[2] = val; break;
        case AM34_REG_D3:  regs->d[3] = val; break;
        case AM34_REG_A0:  regs->a[0] = val; break;
        case AM34_REG_A1:  regs->a[1] = val; break;
        case AM34_REG_A2:  regs->a[2] = val; break;
        case AM34_REG_A3:  regs->a[3] = val; break;
        case AM34_REG_MDR: regs->mdr = val; break;
        case AM34_REG_PSW: regs->psw = val; break;
        case AM34_REG_PC:  regs->pc = val; break;
        case AM34_REG_SP:  regs->sp = val; break;
        default:           break;
    }
}

void gdb_cpu_init_registers(am34_reg_file_t *regs) {
    if (regs == NULL) {
        return;
    }
    memset(regs, 0, sizeof(*regs));
    regs->pc = 0x80000000UL;
    regs->sp = 0x0003FFF0UL;
}

void gdb_cpu_sync_registers(am34_reg_file_t *regs) {
    if (regs == NULL) {
        return;
    }
    regs->is_halted = p7501_hw_is_cpu_halted();
    const uint32_t dcsr = p7501_hw_read_dcsr();
    if ((dcsr != P7501_BUS_RESP_OPEN_HIGH) && (dcsr != P7501_BUS_RESP_OPEN_LOW)) {
        regs->psw = (regs->psw & 0xFFFF0000UL) | (dcsr & 0x0000FFFFUL);
    }
    if (regs->pc == 0U) {
        regs->pc = 0x80000000UL;
    }
    if (regs->sp == 0U) {
        regs->sp = 0x0003FFF0UL;
    }
}

void gdb_cpu_handle_ctrl_c(gdb_rsp_ctx_t *ctx) {
    p7501_hw_sw_uart_set_low(false);
    if (ctx == NULL) {
        return;
    }
    ctx->regs.is_halted = true;
    gdb_rsp_ctx_send_packet(ctx, "S05");
}

static void handle_query_cmd(
    gdb_rsp_ctx_t *ctx,
    const char *cmd,
    char *resp_buf,
    size_t resp_cap
) {
    if (strncmp(cmd, "qSupported", 10U) == 0) {
        gdb_rsp_ctx_send_packet(ctx, "PacketSize=800;qXfer:memory-map:read+;qAttached+");
    } else if (strncmp(cmd, "qXfer:memory-map:read::", 23U) == 0) {
        gdb_mem_send_memory_map_xml(ctx, resp_buf, resp_cap);
    } else if (strcmp(cmd, "qAttached") == 0) {
        gdb_rsp_ctx_send_packet(ctx, "1");
    } else if (strcmp(cmd, "qOffsets") == 0) {
        gdb_rsp_ctx_send_packet(ctx, "Text=0;Data=0;Bss=0");
    } else {
        gdb_rsp_ctx_send_packet(ctx, "");
    }
}

static void handle_read_regs(gdb_rsp_ctx_t *ctx, char *resp_buf) {
    gdb_cpu_sync_registers(&ctx->regs);
    char *ptr = resp_buf;
    for (size_t i = 0U; i < AM34_REG_COUNT; i++) {
        const uint32_t val = reg_file_get(&ctx->regs, i);
        for (int b = 0; b < 4; b++) {
            const uint8_t byte = (uint8_t)((val >> (b * 8)) & 0xFFU);
            *ptr++ = gdb_nibble_to_hex_char((uint8_t)(byte >> 4));
            *ptr++ = gdb_nibble_to_hex_char((uint8_t)(byte & 0x0FU));
        }
    }
    *ptr = '\0';
    gdb_rsp_ctx_send_packet(ctx, resp_buf);
}

static void handle_write_regs(gdb_rsp_ctx_t *ctx, const char *hex_str) {
    if (strlen(hex_str) < (AM34_REG_COUNT * 8U)) {
        gdb_rsp_ctx_send_packet(ctx, "E01");
        return;
    }
    for (size_t i = 0U; i < AM34_REG_COUNT; i++) {
        uint32_t word = 0U;
        for (int b = 0; b < 4; b++) {
            const int h = gdb_hex_char_to_nibble(*hex_str++);
            const int l = gdb_hex_char_to_nibble(*hex_str++);
            if ((h >= 0) && (l >= 0)) {
                const uint8_t byte = (uint8_t)((h << 4) | l);
                word |= ((uint32_t)byte) << (b * 8);
            }
        }
        reg_file_set(&ctx->regs, i, word);
    }
    p7501_hw_set_pc(ctx->regs.pc);
    gdb_rsp_ctx_send_packet(ctx, "OK");
}

static bool apply_optional_pc_arg(gdb_rsp_ctx_t *ctx, const char *arg) {
    if (*arg == '\0') {
        return true;
    }
    uint32_t target_addr = 0U;
    if (gdb_parse_hex_u32(arg, &target_addr, '\0') == NULL) {
        return false;
    }
    ctx->regs.pc = target_addr;
    p7501_hw_set_pc(target_addr);
    return true;
}

void gdb_cpu_dispatch_command(gdb_rsp_ctx_t *ctx, const char *cmd) {
    if ((ctx == NULL) || (cmd == NULL)) {
        return;
    }
    char *resp_buf = ctx->tx_resp_buf;
    const size_t resp_cap = sizeof(ctx->tx_resp_buf);
    resp_buf[0] = '\0';

    switch (cmd[0]) {
        case '?':
            if (p7501_hw_read_dcsr() == P7501_BUS_RESP_OPEN_HIGH) {
                gdb_rsp_ctx_send_packet(ctx, "S00");
            } else {
                gdb_cpu_sync_registers(&ctx->regs);
                gdb_rsp_ctx_send_packet(ctx, ctx->regs.is_halted ? "S05" : "S00");
            }
            break;

        case 'q':
            handle_query_cmd(ctx, cmd, resp_buf, resp_cap);
            break;

        case 'g':
            handle_read_regs(ctx, resp_buf);
            break;

        case 'G':
            if (p7501_hw_read_dcsr() == P7501_BUS_RESP_OPEN_HIGH) {
                gdb_rsp_ctx_send_packet(ctx, "E01");
            } else {
                handle_write_regs(ctx, &cmd[1]);
            }
            break;

        case 'm':
            gdb_mem_handle_read_cmd(ctx, &cmd[1], resp_buf);
            break;

        case 'M':
            gdb_mem_handle_write_cmd(ctx, &cmd[1]);
            break;

        case 'X': {
            const size_t raw_len = ctx->rx_len;
            gdb_mem_handle_write_bin_cmd(ctx, &cmd[1], (raw_len > 1U) ? (raw_len - 1U) : 0U);
            break;
        }

        case 'c':
            if (p7501_hw_read_dcsr() == P7501_BUS_RESP_OPEN_HIGH ||
                !apply_optional_pc_arg(ctx, &cmd[1])) {
                gdb_rsp_ctx_send_packet(ctx, "E01");
                break;
            }
            p7501_hw_resume_cpu();
            ctx->regs.is_halted = false;
            gdb_rsp_ctx_send_packet(ctx, "S05");
            break;

        case 's':
            if (p7501_hw_read_dcsr() == P7501_BUS_RESP_OPEN_HIGH ||
                !apply_optional_pc_arg(ctx, &cmd[1])) {
                gdb_rsp_ctx_send_packet(ctx, "E01");
                break;
            }
            ctx->regs.pc += 2U;
            p7501_hw_set_pc(ctx->regs.pc);
            gdb_rsp_ctx_send_packet(ctx, "S05");
            break;

        case 'k':
            gdb_rsp_ctx_set_upper_nor_unlocked(ctx, false);
            p7501_hw_reset_assert();
            sleep_ms(P7501_RESET_ASSERT_MS);
            p7501_hw_reset_release();
            gdb_rsp_ctx_invalidate_cache(ctx);
            gdb_rsp_ctx_send_packet(ctx, "OK");
            break;

        case 'D':
            gdb_rsp_ctx_set_upper_nor_unlocked(ctx, false);
            gdb_rsp_ctx_send_packet(ctx, "OK");
            ctx->active = false;
            break;

        case 'R':
            gdb_rsp_ctx_set_upper_nor_unlocked(ctx, false);
            p7501_hw_boot_sequence(P7501_BOOT_MODE_NORMAL_HIZ);
            gdb_rsp_ctx_invalidate_cache(ctx);
            ctx->regs.pc = P7501_FLASH_CACHED_BASE;
            gdb_rsp_ctx_send_packet(ctx, "OK");
            break;

        case 'Z':
        case 'z':
            gdb_rsp_ctx_send_packet(ctx, ((cmd[1] == '0') || (cmd[1] == '1')) ? "OK" : "");
            break;

        default:
            gdb_rsp_ctx_send_packet(ctx, "");
            break;
    }
}
