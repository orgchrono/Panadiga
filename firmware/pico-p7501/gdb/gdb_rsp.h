/**
 * @file gdb_rsp.h
 * @brief Public GDB Remote Serial Protocol (RSP) Server API for Panasonic AM34 / MN103S.
 *
 * Implements explicit context-state GDB remote protocol framing (`$packet#checksum`),
 * register access, memory read/write with hardware IPL air-gap, breakpoint management,
 * and single-step control over USB CDC serial on RP2040 / RP2350.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef GDB_RSP_H
#define GDB_RSP_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include "p7501_types.h"
#include "flash/p7501_fast_stream_codec.h"

#ifdef __cplusplus
extern "C" {
#endif

/** Maximum size of GDB RSP packet buffer. */
#define GDB_RSP_MAX_PACKET_LEN      2048U

/**
 * Bounded sector cache derived from SSOT (`512 B`) for both RP2040 and RP2350 so
 * cache refills stay < 30 ms (never tripping the 4000 ms WDT) and keep `.bss` lean.
 */
#define GDB_SECTOR_CACHE_SIZE       P7501_SSOT_GDB_SECTOR_CACHE_SIZE
#define GDB_TARGET_CHIP_LABEL       p7501_describe_gdb_target(P7501_IS_RP2350_BUILD != 0, P7501_IS_RISCV_BUILD != 0)

/** AM34 / MN103S register layout indices exposed to GDB. */
enum am34_gdb_reg {
    AM34_REG_D0  = 0,
    AM34_REG_D1  = 1,
    AM34_REG_D2  = 2,
    AM34_REG_D3  = 3,
    AM34_REG_A0  = 4,
    AM34_REG_A1  = 5,
    AM34_REG_A2  = 6,
    AM34_REG_A3  = 7,
    AM34_REG_MDR = 8,
    AM34_REG_PSW = 9,
    AM34_REG_PC  = 10,
    AM34_REG_SP  = 11,
    AM34_REG_COUNT = 12
};
typedef enum am34_gdb_reg am34_gdb_reg_t;

/** Framing parser FSM states for GDB RSP stream decoding. */
typedef enum gdb_parser_state {
    GDB_STATE_IDLE = 0,
    GDB_STATE_DATA,
    GDB_STATE_CHKSUM_MSB,
    GDB_STATE_CHKSUM_LSB
} gdb_parser_state_t;

/** Shadow register file maintaining current AM34 CPU state. */
typedef struct am34_reg_file {
    uint32_t d[4];
    uint32_t a[4];
    uint32_t mdr;
    uint32_t psw;
    uint32_t pc;
    uint32_t sp;
    bool is_halted;
} am34_reg_file_t;

/** Self-contained GDB RSP server context (zero file-scope state inside CPU/Memory submodules). */
typedef struct gdb_rsp_ctx {
    bool               active;
    gdb_parser_state_t parser_state;
    char               rx_buffer[GDB_RSP_MAX_PACKET_LEN];
    size_t             rx_len;
    bool               rx_escaped;
    uint8_t            calc_checksum;
    uint8_t            received_checksum;
    char               last_tx_packet[GDB_RSP_MAX_PACKET_LEN];
    char               tx_resp_buf[GDB_RSP_MAX_PACKET_LEN];
    am34_reg_file_t    regs;
    uint8_t            sector_cache[GDB_SECTOR_CACHE_SIZE];
    uint32_t           cache_base_addr;
    bool               cache_valid;
    uint32_t           flash_size_bytes;
    bool               upper_nor_unlocked;
#if defined(PICO_HOST_TEST) && (PICO_HOST_TEST != 0)
    char               test_last_resp[GDB_RSP_MAX_PACKET_LEN];
#endif
} gdb_rsp_ctx_t;

/* Explicit-context Functional Core / State-injected API */
void gdb_rsp_ctx_init(gdb_rsp_ctx_t *ctx);
bool gdb_rsp_ctx_is_active(const gdb_rsp_ctx_t *ctx);
void gdb_rsp_ctx_set_active(gdb_rsp_ctx_t *ctx, bool active);
void gdb_rsp_ctx_process_char(gdb_rsp_ctx_t *ctx, char ch);
void gdb_rsp_ctx_send_packet(gdb_rsp_ctx_t *ctx, const char *payload);
void gdb_rsp_ctx_invalidate_cache(gdb_rsp_ctx_t *ctx);
bool gdb_rsp_ctx_set_flash_size(gdb_rsp_ctx_t *ctx, uint32_t flash_size_bytes);
uint32_t gdb_rsp_ctx_get_flash_size(const gdb_rsp_ctx_t *ctx);
void gdb_rsp_ctx_set_upper_nor_unlocked(gdb_rsp_ctx_t *ctx, bool unlocked);
bool gdb_rsp_ctx_is_upper_nor_unlocked(const gdb_rsp_ctx_t *ctx);

#if defined(PICO_HOST_TEST) && (PICO_HOST_TEST != 0)
void gdb_rsp_ctx_test_inject_packet(gdb_rsp_ctx_t *ctx, const char *payload);
void gdb_rsp_ctx_test_inject_packet_len(gdb_rsp_ctx_t *ctx, const uint8_t *payload, size_t len);
void gdb_rsp_ctx_test_get_last_response(const gdb_rsp_ctx_t *ctx, char *out, size_t max_len);
void gdb_rsp_test_inject_packet(const char *payload);
void gdb_rsp_test_inject_packet_len(const uint8_t *payload, size_t len);
void gdb_rsp_test_get_last_response(char *out, size_t max_len);
#endif

/* Default global-singleton compatibility wrappers for CLI/main loop */
void gdb_rsp_init(void);
bool gdb_rsp_is_active(void);
void gdb_rsp_set_active(bool active);
void gdb_rsp_process_char(char ch);
void gdb_rsp_send_packet(const char *payload);
void gdb_rsp_invalidate_cache(void);
const char *gdb_rsp_get_status_info(void);
bool gdb_rsp_set_flash_size(uint32_t flash_size_bytes);
uint32_t gdb_rsp_get_flash_size(void);
void gdb_rsp_set_upper_nor_unlocked(bool unlocked);
bool gdb_rsp_is_upper_nor_unlocked(void);

#ifdef __cplusplus
}
#endif

#endif /* GDB_RSP_H */
