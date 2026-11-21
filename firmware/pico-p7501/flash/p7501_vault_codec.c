/**
 * @file p7501_vault_codec.c
 * @brief Pure C11 Pico QSPI Flash Forensic Vault (`P2_VAULT` A/B) manifest serializer & CRC32 verifier.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/p7501_vault_codec.h"
#include "common/p7501_hex.h"
#include <stddef.h>
#include <string.h>

static uint32_t compute_manifest_crc(const p7501_vault_manifest_t *m) {
    return p7501_crc32_ieee((const uint8_t *)m, offsetof(p7501_vault_manifest_t, header_crc32));
}

void p7501_vault_manifest_init(p7501_vault_manifest_t *m) {
    if (m == NULL) {
        return;
    }
    memset(m, 0, sizeof(*m));
    m->magic = P7501_VAULT_MAGIC;
    m->version = P7501_VAULT_VERSION;
    m->header_crc32 = compute_manifest_crc(m);
}

uint32_t p7501_vault_bank_offset(uint32_t total_qspi_bytes, uint8_t bank_ab) {
    if ((total_qspi_bytes < P7501_VAULT_MIN_FLASH_BYTES) || (bank_ab >= P7501_VAULT_BANK_COUNT)) {
        return 0UL;
    }
    const uint32_t off = P7501_QSPI_P2_VAULT_OFFSET + ((uint32_t)bank_ab * P7501_QSPI_VAULT_BANK_BYTES);
    if (!p7501_qspi_is_writable_range(
            P7501_QSPI_PART_VAULT, off, P7501_QSPI_VAULT_BANK_BYTES, total_qspi_bytes)) {
        return 0UL;
    }
    return off;
}

uint32_t p7501_vault_base_offset(uint32_t total_qspi_bytes) {
    return p7501_vault_bank_offset(total_qspi_bytes, 0u);
}

bool p7501_vault_slot_qspi_offset_ab(
    uint32_t total_qspi_bytes,
    uint8_t bank_ab,
    uint8_t slot_index,
    uint32_t *out_offset
) {
    const uint32_t base = p7501_vault_bank_offset(total_qspi_bytes, bank_ab);
    if ((base == 0UL) || (out_offset == NULL) ||
        (slot_index == 0u) || (slot_index > P7501_VAULT_PAYLOAD_SLOTS)) {
        return false;
    }
    const uint32_t off = base + ((uint32_t)slot_index * P7501_VAULT_SLOT_BYTES);
    if (!p7501_qspi_is_writable_range(
            P7501_QSPI_PART_VAULT, off, P7501_VAULT_SLOT_BYTES, total_qspi_bytes)) {
        return false;
    }
    *out_offset = off;
    return true;
}

bool p7501_vault_slot_qspi_offset(
    uint32_t total_qspi_bytes,
    uint8_t slot_index,
    uint32_t *out_offset
) {
    return p7501_vault_slot_qspi_offset_ab(total_qspi_bytes, 0u, slot_index, out_offset);
}

bool p7501_vault_manifest_record_slot_crc(
    p7501_vault_manifest_t *m,
    uint8_t slot_index,
    uint32_t target_addr,
    size_t payload_len,
    uint32_t data_crc32,
    const char *model_tag
) {
    if ((m == NULL) || (payload_len == 0u) ||
        (payload_len > P7501_VAULT_SLOT_BYTES) ||
        (slot_index == 0u) || (slot_index > P7501_VAULT_PAYLOAD_SLOTS)) {
        return false;
    }
    if (!p7501_vault_manifest_verify(m)) {
        p7501_vault_manifest_init(m);
    }
    p7501_vault_slot_meta_t *s = &m->slots[slot_index - 1u];
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

bool p7501_vault_manifest_record_slot(
    p7501_vault_manifest_t *m,
    uint8_t slot_index,
    uint32_t target_addr,
    const uint8_t *payload,
    size_t payload_len,
    const char *model_tag
) {
    if (payload == NULL) {
        return false;
    }
    return p7501_vault_manifest_record_slot_crc(
        m,
        slot_index,
        target_addr,
        payload_len,
        p7501_crc32_ieee(payload, payload_len),
        model_tag
    );
}

bool p7501_vault_manifest_verify(const p7501_vault_manifest_t *m) {
    if ((m == NULL) || (m->magic != P7501_VAULT_MAGIC) ||
        (m->version != P7501_VAULT_VERSION)) {
        return false;
    }
    return m->header_crc32 == compute_manifest_crc(m);
}

int p7501_vault_select_ab(
    const p7501_vault_manifest_t *bank_a,
    const p7501_vault_manifest_t *bank_b,
    p7501_vault_manifest_t *out_active
) {
    const bool ok_a = p7501_vault_manifest_verify(bank_a);
    const bool ok_b = p7501_vault_manifest_verify(bank_b);
    if (!ok_a && !ok_b) {
        if (out_active != NULL) {
            p7501_vault_manifest_init(out_active);
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

bool p7501_vault_manifest_clear_slot(p7501_vault_manifest_t *m, uint8_t slot_index) {
    if ((m == NULL) || (slot_index == 0u) || (slot_index > P7501_VAULT_PAYLOAD_SLOTS)) {
        return false;
    }
    if (!p7501_vault_manifest_verify(m)) {
        p7501_vault_manifest_init(m);
    }
    memset(&m->slots[slot_index - 1u], 0, sizeof(m->slots[slot_index - 1u]));
    m->write_count += 1u;
    m->header_crc32 = compute_manifest_crc(m);
    return true;
}
