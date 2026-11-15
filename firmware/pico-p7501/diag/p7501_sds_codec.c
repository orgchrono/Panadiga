/**
 * @file p7501_sds_codec.c
 * @brief Pure ISO C11 passive SD 1-bit/4-bit bus sniffer decoder implementation.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/p7501_sds_codec.h"

#include "common/p7501_hex.h"

#include <string.h>

uint8_t p7501_sds_crc7_bytes(const uint8_t *data, size_t len) {
    uint8_t crc = 0u;
    for (size_t byte_i = 0u; byte_i < len; ++byte_i) {
        const uint8_t b = data[byte_i];
        for (int bit_i = 7; bit_i >= 0; --bit_i) {
            const uint8_t in_bit = (uint8_t)((b >> (uint8_t)bit_i) & 1u);
            const uint8_t feedback = (uint8_t)(in_bit ^ ((crc >> 6u) & 1u));
            crc = (uint8_t)((crc << 1u) & 0x7Fu);
            if (feedback != 0u) {
                crc ^= 0x09u;
            }
        }
    }
    return crc;
}

uint16_t p7501_sds_crc16_step(uint16_t crc, uint8_t bit) {
    const uint8_t msb = (uint8_t)((crc >> 15u) & 1u);
    crc = (uint16_t)(crc << 1u);
    if ((msb ^ (bit & 1u)) != 0u) {
        crc ^= 0x1021u;
    }
    return crc;
}

bool p7501_sds_parse_args(const char *arg, p7501_sds_spec_t *out_spec) {
    if (out_spec == NULL) {
        return false;
    }
    p7501_parse_window_boot_args(arg, 4000u, 200u, 60000u, &out_spec->boot);
    out_spec->default_block_bytes = 0u;

    const char *cur = arg;
    char tok[32];
    while ((cur = p7501_next_token(cur, tok, sizeof(tok))) != NULL) {
        char tok_upper[32];
        p7501_ascii_upper_copy(tok_upper, sizeof(tok_upper), tok);
        if (strncmp(tok_upper, "LEN:", 4u) == 0) {
            const uint32_t val = p7501_parse_uint32_clamped(tok + 4u, 64u, 64u, P7501_SDS_MAX_BLOCK);
            if (val != 64u && val != 512u) {
                return false;
            }
            out_spec->default_block_bytes = (uint16_t)val;
        }
    }
    return true;
}

void p7501_sds_init(p7501_sds_state_t *st, uint16_t default_block_bytes) {
    if (st == NULL) {
        return;
    }
    memset(st, 0, sizeof(*st));
    st->bus_width = 1u;
    st->default_block_bytes = default_block_bytes;
    st->next_block_bytes = (default_block_bytes > 0u) ? default_block_bytes : P7501_SDS_MAX_BLOCK;
}

static void sds_complete_cmd_frame(p7501_sds_state_t *st) {
    if (st->cmd_target_bits == 136u) {
        const bool end_ok = (st->cmd_buf[16] & 1u) != 0u;
        const uint8_t rx_crc7 = (uint8_t)(st->cmd_buf[16] >> 1u);
        const uint8_t exp_crc7 = p7501_sds_crc7_bytes(&st->cmd_buf[1], 15u);
        st->expect_r2_resp = false;
        if (end_ok && rx_crc7 == exp_crc7) {
            ++st->card_resps_ok;
        } else {
            ++st->cmd_crc_err;
        }
        return;
    }

    const bool dir_host = (st->cmd_buf[0] & 0x40u) != 0u;
    const uint8_t cmd_idx = (uint8_t)(st->cmd_buf[0] & 0x3Fu);
    const uint32_t arg =
        ((uint32_t)st->cmd_buf[1] << 24u) |
        ((uint32_t)st->cmd_buf[2] << 16u) |
        ((uint32_t)st->cmd_buf[3] << 8u) |
        (uint32_t)st->cmd_buf[4];
    const bool end_ok = (st->cmd_buf[5] & 1u) != 0u;
    const uint8_t rx_crc7 = (uint8_t)(st->cmd_buf[5] >> 1u);
    const bool is_r3_ocr = (!dir_host && cmd_idx == 0x3Fu && rx_crc7 == 0x7Fu);
    const bool crc_ok = end_ok && (is_r3_ocr || (rx_crc7 == p7501_sds_crc7_bytes(st->cmd_buf, 5u)));

    st->last_cmd_idx = cmd_idx;
    st->last_cmd_arg = arg;
    if (!crc_ok) {
        ++st->cmd_crc_err;
        return;
    }

    if (!dir_host) {
        ++st->card_resps_ok;
        st->expect_r2_resp = false;
        return;
    }

    ++st->host_cmds_ok;
    st->expect_r2_resp = (!st->in_app_cmd && (cmd_idx == 2u || cmd_idx == 9u || cmd_idx == 10u));
    if (cmd_idx == 55u) {
        st->in_app_cmd = true;
        st->saw_cmd55 = true;
        return;
    }

    const bool was_app = st->in_app_cmd;
    st->in_app_cmd = false;
    if (was_app && cmd_idx == 6u) {
        if ((arg & 3u) == 2u) {
            st->bus_width = 4u;
            st->saw_acmd6_4bit = true;
        } else if ((arg & 3u) == 0u) {
            st->bus_width = 1u;
        }
    } else if (was_app && cmd_idx == 13u) {
        st->saw_acmd13 = true;
        st->expecting_acmd13 = true;
        st->next_block_bytes = 64u;
    } else if (!was_app && (cmd_idx == 17u || cmd_idx == 18u || cmd_idx == 24u || cmd_idx == 25u)) {
        st->expecting_acmd13 = false;
        st->next_block_bytes = P7501_SDS_MAX_BLOCK;
    }
}

static void sds_step_cmd(p7501_sds_state_t *st, uint8_t cmd_bit) {
    if (st->cmd_bits == 0u) {
        if (cmd_bit == 0u) {
            memset(st->cmd_buf, 0, sizeof(st->cmd_buf));
            st->cmd_bits = 1u;
            st->cmd_target_bits = 48u;
        }
        return;
    }
    const size_t byte_i = (size_t)(st->cmd_bits >> 3u);
    const uint8_t bit_shift = (uint8_t)(7u - (st->cmd_bits & 7u));
    if (cmd_bit != 0u) {
        st->cmd_buf[byte_i] |= (uint8_t)(1u << bit_shift);
    }
    ++st->cmd_bits;
    if (st->cmd_bits == 8u && st->expect_r2_resp && st->cmd_buf[0] == 0x3Fu) {
        st->cmd_target_bits = 136u;
    }
    if (st->cmd_bits >= st->cmd_target_bits) {
        sds_complete_cmd_frame(st);
        st->cmd_bits = 0u;
    }
}

static void sds_finish_data_block(p7501_sds_state_t *st, uint8_t end_nibble) {
    const uint8_t active_lanes = (st->data_active_width == 4u) ? 4u : 1u;
    const uint8_t end_mask = (active_lanes == 4u) ? 0x0Fu : 0x01u;
    const bool end_ok = ((end_nibble & end_mask) == end_mask);
    uint8_t fail_mask = 0u;

    for (uint8_t lane = 0u; lane < active_lanes; ++lane) {
        if (st->lane_rx_crc[lane] != st->lane_calc_crc[lane]) {
            fail_mask |= (uint8_t)(1u << lane);
            ++st->lane_crc_err[lane];
        }
    }

    if (fail_mask == 0u && end_ok) {
        ++st->data_blocks_ok;
    } else {
        ++st->data_crc_err;
        st->last_fail_lane_mask = fail_mask;
    }

    if ((st->expecting_acmd13 || st->active_block_bytes == 64u) && st->active_block_bytes == 64u) {
        memcpy(st->acmd13_raw, st->data_buf, 64u);
        st->acmd13_captured = true;
        st->acmd13_crc_ok = (fail_mask == 0u && end_ok);
        st->acmd13_dat_bus_width = (uint8_t)((st->data_buf[0] >> 6u) & 0x03u);
        st->acmd13_sd_card_type = (uint16_t)(((uint16_t)st->data_buf[2] << 8u) | st->data_buf[3]);
        st->expecting_acmd13 = false;
        st->next_block_bytes = (st->default_block_bytes > 0u)
            ? st->default_block_bytes
            : P7501_SDS_MAX_BLOCK;
    }
    st->data_phase = P7501_SDS_DATA_IDLE;
}

static void sds_step_data(p7501_sds_state_t *st, uint8_t nibble) {
    const uint8_t active_lanes = (st->data_active_width == 4u) ? 4u : 1u;
    const uint16_t payload_clocks = (uint16_t)(
        (st->data_active_width == 4u)
            ? (st->active_block_bytes * 2u)
            : (st->active_block_bytes * 8u)
    );

    if (st->data_phase == P7501_SDS_DATA_IDLE) {
        if ((nibble & 1u) == 0u) {
            st->data_active_width = (st->bus_width == 4u) ? 4u : 1u;
            if (st->data_active_width == 4u && (nibble & 0x0Eu) != 0u) {
                st->start_bit_bad_mask |= (uint8_t)(nibble & 0x0Eu);
            }
            st->active_block_bytes = st->expecting_acmd13 ? 64u : st->next_block_bytes;
            if (st->active_block_bytes == 0u || st->active_block_bytes > P7501_SDS_MAX_BLOCK) {
                st->active_block_bytes = P7501_SDS_MAX_BLOCK;
            }
            memset(st->data_buf, 0, st->active_block_bytes);
            memset(st->lane_calc_crc, 0, sizeof(st->lane_calc_crc));
            memset(st->lane_rx_crc, 0, sizeof(st->lane_rx_crc));
            st->data_clock_pos = 0u;
            st->data_phase = P7501_SDS_DATA_PAYLOAD;
        }
        return;
    }

    if (st->data_phase == P7501_SDS_DATA_PAYLOAD) {
        if (st->data_active_width == 4u) {
            const size_t byte_i = (size_t)(st->data_clock_pos >> 1u);
            if ((st->data_clock_pos & 1u) == 0u) {
                st->data_buf[byte_i] = (uint8_t)(nibble << 4u);
            } else {
                st->data_buf[byte_i] |= nibble;
            }
        } else {
            const size_t byte_i = (size_t)(st->data_clock_pos >> 3u);
            const uint8_t bit_shift = (uint8_t)(7u - (st->data_clock_pos & 7u));
            if ((nibble & 1u) != 0u) {
                st->data_buf[byte_i] |= (uint8_t)(1u << bit_shift);
            }
        }
        for (uint8_t lane = 0u; lane < active_lanes; ++lane) {
            st->lane_calc_crc[lane] = p7501_sds_crc16_step(
                st->lane_calc_crc[lane],
                (uint8_t)((nibble >> lane) & 1u)
            );
        }
        ++st->data_clock_pos;
        if (st->data_clock_pos >= payload_clocks) {
            st->data_clock_pos = 0u;
            st->data_phase = P7501_SDS_DATA_CRC16;
        }
        return;
    }

    if (st->data_phase == P7501_SDS_DATA_CRC16) {
        for (uint8_t lane = 0u; lane < active_lanes; ++lane) {
            st->lane_rx_crc[lane] = (uint16_t)(
                (st->lane_rx_crc[lane] << 1u) | ((uint16_t)((nibble >> lane) & 1u))
            );
        }
        ++st->data_clock_pos;
        if (st->data_clock_pos >= 16u) {
            st->data_phase = P7501_SDS_DATA_END;
        }
        return;
    }

    sds_finish_data_block(st, nibble);
}

void p7501_sds_feed_sample(p7501_sds_state_t *st, uint8_t sample) {
    if (st == NULL) {
        return;
    }
    const uint8_t s = (uint8_t)(sample & 0x1Fu);
    ++st->total_clocks;
    if (st->has_prev) {
        const uint8_t diff = (uint8_t)(st->prev_sample ^ s);
        for (uint8_t i = 0u; i < 5u; ++i) {
            if (((diff >> i) & 1u) != 0u) {
                ++st->transitions[i];
            }
        }
    } else {
        st->has_prev = true;
    }
    st->prev_sample = s;

    sds_step_cmd(st, (uint8_t)(s & 1u));
    sds_step_data(st, (uint8_t)((s >> 1u) & 0x0Fu));
}

void p7501_sds_feed_samples(p7501_sds_state_t *st, const uint8_t *samples, size_t count) {
    if (st == NULL || samples == NULL) {
        return;
    }
    for (size_t i = 0u; i < count; ++i) {
        p7501_sds_feed_sample(st, samples[i]);
    }
}
