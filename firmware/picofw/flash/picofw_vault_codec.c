/**
 * @file picofw_vault_codec.c
 * @brief Pure C11 Pico QSPI Flash Forensic Vault (`P2_VAULT` A/B) manifest serializer & CRC32 verifier.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/picofw_vault_codec.h"
#include "common/picofw_hex.h"
#include <stddef.h>
#include <string.h>

static uint32_t compute_manifest_crc(const picofw_vault_manifest_t *m) {
    return picofw_crc32_ieee((const uint8_t *)m, offsetof(picofw_vault_manifest_t, header_crc32));
}

void picofw_vault_manifest_init(picofw_vault_manifest_t *m) {
    if (m == NULL) {
        return;
    }
    memset(m, 0, sizeof(*m));
    m->magic = PICOFW_VAULT_MAGIC;
    m->version = PICOFW_VAULT_VERSION;
    m->header_crc32 = compute_manifest_crc(m);
}

uint32_t picofw_vault_bank_offset(uint32_t total_qspi_bytes, uint8_t bank_ab) {
    if ((total_qspi_bytes < PICOFW_VAULT_MIN_FLASH_BYTES) || (bank_ab >= PICOFW_VAULT_BANK_COUNT)) {
        return 0UL;
    }
    const uint32_t off = PICOFW_QSPI_P2_VAULT_OFFSET + ((uint32_t)bank_ab * PICOFW_QSPI_VAULT_BANK_BYTES);
    if (!picofw_qspi_is_writable_range(
            PICOFW_QSPI_PART_VAULT, off, PICOFW_QSPI_VAULT_BANK_BYTES, total_qspi_bytes)) {
        return 0UL;
    }
    return off;
}

uint32_t picofw_vault_base_offset(uint32_t total_qspi_bytes) {
    return picofw_vault_bank_offset(total_qspi_bytes, 0u);
}

bool picofw_vault_slot_qspi_offset_ab(
    uint32_t total_qspi_bytes,
    uint8_t bank_ab,
    uint8_t slot_index,
    uint32_t *out_offset
) {
    const uint32_t base = picofw_vault_bank_offset(total_qspi_bytes, bank_ab);
    if ((base == 0UL) || (out_offset == NULL) ||
        (slot_index == 0u) || (slot_index > PICOFW_VAULT_PAYLOAD_SLOTS)) {
        return false;
    }
    const uint32_t off = base + ((uint32_t)slot_index * PICOFW_VAULT_SLOT_BYTES);
    if (!picofw_qspi_is_writable_range(
            PICOFW_QSPI_PART_VAULT, off, PICOFW_VAULT_SLOT_BYTES, total_qspi_bytes)) {
        return false;
    }
    *out_offset = off;
    return true;
}

bool picofw_vault_slot_qspi_offset(
    uint32_t total_qspi_bytes,
    uint8_t slot_index,
    uint32_t *out_offset
) {
    return picofw_vault_slot_qspi_offset_ab(total_qspi_bytes, 0u, slot_index, out_offset);
}

bool picofw_vault_manifest_record_slot_crc(
    picofw_vault_manifest_t *m,
    uint8_t slot_index,
    uint32_t target_addr,
    size_t payload_len,
    uint32_t data_crc32,
    const char *model_tag
) {
    if ((m == NULL) || (payload_len == 0u) ||
        (payload_len > PICOFW_VAULT_SLOT_BYTES) ||
        (slot_index == 0u) || (slot_index > PICOFW_VAULT_PAYLOAD_SLOTS)) {
        return false;
    }
    if (!picofw_vault_manifest_verify(m)) {
        picofw_vault_manifest_init(m);
    }
    picofw_vault_slot_meta_t *s = &m->slots[slot_index - 1u];
    memset(s, 0, sizeof(*s));
    s->valid = 1u;
    s->target_addr = target_addr;
    s->byte_len = (uint32_t)payload_len;
    s->data_crc32 = data_crc32;
    m->write_count += 1u;
    s->sequence = m->write_count;
    if (model_tag != NULL) {
        for (size_t i = 0u; (i + 1u) < sizeof(s->model_tag) && model_tag[i] != '\0'; ++i) {
            s->model_tag[i] = model_tag[i];
        }
    }
    m->header_crc32 = compute_manifest_crc(m);
    return true;
}

bool picofw_vault_manifest_record_slot(
    picofw_vault_manifest_t *m,
    uint8_t slot_index,
    uint32_t target_addr,
    const uint8_t *payload,
    size_t payload_len,
    const char *model_tag
) {
    if (payload == NULL) {
        return false;
    }
    return picofw_vault_manifest_record_slot_crc(
        m,
        slot_index,
        target_addr,
        payload_len,
        picofw_crc32_ieee(payload, payload_len),
        model_tag
    );
}

bool picofw_vault_manifest_verify(const picofw_vault_manifest_t *m) {
    if ((m == NULL) || (m->magic != PICOFW_VAULT_MAGIC) ||
        (m->version != PICOFW_VAULT_VERSION)) {
        return false;
    }
    return m->header_crc32 == compute_manifest_crc(m);
}

int picofw_vault_select_ab(
    const picofw_vault_manifest_t *bank_a,
    const picofw_vault_manifest_t *bank_b,
    picofw_vault_manifest_t *out_active
) {
    const bool ok_a = picofw_vault_manifest_verify(bank_a);
    const bool ok_b = picofw_vault_manifest_verify(bank_b);
    if (!ok_a && !ok_b) {
        if (out_active != NULL) {
            picofw_vault_manifest_init(out_active);
        }
        return -1;
    }
    int winner = 0;
    if (ok_a && ok_b) {
        winner = (bank_b->write_count > bank_a->write_count) ? 1 : 0;
    } else {
        winner = ok_b ? 1 : 0;
    }
    if (out_active != NULL) {
        *out_active = (winner == 1) ? *bank_b : *bank_a;
    }
    return winner;
}

bool picofw_vault_manifest_clear_slot(picofw_vault_manifest_t *m, uint8_t slot_index) {
    if ((m == NULL) || (slot_index == 0u) || (slot_index > PICOFW_VAULT_PAYLOAD_SLOTS)) {
        return false;
    }
    if (!picofw_vault_manifest_verify(m)) {
        picofw_vault_manifest_init(m);
    }
    memset(&m->slots[slot_index - 1u], 0, sizeof(m->slots[slot_index - 1u]));
    m->write_count += 1u;
    m->header_crc32 = compute_manifest_crc(m);
    return true;
}

bool picofw_vault_verify_slot_with_crc(
    const picofw_vault_manifest_t *m,
    uint8_t slot_index,
    const uint8_t *slot_payload,
    size_t payload_buf_len,
    picofw_vault_crc32_fn_t crc_fn,
    uint32_t *out_crc32,
    uint32_t *out_non_ff_bytes,
    bool *out_crc_mismatch
) {
    if (out_crc32 != NULL) {
        *out_crc32 = 0UL;
    }
    if (out_non_ff_bytes != NULL) {
        *out_non_ff_bytes = 0UL;
    }
    if (out_crc_mismatch != NULL) {
        *out_crc_mismatch = false;
    }
    if ((m == NULL) || !picofw_vault_manifest_verify(m) || (crc_fn == NULL) ||
        (slot_index == 0u) || (slot_index > PICOFW_VAULT_PAYLOAD_SLOTS)) {
        return false;
    }
    const picofw_vault_slot_meta_t *sm = &m->slots[slot_index - 1u];
    if (!sm->valid || (sm->byte_len == 0UL) || (sm->byte_len > PICOFW_VAULT_SLOT_BYTES) ||
        (slot_payload == NULL) || (payload_buf_len < (size_t)sm->byte_len)) {
        return false;
    }
    const uint32_t actual_crc = crc_fn(slot_payload, (size_t)sm->byte_len);
    const uint32_t non_ff = picofw_buf_count_non_ff(slot_payload, (size_t)sm->byte_len);
    if (out_crc32 != NULL) {
        *out_crc32 = actual_crc;
    }
    if (out_non_ff_bytes != NULL) {
        *out_non_ff_bytes = non_ff;
    }
    if (actual_crc != sm->data_crc32) {
        if (out_crc_mismatch != NULL) {
            *out_crc_mismatch = true;
        }
        return false;
    }
    return true;
}

bool picofw_vault_verify_slot_buffer(
    const picofw_vault_manifest_t *m,
    uint8_t slot_index,
    const uint8_t *slot_payload,
    size_t payload_buf_len,
    uint32_t *out_crc32,
    uint32_t *out_non_ff_bytes,
    bool *out_crc_mismatch
) {
    return picofw_vault_verify_slot_with_crc(
        m,
        slot_index,
        slot_payload,
        payload_buf_len,
        picofw_crc32_ieee,
        out_crc32,
        out_non_ff_bytes,
        out_crc_mismatch
    );
}
