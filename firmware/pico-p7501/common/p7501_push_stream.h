/**
 * @file p7501_push_stream.h
 * @brief Dedicated 4 KiB SRAM staged `@P7501_FRAME` push-frame bus for host buffering.
 *
 * Keeps an isolated 4096-byte SRAM staging ring completely separate from `g_p7501_cmd_arena`
 * so active target captures and decompression never collide with host push frames.
 * Automatically flushes when reaching the 3072-byte high watermark or on explicit flush.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_PUSH_STREAM_H
#define P7501_PUSH_STREAM_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define P7501_PUSH_RING_CAP  4096u
#define P7501_PUSH_WATERMARK 3072u

typedef void (*p7501_push_sink_fn)(const uint8_t *data, size_t len, void *ctx);

typedef struct p7501_push_stats {
    uint32_t seq;
    uint32_t frames_staged;
    uint32_t bytes_staged;
    uint32_t flushes;
    uint32_t buffered_bytes;
    uint32_t high_water_bytes;
    uint32_t drops;
    uint32_t last_crc32;
} p7501_push_stats_t;

typedef struct p7501_push_bus {
    uint8_t buf[P7501_PUSH_RING_CAP];
    size_t used;
    p7501_push_sink_fn sink;
    void *sink_ctx;
    p7501_push_stats_t stats;
} p7501_push_bus_t;

/** Initialize push bus with optional host sink callback (NULL uses drop-on-full or default). */
void p7501_push_init(p7501_push_bus_t *bus, p7501_push_sink_fn sink, void *sink_ctx);

/** Clear staged buffer and statistics while preserving registered host sink. */
void p7501_push_reset(p7501_push_bus_t *bus);

/** Flush any pending staged bytes immediately through the registered host sink. */
bool p7501_push_flush(p7501_push_bus_t *bus);

/**
 * @brief Frame and stage one `@P7501_FRAME` line in SRAM, auto-flushing at `P7501_PUSH_WATERMARK`.
 */
bool p7501_push_stage_frame(
    p7501_push_bus_t *bus,
    const char *type_tag,
    uint64_t ts_us,
    const char *payload,
    size_t payload_len
);

/** Access the probe-wide singleton push-frame bus (lazy-initialized to stdio stdout sink). */
p7501_push_bus_t *p7501_push_global(void);

/** Stage a null-terminated payload into the global SRAM push bus (with current probe timestamp). */
bool p7501_push_emit_str(const char *type_tag, const char *payload);

/** Flush the global SRAM push bus cleanly over USB CDC / stdout. */
bool p7501_push_flush_global(void);

/** CLI command `STREAM [STATUS|TEST|RESET]`. */
void p7501_cmd_stream(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* P7501_PUSH_STREAM_H */
