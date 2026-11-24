/**
 * @file test_openos_host.c
 * @brief Host C11 Verification Suite (`-DPANADIGA_OPENOS_HOST_TEST=1`) for Panadiga OpenOS.
 *
 * Verifies lethal MMIO barriers (`0xC000100C`, `0xDC0F0000`), permanent `SA0..SA10` IPL write
 * lockouts, `IC7501` `TBUS` checksum math (`0x30`), `NTSC` (`0x8020`) default bring-up,
 * `1000 ms` StarTech `IDE2SAT2` post-reset wait, and UART0 banner output.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "drivers/openos_drivers.h"

#include <assert.h>
#include <stdio.h>
#include <string.h>

typedef struct mock_mmio_state {
    char uart_log[2048];
    size_t uart_len;
    char rx_queue[128];
    size_t rx_head;
    size_t rx_tail;
    uint32_t sdcd_ctrl;
    uint32_t sdcd_cfg4;
    uint32_t sa11_w0;
    uint32_t sa11_magic_fc;
    uint32_t eemul_w0;
    uint16_t bgo_lbn[45];
    uint32_t tbus_mbox[4];
    uint32_t tbus_ctrl;
    uint32_t tbus_tx_count;
    char tbus_vfd_history[16][8];
    size_t tbus_vfd_count;
    uint16_t aiof_v;
    uint16_t aiof_vmodesel;
    uint16_t aiof_fs0;
    uint16_t aiof_yc3;
    uint16_t aiof_cont;
    uint16_t avdd_npalctl0;
    uint16_t avdd_voutmode0;
    uint16_t avdd_dacout1;
    uint8_t iic1_writes[8];
    size_t iic1_write_count;
    uint8_t iic1_last_tx;
    uint8_t iic1_nack_addr_a;
    uint8_t iic1_nack_addr_b;
    bool iic1_nack_all;
    uint16_t davis_ch1_rst;
    uint32_t slept_ms_total;
    bool forbidden_touched;
} mock_mmio_state_t;

static mock_mmio_state_t g_mock;

uint32_t openos_host_mmio_read32(uint32_t addr) {
    if (am34_mmio_is_forbidden(addr)) {
        g_mock.forbidden_touched = true;
    }
    if (addr == (AM34_OPENOS_IIC1_BASE + AM34_OPENOS_IIC_STAT_OFF)) {
        if (g_mock.iic1_nack_all ||
            (g_mock.iic1_last_tx == g_mock.iic1_nack_addr_a) ||
            (g_mock.iic1_last_tx == g_mock.iic1_nack_addr_b)) {
            return AM34_OPENOS_IIC_STAT_NACK_BIT;
        }
        return 0UL;
    }
    if (addr == AM34_OPENOS_SDCD_BASE) {
        return g_mock.sdcd_ctrl;
    }
    if (addr == AM34_OPENOS_SDCD_BUS_CFG) {
        return g_mock.sdcd_cfg4;
    }
    if (addr == OPENOS_SSOT_NORFROM_SA11_BASE) {
        return g_mock.sa11_w0;
    }
    if (addr == (OPENOS_SSOT_NORFROM_SA11_BASE + OPENOS_SSOT_NOR_AUTOBOOT_MAGIC_OFFSET)) {
        return g_mock.sa11_magic_fc;
    }
    if (addr == OPENOS_SSOT_EEMUL_BASE) {
        return g_mock.eemul_w0;
    }
    return 0u;
}

uint16_t openos_host_mmio_read16(uint32_t addr) {
    if (am34_mmio_is_forbidden(addr)) {
        g_mock.forbidden_touched = true;
    }
    if (addr == AM34_OPENOS_SC0_STR) {
        return (g_mock.rx_head < g_mock.rx_tail) ? AM34_OPENOS_SC0_RX_READY_MASK : 0x0000U;
    }
    if (addr == AM34_OPENOS_DAVIS_CH1_BASE) {
        return 0x0050U; /* DRDY | DSC */
    }
    if ((addr >= (OPENOS_SSOT_BGO_BASE + 4UL)) &&
        (((addr - OPENOS_SSOT_BGO_BASE) % OPENOS_SSOT_BGO_SECTOR_BYTES) == 4UL)) {
        const uint32_t idx = (addr - OPENOS_SSOT_BGO_BASE) / OPENOS_SSOT_BGO_SECTOR_BYTES;
        if (idx < 45UL) {
            return g_mock.bgo_lbn[idx];
        }
    }
    return 0u;
}

uint8_t openos_host_mmio_read8(uint32_t addr) {
    if (am34_mmio_is_forbidden(addr)) {
        g_mock.forbidden_touched = true;
        return 0u;
    }
    if ((addr == AM34_OPENOS_SC0_RXB) && (g_mock.rx_head < g_mock.rx_tail)) {
        return (uint8_t)g_mock.rx_queue[g_mock.rx_head++];
    }
    return 0u;
}

void openos_host_mmio_write32(uint32_t addr, uint32_t val) {
    if (!am34_mmio_write_allowed(addr)) {
        g_mock.forbidden_touched = true;
        return;
    }
    if ((addr >= AM34_OPENOS_TBUS_MBOX) && (addr < (AM34_OPENOS_TBUS_MBOX + 16u))) {
        g_mock.tbus_mbox[(addr - AM34_OPENOS_TBUS_MBOX) / 4u] = val;
    } else if (addr == AM34_OPENOS_TBUS_BASE) {
        g_mock.tbus_ctrl = val;
        g_mock.tbus_tx_count += 1u;
        if (((g_mock.tbus_mbox[0] & 0xFFu) == 0xA3u) &&
            (g_mock.tbus_vfd_count < (sizeof(g_mock.tbus_vfd_history) / sizeof(g_mock.tbus_vfd_history[0])))) {
            char *dst = g_mock.tbus_vfd_history[g_mock.tbus_vfd_count++];
            dst[0] = (char)((g_mock.tbus_mbox[0] >> 24u) & 0xFFu);
            dst[1] = (char)(g_mock.tbus_mbox[1] & 0xFFu);
            dst[2] = (char)((g_mock.tbus_mbox[1] >> 8u) & 0xFFu);
            dst[3] = (char)((g_mock.tbus_mbox[1] >> 16u) & 0xFFu);
            dst[4] = (char)((g_mock.tbus_mbox[1] >> 24u) & 0xFFu);
            dst[5] = (char)(g_mock.tbus_mbox[2] & 0xFFu);
            dst[6] = (char)((g_mock.tbus_mbox[2] >> 8u) & 0xFFu);
            dst[7] = '\0';
        }
    }
}

void openos_host_mmio_write16(uint32_t addr, uint16_t val) {
    if (!am34_mmio_write_allowed(addr)) {
        g_mock.forbidden_touched = true;
        return;
    }
    if (addr == (AM34_OPENOS_AIOF_BASE + AM34_OPENOS_AIOF_V_OFF)) {
        g_mock.aiof_v = val;
    } else if (addr == (AM34_OPENOS_AIOF_BASE + AM34_OPENOS_AIOF_VMODESEL_OFF)) {
        g_mock.aiof_vmodesel = val;
    } else if (addr == (AM34_OPENOS_AIOF_BASE + AM34_OPENOS_AIOF_FS0_OFF)) {
        g_mock.aiof_fs0 = val;
    } else if (addr == (AM34_OPENOS_AIOF_BASE + AM34_OPENOS_AIOF_YC3_OFF)) {
        g_mock.aiof_yc3 = val;
    } else if (addr == (AM34_OPENOS_AIOF_BASE + AM34_OPENOS_AIOF_CONT_OFF)) {
        g_mock.aiof_cont = val;
    } else if (addr == (AM34_OPENOS_AVDD_BASE + AM34_OPENOS_AVDD_NPALCTL0_OFF)) {
        g_mock.avdd_npalctl0 = val;
    } else if (addr == (AM34_OPENOS_AVDD_BASE + AM34_OPENOS_AVDD_VOUTMODE0_OFF)) {
        g_mock.avdd_voutmode0 = val;
    } else if (addr == (AM34_OPENOS_AVDD_BASE + AM34_OPENOS_AVDD_DACOUT1_OFF)) {
        g_mock.avdd_dacout1 = val;
    } else if (addr == AM34_OPENOS_DAVIS_CH1_RST) {
        g_mock.davis_ch1_rst = val;
    }
}

void openos_host_mmio_write8(uint32_t addr, uint8_t val) {
    if (!am34_mmio_write_allowed(addr)) {
        g_mock.forbidden_touched = true;
        return;
    }
    if ((addr == AM34_OPENOS_SC0_TXB) && (g_mock.uart_len + 1u < sizeof(g_mock.uart_log))) {
        g_mock.uart_log[g_mock.uart_len++] = (char)val;
        g_mock.uart_log[g_mock.uart_len] = '\0';
    } else if (addr == (AM34_OPENOS_IIC1_BASE + AM34_OPENOS_IIC_TX_OFF)) {
        g_mock.iic1_last_tx = val;
        if (g_mock.iic1_write_count < sizeof(g_mock.iic1_writes)) {
            g_mock.iic1_writes[g_mock.iic1_write_count++] = val;
        }
    }
}

void openos_host_sleep_ms(uint32_t ms) {
    g_mock.slept_ms_total += ms;
}

typedef struct mock_prov_rec {
    uint32_t stage_calls;
    uint32_t lbas[8];
    uint32_t counts[8];
    uint32_t stages[8];
    int32_t fail_on_stage;
} mock_prov_rec_t;

static bool mock_storage_write_cb(uint32_t start_lba, uint32_t sector_count, uint32_t stage_tag, void *ctx) {
    mock_prov_rec_t *rec = (mock_prov_rec_t *)ctx;
    if (rec->stage_calls < 8u) {
        rec->lbas[rec->stage_calls] = start_lba;
        rec->counts[rec->stage_calls] = sector_count;
        rec->stages[rec->stage_calls] = stage_tag;
        rec->stage_calls += 1u;
    }
    if ((rec->fail_on_stage >= 0) && ((uint32_t)rec->fail_on_stage == stage_tag)) {
        return false;
    }
    return true;
}

int main(void) {
    memset(&g_mock, 0, sizeof(g_mock));

    /* 1. Verify lethal MMIO & SA0..SA10 IPL write barriers + SSOT AIOF_VMODESEL offset */
    assert(AM34_OPENOS_AIOF_VMODESEL_OFF == 0x0000UL);
    assert(am34_mmio_is_forbidden(0xC000100CUL));
    assert(am34_mmio_is_forbidden(0xDC0F0000UL));
    uint32_t dummy = 0u;
    assert(!am34_mmio_read32(0xC000100CUL, &dummy));
    assert(!am34_mmio_write32(0xC000100CUL, 0x12345678UL));
    assert(!am34_mmio_write32(0x80000000UL, 0xDEADBEEFUL));
    assert(!am34_mmio_write32(0x8403FFFCUL, 0xDEADBEEFUL));
    assert(!g_mock.forbidden_touched);

    /* 2. Verify IC7501 TBUS 255s frame, VFD builder, and BR IC7501 safe-opcode guard */
    uint8_t wdt16[16];
    openos_tbus_build_wdt_255s_frame(wdt16);
    assert(wdt16[0] == 0x8Au && wdt16[1] == 0x02u && wdt16[3] == 0xFFu && wdt16[15] == 0x30u);

    uint8_t vfd16[16];
    openos_tbus_build_vfd_frame("OPENOS", vfd16);
    assert(vfd16[0] == 0xA3u && memcmp(&vfd16[3], "OPENOS ", 7u) == 0);

    assert(openos_tbus_opcode_safe_for_br_ic7501(0x8Au));
    assert(openos_tbus_opcode_safe_for_br_ic7501(0xA3u));
    static const uint8_t blocked_ops[13] = {
        0x85u, 0x89u, 0x9Bu, 0xA9u, 0xAAu, 0xC9u, 0xDAu,
        0xE3u, 0xE4u, 0xECu, 0xF0u, 0xF9u, 0xFAu
    };
    for (size_t i = 0u; i < (sizeof(blocked_ops) / sizeof(blocked_ops[0])); ++i) {
        assert(!openos_tbus_opcode_safe_for_br_ic7501(blocked_ops[i]));
    }

    uint8_t bad_op16[16];
    memset(bad_op16, 0, sizeof(bad_op16));
    bad_op16[0] = 0xA9u;
    bad_op16[15] = openos_tbus_checksum15(bad_op16);
    assert(!openos_tbus_send_frame_checked(bad_op16));

    uint8_t bad_csum16[16];
    memcpy(bad_csum16, wdt16, sizeof(bad_csum16));
    bad_csum16[15] ^= 0x01u;
    assert(!openos_tbus_send_frame_checked(bad_csum16));
    assert(g_mock.tbus_tx_count == 0u);

    /* 3. Execute openos_kernel_entry() and verify NTSC+TBC defaults + VFD + 1000ms DAVIS CH1 */
    openos_kernel_entry();
    assert(!g_mock.forbidden_touched);
    assert(g_mock.tbus_ctrl == 0x10u);
    assert(g_mock.tbus_tx_count == 2u);
    assert(g_mock.aiof_v == AM34_OPENOS_AIOF_V_ENABLE);
    assert(g_mock.aiof_vmodesel == AM34_OPENOS_VMODESEL_NTSC);
    assert(g_mock.aiof_fs0 == AM34_OPENOS_AIOF_FS0_STD);
    assert(g_mock.aiof_yc3 == AM34_OPENOS_AIOF_YC3_MVAGC_TBC);
    assert(g_mock.aiof_cont == AM34_OPENOS_AIOF_CONT_75IRE);
    assert(g_mock.slept_ms_total == OPENOS_SATA_BRIDGE_WAIT_MS);
    assert(strstr(g_mock.uart_log, "[OPENOS] M0..M3 bring-up OK (TV=NTSC, SATA_WAIT=1000ms, IPL=RO_E05)") != NULL);
    assert(strstr(g_mock.uart_log, "OPENOS> ") != NULL);

    /* 4. Verify NFROM config1 ID 0x0117 decoder + TBC profile switching + OPENOS> shell dispatcher */
    assert(openos_tv_from_nfrom_id0117(0u) == OPENOS_TV_NTSC);
    assert(openos_tv_from_nfrom_id0117(1u) == OPENOS_TV_PAL);

    assert(openos_aiof_apply_tbc_profile(OPENOS_TV_PAL_M, true, true));
    assert(g_mock.aiof_vmodesel == AM34_OPENOS_VMODESEL_PAL_M);
    assert(g_mock.aiof_fs0 == AM34_OPENOS_AIOF_FS0_NONSTD);
    assert(g_mock.aiof_yc3 == AM34_OPENOS_AIOF_YC3_MVAGC_TBC);
    assert(g_mock.aiof_cont == AM34_OPENOS_AIOF_CONT_0IRE);

    assert(openos_shell_exec_line("help"));
    assert(openos_shell_exec_line("palm"));
    assert(g_mock.aiof_vmodesel == AM34_OPENOS_VMODESEL_PAL_M);
    assert(openos_shell_exec_line("tbc on"));
    assert(g_mock.aiof_fs0 == AM34_OPENOS_AIOF_FS0_NONSTD);
    assert(openos_shell_exec_line("tbc off"));
    assert(g_mock.aiof_fs0 == AM34_OPENOS_AIOF_FS0_STD);
    assert(g_mock.aiof_yc3 == 0x0000U);
    assert(openos_shell_exec_line("ntsc"));
    assert(g_mock.aiof_vmodesel == AM34_OPENOS_VMODESEL_NTSC);
    assert(openos_shell_exec_line("vfd READY"));
    assert(openos_shell_exec_line("wdt"));
    assert(openos_shell_exec_line("sata"));
    assert(openos_shell_exec_line("status"));

    /* 5. Verify AVDD DAC + IIC1 IC3001 (BH7629KS2 0x90/0x92 BR/NA vs C1AB00002379 0x94 EC) */
    assert(openos_ic3001_addr8(OPENOS_VIDEO_IC_BH7629KS2_BR_NA, false) == 0x90u);
    assert(openos_ic3001_addr8(OPENOS_VIDEO_IC_BH7629KS2_BR_NA, true) == 0x92u);
    assert(openos_ic3001_addr8(OPENOS_VIDEO_IC_C1AB2379_EC, false) == 0x94u);
    assert(g_mock.avdd_dacout1 == 0x0007U);
    assert(g_mock.iic1_write_count == 6u && g_mock.iic1_writes[0] == 0x90u);
    assert(!openos_iic_write_reg_checked(0xC0001000UL, 0x90u, 0x00u, 0x00u));

    g_mock.iic1_write_count = 0u;
    assert(openos_shell_exec_line("video ec"));
    assert(g_mock.avdd_npalctl0 == 0x0002U);
    assert(g_mock.iic1_write_count == 6u && g_mock.iic1_writes[0] == 0x94u);
    assert(openos_shell_exec_line("dac off"));
    assert(g_mock.avdd_voutmode0 == 0x0000U && g_mock.avdd_dacout1 == 0x0000U);
    assert(openos_shell_exec_line("dac on"));
    assert(g_mock.avdd_voutmode0 == 0x0001U && g_mock.avdd_dacout1 == 0x0007U);
    assert(openos_shell_exec_line("video br"));
    assert(g_mock.avdd_npalctl0 == 0x0000U);

    /* 6. Verify IIC1 IC3001 probe (0x90, 0x92 fallback, 0x94 EC, NACK failure) & AVDD colorbar */
    uint8_t probed = 0u;
    assert(!openos_iic_probe_ic3001(OPENOS_VIDEO_IC_BH7629KS2_BR_NA, NULL));
    assert(openos_iic_probe_ic3001(OPENOS_VIDEO_IC_BH7629KS2_BR_NA, &probed));
    assert(probed == 0x90u);

    g_mock.iic1_nack_addr_a = 0x90u;
    assert(openos_iic_probe_ic3001(OPENOS_VIDEO_IC_BH7629KS2_BR_NA, &probed));
    assert(probed == 0x92u);
    assert(openos_iic_probe_ic3001(OPENOS_VIDEO_IC_C1AB2379_EC, &probed));
    assert(probed == 0x94u);

    g_mock.iic1_nack_addr_b = 0x92u;
    assert(openos_shell_exec_line("video probe"));
    assert(strstr(g_mock.uart_log, "[OPENOS] IC3001_PROBE=C1AB2379_0x94") != NULL);

    g_mock.iic1_nack_all = true;
    assert(!openos_iic_probe_ic3001(OPENOS_VIDEO_IC_BH7629KS2_BR_NA, &probed));
    assert(!openos_iic_probe_ic3001(OPENOS_VIDEO_IC_C1AB2379_EC, &probed));
    assert(!openos_shell_exec_line("video probe"));
    assert(strstr(g_mock.uart_log, "[OPENOS] IC3001_PROBE=NONE") != NULL);
    g_mock.iic1_nack_all = false;
    g_mock.iic1_nack_addr_a = 0u;
    g_mock.iic1_nack_addr_b = 0u;

    assert(openos_shell_exec_line("colorbar on"));
    assert(g_mock.avdd_voutmode0 == AM34_OPENOS_AVDD_VOUTMODE0_COLORBAR);
    assert(g_mock.avdd_dacout1 == AM34_OPENOS_AVDD_DACOUT1_ENABLE);
    assert(openos_shell_exec_line("colorbar off"));
    assert(g_mock.avdd_voutmode0 == AM34_OPENOS_AVDD_VOUTMODE0_NORMAL);

    /* 7. Verify deterministic full-storage auto-sizer (5 GB, 250 GB, 2 TB + negative guards) */
    openos_storage_plan_t plan;
    assert(!openos_storage_plan_compute(488281250UL, 0x00645048UL, 0x32873975UL, NULL));
    assert(!openos_storage_plan_compute(488281250UL, 0UL, 0x32873975UL, &plan));
    assert(!openos_storage_plan_compute(100000UL, 0x00645048UL, 0x32873975UL, &plan));
    assert(!plan.valid);

    assert(openos_storage_plan_compute(AM34_OPENOS_MEIHFS_MIN_DISK_LBA, 0x00645048UL, 0x32873975UL, &plan));
    assert(plan.valid && (plan.fw_sectors == 12841UL));
    assert(plan.mirror_fw_lba == (AM34_OPENOS_MEIHFS_MIN_DISK_LBA - AM34_OPENOS_MEIHFS_MIRROR_DELTA_LBA));

    assert(openos_storage_plan_compute(488281250UL, 0x00645048UL, 0x32873975UL, &plan));
    assert(plan.valid && ((plan.data_start_lba & 3UL) == 0UL) && (plan.usable_blocks_2kb > 0UL));

    openos_storage_plan_t plan_2tb;
    assert(openos_storage_plan_compute(0xFFFFFFFFUL, 0x00645048UL, 0x32873975UL, &plan_2tb));
    assert(plan_2tb.valid && (plan_2tb.usable_blocks_2kb <= AM34_OPENOS_MEIHFS_MAX_BLOCKS_2KB));

    /* 8. Verify M FIRM -> MAIN -> UPDATE -> UPDOK -> FINISH VFD sequence & fail-closed halt */
    g_mock.tbus_vfd_count = 0u;
    mock_prov_rec_t rec;
    memset(&rec, 0, sizeof(rec));
    rec.fail_on_stage = -1;
    openos_provision_stage_t last_stage = OPENOS_PROV_STAGE_M_FIRM;
    assert(openos_storage_provision_execute(&plan, mock_storage_write_cb, &rec, &last_stage));
    assert(last_stage == OPENOS_PROV_STAGE_FINISH);
    assert(rec.stage_calls == 5u); /* MBR + MBR mirror + MAIN + UPDATE + journal */
    assert((rec.lbas[0] == 0UL) && (rec.lbas[1] == AM34_OPENOS_MEIHFS_MBR_MIRROR_LBA));
    assert((rec.lbas[2] == AM34_OPENOS_MEIHFS_PRIMARY_FW_LBA) && (rec.lbas[3] == plan.mirror_fw_lba));
    assert((rec.lbas[4] == AM34_OPENOS_MEIHFS_JOURNAL_LBA) && (rec.counts[4] == 10UL));
    assert(g_mock.tbus_vfd_count == 5u);
    assert(strcmp(g_mock.tbus_vfd_history[0], "M FIRM ") == 0);
    assert(strcmp(g_mock.tbus_vfd_history[1], "MAIN   ") == 0);
    assert(strcmp(g_mock.tbus_vfd_history[2], "UPDATE ") == 0);
    assert(strcmp(g_mock.tbus_vfd_history[3], "UPDOK  ") == 0);
    assert(strcmp(g_mock.tbus_vfd_history[4], "FINISH ") == 0);

    g_mock.tbus_vfd_count = 0u;
    memset(&rec, 0, sizeof(rec));
    rec.fail_on_stage = (int32_t)OPENOS_PROV_STAGE_UPDATE;
    assert(!openos_storage_provision_execute(&plan, mock_storage_write_cb, &rec, &last_stage));
    assert(last_stage == OPENOS_PROV_STAGE_UPDATE);
    assert(g_mock.tbus_vfd_count == 3u); /* M FIRM, MAIN, UPDATE only - never reached UPDOK */

    assert(openos_shell_exec_line("provision"));
    assert(strstr(g_mock.uart_log, "[OPENOS] PROVISION PLAN OK") != NULL);

    /* 9. Verify SC0 read8 barrier, SDCD 4-bit/1-bit probe, read-only BGO/NorAutoboot/EEMUL & interactive RX shell */
    uint8_t b8 = 0u;
    assert(!am34_mmio_read8(0xC000100CUL, &b8));
    assert(!g_mock.forbidden_touched);

    g_mock.sdcd_ctrl = 0x00000001UL;
    g_mock.sdcd_cfg4 = 0x000000B8UL;
    g_mock.sa11_w0 = 0xFFFFFFFFUL;
    g_mock.sa11_magic_fc = 0xFFFFFFFFUL;
    g_mock.eemul_w0 = OPENOS_SSOT_EEMUL_TAG_1284_LE;
    uint16_t p2_id = 0xF201u;
    for (size_t i = 0u; i < 45u; ++i) {
        if (i >= 41u) {
            g_mock.bgo_lbn[i] = 0xFFFFu;
        } else if (i == 3u) {
            g_mock.bgo_lbn[i] = 0xF101u;
        } else {
            g_mock.bgo_lbn[i] = p2_id++;
        }
    }
    assert(openos_shell_exec_line("sd"));
    assert(strstr(g_mock.uart_log, "[OPENOS] SDCD bus=4BIT") != NULL);
    g_mock.sdcd_cfg4 = 0x000080B8UL;
    openos_sdcd_status_t sd_chk;
    assert(openos_sdcd_probe_status(&sd_chk) && !sd_chk.bus_width_4bit);
    g_mock.sdcd_cfg4 = 0x000000B8UL;

    assert(openos_shell_exec_line("bgo"));
    assert(strstr(g_mock.uart_log, "[OPENOS] BGO p1_sa=29 p2=40 erased=4 sa11=FF eemul1284=1 nor_autoboot=READY") != NULL);
    g_mock.sa11_w0 = 0xE0FEF800UL;
    g_mock.sa11_magic_fc = OPENOS_SSOT_NOR_AUTOBOOT_MAGIC_LE;
    openos_unbrick_status_t ub_naut;
    assert(openos_nor_inspect_unbrick(&ub_naut) && ub_naut.sa11_has_autoboot_capsule && !ub_naut.sa11_is_erased);
    assert(openos_shell_exec_line("bgo"));
    assert(strstr(g_mock.uart_log, "sa11=NAUT eemul1284=1 nor_autoboot=NO") != NULL);
    assert(openos_shell_exec_line("ssot"));
    assert(strstr(g_mock.uart_log, "[OPENOS] SSOT=" OPENOS_SSOT_CONTRACT_HASH_STR " IPL_RO=E05") != NULL);

    static const char k_rx_cmd[] = "sx\bd\r";
    memcpy(g_mock.rx_queue, k_rx_cmd, sizeof(k_rx_cmd) - 1u);
    g_mock.rx_head = 0u;
    g_mock.rx_tail = sizeof(k_rx_cmd) - 1u;
    openos_shell_state_t sh;
    openos_shell_init(&sh);
    bool ran_cmd = false;
    for (size_t i = 0u; i < 16u; ++i) {
        bool step_ran = false;
        (void)openos_shell_poll_once(&sh, &step_ran);
        if (step_ran) {
            ran_cmd = true;
            break;
        }
    }
    assert(ran_cmd);
    for (uint32_t p = 0u; p < 65536UL; ++p) {
        (void)openos_shell_poll_once(&sh, NULL);
    }
    assert(sh.keepalives_sent == 1UL);

    assert(!openos_shell_exec_line("bogus"));
    assert(!openos_shell_exec_line(NULL));

    printf("[PASS] test_openos_host: MMIO barriers, 13-op BR TBUS guard, AVDD colorbar/IIC1 probe, storage provisioner, SDCD/BGO RO, interactive RX shell & 1000ms DAVIS CH1 verified.\n");
    return 0;
}
