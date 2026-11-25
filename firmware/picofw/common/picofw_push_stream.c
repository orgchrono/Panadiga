/**
 * @file picofw_push_stream.c
 * @brief Dedicated 4 KiB SRAM staged `@PICO_FRAME` push-frame bus & `STREAM` CLI handler.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "common/picofw_push_stream.h"
#include "common/picofw_hex.h"

#if !defined(PICO_HOST_TEST) || (PICO_HOST_TEST == 0)
#include "cli/picofw_cli.h"
#include "pico/time.h"
#endif

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static picofw_push_bus_t s_global_push;
static bool s_global_push_init = false;

static void stdout_push_sink(const uint8_t *data, size_t len, void *ctx) {
    (void)ctx;
    if ((data == NULL) || (len == 0u)) {
        return;
    }
    (void)fwrite(data, 1u, len, stdout);
    (void)fflush(stdout);
}

static uint64_t push_now_us(void) {
#if !defined(PICO_HOST_TEST) || (PICO_HOST_TEST == 0)
    return time_us_64();
#else
    return 0ULL;
#endif
}

static size_t trimmed_single_line_len(const char *payload, size_t payload_len) {
    size_t clean = payload_len;
    while ((clean > 0u) && ((payload[clean - 1u] == '\n') || (payload[clean - 1u] == '\r'))) {
        clean -= 1u;
    }
    return clean;
}

void picofw_push_init(picofw_push_bus_t *bus, picofw_push_sink_fn sink, void *sink_ctx) {
    if (bus == NULL) {
        return;
    }
    memset(bus, 0, sizeof(*bus));
    bus->sink = sink;
    bus->sink_ctx = sink_ctx;
}

void picofw_push_reset(picofw_push_bus_t *bus) {
    if (bus == NULL) {
        return;
    }
    bus->used = 0u;
    memset(&bus->stats, 0, sizeof(bus->stats));
}

bool picofw_push_flush(picofw_push_bus_t *bus) {
    if (bus == NULL) {
        return false;
    }
    if (bus->used == 0u) {
        return true;
    }
    if (bus->sink == NULL) {
        return false;
    }
    bus->sink(bus->buf, bus->used, bus->sink_ctx);
    bus->used = 0u;
    bus->stats.buffered_bytes = 0u;
    bus->stats.flushes += 1u;
    return true;
}

bool picofw_push_stage_frame(
    picofw_push_bus_t *bus,
    const char *type_tag,
    uint64_t ts_us,
    const char *payload,
    size_t payload_len
) {
    if ((bus == NULL) || (type_tag == NULL) || (type_tag[0] == '\0') ||
        (payload_len >= PICOFW_PUSH_RING_CAP) ||
        ((payload_len > 0u) && (payload == NULL))) {
        if (bus != NULL) {
            bus->stats.drops += 1u;
        }
        return false;
    }

    const size_t clean_len = (payload_len > 0u) ? trimmed_single_line_len(payload, payload_len) : 0u;
    const uint32_t next_seq = bus->stats.seq + 1u;
    const uint32_t crc = (clean_len > 0u)
        ? picofw_crc32_ieee((const uint8_t *)payload, clean_len)
        : 0u;

    char hdr[112];
    const int hlen_i = snprintf(
        hdr,
        sizeof(hdr),
        "@PICO_FRAME seq=%" PRIu32 " ts_us=%llu type=%s len=%u crc32=%08" PRIX32 "%s",
        next_seq,
        (unsigned long long)ts_us,
        type_tag,
        (unsigned)clean_len,
        crc,
        (clean_len > 0u) ? " " : ""
    );
    if ((hlen_i <= 0) || ((size_t)hlen_i >= sizeof(hdr))) {
        bus->stats.drops += 1u;
        return false;
    }

    const size_t hlen = (size_t)hlen_i;
    const size_t rec_len = hlen + clean_len + 1u;
    if (rec_len > PICOFW_PUSH_RING_CAP) {
        bus->stats.drops += 1u;
        return false;
    }

    if ((bus->used + rec_len) > PICOFW_PUSH_RING_CAP) {
        if (!picofw_push_flush(bus) || ((bus->used + rec_len) > PICOFW_PUSH_RING_CAP)) {
            bus->stats.drops += 1u;
            return false;
        }
    }

    memcpy(&bus->buf[bus->used], hdr, hlen);
    bus->used += hlen;
    if (clean_len > 0u) {
        memcpy(&bus->buf[bus->used], payload, clean_len);
        bus->used += clean_len;
    }
    bus->buf[bus->used++] = (uint8_t)'\n';

    bus->stats.seq = next_seq;
    bus->stats.frames_staged += 1u;
    bus->stats.bytes_staged += (uint32_t)rec_len;
    bus->stats.buffered_bytes = (uint32_t)bus->used;
    if (bus->stats.buffered_bytes > bus->stats.high_water_bytes) {
        bus->stats.high_water_bytes = bus->stats.buffered_bytes;
    }
    bus->stats.last_crc32 = crc;

    if (bus->used >= PICOFW_PUSH_WATERMARK) {
        (void)picofw_push_flush(bus);
    }
    return true;
}

picofw_push_bus_t *picofw_push_global(void) {
    if (!s_global_push_init) {
        picofw_push_init(&s_global_push, stdout_push_sink, NULL);
        s_global_push_init = true;
    }
    return &s_global_push;
}

bool picofw_push_emit_str(const char *type_tag, const char *payload) {
    const size_t len = (payload != NULL) ? strlen(payload) : 0u;
    return picofw_push_stage_frame(picofw_push_global(), type_tag, push_now_us(), payload, len);
}

bool picofw_push_flush_global(void) {
    return picofw_push_flush(picofw_push_global());
}

void picofw_cmd_stream(const char *arg) {
    picofw_push_bus_t *const bus = picofw_push_global();
    const bool has_arg = (arg != NULL) && (*arg != '\0');
    if (has_arg && picofw_ascii_ieq(arg, "RESET")) {
        picofw_push_reset(bus);
    } else if (has_arg && picofw_ascii_ieq(arg, "TEST")) {
        static const char k_probe_msg[] = "probe=PICOFW_SRAM_BUS watermark=3072 cap=4096 ok=1";
        const bool staged = picofw_push_emit_str("STREAM_TEST", k_probe_msg);
        const bool flushed = picofw_push_flush(bus);
        if (!staged || !flushed) {
#if !defined(PICO_HOST_TEST) || (PICO_HOST_TEST == 0)
            picofw_cli_set_result("ERR", "EIO");
#endif
            printf("STREAM: ERROR (staging or flush failed)\n");
            return;
        }
    } else if (has_arg && !picofw_ascii_ieq(arg, "STATUS")) {
#if !defined(PICO_HOST_TEST) || (PICO_HOST_TEST == 0)
        picofw_cli_set_result("ERR", "EINVAL");
#endif
        printf("STREAM: ERROR (Usage: STREAM [STATUS|TEST|RESET])\n");
        return;
    }

    printf("[STREAM] cap=%u watermark=%u buffered=%" PRIu32 " seq=%" PRIu32 " frames=%" PRIu32
           " bytes=%" PRIu32 " flushes=%" PRIu32 " high_water=%" PRIu32 " drops=%" PRIu32
           " last_crc32=0x%08" PRIX32 "\n",
           (unsigned)PICOFW_PUSH_RING_CAP,
           (unsigned)PICOFW_PUSH_WATERMARK,
           bus->stats.buffered_bytes,
           bus->stats.seq,
           bus->stats.frames_staged,
           bus->stats.bytes_staged,
           bus->stats.flushes,
           bus->stats.high_water_bytes,
           bus->stats.drops,
           bus->stats.last_crc32);
    printf("STREAM: OK\n");
}
