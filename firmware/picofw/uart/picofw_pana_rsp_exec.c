/**
 * @file picofw_pana_rsp_exec.c
 * @brief Stub upload, `DUMPNOR`, JEDEC autoselect/CFI/checksum, and BGO bitclear/erase executors for `PANARSP`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "uart/picofw_pana_rsp_exec.h"
#include "cli/picofw_cli.h"
#include "common/picofw_hex.h"
#include "flash/picofw_flash.h"
#include "picofw_types.h"
#include "uart/picofw_pana_dumpnor.h"
#include "uart/picofw_pana_jedec.h"
#include "uart/picofw_uart_pio_engine.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static picofw_dumpnor_resume_state_t s_dumpnor_resume = {0};
static uint8_t s_rsp_stub_buf[PICOFW_JEDEC_MAX_STUB_SIZE];

size_t panarsp_tx_sequence(
    panarsp_rx_cap_t *cap,
    const char *const *payloads,
    size_t count,
    uint32_t per_step_ms
) {
    size_t first_off = cap->len;
    for (size_t i = 0u; i < count; ++i) {
        const size_t step_off = cap->len;
        if (i == 0u) {
            first_off = step_off;
        }
        (void)panarsp_tx_frame(payloads[i]);
        panarsp_drain_for_ms(cap, per_step_ms, true, step_off);
    }
    return first_off;
}

static bool panarsp_verify_bgo_bitclear_live(
    panarsp_rx_cap_t *cap,
    const picofw_pana_rsp_spec_t *spec,
    const char **out_err_code
) {
    char cmd_payload[32];
    const int n = snprintf(
        cmd_payload,
        sizeof(cmd_payload),
        "m%08lX,%X",
        (unsigned long)spec->addr,
        (unsigned)spec->data_len
    );
    if ((n <= 0) || ((size_t)n >= sizeof(cmd_payload))) {
        *out_err_code = "EINVAL";
        return false;
    }
    const size_t pre_off = cap->len;
    (void)panarsp_tx_frame(cmd_payload);
    (void)panarsp_drain_for_ms(cap, 400u, true, pre_off);

    picofw_rsp_frame_info_t info;
    if (!picofw_rsp_inspect_frame_from(cap->buf, cap->len, pre_off, &info) || !info.checksum_ok) {
        printf("[PANARSP] ERROR EIO: BGO_BITCLEAR pre-read @ 0x%08lX failed (no valid RSP reply)\n",
               (unsigned long)spec->addr);
        *out_err_code = "EIO";
        return false;
    }
    char hex_buf[(PICOFW_JEDEC_MAX_DATA_LEN * 2u) + 1u];
    if ((info.payload_len != (spec->data_len * 2u)) || (info.payload_len >= sizeof(hex_buf))) {
        printf("[PANARSP] ERROR EIO: BGO_BITCLEAR pre-read length mismatch (%u vs %u hex)\n",
               (unsigned)info.payload_len, (unsigned)(spec->data_len * 2u));
        *out_err_code = "EIO";
        return false;
    }
    memcpy(hex_buf, &cap->buf[info.payload_offset], info.payload_len);
    hex_buf[info.payload_len] = '\0';

    uint8_t old_bytes[PICOFW_JEDEC_MAX_DATA_LEN];
    size_t decoded_len = 0u;
    if (!picofw_decode_hex_bytes(hex_buf, old_bytes, sizeof(old_bytes), &decoded_len) ||
        (decoded_len != spec->data_len)) {
        printf("[PANARSP] ERROR EIO: BGO_BITCLEAR pre-read returned invalid hex\n");
        *out_err_code = "EIO";
        return false;
    }
    if (!picofw_jedec_validate_bitclear(old_bytes, spec->data_bytes, spec->data_len)) {
        printf("[PANARSP] ERROR E05: BGO_BITCLEAR rejected 0->1 bit transition @ 0x%08lX (requires sector erase)\n",
               (unsigned long)spec->addr);
        *out_err_code = "E05";
        return false;
    }
    return true;
}

static void dumpnor_ff_tracker_cb(void *ctx, uint8_t b) {
    picofw_dumpnor_ff_tracker_feed((picofw_dumpnor_ff_tracker_t *)ctx, b);
}

size_t panarsp_execute_dumpnor(
    panarsp_rx_cap_t *cap,
    const picofw_pana_rsp_spec_t *spec,
    bool *out_hdr_seen,
    bool *out_ftr_seen,
    uint32_t *out_stream_bytes,
    uint32_t *out_ff_blocks
) {
    if (out_ff_blocks != NULL) {
        *out_ff_blocks = 0u;
    }
    uint32_t addr = spec->addr;
    uint32_t len = spec->value_u32;
    if (spec->action == PANA_RSP_ACT_DUMPNOR_RESUME) {
        if (!picofw_dumpnor_resume_next_slice(&s_dumpnor_resume, &addr, &len)) {
            picofw_cli_set_result("ERR", "EINVAL");
            printf("[PANARSP] ERROR: No active DUMPNOR resume checkpoint.\n");
            return cap->len;
        }
    } else if (spec->action == PANA_RSP_ACT_FAST_DUMPNOR) {
        if ((spec->baud != 0u) && (spec->baud != PICOFW_SSOT_DEFAULT_BAUD)) {
            picofw_cli_set_result("ERR", "EBAUD");
            printf("[PANARSP] ERROR EBAUD: AM34 SC0 BRG baud switch not implemented in stub; use 115200\n");
            return cap->len;
        }
        (void)picofw_dumpnor_resume_begin(&s_dumpnor_resume, addr, len, spec->baud);
    } else {
        (void)picofw_dumpnor_resume_begin(&s_dumpnor_resume, addr, len, picofw_pio_get_baud());
    }

    const size_t stub_len = picofw_dumpnor_build_stub(
        addr, len, PICOFW_DUMPNOR_STUB_ADDR, s_rsp_stub_buf, PICOFW_DUMPNOR_STUB_SIZE
    );
    if (stub_len == 0u) {
        picofw_cli_set_result("ERR", "EINVAL");
        return cap->len;
    }
    const uint32_t blk_sz = picofw_dumpnor_block_size(len);
    const uint32_t nblk = (blk_sz > 0u) ? (len / blk_sz) : 0u;
    /* 12-byte metadata + raw payload + 4-byte LE sum per block + 8-byte "PANADONE". */
    const uint32_t min_post_hdr = 12u + len + (nblk * 4u) + 8u;
    uint32_t post_hdr_bytes = 0u;
    picofw_dumpnor_ff_tracker_t ff_tracker;
    picofw_dumpnor_ff_tracker_init(&ff_tracker, blk_sz);
    printf("[PANARSP] Uploading %uB PANADMP1 stub @ %" PRIu32 "bps -> 0x%08lX (range 0x%08lX+0x%08lX)...\n",
           (unsigned)stub_len, picofw_pio_get_baud(), (unsigned long)PICOFW_DUMPNOR_STUB_ADDR,
           (unsigned long)addr, (unsigned long)len);
    const size_t ret = panarsp_stream_until_tokens_ex(
        cap, PICOFW_DUMPNOR_STUB_ADDR, s_rsp_stub_buf, stub_len, spec->timeout_ms,
        "PANADMP1", "PANADONE", NULL, min_post_hdr, true,
        dumpnor_ff_tracker_cb, &ff_tracker,
        out_hdr_seen, out_ftr_seen, NULL, out_stream_bytes, &post_hdr_bytes
    );
    if (out_ff_blocks != NULL) {
        *out_ff_blocks = ff_tracker.ff_blocks;
    }
    if (*out_ftr_seen) {
        picofw_dumpnor_resume_advance(&s_dumpnor_resume, len);
    } else if (*out_hdr_seen && (blk_sz > 0u) && (post_hdr_bytes > 12u)) {
        const uint32_t stride = blk_sz + 4u;
        const uint32_t full_blocks = (post_hdr_bytes - 12u) / stride;
        /* Retain 1-block safety overlap when >=2 blocks arrived so an interrupted boundary block is re-read cleanly. */
        const uint32_t safe_blocks = (full_blocks > 1u) ? (full_blocks - 1u) : full_blocks;
        if (safe_blocks > 0u) {
            picofw_dumpnor_resume_advance(&s_dumpnor_resume, safe_blocks * blk_sz);
        }
    }
    return ret;
}

size_t panarsp_execute_jedec_id_or_sum(
    panarsp_rx_cap_t *cap,
    const picofw_pana_rsp_spec_t *spec,
    bool *out_ok,
    uint32_t *out_bytes
) {
    const bool is_id = (spec->action == PANA_RSP_ACT_JEDEC_ID) || (spec->action == PANA_RSP_ACT_CFI_QUERY);
    const bool is_cfi = (spec->action == PANA_RSP_ACT_CFI_QUERY);
    const size_t stub_len = is_id
        ? picofw_jedec_build_autoselect_stub(is_cfi, PICOFW_JEDEC_STUB_ADDR, s_rsp_stub_buf, sizeof(s_rsp_stub_buf))
        : picofw_dumpnor_build_checksum_stub(
              spec->addr, spec->value_u32, PICOFW_DUMPNOR_STUB_ADDR, s_rsp_stub_buf, sizeof(s_rsp_stub_buf)
          );
    if (stub_len == 0u) {
        return cap->len;
    }
    const char *hdr = is_id
        ? (is_cfi ? PICOFW_CFI_QUERY_MAGIC : PICOFW_JEDEC_ID_MAGIC)
        : PICOFW_DUMPNOR_SUM_MAGIC;
    uint32_t min_post_hdr = 0u;
    if (!is_id) {
        const uint32_t blk_sz = picofw_dumpnor_block_size(spec->value_u32);
        const uint32_t nblk = (blk_sz > 0u) ? (spec->value_u32 / blk_sz) : 0u;
        min_post_hdr = 12u + (nblk * 4u) + 8u;
    }
    bool hdr_seen = false;
    const size_t off = panarsp_stream_until_tokens_ex(
        cap, PICOFW_JEDEC_STUB_ADDR, s_rsp_stub_buf, stub_len, spec->timeout_ms,
        hdr, "PANADONE", NULL, min_post_hdr, !is_id, NULL, NULL,
        &hdr_seen, out_ok, NULL, out_bytes, NULL
    );
    if (is_id) {
        picofw_jedec_id_reply_t rep;
        if (picofw_jedec_parse_autoselect_reply(&cap->buf[off], cap->len - off, is_cfi, &rep)) {
            if (is_cfi) {
                printf("[CFI_QUERY] QRY=%c%c%c size=%lu bytes\n",
                       rep.qry[0], rep.qry[1], rep.qry[2], (unsigned long)rep.size_bytes);
            } else {
                const bool match = picofw_jedec_verify_chip_match(
                    rep.mfg, rep.dev, picofw_flash_get_active_size()
                );
                picofw_session_default_ctx()->jedec_mfg = rep.mfg;
                memcpy(picofw_session_default_ctx()->jedec_dev, rep.dev, sizeof(rep.dev));
                picofw_session_default_ctx()->jedec_verified = match;
                printf("[JEDEC_ID] mfg=0x%04X dev=[0x%04X,0x%04X,0x%04X] size=%lu profile_match=%s\n",
                       rep.mfg, rep.dev[0], rep.dev[1], rep.dev[2],
                       (unsigned long)rep.size_bytes, match ? "YES" : "NO");
            }
        }
    } else {
        const uint32_t blk_sz = picofw_dumpnor_block_size(spec->value_u32);
        const uint32_t nblk = (blk_sz > 0u) ? (spec->value_u32 / blk_sz) : 0u;
        picofw_nor_check_result_t chk;
        if (picofw_dumpnor_eval_sum_only_reply(&cap->buf[off], cap->len - off, blk_sz, nblk, &chk)) {
            printf("[NOR_CHECK] blocks=%lu blank=%lu all_blank=%s digest_crc32=0x%08lX\n",
                   (unsigned long)chk.blocks_checked, (unsigned long)chk.blank_blocks,
                   chk.all_blank ? "YES" : "NO", (unsigned long)chk.digest_crc32);
        }
    }
    return off;
}

size_t panarsp_execute_jedec(
    panarsp_rx_cap_t *cap,
    const picofw_pana_rsp_spec_t *spec,
    bool *out_ok_seen,
    bool *out_to_seen,
    uint32_t *out_stream_bytes,
    const char **out_err_code
) {
    if (spec->action == PANA_RSP_ACT_BGO_BITCLEAR) {
        if (!panarsp_verify_bgo_bitclear_live(cap, spec, out_err_code)) {
            *out_ok_seen = false;
            *out_to_seen = false;
            *out_stream_bytes = 0u;
            return cap->len;
        }
    }
    const size_t stub_len = (spec->action == PANA_RSP_ACT_JEDEC_ERASE)
        ? picofw_jedec_build_erase_stub(spec->addr, PICOFW_JEDEC_STUB_ADDR, s_rsp_stub_buf, sizeof(s_rsp_stub_buf))
        : picofw_jedec_build_prog_stub(
              spec->addr, spec->data_bytes, spec->data_len, PICOFW_JEDEC_STUB_ADDR,
              s_rsp_stub_buf, sizeof(s_rsp_stub_buf)
          );
    if (stub_len == 0u) {
        *out_err_code = "E05";
        return cap->len;
    }
    printf("[PANARSP] Uploading %uB JEDEC %s stub -> 0x%08lX (target 0x%08lX)...\n",
           (unsigned)stub_len,
           (spec->action == PANA_RSP_ACT_JEDEC_ERASE) ? "ERASE" : "PROG",
           (unsigned long)PICOFW_JEDEC_STUB_ADDR, (unsigned long)spec->addr);
    bool hdr_unused = false;
    return panarsp_stream_until_tokens(
        cap, PICOFW_JEDEC_STUB_ADDR, s_rsp_stub_buf, stub_len, spec->timeout_ms,
        NULL, PICOFW_JEDEC_OK_TOKEN, PICOFW_JEDEC_TO_TOKEN,
        &hdr_unused, out_ok_seen, out_to_seen, out_stream_bytes
    );
}
