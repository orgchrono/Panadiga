/**
 * @file p7501_math_gps.h
 * @brief Declarative Mathematical GPS, fixed-point statistics, MN103S disassembler & PiE-OS router.
 *
 * Split by SRP into pure host-testable modules (`p7501_math_stats.c`, `p7501_mn103_disasm.c`,
 * `p7501_math_gps.c`) with zero floats, zero heap allocation, and O(1) bounded loops.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_MATH_GPS_H
#define P7501_MATH_GPS_H

#include "diag/p7501_boottrace_codec.h"
#include "generated/p7501_ssot.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define P7501_CAL_ELEC   0x01u
#define P7501_CAL_IC3001 0x02u
#define P7501_CAL_PIEOS  0x04u
#define P7501_CAL_AFFINE 0x08u

typedef enum p7501_block_kind {
    P7501_BLK_EMPTY_ZERO = 0,
    P7501_BLK_ERASED_FF = 1,
    P7501_BLK_ASCII_TEXT = 2,
    P7501_BLK_MN103S_CODE = 3,
    P7501_BLK_COMPRESSED = 4,
    P7501_BLK_ENCRYPTED_FEISTEL = 5,
    P7501_BLK_HIGH_ENTROPY = 6,
    P7501_BLK_STRUCTURED_DATA = 7
} p7501_block_kind_t;

typedef struct p7501_math_block_stats {
    uint32_t len;
    uint32_t crc32;
    uint32_t entropy_mbit; /* 0..8000 milli-bits/byte */
    uint8_t ff_pct;
    uint8_t zero_pct;
    uint8_t ascii_pct;
    uint16_t mn103_hits;
    bool byte_swapped_16;
    bool zlib_header;
    int8_t feistel_key_idx; /* -1 if none matched */
    p7501_block_kind_t kind;
} p7501_math_block_stats_t;

typedef struct p7501_math_pattern_report {
    p7501_math_block_stats_t stats;
    uint32_t chi2_q8;       /* chi^2 goodness-of-fit in Q8 (0 if len < 256) */
    uint32_t mirror_period; /* smallest repeating half-period in bytes, or 0 */
    uint16_t ecb64_repeats; /* repeated non-blank 8-byte ECB ciphertext blocks */
    bool sbox_256_found;    /* true if a valid non-affine 256-B S-box was found */
    uint16_t sbox_offset;   /* byte offset of detected S-box */
} p7501_math_pattern_report_t;

typedef enum p7501_mn103_edge_kind {
    P7501_EDGE_PROLOGUE = 0,
    P7501_EDGE_CALL = 1,
    P7501_EDGE_MMIO = 2
} p7501_mn103_edge_kind_t;

typedef struct p7501_mn103_edge {
    p7501_mn103_edge_kind_t kind;
    uint32_t site_va;
    uint32_t target_va;
    const char *label;
} p7501_mn103_edge_t;

typedef enum p7501_mn103_imm_kind {
    P7501_IMM_NONE = 0,
    P7501_IMM_U8 = 1,
    P7501_IMM_U16 = 2,
    P7501_IMM_U32 = 3
} p7501_mn103_imm_kind_t;

typedef struct p7501_mn103_insn_info {
    uint8_t len;
    bool valid;
    bool is_rel_branch;
    bool is_ret;
    p7501_mn103_imm_kind_t imm_kind;
    uint8_t imm_off;
    uint32_t imm_val;
    uint8_t reg_idx; /* 0..3: D0..D3, 4..7: A0..A3, 8: SP */
    char mnemonic[28];
} p7501_mn103_insn_info_t;

typedef enum p7501_route_domain {
    P7501_ROUTE_BOOT = 0,
    P7501_ROUTE_VIDEO = 1,
    P7501_ROUTE_BGO = 2,
    P7501_ROUTE_HDD = 3,
    P7501_ROUTE_NVRAM = 4,
    P7501_ROUTE_UNBRICK = 5,
    P7501_ROUTE_COUNT = 6
} p7501_route_domain_t;

typedef struct p7501_gps_env {
    uint32_t flash_bytes;
    uint32_t bgo_base;
    uint32_t bgo_sectors;
    uint32_t nvram_base;
    uint32_t vdd_mv;
    bool nsw5v_high;
    bool monitor_online;
    uint8_t ic3001_mask;
    uint8_t calibrated_mask;
    uint8_t main_build;
    uint8_t region_code;
    bool has_region;
    uint16_t avin_iic_err;
    uint16_t bgo_err;
    uint16_t fpc_err;
    uint16_t hdd_fail;
    int8_t feistel_key_idx;
    bool affine_valid;
    uint8_t affine_anchor_idx;
    int32_t affine_delta;
    uint32_t override_va[P7501_SSOT_PIEOS_ANCHOR_COUNT];
    p7501_ssot_target_profile_id_t profile_id;
    uint16_t aiof_vmodesel;
    uint8_t dvd_region;
    uint8_t ic3001_expected_addr7;
    char model_id[20];
    char region_suffix[8];
    char tv_standard[16];
    char hw_sys[16];
    char market[16];
    char timer_ver[8];
} p7501_gps_env_t;

/* --- Fixed-point statistical & structural pattern engines (`diag/p7501_math_stats.c`) --- */
uint32_t p7501_math_log2_q10(uint32_t x);
const char *p7501_block_kind_name(p7501_block_kind_t kind);
bool p7501_math_analyze_block(
    const uint8_t *buf,
    size_t len,
    uint32_t hist256[256],
    p7501_math_block_stats_t *out
);
uint32_t p7501_math_chi2_q8(const uint32_t hist256[256], uint32_t len);
uint16_t p7501_math_detect_ecb64(const uint8_t *buf, size_t len);
uint32_t p7501_math_detect_addr_mirror(const uint8_t *buf, size_t len);
bool p7501_math_is_crypto_sbox256(const uint8_t win[256]);
bool p7501_math_scan_patterns(
    const uint8_t *buf,
    size_t len,
    uint32_t hist256[256],
    p7501_math_pattern_report_t *out
);

/* --- Pure MN103S prologue/CFG/disassembler (`diag/p7501_mn103_disasm.c`) --- */
bool p7501_mn103_detect_prologue(const uint8_t *buf, size_t len, size_t off, const char **out_kind);
size_t p7501_mn103_scan_edges(
    const uint8_t *buf,
    size_t len,
    uint32_t base_va,
    p7501_mn103_edge_t *out_edges,
    size_t max_edges
);
bool p7501_mn103_decode_insn(
    const uint8_t *buf,
    size_t len,
    uint32_t pc_va,
    p7501_mn103_insn_info_t *out
);
bool p7501_mn103_cover_span(
    const uint8_t *buf,
    size_t len,
    uint32_t base_va,
    size_t min_bytes,
    size_t *out_cover,
    bool *out_reloc_ok
);
bool p7501_mn103_assemble_insn(
    const char *text,
    uint32_t pc_va,
    uint8_t out_bytes[8],
    uint8_t *out_len
);

/* --- Declarative model+region calibration & routing (`diag/p7501_math_gps.c`) --- */
void p7501_gps_env_init(p7501_gps_env_t *env);
bool p7501_gps_parse_model_suffix(
    const char *spec,
    char *out_model,
    size_t cap_m,
    char *out_suffix,
    size_t cap_s
);
bool p7501_gps_autocal_model_suffix(p7501_gps_env_t *env, const char *spec);
void p7501_gps_calibrate_from_classifier(p7501_gps_env_t *env, const p7501_boot_classifier_t *cls);
bool p7501_gps_calibrate_affine(p7501_gps_env_t *env, size_t anchor_idx, uint32_t observed_va);
uint32_t p7501_gps_resolve_anchor(const p7501_gps_env_t *env, size_t idx);
bool p7501_gps_find_anchor_by_name(const char *name, size_t *out_idx);
bool p7501_gps_find_anchor_by_va(const p7501_gps_env_t *env, uint32_t va, size_t *out_idx);
bool p7501_gps_parse_route_domain(const char *tok, p7501_route_domain_t *out_dom);
const char *p7501_gps_build_name(uint8_t main_build);
size_t p7501_gps_format_env(const p7501_gps_env_t *env, char *out, size_t cap);
size_t p7501_gps_build_route(
    const p7501_gps_env_t *env,
    p7501_route_domain_t dom,
    char *out,
    size_t cap
);

#ifdef __cplusplus
}
#endif

#endif /* P7501_MATH_GPS_H */
