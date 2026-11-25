/**
 * @file picofw_push_stream.h
 * @brief Dedicated 4 KiB SRAM staged `@PICO_FRAME` push-frame bus for host buffering.
 *
 * Keeps an isolated 4096-byte SRAM staging ring completely separate from `g_picofw_cmd_arena`
 * so active target captures and decompression never collide with host push frames.
 * Automatically flushes when reaching the 3072-byte high watermark or on explicit flush.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_PUSH_STREAM_H
#define PICOFW_PUSH_STREAM_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define PICOFW_PUSH_RING_CAP  4096u
#define PICOFW_PUSH_WATERMARK 3072u

typedef void (*picofw_push_sink_fn)(const uint8_t *data, size_t len, void *ctx);

typedef struct picofw_push_stats {
    uint32_t seq;
    uint32_t frames_staged;
    uint32_t bytes_staged;
    uint32_t flushes;
    uint32_t buffered_bytes;
    uint32_t high_water_bytes;
    uint32_t drops;
    uint32_t last_crc32;
} picofw_push_stats_t;

typedef struct picofw_push_bus {
    uint8_t buf[PICOFW_PUSH_RING_CAP];
    size_t used;
    picofw_push_sink_fn sink;
    void *sink_ctx;
    picofw_push_stats_t stats;
} picofw_push_bus_t;

/** Initialize push bus with optional host sink callback (NULL uses drop-on-full or default). */
void picofw_push_init(picofw_push_bus_t *bus, picofw_push_sink_fn sink, void *sink_ctx);

/** Clear staged buffer and statistics while preserving registered host sink. */
void picofw_push_reset(picofw_push_bus_t *bus);

/** Flush any pending staged bytes immediately through the registered host sink. */
bool picofw_push_flush(picofw_push_bus_t *bus);

/**
 * @brief Frame and stage one `@PICO_FRAME` line in SRAM, auto-flushing at `PICOFW_PUSH_WATERMARK`.
 */
bool picofw_push_stage_frame(
    picofw_push_bus_t *bus,
    const char *type_tag,
    uint64_t ts_us,
    const char *payload,
    size_t payload_len
);

/** Access the probe-wide singleton push-frame bus (lazy-initialized to stdio stdout sink). */
picofw_push_bus_t *picofw_push_global(void);

/** Stage a null-terminated payload into the global SRAM push bus (with current probe timestamp). */
bool picofw_push_emit_str(const char *type_tag, const char *payload);

/** Flush the global SRAM push bus cleanly over USB CDC / stdout. */
bool picofw_push_flush_global(void);

/** CLI command `STREAM [STATUS|TEST|RESET]`. */
void picofw_cmd_stream(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_PUSH_STREAM_H */
