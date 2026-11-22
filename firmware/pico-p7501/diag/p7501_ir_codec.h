/**
 * @file p7501_ir_codec.h
 * @brief Portable ISO C11 Panasonic Kaseikyo 38 kHz IR encoder and GPIO safety validator.
 *
 * Host-testable with zero Pico SDK dependencies (`C11_HOST_SOURCES`).
 * Verified against Flipper-IRDB captures (`DMR-E50`, `DMR-EX77`, `DMR-BCT73`, `Panasonic_DVD_TV`)
 * and `pana-fw/docs/IR.md` (`VendorID 0x2002`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_IR_CODEC_H
#define P7501_IR_CODEC_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define P7501_IR_KASEIKYO_FRAME_BYTES 6u
#define P7501_IR_VENDOR_LSB0          0x02u
#define P7501_IR_VENDOR_LSB1          0x20u
#define P7501_IR_DEV_DIGA_RECORDER    0xB0u
#define P7501_IR_DEV_ND_SERVICE       0xB0u
#define P7501_IR_DEFAULT_GPIO         6u
#define P7501_IR_DEFAULT_RX_GPIO      7u
#define P7501_IR_DEFAULT_REPEAT       2u
#define P7501_IR_MAX_REPEAT           8u
#define P7501_IR_CARRIER_HZ           38000UL
#define P7501_IR_UNIT_US              432u
#define P7501_IR_LEADER_MARK_US       (8u * P7501_IR_UNIT_US)
#define P7501_IR_LEADER_SPACE_US      (4u * P7501_IR_UNIT_US)
#define P7501_IR_BIT_ZERO_SPACE_US    (1u * P7501_IR_UNIT_US)
#define P7501_IR_BIT_ONE_SPACE_US     (3u * P7501_IR_UNIT_US)
#define P7501_IR_FRAME_GAP_US         74000u
#define P7501_IR_PULSE_COUNT          99u

typedef struct p7501_ir_spec {
    uint8_t frame[P7501_IR_KASEIKYO_FRAME_BYTES];
    uint32_t gpio;
    uint32_t repeat;
    char label[24];
} p7501_ir_spec_t;

typedef struct p7501_ir_decode_result {
    bool valid;
    bool vendor_ok;
    bool parity_ok;
    uint8_t frame[P7501_IR_KASEIKYO_FRAME_BYTES];
    uint8_t dev;
    uint8_t sub;
    uint8_t cmd;
    uint8_t xor_rx;
    uint8_t xor_expected;
} p7501_ir_decode_result_t;

/** Validate that `gpio` is a free, non-P7501/non-SD/non-ADC/non-SMPS spare RP2 pin (GP6..9 or GP16..22). */
bool p7501_ir_is_safe_gpio(uint32_t gpio);

/** Parse a `GP<n>` or `<n>` token and validate `p7501_ir_is_safe_gpio`. */
bool p7501_ir_parse_gpio_token(const char *tok, uint32_t *out_gpio);

/** Build a 6-byte Panasonic Kaseikyo wire frame `[02, 20, dev, sub, cmd, dev^sub^cmd]`. */
void p7501_ir_encode_kaseikyo(
    uint8_t dev_byte,
    uint8_t sub_byte,
    uint8_t cmd_byte,
    uint8_t out_frame[P7501_IR_KASEIKYO_FRAME_BYTES]
);

/** Lookup matching preset name for `(dev, sub, cmd)`, or NULL if unknown. */
const char *p7501_ir_lookup_preset_name(uint8_t dev_byte, uint8_t sub_byte, uint8_t cmd_byte);

/** Build deterministic alternating mark/space durations (`P7501_IR_PULSE_COUNT` entries) in microseconds. */
size_t p7501_ir_build_pulse_table(
    const uint8_t frame[P7501_IR_KASEIKYO_FRAME_BYTES],
    uint32_t *out_durations_us,
    size_t cap
);

/** Decode alternating mark/space durations (leader mark..stop) into a Kaseikyo 48-bit frame. */
bool p7501_ir_decode_pulses(
    const uint32_t *durations_us,
    size_t count,
    p7501_ir_decode_result_t *out_res
);

/** Verify a 48-bit (`12`-hex-digit) Panasonic Kaseikyo frame (`0x2002`, vendor parity nibble, XOR byte). */
bool p7501_ir_verify_hex_frame(const char *hex_frame, p7501_ir_decode_result_t *out_res);

/** Parse `IRTX <preset|sub:cmd|cmd> [GPn|gpio] [repeat]` deterministically. */
bool p7501_ir_parse_args(const char *arg, p7501_ir_spec_t *out);

/** Parse `IRRX [ms] [GPn|gpio]` deterministically. */
bool p7501_ir_parse_rx_args(const char *arg, uint32_t *out_ms, uint32_t *out_gpio);

#ifdef __cplusplus
}
#endif

#endif /* P7501_IR_CODEC_H */
