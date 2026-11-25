/**
 * @file digaos_main.c
 * @brief `DigaOS` (`M0 -> M3` Bring-Up + Declarative SSOT Menu/Theme/Power/Multi-SoC Shell) Entrypoint.
 *
 * HARDWARE-INTEROPERABILITY PROTOTYPE -- NOT LEGALLY CLEAN-ROOM.
 *
 * Absorbs `SW_GUARDS`, `DIGAOS_TV_SECAM`, and `probe_tbc_aiof_baseline` from retired `blackbox-fw`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "drivers/digaos_drivers.h"

#include <string.h>

typedef bool (*digaos_cmd_handler_fn)(const char *arg);

typedef struct digaos_shell_cmd {
    const char *name;
    bool has_arg;
    digaos_cmd_handler_fn handler;
} digaos_shell_cmd_t;

static void digaos_put_hex_u32(uint32_t val) {
    static const char k_hex[] = "0123456789ABCDEF";
    digaos_uart0_puts("0x");
    for (int shift = 28; shift >= 0; shift -= 4) {
        (void)digaos_uart0_putc(k_hex[(val >> (uint32_t)shift) & 0x0FUL]);
    }
}

static void digaos_put_hex_u16(uint16_t val) {
    static const char k_hex[] = "0123456789ABCDEF";
    digaos_uart0_puts("0x");
    for (int shift = 12; shift >= 0; shift -= 4) {
        (void)digaos_uart0_putc(k_hex[(val >> (uint16_t)shift) & 0x000FU]);
    }
}

static void digaos_put_u16_dec(uint16_t val) {
    char buf[8];
    size_t idx = 0u;
    if (val == 0u) {
        (void)digaos_uart0_putc('0');
        return;
    }
    uint16_t tmp = val;
    while ((tmp > 0u) && (idx < sizeof(buf))) {
        buf[idx++] = (char)('0' + (tmp % 10u));
        tmp = (uint16_t)(tmp / 10u);
    }
    while (idx > 0u) {
        (void)digaos_uart0_putc(buf[--idx]);
    }
}

static bool digaos_apply_standard_and_vfd(digaos_tv_system_t sys, const char *vfd_label, const char *msg) {
    const bool ok = digaos_aiof_apply_tbc_profile(sys, false, true);
    if (ok) {
        (void)digaos_tbus_send_vfd_text(vfd_label);
        digaos_uart0_puts(msg);
    }
    return ok;
}

static bool digaos_probe_log_step(digaos_board_video_ic_t ic, const char *addr8_label) {
    uint8_t detected_addr8 = 0u;
    if (!digaos_iic_probe_ic3001(ic, &detected_addr8)) {
        return false;
    }
    if (ic == DIGAOS_VIDEO_IC_C1AB2379_EC) {
        digaos_uart0_puts("[DIGAOS] IC3001_PROBE=C1AB2379_0x94\n");
    } else if (detected_addr8 == AM34_DIGAOS_IC3001_BR_BH7629_ADDR8_H) {
        digaos_uart0_puts("[DIGAOS] IC3001_PROBE=BH7629KS2_0x92\n");
    } else {
        digaos_uart0_puts(addr8_label);
    }
    return true;
}

static bool digaos_provision_noop_writer(
    uint32_t start_lba,
    uint32_t sector_count,
    uint32_t stage_tag,
    void *ctx
) {
    (void)start_lba;
    (void)sector_count;
    (void)stage_tag;
    (void)ctx;
    return true;
}

static bool cmd_help(const char *arg);

static bool cmd_status(const char *arg) {
    (void)arg;
    digaos_uart0_puts("[DIGAOS] HAL=AM34 IPL=RO_E05 VIDEO=AIOF+AVDD+IC3001 TBUS=BR_SAFE SATA=1000ms\n");
    digaos_uart0_puts("SW_GUARDS: 0xC000100C=LOCKED 0xDC0F0000=LOCKED IPL_RO=LOCKED\n");
    return true;
}

static bool cmd_ntsc(const char *arg) {
    (void)arg;
    return digaos_apply_standard_and_vfd(DIGAOS_TV_NTSC, "NTSC", "[DIGAOS] TV=NTSC TBC=ON\n");
}

static bool cmd_palm(const char *arg) {
    (void)arg;
    return digaos_apply_standard_and_vfd(DIGAOS_TV_PAL_M, "PAL-M", "[DIGAOS] TV=PAL-M TBC=ON\n");
}

static bool cmd_pal(const char *arg) {
    (void)arg;
    return digaos_apply_standard_and_vfd(DIGAOS_TV_PAL, "PAL", "[DIGAOS] TV=PAL TBC=ON\n");
}

static bool cmd_secam(const char *arg) {
    (void)arg;
    (void)probe_tbc_aiof_baseline(DIGAOS_TV_SECAM);
    return digaos_apply_standard_and_vfd(DIGAOS_TV_SECAM, "SECAM", "[DIGAOS] TV=SECAM TBC=PROBE\n");
}

static bool cmd_tbc_on(const char *arg) {
    (void)arg;
    const bool ok = digaos_aiof_apply_tbc_profile(DIGAOS_TV_NTSC, true, true);
    if (ok) {
        digaos_uart0_puts("[DIGAOS] TBC=NONSTD+MVAGC ON\n");
    }
    return ok;
}

static bool cmd_tbc_off(const char *arg) {
    (void)arg;
    const bool ok = digaos_aiof_apply_tbc_profile(DIGAOS_TV_NTSC, false, false);
    if (ok) {
        digaos_uart0_puts("[DIGAOS] TBC=BYPASS\n");
    }
    return ok;
}

static bool cmd_video_br(const char *arg) {
    (void)arg;
    const bool ok = digaos_video_bringup_board(
        DIGAOS_VIDEO_IC_BH7629KS2_BR_NA, DIGAOS_TV_NTSC, false, true
    );
    if (ok) {
        digaos_uart0_puts("[DIGAOS] VIDEO=BH7629KS2_0x90 NTSC\n");
    }
    return ok;
}

static bool cmd_video_ec(const char *arg) {
    (void)arg;
    const bool ok = digaos_video_bringup_board(
        DIGAOS_VIDEO_IC_C1AB2379_EC, DIGAOS_TV_PAL, false, true
    );
    if (ok) {
        digaos_uart0_puts("[DIGAOS] VIDEO=C1AB2379_0x94 PAL\n");
    }
    return ok;
}

static bool cmd_video_probe(const char *arg) {
    (void)arg;
    if (digaos_probe_log_step(DIGAOS_VIDEO_IC_BH7629KS2_BR_NA, "[DIGAOS] IC3001_PROBE=BH7629KS2_0x90\n")) {
        return true;
    }
    if (digaos_probe_log_step(DIGAOS_VIDEO_IC_C1AB2379_EC, "[DIGAOS] IC3001_PROBE=C1AB2379_0x94\n")) {
        return true;
    }
    digaos_uart0_puts("[DIGAOS] IC3001_PROBE=NONE\n");
    return false;
}

static bool cmd_dac_on(const char *arg) {
    (void)arg;
    const bool ok = digaos_avdd_configure_output(DIGAOS_TV_NTSC, true);
    if (ok) {
        digaos_uart0_puts("[DIGAOS] AVDD_DAC=ON\n");
    }
    return ok;
}

static bool cmd_dac_off(const char *arg) {
    (void)arg;
    const bool ok = digaos_avdd_configure_output(DIGAOS_TV_NTSC, false);
    if (ok) {
        digaos_uart0_puts("[DIGAOS] AVDD_DAC=MUTED\n");
    }
    return ok;
}

static bool cmd_colorbar_on(const char *arg) {
    (void)arg;
    const bool ok = digaos_avdd_set_colorbar(true);
    if (ok) {
        digaos_uart0_puts("[DIGAOS] AVDD_COLORBAR=ON\n");
    }
    return ok;
}

static bool cmd_colorbar_off(const char *arg) {
    (void)arg;
    const bool ok = digaos_avdd_set_colorbar(false);
    if (ok) {
        digaos_uart0_puts("[DIGAOS] AVDD_COLORBAR=OFF\n");
    }
    return ok;
}

static bool cmd_provision(const char *arg) {
    (void)arg;
    digaos_storage_plan_t plan;
    digaos_provision_stage_t last_stage = DIGAOS_PROV_STAGE_M_FIRM;
    if (!digaos_storage_plan_compute(488281250UL, 0x00645048UL, 0x32873975UL, &plan)) {
        return false;
    }
    if (!digaos_storage_provision_execute(&plan, digaos_provision_noop_writer, NULL, &last_stage)) {
        return false;
    }
    digaos_uart0_puts("[DIGAOS] PROVISION PLAN OK\n");
    return true;
}

static bool cmd_sd(const char *arg) {
    (void)arg;
    digaos_sdcd_status_t sd;
    if (!digaos_sdcd_probe_status(&sd)) {
        return false;
    }
    digaos_uart0_puts("[DIGAOS] SDCD bus=");
    digaos_uart0_puts(sd.bus_width_4bit ? "4BIT" : "1BIT");
    digaos_uart0_puts(" ctrl=");
    digaos_put_hex_u32(sd.ctrl_reg);
    digaos_uart0_puts(" cfg=");
    digaos_put_hex_u32(sd.bus_cfg_reg);
    digaos_uart0_putc('\n');
    return true;
}

static bool cmd_bgo(const char *arg) {
    (void)arg;
    digaos_unbrick_status_t st;
    if (!digaos_nor_inspect_unbrick(&st)) {
        return false;
    }
    digaos_uart0_puts("[DIGAOS] BGO p1_sa=");
    if (st.bgo_p1_sa == 0xFFFFu) {
        digaos_uart0_puts("NONE");
    } else {
        digaos_put_u16_dec(st.bgo_p1_sa);
    }
    digaos_uart0_puts(" p2=");
    digaos_put_u16_dec(st.bgo_p2_count);
    digaos_uart0_puts(" erased=");
    digaos_put_u16_dec(st.bgo_erased_count);
    digaos_uart0_puts(" sa11=");
    digaos_uart0_puts(st.sa11_is_erased ? "FF" : (st.sa11_has_autoboot_capsule ? "NAUT" : "USED"));
    digaos_uart0_puts(" eemul1284=");
    digaos_uart0_puts(st.eemul_has_1284 ? "1" : "0");
    digaos_uart0_puts(" nor_autoboot=");
    digaos_uart0_puts(st.nor_autoboot_ready ? "READY\n" : "NO\n");
    return true;
}

static bool cmd_ssot(const char *arg) {
    (void)arg;
    digaos_uart0_puts("[DIGAOS] SSOT=" DIGAOS_SSOT_CONTRACT_HASH_STR " IPL_RO=E05\n");
    return true;
}

static bool cmd_menus(const char *arg) {
    (void)arg;
    digaos_uart0_puts("[DIGAOS] MENUS count=");
    digaos_put_u16_dec((uint16_t)DIGAOS_SSOT_MENU_COUNT);
    digaos_uart0_putc('\n');
    for (size_t i = 0u; i < DIGAOS_SSOT_MENU_COUNT; ++i) {
        const digaos_menu_t *m = &k_digaos_ssot_menus[i];
        digaos_uart0_puts("  menu=");
        digaos_uart0_puts(m->id);
        digaos_uart0_puts(" items=");
        digaos_put_u16_dec(m->item_count);
        digaos_uart0_putc('\n');
    }
    return true;
}

static bool cmd_themes(const char *arg) {
    (void)arg;
    for (size_t i = 0u; i < DIGAOS_SSOT_THEME_COUNT; ++i) {
        const digaos_theme_t *th = &k_digaos_ssot_themes[i];
        digaos_uart0_puts("[DIGAOS] THEME=");
        digaos_uart0_puts(th->id);
        digaos_uart0_puts(" bg=");
        digaos_put_hex_u16(th->bg_rgb565);
        digaos_uart0_puts(" fg=");
        digaos_put_hex_u16(th->fg_rgb565);
        digaos_uart0_puts(" contrast=");
        digaos_put_u16_dec((uint16_t)th->contrast_level);
        digaos_uart0_putc('\n');
    }
    return true;
}

static bool cmd_power(const char *arg) {
    (void)arg;
    for (size_t i = 0u; i < DIGAOS_SSOT_POWER_PROFILE_COUNT; ++i) {
        const digaos_power_profile_t *pp = &k_digaos_ssot_power_profiles[i];
        digaos_uart0_puts("[DIGAOS] POWER=");
        digaos_uart0_puts(pp->id);
        digaos_uart0_puts(" core=");
        digaos_uart0_puts(pp->soc_core_id);
        digaos_uart0_puts(" lethal_guard=");
        digaos_uart0_puts(pp->lethal_mmio_guard ? "1" : "0");
        digaos_uart0_puts(" ipl_ro=");
        digaos_uart0_puts(pp->ipl_ro_guard ? "1\n" : "0\n");
    }
    return true;
}

static bool cmd_targets(const char *arg) {
    (void)arg;
    for (size_t i = 0u; i < DIGAOS_SSOT_SOC_TARGET_COUNT; ++i) {
        const digaos_soc_target_t *t = &k_digaos_ssot_soc_targets[i];
        digaos_uart0_puts("[DIGAOS] TARGET=");
        digaos_uart0_puts(t->soc_core_id);
        digaos_uart0_puts(" isa=");
        digaos_uart0_puts(t->isa);
        digaos_uart0_puts(" state=");
        digaos_uart0_puts((t->mmio_verified && t->is_primary_executable) ? "ACTIVE_EXEC\n" : "UNVERIFIED_MMIO\n");
    }
    return true;
}

static bool cmd_wdt(const char *arg) {
    (void)arg;
    const bool ok = digaos_tbus_send_keepalive_255s();
    if (ok) {
        digaos_uart0_puts("[DIGAOS] TBUS 255s OK\n");
    }
    return ok;
}

static bool cmd_sata(const char *arg) {
    (void)arg;
    uint32_t waited_ms = 0u;
    const bool ok = digaos_davis_reset_and_wait_ready(true, &waited_ms);
    if (ok && (waited_ms == DIGAOS_SATA_BRIDGE_WAIT_MS)) {
        digaos_uart0_puts("[DIGAOS] DAVIS CH1 SATA OK\n");
        return true;
    }
    return false;
}

static bool cmd_vfd(const char *arg) {
    const bool ok = digaos_tbus_send_vfd_text(arg);
    if (ok) {
        digaos_uart0_puts("[DIGAOS] VFD OK\n");
    }
    return ok;
}

static const digaos_shell_cmd_t k_shell_cmds[] = {
    { "help",         false, cmd_help },
    { "status",       false, cmd_status },
    { "ntsc",         false, cmd_ntsc },
    { "palm",         false, cmd_palm },
    { "pal",          false, cmd_pal },
    { "secam",        false, cmd_secam },
    { "tbc on",       false, cmd_tbc_on },
    { "tbc off",      false, cmd_tbc_off },
    { "video br",     false, cmd_video_br },
    { "video ec",     false, cmd_video_ec },
    { "video probe",  false, cmd_video_probe },
    { "dac on",       false, cmd_dac_on },
    { "dac off",      false, cmd_dac_off },
    { "colorbar on",  false, cmd_colorbar_on },
    { "colorbar off", false, cmd_colorbar_off },
    { "provision",    false, cmd_provision },
    { "sd",           false, cmd_sd },
    { "bgo",          false, cmd_bgo },
    { "ssot",         false, cmd_ssot },
    { "menus",        false, cmd_menus },
    { "themes",       false, cmd_themes },
    { "power",        false, cmd_power },
    { "targets",      false, cmd_targets },
    { "wdt",          false, cmd_wdt },
    { "sata",         false, cmd_sata },
    { "vfd ",         true,  cmd_vfd },
};

static bool cmd_help(const char *arg) {
    (void)arg;
    digaos_uart0_puts("[DIGAOS] cmds:");
    for (size_t i = 0u; i < (sizeof(k_shell_cmds) / sizeof(k_shell_cmds[0])); ++i) {
        (void)digaos_uart0_putc(' ');
        digaos_uart0_puts(k_shell_cmds[i].name);
        if (k_shell_cmds[i].has_arg) {
            digaos_uart0_puts("<7ch>");
        }
    }
    (void)digaos_uart0_putc('\n');
    return true;
}

bool digaos_shell_exec_line(const char *line) {
    if ((line == NULL) || (*line == '\0')) {
        digaos_uart0_puts("[DIGAOS] ERR unknown cmd\n");
        return false;
    }
    for (size_t i = 0u; i < (sizeof(k_shell_cmds) / sizeof(k_shell_cmds[0])); ++i) {
        const digaos_shell_cmd_t *cmd = &k_shell_cmds[i];
        if (cmd->has_arg) {
            const size_t prefix_len = strlen(cmd->name);
            if (strncmp(line, cmd->name, prefix_len) == 0) {
                return cmd->handler(line + prefix_len);
            }
        } else if (strcmp(line, cmd->name) == 0) {
            return cmd->handler(NULL);
        }
    }
    digaos_uart0_puts("[DIGAOS] ERR unknown cmd\n");
    return false;
}

#define DIGAOS_SHELL_IDLE_KEEPALIVE_POLLS 65536UL

void digaos_shell_init(digaos_shell_state_t *st) {
    if (st == NULL) {
        return;
    }
    memset(st, 0, sizeof(*st));
}

bool digaos_shell_poll_once(digaos_shell_state_t *st, bool *out_cmd_ran) {
    if (out_cmd_ran != NULL) {
        *out_cmd_ran = false;
    }
    if (st == NULL) {
        return false;
    }
    char ch = '\0';
    if (digaos_uart0_try_getc(&ch)) {
        st->idle_ticks = 0UL;
        if ((ch == '\r') || (ch == '\n')) {
            (void)digaos_uart0_putc('\n');
            if (st->len > 0u) {
                st->line[st->len] = '\0';
                (void)digaos_shell_exec_line(st->line);
                st->len = 0u;
                st->line[0] = '\0';
                if (out_cmd_ran != NULL) {
                    *out_cmd_ran = true;
                }
            }
            digaos_uart0_puts(DIGAOS_PROMPT " ");
        } else if ((ch == '\b') || ((uint8_t)ch == 0x7Fu)) {
            if (st->len > 0u) {
                st->len -= 1u;
                st->line[st->len] = '\0';
                digaos_uart0_puts("\b \b");
            }
        } else if (((uint8_t)ch >= 0x20u) && ((uint8_t)ch <= 0x7Eu) &&
                   ((st->len + 1u) < DIGAOS_SHELL_LINE_CAP)) {
            st->line[st->len] = ch;
            st->len += 1u;
            st->line[st->len] = '\0';
            (void)digaos_uart0_putc(ch);
        }
        return true;
    }

    st->idle_ticks += 1UL;
    if (st->idle_ticks >= DIGAOS_SHELL_IDLE_KEEPALIVE_POLLS) {
        st->idle_ticks = 0UL;
        if (digaos_tbus_send_keepalive_255s()) {
            st->keepalives_sent += 1UL;
        }
    }
    return false;
}

void digaos_kernel_entry(void) {
    digaos_uart0_init();
    digaos_uart0_puts("\n[DIGAOS] Booting DigaOS in SDRAM (SA0..SA10 IPL RO E05)\n");
    digaos_uart0_puts("SW_GUARDS: 0xC000100C=LOCKED 0xDC0F0000=LOCKED IPL_RO=LOCKED\n");
    (void)probe_tbc_aiof_baseline(DIGAOS_TV_NTSC);

    const bool tbus_ok = digaos_tbus_send_keepalive_255s();
    const bool vfd_ok = digaos_tbus_send_vfd_text("DIGAOS");
    const bool video_ok = digaos_video_bringup_board(
        DIGAOS_VIDEO_IC_BH7629KS2_BR_NA, DIGAOS_TV_NTSC, false, true
    );

    uint32_t ch1_wait_ms = 0u;
    const bool davis_ok = digaos_davis_reset_and_wait_ready(true, &ch1_wait_ms);

    if (tbus_ok && vfd_ok && video_ok && davis_ok && (ch1_wait_ms == DIGAOS_SATA_BRIDGE_WAIT_MS)) {
        digaos_uart0_puts("[DIGAOS] M0..M3 bring-up OK (TV=NTSC, SATA_WAIT=1000ms, IPL=RO_E05)\n");
    } else {
        digaos_uart0_puts("[DIGAOS] WARN: Peripheral bring-up incomplete\n");
    }
    digaos_uart0_puts(DIGAOS_PROMPT " ");
}
