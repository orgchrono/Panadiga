/**
 * @file picofw_math_gps.h
 * @brief Declarative Mathematical GPS, fixed-point statistics, MN103S disassembler & PiE-OS router.
 *
 * Split by SRP into pure host-testable modules (`picofw_math_stats.c`, `picofw_mn103_disasm.c`,
 * `picofw_math_gps.c`) with zero floats, zero heap allocation, and O(1) bounded loops.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_MATH_GPS_H
#define PICOFW_MATH_GPS_H

#include "diag/picofw_boottrace_codec.h"
#include "generated/picofw_ssot.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define PICOFW_CAL_ELEC   0x01u
#define PICOFW_CAL_IC3001 0x02u
#define PICOFW_CAL_PIEOS  0x04u
#define PICOFW_CAL_AFFINE 0x08u

typedef enum picofw_block_kind {
    PICOFW_BLK_EMPTY_ZERO = 0,
    PICOFW_BLK_ERASED_FF = 1,
    PICOFW_BLK_ASCII_TEXT = 2,
    PICOFW_BLK_MN103S_CODE = 3,
    PICOFW_BLK_COMPRESSED = 4,
    PICOFW_BLK_ENCRYPTED_FEISTEL = 5,
    PICOFW_BLK_HIGH_ENTROPY = 6,
    PICOFW_BLK_STRUCTURED_DATA = 7
} picofw_block_kind_t;

typedef struct picofw_math_block_stats {
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
    picofw_block_kind_t kind;
} picofw_math_block_stats_t;

typedef struct picofw_math_pattern_report {
    picofw_math_block_stats_t stats;
    uint32_t chi2_q8;       /* chi^2 goodness-of-fit in Q8 (0 if len < 256) */
    uint32_t mirror_period; /* smallest repeating half-period in bytes, or 0 */
    uint16_t ecb64_repeats; /* repeated non-blank 8-byte ECB ciphertext blocks */
    bool sbox_256_found;    /* true if a valid non-affine 256-B S-box was found */
    uint16_t sbox_offset;   /* byte offset of detected S-box */
} picofw_math_pattern_report_t;

typedef enum picofw_mn103_edge_kind {
    PICOFW_EDGE_PROLOGUE = 0,
    PICOFW_EDGE_CALL = 1,
    PICOFW_EDGE_MMIO = 2
} picofw_mn103_edge_kind_t;

typedef struct picofw_mn103_edge {
    picofw_mn103_edge_kind_t kind;
    uint32_t site_va;
    uint32_t target_va;
    const char *label;
} picofw_mn103_edge_t;

typedef enum picofw_mn103_imm_kind {
    PICOFW_IMM_NONE = 0,
    PICOFW_IMM_U8 = 1,
    PICOFW_IMM_U16 = 2,
    PICOFW_IMM_U32 = 3
} picofw_mn103_imm_kind_t;

typedef struct picofw_mn103_insn_info {
    uint8_t len;
    bool valid;
    bool is_rel_branch;
    bool is_ret;
    picofw_mn103_imm_kind_t imm_kind;
    uint8_t imm_off;
    uint32_t imm_val;
    uint8_t reg_idx; /* 0..3: D0..D3, 4..7: A0..A3, 8: SP */
    char mnemonic[28];
} picofw_mn103_insn_info_t;

typedef enum picofw_route_domain {
    PICOFW_ROUTE_BOOT = 0,
    PICOFW_ROUTE_VIDEO = 1,
    PICOFW_ROUTE_BGO = 2,
    PICOFW_ROUTE_HDD = 3,
    PICOFW_ROUTE_NVRAM = 4,
    PICOFW_ROUTE_UNBRICK = 5,
    PICOFW_ROUTE_COUNT = 6
} picofw_route_domain_t;

typedef struct picofw_gps_env {
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
    uint32_t override_va[PICOFW_SSOT_PIEOS_ANCHOR_COUNT];
    picofw_ssot_target_profile_id_t profile_id;
    uint16_t aiof_vmodesel;
    uint8_t dvd_region;
    uint8_t ic3001_expected_addr7;
    char model_id[20];
    char region_suffix[8];
    char tv_standard[16];
    char hw_sys[16];
    char market[16];
    char timer_ver[8];
} picofw_gps_env_t;

/* --- Fixed-point statistical & structural pattern engines (`diag/picofw_math_stats.c`) --- */
uint32_t picofw_math_log2_q10(uint32_t x);
const char *picofw_block_kind_name(picofw_block_kind_t kind);
bool picofw_math_analyze_block(
    const uint8_t *buf,
    size_t len,
    uint32_t hist256[256],
    picofw_math_block_stats_t *out
);
uint32_t picofw_math_chi2_q8(const uint32_t hist256[256], uint32_t len);
uint16_t picofw_math_detect_ecb64(const uint8_t *buf, size_t len);
uint32_t picofw_math_detect_addr_mirror(const uint8_t *buf, size_t len);
bool picofw_math_is_crypto_sbox256(const uint8_t win[256]);
bool picofw_math_scan_patterns(
    const uint8_t *buf,
    size_t len,
    uint32_t hist256[256],
    picofw_math_pattern_report_t *out
);

/* --- Pure MN103S prologue/CFG/disassembler (`diag/picofw_mn103_disasm.c`) --- */
bool picofw_mn103_detect_prologue(const uint8_t *buf, size_t len, size_t off, const char **out_kind);
size_t picofw_mn103_scan_edges(
    const uint8_t *buf,
    size_t len,
    uint32_t base_va,
    picofw_mn103_edge_t *out_edges,
    size_t max_edges
);
bool picofw_mn103_decode_insn(
    const uint8_t *buf,
    size_t len,
    uint32_t pc_va,
    picofw_mn103_insn_info_t *out
);
bool picofw_mn103_cover_span(
    const uint8_t *buf,
    size_t len,
    uint32_t base_va,
    size_t min_bytes,
    size_t *out_cover,
    bool *out_reloc_ok
);
bool picofw_mn103_assemble_insn(
    const char *text,
    uint32_t pc_va,
    uint8_t out_bytes[8],
    uint8_t *out_len
);

/* --- Declarative model+region calibration & routing (`diag/picofw_math_gps.c`) --- */
void picofw_gps_env_init(picofw_gps_env_t *env);
bool picofw_gps_parse_model_suffix(
    const char *spec,
    char *out_model,
    size_t cap_m,
    char *out_suffix,
    size_t cap_s
);
bool picofw_gps_autocal_model_suffix(picofw_gps_env_t *env, const char *spec);
void picofw_gps_calibrate_from_classifier(picofw_gps_env_t *env, const picofw_boot_classifier_t *cls);
bool picofw_gps_calibrate_affine(picofw_gps_env_t *env, size_t anchor_idx, uint32_t observed_va);
uint32_t picofw_gps_resolve_anchor(const picofw_gps_env_t *env, size_t idx);
bool picofw_gps_find_anchor_by_name(const char *name, size_t *out_idx);
bool picofw_gps_find_anchor_by_va(const picofw_gps_env_t *env, uint32_t va, size_t *out_idx);
bool picofw_gps_parse_route_domain(const char *tok, picofw_route_domain_t *out_dom);
const char *picofw_gps_build_name(uint8_t main_build);
size_t picofw_gps_format_env(const picofw_gps_env_t *env, char *out, size_t cap);
size_t picofw_gps_build_route(
    const picofw_gps_env_t *env,
    picofw_route_domain_t dom,
    char *out,
    size_t cap
);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_MATH_GPS_H */
