/**
 * @file test_digaos_host.c
 * @brief Host C11 Verification Suite (`-DPANADIGA_DIGAOS_HOST_TEST=1`) for `DigaOS`.
 *
 * HARDWARE-INTEROPERABILITY PROTOTYPE -- NOT LEGALLY CLEAN-ROOM.
 *
 * Verifies lethal MMIO barriers (`0xC000100C`, `0xDC0F0000..0xDC0F0FFF`), permanent `SA0..SA10`
 * IPL write lockouts, `IC7501` `TBUS` checksum math (`0x30`), `NTSC`/`PAL-M`/`PAL`/`SECAM` TBC
 * bring-up, `1000 ms` `IDE2SAT2` post-reset wait, declarative SSOT tables (`menus`, `themes`,
 * `power_profiles`, `soc_targets`), multi-SoC fail-closed `UNVERIFIED_MMIO` HAL stubs, and
 * interactive `DIGAOS>` shell execution.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "drivers/digaos_drivers.h"

#include <assert.h>
#include <stdio.h>
#include <string.h>

typedef struct mock_mmio_state {
    char uart_log[4096];
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
    uint32_t avdd_npalctl0_32;
    uint16_t avdd_clk;
    uint32_t avdd_active_size;
    uint32_t avdd_sync;
    uint16_t avdd_voutmode0;
    uint16_t avdd_dacout1;
    uint8_t iic0_writes[16];
    size_t iic0_write_count;
    uint8_t iic1_writes[8];
    size_t iic1_write_count;
    uint8_t iic1_last_tx;
    uint8_t iic1_nack_addr_a;
    uint8_t iic1_nack_addr_b;
    bool iic1_nack_all;
    uint16_t davis_ch0_rst;
    uint16_t davis_ch0_status;
    uint16_t davis_ch1_rst;
    uint32_t slept_ms_total;
    bool forbidden_touched;
} mock_mmio_state_t;

static mock_mmio_state_t g_mock;

uint32_t digaos_host_mmio_read32(uint32_t addr) {
    if (am34_mmio_is_forbidden(addr)) {
        g_mock.forbidden_touched = true;
    }
    if (addr == (AM34_DIGAOS_IIC1_BASE + AM34_DIGAOS_IIC_STAT_OFF)) {
        if (g_mock.iic1_nack_all ||
            (g_mock.iic1_last_tx == g_mock.iic1_nack_addr_a) ||
            (g_mock.iic1_last_tx == g_mock.iic1_nack_addr_b)) {
            return AM34_DIGAOS_IIC_STAT_NACK_BIT;
        }
        return 0UL;
    }
    if (addr == AM34_DIGAOS_AIOF_BASE) {
        return 0xA10F0001UL;
    }
    if (addr == AM34_DIGAOS_SDCD_BASE) {
        return g_mock.sdcd_ctrl;
    }
    if (addr == AM34_DIGAOS_SDCD_BUS_CFG) {
        return g_mock.sdcd_cfg4;
    }
    if (addr == DIGAOS_SSOT_NORFROM_SA11_BASE) {
        return g_mock.sa11_w0;
    }
    if (addr == (DIGAOS_SSOT_NORFROM_SA11_BASE + DIGAOS_SSOT_NOR_AUTOBOOT_MAGIC_OFFSET)) {
        return g_mock.sa11_magic_fc;
    }
    if (addr == DIGAOS_SSOT_EEMUL_BASE) {
        return g_mock.eemul_w0;
    }
    if (addr == (AM34_DIGAOS_AVDD_BASE + 0x0408UL)) {
        return g_mock.avdd_active_size;
    }
    if (addr == (AM34_DIGAOS_AVDD_BASE + 0x040CUL)) {
        return g_mock.avdd_sync;
    }
    if (addr == (AM34_DIGAOS_AVDD_BASE + AM34_DIGAOS_AVDD_NPALCTL0_OFF)) {
        return g_mock.avdd_npalctl0_32;
    }
    return 0u;
}

uint16_t digaos_host_mmio_read16(uint32_t addr) {
    if (am34_mmio_is_forbidden(addr)) {
        g_mock.forbidden_touched = true;
    }
    if (addr == AM34_DIGAOS_SC0_STR) {
        return (g_mock.rx_head < g_mock.rx_tail) ? AM34_DIGAOS_SC0_RX_READY_MASK : 0x0000U;
    }
    if (addr == AM34_DIGAOS_DAVIS_CH0_BASE) {
        return g_mock.davis_ch0_status;
    }
    if (addr == AM34_DIGAOS_DAVIS_CH1_BASE) {
        return 0x0050U; /* DRDY | DSC */
    }
    if (addr == (AM34_DIGAOS_AIOF_BASE + AM34_DIGAOS_AIOF_VMODESEL_OFF)) {
        return g_mock.aiof_vmodesel;
    }
    if (addr == (AM34_DIGAOS_AIOF_BASE + AM34_DIGAOS_AIOF_CONT_OFF)) {
        return g_mock.aiof_cont;
    }
    if (addr == (AM34_DIGAOS_AVDD_BASE + AM34_DIGAOS_AVDD_CLK_OFF)) {
        return g_mock.avdd_clk;
    }
    if (addr == (AM34_DIGAOS_AVDD_BASE + AM34_DIGAOS_AVDD_DACOUT1_OFF)) {
        return g_mock.avdd_dacout1;
    }
    if (addr == (AM34_DIGAOS_AVDD_BASE + AM34_DIGAOS_AVDD_VOUTMODE0_OFF)) {
        return g_mock.avdd_voutmode0;
    }
    if ((addr >= (DIGAOS_SSOT_BGO_BASE + 4UL)) &&
        (((addr - DIGAOS_SSOT_BGO_BASE) % DIGAOS_SSOT_BGO_SECTOR_BYTES) == 4UL)) {
        const uint32_t idx = (addr - DIGAOS_SSOT_BGO_BASE) / DIGAOS_SSOT_BGO_SECTOR_BYTES;
        if (idx < 45UL) {
            return g_mock.bgo_lbn[idx];
        }
    }
    return 0u;
}

uint8_t digaos_host_mmio_read8(uint32_t addr) {
    if (am34_mmio_is_forbidden(addr)) {
        g_mock.forbidden_touched = true;
        return 0u;
    }
    if ((addr == AM34_DIGAOS_SC0_RXB) && (g_mock.rx_head < g_mock.rx_tail)) {
        return (uint8_t)g_mock.rx_queue[g_mock.rx_head++];
    }
    return 0u;
}

void digaos_host_mmio_write32(uint32_t addr, uint32_t val) {
    if (!am34_mmio_write_allowed(addr)) {
        g_mock.forbidden_touched = true;
        return;
    }
    if ((addr >= AM34_DIGAOS_TBUS_MBOX) && (addr < (AM34_DIGAOS_TBUS_MBOX + 16u))) {
        g_mock.tbus_mbox[(addr - AM34_DIGAOS_TBUS_MBOX) / 4u] = val;
    } else if (addr == AM34_DIGAOS_TBUS_BASE) {
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
    } else if (addr == (AM34_DIGAOS_AVDD_BASE + 0x0408UL)) {
        g_mock.avdd_active_size = val;
    } else if (addr == (AM34_DIGAOS_AVDD_BASE + 0x040CUL)) {
        g_mock.avdd_sync = val;
    } else if (addr == (AM34_DIGAOS_AVDD_BASE + AM34_DIGAOS_AVDD_NPALCTL0_OFF)) {
        g_mock.avdd_npalctl0_32 = val;
    }
}

void digaos_host_mmio_write16(uint32_t addr, uint16_t val) {
    if (!am34_mmio_write_allowed(addr)) {
        g_mock.forbidden_touched = true;
        return;
    }
    if (addr == (AM34_DIGAOS_AIOF_BASE + AM34_DIGAOS_AIOF_V_OFF)) {
        g_mock.aiof_v = val;
    } else if (addr == (AM34_DIGAOS_AIOF_BASE + AM34_DIGAOS_AIOF_VMODESEL_OFF)) {
        g_mock.aiof_vmodesel = val;
    } else if (addr == (AM34_DIGAOS_AIOF_BASE + AM34_DIGAOS_AIOF_FS0_OFF)) {
        g_mock.aiof_fs0 = val;
    } else if (addr == (AM34_DIGAOS_AIOF_BASE + AM34_DIGAOS_AIOF_YC3_OFF)) {
        g_mock.aiof_yc3 = val;
    } else if (addr == (AM34_DIGAOS_AIOF_BASE + AM34_DIGAOS_AIOF_CONT_OFF)) {
        g_mock.aiof_cont = val;
    } else if (addr == (AM34_DIGAOS_AVDD_BASE + AM34_DIGAOS_AVDD_CLK_OFF)) {
        g_mock.avdd_clk = val;
    } else if (addr == (AM34_DIGAOS_AVDD_BASE + AM34_DIGAOS_AVDD_NPALCTL0_OFF)) {
        g_mock.avdd_npalctl0 = val;
    } else if (addr == (AM34_DIGAOS_AVDD_BASE + AM34_DIGAOS_AVDD_VOUTMODE0_OFF)) {
        g_mock.avdd_voutmode0 = val;
    } else if (addr == (AM34_DIGAOS_AVDD_BASE + AM34_DIGAOS_AVDD_DACOUT1_OFF)) {
        g_mock.avdd_dacout1 = val;
    } else if (addr == AM34_DIGAOS_DAVIS_CH0_RST) {
        g_mock.davis_ch0_rst = val;
    } else if (addr == AM34_DIGAOS_DAVIS_CH1_RST) {
        g_mock.davis_ch1_rst = val;
    }
}

void digaos_host_mmio_write8(uint32_t addr, uint8_t val) {
    if (!am34_mmio_write_allowed(addr)) {
        g_mock.forbidden_touched = true;
        return;
    }
    if ((addr == AM34_DIGAOS_SC0_TXB) && (g_mock.uart_len + 1u < sizeof(g_mock.uart_log))) {
        g_mock.uart_log[g_mock.uart_len++] = (char)val;
        g_mock.uart_log[g_mock.uart_len] = '\0';
    } else if (addr == (AM34_DIGAOS_IIC0_BASE + AM34_DIGAOS_IIC_TX_OFF)) {
        if (g_mock.iic0_write_count < sizeof(g_mock.iic0_writes)) {
            g_mock.iic0_writes[g_mock.iic0_write_count++] = val;
        }
    } else if (addr == (AM34_DIGAOS_IIC1_BASE + AM34_DIGAOS_IIC_TX_OFF)) {
        g_mock.iic1_last_tx = val;
        if (g_mock.iic1_write_count < sizeof(g_mock.iic1_writes)) {
            g_mock.iic1_writes[g_mock.iic1_write_count++] = val;
        }
    }
}

void digaos_host_sleep_ms(uint32_t ms) {
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
    assert(AM34_DIGAOS_AIOF_VMODESEL_OFF == 0x0000UL);
    assert(am34_mmio_is_forbidden(0xC000100CUL));
    assert(am34_mmio_is_forbidden(0xDC0F0000UL));
    assert(am34_mmio_is_forbidden(0xDC0F0FFCUL));
    uint32_t dummy = 0u;
    assert(!am34_mmio_read32(0xC000100CUL, &dummy));
    assert(!am34_mmio_write32(0xC000100CUL, 0x12345678UL));
    assert(!am34_mmio_write32(0xDC0F0FFCUL, 0x12345678UL));
    assert(!am34_mmio_write32(0x80000000UL, 0xDEADBEEFUL));
    assert(!am34_mmio_write32(0x8403FFFCUL, 0xDEADBEEFUL));
    assert(!g_mock.forbidden_touched);

    /* 2. Verify IC7501 TBUS 255s frame, VFD builder, and BR IC7501 safe-opcode guard */
    uint8_t wdt16[16];
    digaos_tbus_build_wdt_255s_frame(wdt16);
    assert(wdt16[0] == 0x8Au && wdt16[1] == 0x02u && wdt16[3] == 0xFFu && wdt16[15] == 0x30u);

    uint8_t vfd16[16];
    digaos_tbus_build_vfd_frame("DIGAOS", vfd16);
    assert(vfd16[0] == 0xA3u && memcmp(&vfd16[3], "DIGAOS ", 7u) == 0);

    assert(digaos_tbus_opcode_safe_for_br_ic7501(0x8Au));
    assert(digaos_tbus_opcode_safe_for_br_ic7501(0xA3u));
    static const uint8_t blocked_ops[13] = {
        0x85u, 0x89u, 0x9Bu, 0xA9u, 0xAAu, 0xC9u, 0xDAu,
        0xE3u, 0xE4u, 0xECu, 0xF0u, 0xF9u, 0xFAu
    };
    for (size_t i = 0u; i < (sizeof(blocked_ops) / sizeof(blocked_ops[0])); ++i) {
        assert(!digaos_tbus_opcode_safe_for_br_ic7501(blocked_ops[i]));
    }

    uint8_t bad_op16[16];
    memset(bad_op16, 0, sizeof(bad_op16));
    bad_op16[0] = 0xA9u;
    bad_op16[15] = digaos_tbus_checksum15(bad_op16);
    assert(!digaos_tbus_send_frame_checked(bad_op16));

    uint8_t bad_csum16[16];
    memcpy(bad_csum16, wdt16, sizeof(bad_csum16));
    bad_csum16[15] ^= 0x01u;
    assert(!digaos_tbus_send_frame_checked(bad_csum16));
    assert(g_mock.tbus_tx_count == 0u);

    /* 3. Execute digaos_kernel_entry() and verify NTSC+TBC + SW_GUARDS + VFD + 1000ms DAVIS CH1 */
    digaos_kernel_entry();
    assert(!g_mock.forbidden_touched);
    assert(g_mock.tbus_ctrl == 0x10u);
    assert(g_mock.tbus_tx_count == 2u);
    assert(g_mock.aiof_v == AM34_DIGAOS_AIOF_V_ENABLE);
    assert(g_mock.aiof_vmodesel == AM34_DIGAOS_VMODESEL_NTSC);
    assert(g_mock.aiof_fs0 == AM34_DIGAOS_AIOF_FS0_STD);
    assert(g_mock.aiof_yc3 == AM34_DIGAOS_AIOF_YC3_MVAGC_TBC);
    assert(g_mock.aiof_cont == AM34_DIGAOS_AIOF_CONT_75IRE);
    assert(g_mock.slept_ms_total == DIGAOS_SATA_BRIDGE_WAIT_MS);
    assert(strstr(g_mock.uart_log, "SW_GUARDS: 0xC000100C=LOCKED") != NULL);
    assert(strstr(g_mock.uart_log, "[DIGAOS] M0..M3 bring-up OK (TV=NTSC, SATA_WAIT=1000ms, IPL=RO_E05)") != NULL);
    assert(strstr(g_mock.uart_log, "DIGAOS> ") != NULL);

    /* 4. Verify NFROM ID0117 + TBC + SECAM/SSOT/declarative shell dispatcher */
    assert(digaos_tv_from_nfrom_id0117(0u) == DIGAOS_TV_NTSC);
    assert(digaos_tv_from_nfrom_id0117(1u) == DIGAOS_TV_PAL);

    assert(digaos_aiof_apply_tbc_profile(DIGAOS_TV_PAL_M, true, true));
    assert(g_mock.aiof_vmodesel == AM34_DIGAOS_VMODESEL_PAL_M);
    assert(g_mock.aiof_fs0 == AM34_DIGAOS_AIOF_FS0_NONSTD);
    assert(g_mock.aiof_yc3 == AM34_DIGAOS_AIOF_YC3_MVAGC_TBC);
    assert(g_mock.aiof_cont == AM34_DIGAOS_AIOF_CONT_0IRE);

    assert(digaos_shell_exec_line("help"));
    assert(digaos_shell_exec_line("palm"));
    assert(g_mock.aiof_fs0 == AM34_DIGAOS_AIOF_FS0_STD);
    assert(digaos_shell_exec_line("secam"));
    assert(g_mock.aiof_fs0 == AM34_DIGAOS_AIOF_FS0_NONSTD);
    assert(strstr(g_mock.uart_log, "[DIGAOS] TV=SECAM TBC=PROBE") != NULL);
    assert(digaos_shell_exec_line("tbc on"));
    assert(digaos_shell_exec_line("tbc off"));
    assert(digaos_shell_exec_line("ntsc"));
    assert(g_mock.aiof_fs0 == AM34_DIGAOS_AIOF_FS0_STD);
    assert(digaos_shell_exec_line("vfd READY"));
    assert(digaos_shell_exec_line("wdt"));
    assert(digaos_shell_exec_line("sata"));
    assert(digaos_shell_exec_line("status"));
    assert(digaos_shell_exec_line("menus"));
    assert(digaos_shell_exec_line("themes"));
    assert(digaos_shell_exec_line("power"));
    assert(digaos_shell_exec_line("targets"));
    assert(strstr(g_mock.uart_log, "UNVERIFIED_MMIO") != NULL);

    /* 5. Verify declarative SSOT tables & Multi-SoC fail-closed HAL guards */
    assert(DIGAOS_SSOT_MENU_COUNT >= 4u);
    assert(DIGAOS_SSOT_THEME_COUNT >= 3u);
    assert(DIGAOS_SSOT_POWER_PROFILE_COUNT >= 2u);
    assert(DIGAOS_SSOT_SOC_TARGET_COUNT >= 4u);
    for (size_t i = 0u; i < DIGAOS_SSOT_POWER_PROFILE_COUNT; ++i) {
        assert(k_digaos_ssot_power_profiles[i].lethal_mmio_guard == 1u);
        assert(k_digaos_ssot_power_profiles[i].ipl_ro_guard == 1u);
    }

    const char *hal_err = NULL;
    uint32_t aiof_val = 0u;
    assert(digaos_soc_read32("mn10300_am34", AM34_DIGAOS_AIOF_BASE, &aiof_val, &hal_err));
    assert(aiof_val == 0xA10F0001UL);
    assert(!digaos_soc_read32("mn10300_am34", 0xC000100CUL, &aiof_val, &hal_err));
    assert(strcmp(hal_err, "LETHAL_MMIO_LOCKED") == 0);
    assert(!digaos_soc_write32("mn10300_am34", 0x84000000UL, 0x11223344UL, &hal_err));
    assert(strcmp(hal_err, "IPL_RO_E05") == 0);

    static const char *const k_stub_socs[] = {
        "mn10300_am33_2",
        "arm_cortex_a9_uniphier",
        "arm64_cortex_a53_uniphier",
        "sparc_v8_domino",
    };
    for (size_t i = 0u; i < (sizeof(k_stub_socs) / sizeof(k_stub_socs[0])); ++i) {
        assert(!digaos_soc_read32(k_stub_socs[i], AM34_DIGAOS_AIOF_BASE, &aiof_val, &hal_err));
        assert(strcmp(hal_err, "UNVERIFIED_MMIO") == 0);
        assert(!digaos_soc_write32(k_stub_socs[i], AM34_DIGAOS_AIOF_BASE, 0u, &hal_err));
        assert(strcmp(hal_err, "UNVERIFIED_MMIO") == 0);
    }

    /* 6. Verify AVDD DAC + IIC1 IC3001 probe + colorbar */
    assert(digaos_ic3001_addr8(DIGAOS_VIDEO_IC_BH7629KS2_BR_NA, false) == 0x90u);
    assert(digaos_ic3001_addr8(DIGAOS_VIDEO_IC_BH7629KS2_BR_NA, true) == 0x92u);
    assert(digaos_ic3001_addr8(DIGAOS_VIDEO_IC_C1AB2379_EC, false) == 0x94u);
    assert(g_mock.avdd_dacout1 == AM34_DIGAOS_AVDD_DACOUT1_ENABLE);
    assert(!digaos_iic_write_reg_checked(0xC0001000UL, 0x90u, 0x00u, 0x00u));

    g_mock.iic1_write_count = 0u;
    assert(digaos_shell_exec_line("video ec"));
    assert(g_mock.avdd_npalctl0 == 0x0002U);
    assert(g_mock.avdd_clk == AM34_DIGAOS_AVDD_CLK_576I);
    assert(g_mock.avdd_active_size == AM34_DIGAOS_AVDD_ACTIVE_SIZE_576I);
    assert(g_mock.avdd_sync == AM34_DIGAOS_AVDD_SYNC_576I);
    assert(digaos_shell_exec_line("dac off"));
    assert(g_mock.avdd_voutmode0 == 0x0000U && g_mock.avdd_dacout1 == AM34_DIGAOS_AVDD_DACOUT1_MUTE);
    assert(digaos_shell_exec_line("video status"));
    assert(strstr(g_mock.uart_log, "state=MUTED") != NULL);
    assert(digaos_shell_exec_line("dac on"));
    assert(g_mock.avdd_voutmode0 == 0x0001U && g_mock.avdd_dacout1 == AM34_DIGAOS_AVDD_DACOUT1_ENABLE);
    assert(digaos_shell_exec_line("video status"));
    assert(strstr(g_mock.uart_log, "state=UNMUTED") != NULL);
    g_mock.iic0_write_count = 0u;
    assert(digaos_shell_exec_line("video br"));
    assert(g_mock.avdd_npalctl0 == 0x0000U);
    assert(g_mock.avdd_clk == AM34_DIGAOS_AVDD_CLK_480I);
    assert(g_mock.avdd_dacout1 == 0x0DDFU);
    static const uint8_t k_expected_iic0_av[12] = {
        0x90u, 0x10u, 0x08u, 0x07u, 0x63u, 0xC0u, 0x30u,
        0x84u, 0x03u, 0x80u, 0x00u, 0x22u
    };
    assert(g_mock.iic0_write_count == sizeof(k_expected_iic0_av));
    assert(memcmp(g_mock.iic0_writes, k_expected_iic0_av, sizeof(k_expected_iic0_av)) == 0);

    uint8_t poff81[16];
    uint8_t poff83[16];
    digaos_tbus_build_start_poff_frame(poff81);
    digaos_tbus_build_poff_bye_frame(poff83);
    assert(poff81[0] == 0x81u && poff81[2] == 0x01u && digaos_tbus_opcode_safe_for_br_ic7501(poff81[0]));
    assert(poff83[0] == 0x83u && digaos_tbus_opcode_safe_for_br_ic7501(poff83[0]));

    uint8_t probed = 0u;
    assert(!digaos_iic_probe_ic3001(DIGAOS_VIDEO_IC_BH7629KS2_BR_NA, NULL));
    assert(digaos_iic_probe_ic3001(DIGAOS_VIDEO_IC_BH7629KS2_BR_NA, &probed) && (probed == 0x90u));
    g_mock.iic1_nack_addr_a = 0x90u;
    assert(digaos_iic_probe_ic3001(DIGAOS_VIDEO_IC_BH7629KS2_BR_NA, &probed) && (probed == 0x92u));
    assert(digaos_iic_probe_ic3001(DIGAOS_VIDEO_IC_C1AB2379_EC, &probed) && (probed == 0x94u));
    g_mock.iic1_nack_all = true;
    assert(!digaos_iic_probe_ic3001(DIGAOS_VIDEO_IC_BH7629KS2_BR_NA, &probed));
    g_mock.iic1_nack_all = false;
    g_mock.iic1_nack_addr_a = 0u;

    assert(digaos_shell_exec_line("colorbar on"));
    assert(g_mock.avdd_voutmode0 == AM34_DIGAOS_AVDD_VOUTMODE0_COLORBAR);
    assert(digaos_shell_exec_line("colorbar off"));
    assert(g_mock.avdd_voutmode0 == AM34_DIGAOS_AVDD_VOUTMODE0_NORMAL);

    /* 7. Verify storage auto-sizer & provisioning stages */
    digaos_storage_plan_t plan;
    assert(digaos_storage_plan_compute(AM34_DIGAOS_MEIHFS_MIN_DISK_LBA, 0x00645048UL, 0x32873975UL, &plan));
    assert(digaos_storage_plan_compute(488281250UL, 0x00645048UL, 0x32873975UL, &plan));
    mock_prov_rec_t rec;
    memset(&rec, 0, sizeof(rec));
    rec.fail_on_stage = -1;
    digaos_provision_stage_t last_stage = DIGAOS_PROV_STAGE_M_FIRM;
    g_mock.tbus_vfd_count = 0u;
    assert(digaos_storage_provision_execute(&plan, mock_storage_write_cb, &rec, &last_stage));
    assert(last_stage == DIGAOS_PROV_STAGE_FINISH);
    assert(strcmp(g_mock.tbus_vfd_history[0], "M FIRM ") == 0);
    assert(strcmp(g_mock.tbus_vfd_history[4], "FINISH ") == 0);

    /* 8. Verify SDCD, BGO, SSOT hash output & interactive RX shell */
    g_mock.sdcd_ctrl = 0x00000001UL;
    g_mock.sdcd_cfg4 = 0x000000B8UL;
    g_mock.sa11_w0 = 0xFFFFFFFFUL;
    g_mock.sa11_magic_fc = 0xFFFFFFFFUL;
    g_mock.eemul_w0 = DIGAOS_SSOT_EEMUL_TAG_1284_LE;
    uint16_t p2_id = 0xF201u;
    for (size_t i = 0u; i < 45u; ++i) {
        g_mock.bgo_lbn[i] = (i >= 41u) ? 0xFFFFu : ((i == 3u) ? 0xF101u : p2_id++);
    }
    assert(digaos_shell_exec_line("sd"));
    assert(strstr(g_mock.uart_log, "[DIGAOS] SDCD bus=4BIT") != NULL);
    assert(digaos_shell_exec_line("bgo"));
    assert(strstr(g_mock.uart_log, "[DIGAOS] BGO p1_sa=29 p2=40 erased=4 sa11=FF eemul1284=1 nor_autoboot=READY") != NULL);
    assert(digaos_shell_exec_line("ssot"));
    assert(strstr(g_mock.uart_log, "[DIGAOS] SSOT=" DIGAOS_SSOT_CONTRACT_HASH_STR " IPL_RO=E05") != NULL);

    static const char k_rx_cmd[] = "sx\bd\r";
    memcpy(g_mock.rx_queue, k_rx_cmd, sizeof(k_rx_cmd) - 1u);
    g_mock.rx_head = 0u;
    g_mock.rx_tail = sizeof(k_rx_cmd) - 1u;
    digaos_shell_state_t sh;
    digaos_shell_init(&sh);
    bool ran_cmd = false;
    for (size_t i = 0u; i < 16u; ++i) {
        bool step_ran = false;
        (void)digaos_shell_poll_once(&sh, &step_ran);
        if (step_ran) {
            ran_cmd = true;
            break;
        }
    }
    assert(ran_cmd);
    for (uint32_t p = 0u; p < 65536UL; ++p) {
        (void)digaos_shell_poll_once(&sh, NULL);
    }
    assert(sh.keepalives_sent == 1UL);
    assert(!digaos_shell_exec_line("bogus"));

    /* 9. Verify M6930LB GPIO BH7629 bring-up, 3-Tier bootmap, and barrier-guarded peek/poke16/poke32 */
    assert(digaos_shell_exec_line("bootmap"));
    assert(strstr(g_mock.uart_log, "TIER2=NOR_SA11_SA14(0x84040000,256K,NO_SD_UART=ON)") != NULL);
    g_mock.iic1_write_count = 0u;
    assert(digaos_shell_exec_line("video gpio"));
    assert(g_mock.iic1_write_count == 0u);
    assert(g_mock.avdd_clk == AM34_DIGAOS_AVDD_CLK_480I);
    assert(g_mock.avdd_active_size == AM34_DIGAOS_AVDD_ACTIVE_SIZE_480I);
    assert(g_mock.avdd_sync == AM34_DIGAOS_AVDD_SYNC_480I);
    assert(g_mock.avdd_npalctl0_32 == AM34_DIGAOS_AVDD_NPALCTL0_HI_KEEP);
    assert(digaos_shell_exec_line("peek 0x88000000"));
    assert(strstr(g_mock.uart_log, "[DIGAOS] PEEK 0x88000000=0xA10F0001") != NULL);
    assert(digaos_shell_exec_line("poke16 0x88000000 0x8021"));
    assert(g_mock.aiof_vmodesel == 0x8021U);
    assert(!digaos_shell_exec_line("peek 0xC000100C"));
    assert(!digaos_shell_exec_line("poke32 0xC000100C 0x0"));
    assert(!digaos_shell_exec_line("poke32 0xDC0F0000 0x0"));
    assert(!digaos_shell_exec_line("poke32 0x80000000 0x12345678"));
    assert(!digaos_shell_exec_line("poke32 0x8403FFFC 0x12345678"));
    assert(!g_mock.forbidden_touched);

    /* 10. Verify auto-IC3001 fallback, TBUS RX non-fatal 0x7F, WakeupReason, ODD bypass, & 3410PQ helpers */
    digaos_board_video_ic_t auto_ic = DIGAOS_VIDEO_IC_C1AB2379_EC;
    g_mock.iic1_nack_all = true;
    assert(digaos_video_auto_bringup(DIGAOS_TV_NTSC, &auto_ic));
    assert(auto_ic == DIGAOS_VIDEO_IC_BH7629KS2_GPIO_M6930LB);
    assert(g_mock.avdd_clk == AM34_DIGAOS_AVDD_CLK_480I);
    assert(g_mock.avdd_dacout1 == AM34_DIGAOS_AVDD_DACOUT1_ENABLE);
    g_mock.iic1_nack_all = false;

    static const uint8_t rx_7f_e3[16] = {
        0x7Fu, 0xE3u, 0x02u, 0x30u, 0x46u, 0x46u, 0x30u, 0x30u,
        0x30u, 0x36u, 0x30u, 0x30u, 0x30u, 0x30u, 0x00u, 0xA6u
    };
    static const uint8_t rx_key_3d[16] = {
        0x40u, 0xB0u, 0x00u, 0x3Du, 0x00u, 0x00u, 0x00u, 0x00u,
        0x00u, 0x00u, 0x00u, 0x00u, 0x00u, 0x00u, 0x00u, 0x2Du
    };
    static const uint8_t rx_rtc_10[16] = {
        0x10u, 0x00u, 0x01u, 0x01u, 0x00u, 0x00u, 0x2Fu, 0x06u,
        0x00u, 0x02u, 0x00u, 0x00u, 0x00u, 0x00u, 0x00u, 0x49u
    };
    digaos_tbus_rx_event_t rx_ev;
    assert(digaos_tbus_classify_rx_frame(rx_7f_e3, &rx_ev));
    assert((rx_ev.kind == DIGAOS_TBUS_RX_UNSUPPORTED_0X7F) && (rx_ev.rejected_opcode == 0xE3u) && rx_ev.non_fatal);
    assert(digaos_tbus_classify_rx_frame(rx_key_3d, &rx_ev));
    assert((rx_ev.kind == DIGAOS_TBUS_RX_KEY_EVENT) && (rx_ev.key_code == 0x3Du));
    assert(digaos_tbus_classify_rx_frame(rx_rtc_10, &rx_ev));
    assert(rx_ev.kind == DIGAOS_TBUS_RX_RTC_HEARTBEAT);
    uint8_t bad_rx[16];
    memcpy(bad_rx, rx_7f_e3, sizeof(bad_rx));
    bad_rx[15] ^= 0x01u;
    assert(!digaos_tbus_classify_rx_frame(bad_rx, &rx_ev));
    assert(rx_ev.kind == DIGAOS_TBUS_RX_INVALID);

    assert(digaos_classify_wakeup_reason(AM34_DIGAOS_WAKEUP_XRESET_INVALID) == DIGAOS_WAKEUP_XRESET_70_HAZARD);
    assert(digaos_classify_wakeup_reason(0x41u) == DIGAOS_WAKEUP_NORMAL_UION);
    assert(digaos_classify_wakeup_reason(0x00u) == DIGAOS_WAKEUP_UNKNOWN);
    assert(digaos_shell_exec_line("fpc"));
    assert(strstr(g_mock.uart_log, "RX_0X7F=NON_FATAL WAKEUP=NORMAL_UION HAZARD=0x70") != NULL);

    g_mock.davis_ch0_status = 0x0051U; /* ERR bit set (ODD 80040904) */
    digaos_davis_status_t odd_st;
    assert(digaos_davis_probe_channel(false, &odd_st));
    assert((odd_st.state == DIGAOS_DAVIS_ERR_DEGRADED) && odd_st.degraded_odd_bypass);
    assert(digaos_shell_exec_line("odd"));
    assert(strstr(g_mock.uart_log, "[DIGAOS] ODD=DEGRADED_BYPASS status=0x0051") != NULL);

    assert(digaos_tv_from_main3410_cfg0117(AM34_DIGAOS_MAIN3410_CFG_ID0117_NTSC) == DIGAOS_TV_NTSC);
    assert(digaos_tv_from_main3410_cfg0117(AM34_DIGAOS_MAIN3410_CFG_ID0117_PAL) == DIGAOS_TV_PAL);
    digaos_tv_system_t avout_tv = DIGAOS_TV_PAL;
    assert(digaos_tv_from_avout_tv_system(0UL, &avout_tv) && (avout_tv == DIGAOS_TV_NTSC));
    assert(digaos_tv_from_avout_tv_system(1UL, &avout_tv) && (avout_tv == DIGAOS_TV_PAL));
    assert(!digaos_tv_from_avout_tv_system(28359UL, &avout_tv));
    assert(digaos_autoboot_stamp_matches(0xDC434247UL, 0xDC434247UL));
    assert(!digaos_autoboot_stamp_matches(0xDC434247UL, 0xEF7A4404UL));
    assert(!digaos_autoboot_stamp_matches(0UL, 0UL));
    assert(!digaos_autoboot_stamp_matches(0xFFFFFFFFUL, 0xFFFFFFFFUL));

    assert(digaos_tbus_send_frame_checked(poff81));
    assert(digaos_tbus_send_frame_checked(poff83));
    const uint8_t stream4[4] = { 0x03u, 0x80u, 0x00u, 0x22u };
    assert(digaos_iic_write_stream_checked(AM34_DIGAOS_IIC0_BASE, 0x84u, stream4, sizeof(stream4)));
    assert(!digaos_iic_write_stream_checked(0xC0001000UL, 0x84u, stream4, sizeof(stream4)));

    printf("[PASS] test_digaos_host: MMIO barriers, multi-SoC UNVERIFIED_MMIO stubs, SECAM/TBC, SSOT tables, storage, SDCD/BGO RO & DIGAOS> shell verified.\n");
    return 0;
}
