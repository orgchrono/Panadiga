/**
 * @file picofw_crypto_cli.c
 * @brief CLI handlers `CRYPTO` and `DECOMP` pushing verified results via `@PICO_FRAME`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "crypto/picofw_crypto_cli.h"
#include "cli/picofw_cli.h"
#include "common/picofw_cmd_arena.h"
#include "common/picofw_hex.h"
#include "common/picofw_push_stream.h"
#include "crypto/picofw_decomp.h"
#include "crypto/picofw_feistel.h"
#include "flash/picofw_fast_stream_codec.h"
#include "generated/picofw_ssot.h"
#include "uart/picofw_uart_codec.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static bool resolve_feistel_key_tok(const char *tok, uint8_t rk32[32], char *label, size_t label_cap) {
    uint32_t idx = 0u;
    if (picofw_ascii_is_digits(tok) && picofw_parse_u32_strict(tok, &idx) &&
        (idx < PICOFW_SSOT_FEISTEL_KEY_COUNT)) {
        picofw_feistel_derive_rk_8b(k_picofw_ssot_feistel_keys[idx].key8, rk32);
        (void)snprintf(label, label_cap, "%s", k_picofw_ssot_feistel_keys[idx].name);
        return true;
    }
    for (size_t k = 0u; k < PICOFW_SSOT_FEISTEL_KEY_COUNT; ++k) {
        if (picofw_ascii_ieq(tok, k_picofw_ssot_feistel_keys[k].name)) {
            picofw_feistel_derive_rk_8b(k_picofw_ssot_feistel_keys[k].key8, rk32);
            (void)snprintf(label, label_cap, "%s", k_picofw_ssot_feistel_keys[k].name);
            return true;
        }
    }
    uint8_t raw8[8];
    size_t blen = 0u;
    if (picofw_decode_hex_bytes(tok, raw8, sizeof(raw8), &blen) && (blen == 8u)) {
        picofw_feistel_derive_rk_8b(raw8, rk32);
        (void)snprintf(label, label_cap, "custom");
        return true;
    }
    return false;
}

static bool crypto_run_selftest(void) {
    const bool ok = picofw_feistel_selftest();
    char msg[96];
    (void)snprintf(msg, sizeof(msg), "selftest=%s keys=%u", ok ? "PASS" : "FAIL",
                   (unsigned)PICOFW_SSOT_FEISTEL_KEY_COUNT);
    (void)picofw_push_emit_str("CRYPTO", msg);
    (void)picofw_push_flush_global();
    return ok;
}

void picofw_cmd_crypto(const char *arg) {
    char op[20];
    const char *rest = picofw_next_token(arg, op, sizeof(op));
    if ((op[0] == '\0') || picofw_ascii_ieq(op, "SELFTEST")) {
        if (!crypto_run_selftest()) {
            picofw_cli_set_result("ERR", "EIO");
            printf("CRYPTO: ERROR (selftest failed)\n");
            return;
        }
        printf("CRYPTO: OK\n");
        return;
    }
    if (picofw_ascii_ieq(op, "KEYS")) {
        for (size_t i = 0u; i < PICOFW_SSOT_FEISTEL_KEY_COUNT; ++i) {
            char hex_k[20];
            (void)picofw_fast_format_page_hex(k_picofw_ssot_feistel_keys[i].key8, 8u, hex_k, sizeof(hex_k));
            char line[64];
            (void)snprintf(line, sizeof(line), "idx=%u name=%s key=%s",
                           (unsigned)i, k_picofw_ssot_feistel_keys[i].name, hex_k);
            (void)picofw_push_emit_str("CRYPTO_KEY", line);
        }
        (void)picofw_push_flush_global();
        printf("CRYPTO: OK\n");
        return;
    }
    if (picofw_ascii_ieq(op, "AUTODETECT")) {
        char tok_hex[32];
        (void)picofw_next_token(rest, tok_hex, sizeof(tok_hex));
        uint8_t blk[8];
        uint8_t plain[8];
        size_t blen = 0u;
        size_t kidx = 0u;
        if (!picofw_decode_hex_bytes(tok_hex, blk, sizeof(blk), &blen) || (blen != 8u)) {
            picofw_cli_set_result("ERR", "EINVAL");
            printf("CRYPTO: ERROR (Usage: CRYPTO AUTODETECT <16_hex_chars>)\n");
            return;
        }
        if (!picofw_feistel_autodetect_key(blk, plain, &kidx)) {
            picofw_cli_set_result("ERR", "ENOENT");
            printf("CRYPTO: ERROR (No matching SSOT Feistel key)\n");
            return;
        }
        char phex[20];
        (void)picofw_fast_format_page_hex(plain, 8u, phex, sizeof(phex));
        char msg[80];
        (void)snprintf(msg, sizeof(msg), "key_idx=%u name=%s plain=%s",
                       (unsigned)kidx, k_picofw_ssot_feistel_keys[kidx].name, phex);
        (void)picofw_push_emit_str("CRYPTO", msg);
        (void)picofw_push_flush_global();
        printf("CRYPTO: OK\n");
        return;
    }
    if (picofw_ascii_ieq(op, "ADLER32")) {
        char tok_stride[16];
        const char *hex_part = picofw_next_token(rest, tok_stride, sizeof(tok_stride));
        uint32_t stride = 1u;
        uint8_t *const buf = g_picofw_cmd_arena.raw;
        size_t blen = 0u;
        if (!picofw_parse_u32_auto(tok_stride, &stride) || (stride == 0u) ||
            !picofw_decode_hex_bytes(hex_part, buf, 1024u, &blen) || (blen == 0u)) {
            picofw_cli_set_result("ERR", "EINVAL");
            printf("CRYPTO: ERROR (Usage: CRYPTO ADLER32 <stride> <hex>)\n");
            return;
        }
        const uint32_t pana = picofw_adler32_strided(buf, blen, stride);
        const uint32_t rfc = picofw_adler32_rfc1950(buf, blen);
        char msg[80];
        (void)snprintf(msg, sizeof(msg), "len=%u pana_strided=0x%08" PRIX32 " rfc1950=0x%08" PRIX32,
                       (unsigned)blen, pana, rfc);
        (void)picofw_push_emit_str("CRYPTO", msg);
        (void)picofw_push_flush_global();
        printf("CRYPTO: OK\n");
        return;
    }
    const bool is_dec = picofw_ascii_ieq(op, "DEC");
    const bool is_enc = picofw_ascii_ieq(op, "ENC");
    if (is_dec || is_enc) {
        char tok_key[32];
        const char *hex_in = picofw_next_token(rest, tok_key, sizeof(tok_key));
        uint8_t rk[32];
        char klabel[24];
        uint8_t *const buf = g_picofw_cmd_arena.raw;
        char *const out_hex = (char *)&g_picofw_cmd_arena.raw[2048u];
        size_t blen = 0u;
        if (!resolve_feistel_key_tok(tok_key, rk, klabel, sizeof(klabel)) ||
            !picofw_decode_hex_bytes(hex_in, buf, 256u, &blen) || (blen < 8u) || ((blen & 7u) != 0u)) {
            picofw_cli_set_result("ERR", "EINVAL");
            printf("CRYPTO: ERROR (Usage: CRYPTO <DEC|ENC> <idx|name|16hex> <8N_hex>)\n");
            return;
        }
        picofw_feistel_crypt_buffer(buf, blen, rk, is_dec);
        (void)picofw_fast_format_page_hex(buf, blen, out_hex, 513u);
        char msg[96];
        (void)snprintf(msg, sizeof(msg), "mode=%s key=%s len=%u out=",
                       is_dec ? "DEC" : "ENC", klabel, (unsigned)blen);
        (void)snprintf(out_hex + 520u, 700u, "%s%s", msg, out_hex);
        (void)picofw_push_emit_str("CRYPTO", out_hex + 520u);
        (void)picofw_push_flush_global();
        printf("CRYPTO: OK\n");
        return;
    }

    picofw_cli_set_result("ERR", "EINVAL");
    printf("CRYPTO: ERROR (Unknown subcommand '%s')\n", op);
}

static bool decomp_run_selftest(void) {
    bool lz_ok = false;
    bool zl_ok = false;
    (void)picofw_decomp_selftest(g_picofw_cmd_arena.raw, &lz_ok, &zl_ok);
    char msg[96];
    (void)snprintf(msg, sizeof(msg), "selftest=%s lzss=%u zlib=%u",
                   (lz_ok && zl_ok) ? "PASS" : "FAIL", lz_ok ? 1u : 0u, zl_ok ? 1u : 0u);
    (void)picofw_push_emit_str("DECOMP", msg);
    (void)picofw_push_flush_global();
    return lz_ok && zl_ok;
}

void picofw_cmd_decomp(const char *arg) {
    char op[20];
    const char *hex_arg = picofw_next_token(arg, op, sizeof(op));
    if ((op[0] == '\0') || picofw_ascii_ieq(op, "SELFTEST")) {
        if (!decomp_run_selftest()) {
            picofw_cli_set_result("ERR", "EIO");
            printf("DECOMP: ERROR (selftest failed)\n");
            return;
        }
        printf("DECOMP: OK\n");
        return;
    }

    const bool is_lzss = picofw_ascii_ieq(op, "LZSS");
    const bool is_lzss_enc = picofw_ascii_ieq(op, "LZSS_ENC");
    const bool is_zlib = picofw_ascii_ieq(op, "ZLIB");
    const bool is_zlib_enc = picofw_ascii_ieq(op, "ZLIB_ENC");
    const bool is_raw = picofw_ascii_ieq(op, "RAW");
    if (!is_lzss && !is_lzss_enc && !is_zlib && !is_zlib_enc && !is_raw) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("DECOMP: ERROR (Usage: DECOMP <SELFTEST|LZSS|LZSS_ENC|ZLIB|ZLIB_ENC|RAW> <hex>)\n");
        return;
    }

    uint8_t *const in_buf = g_picofw_cmd_arena.raw;
    uint8_t *const out_buf = &g_picofw_cmd_arena.raw[1024u];
    uint8_t *const win4k = &g_picofw_cmd_arena.raw[5120u];
    picofw_inflate_scratch_t *const sc = (picofw_inflate_scratch_t *)(void *)&g_picofw_cmd_arena.raw[9216u];
    size_t in_len = 0u;
    size_t out_len = 0u;
    if (!picofw_decode_hex_bytes(hex_arg, in_buf, 1024u, &in_len) || (in_len == 0u)) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("DECOMP: ERROR (Invalid hex payload)\n");
        return;
    }

    bool ok = false;
    if (is_lzss) {
        ok = picofw_lzss_decompress(in_buf, in_len, out_buf, 1024u, &out_len, win4k);
    } else if (is_lzss_enc) {
        ok = picofw_lzss_compress(in_buf, in_len, out_buf, 1024u, &out_len, win4k);
    } else if (is_zlib) {
        ok = picofw_zlib_decompress(in_buf, in_len, out_buf, 1024u, &out_len, sc);
    } else if (is_zlib_enc) {
        ok = picofw_zlib_compress(in_buf, in_len, out_buf, 1024u, &out_len);
    } else {
        ok = picofw_inflate_raw(in_buf, in_len, out_buf, 1024u, &out_len, sc);
    }
    if (!ok) {
        picofw_cli_set_result("ERR", "EIO");
        printf("DECOMP: ERROR (Decompression or Adler-32 failed)\n");
        return;
    }

    const uint32_t crc = picofw_crc32_ieee(out_buf, out_len);
    char *const hex_out = (char *)&g_picofw_cmd_arena.raw[16384u];
    const size_t show = (out_len > 256u) ? 256u : out_len;
    (void)picofw_fast_format_page_hex(out_buf, show, hex_out, 513u);
    char *const frame = (char *)&g_picofw_cmd_arena.raw[17000u];
    (void)snprintf(frame, 1024u, "algo=%s in=%u out=%u crc32=0x%08" PRIX32 " data=%s",
                   op, (unsigned)in_len, (unsigned)out_len, crc, hex_out);
    (void)picofw_push_emit_str("DECOMP", frame);
    (void)picofw_push_flush_global();
    printf("DECOMP: OK\n");
}
