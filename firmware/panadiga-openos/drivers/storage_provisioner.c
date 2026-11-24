/**
 * @file storage_provisioner.c
 * @brief Deterministic Full-Storage MEIHFS Auto-Sizing Planner & `M FIRM -> MAIN -> UPDATE -> UPDOK -> FINISH` Shell.
 *
 * HARDWARE-INTEROPERABILITY PROTOTYPE -- NOT LEGALLY CLEAN-ROOM.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "drivers/openos_drivers.h"

#include <string.h>

static const char *const k_stage_vfd[5] = {
    "M FIRM",
    "MAIN",
    "UPDATE",
    "UPDOK",
    "FINISH",
};

const char *openos_provision_stage_vfd(openos_provision_stage_t stage) {
    const size_t idx = (size_t)stage;
    if (idx < (sizeof(k_stage_vfd) / sizeof(k_stage_vfd[0]))) {
        return k_stage_vfd[idx];
    }
    return "ERR";
}

bool openos_storage_plan_compute(
    uint32_t total_lba,
    uint32_t payload_bytes,
    uint32_t payload_crc32,
    openos_storage_plan_t *out_plan
) {
    if (out_plan == NULL) {
        return false;
    }
    memset(out_plan, 0, sizeof(*out_plan));

    if ((payload_bytes == 0UL) || (total_lba < AM34_OPENOS_MEIHFS_MIN_DISK_LBA)) {
        return false;
    }
    if (total_lba <= (AM34_OPENOS_MEIHFS_PRIMARY_FW_LBA + AM34_OPENOS_MEIHFS_MIRROR_DELTA_LBA)) {
        return false;
    }

    const uint32_t primary_fw_lba = AM34_OPENOS_MEIHFS_PRIMARY_FW_LBA;
    const uint32_t mirror_fw_lba = total_lba - AM34_OPENOS_MEIHFS_MIRROR_DELTA_LBA;
    const uint32_t fw_sectors =
        (payload_bytes / AM34_OPENOS_SECTOR_BYTES) +
        (((payload_bytes % AM34_OPENOS_SECTOR_BYTES) != 0UL) ? 1UL : 0UL);
    if ((fw_sectors == 0UL) || (fw_sectors > (mirror_fw_lba - primary_fw_lba))) {
        return false;
    }

    const uint32_t raw_end_lba = primary_fw_lba + fw_sectors;
    const uint32_t align_rem = raw_end_lba & 3UL;
    const uint32_t align_pad = (align_rem == 0UL) ? 0UL : (4UL - align_rem);
    if (align_pad > (mirror_fw_lba - raw_end_lba)) {
        return false;
    }
    const uint32_t data_start_lba = raw_end_lba + align_pad;
    if (data_start_lba >= mirror_fw_lba) {
        return false;
    }

    const uint32_t usable_sectors = mirror_fw_lba - data_start_lba;
    uint32_t usable_blocks_2kb = usable_sectors / 4UL;
    if (usable_blocks_2kb == 0UL) {
        return false;
    }
    if (usable_blocks_2kb > AM34_OPENOS_MEIHFS_MAX_BLOCKS_2KB) {
        usable_blocks_2kb = AM34_OPENOS_MEIHFS_MAX_BLOCKS_2KB;
    }

    out_plan->total_lba = total_lba;
    out_plan->mbr_lba = AM34_OPENOS_MEIHFS_MBR_LBA;
    out_plan->mbr_mirror_lba = AM34_OPENOS_MEIHFS_MBR_MIRROR_LBA;
    out_plan->journal_lba = AM34_OPENOS_MEIHFS_JOURNAL_LBA;
    out_plan->primary_fw_lba = primary_fw_lba;
    out_plan->mirror_fw_lba = mirror_fw_lba;
    out_plan->fw_sectors = fw_sectors;
    out_plan->data_start_lba = data_start_lba;
    out_plan->usable_blocks_2kb = usable_blocks_2kb;
    out_plan->payload_crc32 = payload_crc32;
    out_plan->valid = true;
    return true;
}

typedef struct openos_prov_step_desc {
    openos_provision_stage_t stage;
    uint32_t lba_a;
    uint32_t cnt_a;
    uint32_t lba_b;
    uint32_t cnt_b;
} openos_prov_step_desc_t;

bool openos_storage_provision_execute(
    const openos_storage_plan_t *plan,
    openos_storage_write_fn write_fn,
    void *ctx,
    openos_provision_stage_t *out_last_stage
) {
    if ((plan == NULL) || !plan->valid || (write_fn == NULL)) {
        return false;
    }

    uint32_t waited_ms = 0UL;
    if (!openos_davis_reset_and_wait_ready(true, &waited_ms) ||
        (waited_ms != OPENOS_SATA_BRIDGE_WAIT_MS)) {
        return false;
    }

    const openos_prov_step_desc_t steps[5] = {
        { OPENOS_PROV_STAGE_M_FIRM, plan->mbr_lba,        1UL,              plan->mbr_mirror_lba, 1UL },
        { OPENOS_PROV_STAGE_MAIN,   plan->primary_fw_lba, plan->fw_sectors, 0UL,                  0UL },
        { OPENOS_PROV_STAGE_UPDATE, plan->mirror_fw_lba,  plan->fw_sectors, 0UL,                  0UL },
        { OPENOS_PROV_STAGE_UPDOK,  plan->journal_lba,    10UL,             0UL,                  0UL },
        { OPENOS_PROV_STAGE_FINISH, 0UL,                  0UL,              0UL,                  0UL },
    };

    for (size_t i = 0u; i < (sizeof(steps) / sizeof(steps[0])); ++i) {
        const openos_prov_step_desc_t *step = &steps[i];
        if (out_last_stage != NULL) {
            *out_last_stage = step->stage;
        }
        if (!openos_tbus_send_vfd_text(openos_provision_stage_vfd(step->stage)) ||
            !openos_tbus_send_keepalive_255s()) {
            return false;
        }
        if ((step->cnt_a > 0UL) &&
            !write_fn(step->lba_a, step->cnt_a, (uint32_t)step->stage, ctx)) {
            return false;
        }
        if ((step->cnt_b > 0UL) &&
            !write_fn(step->lba_b, step->cnt_b, (uint32_t)step->stage, ctx)) {
            return false;
        }
    }

    openos_uart0_puts("[OPENOS] PROVISION=FINISH\n");
    return true;
}
