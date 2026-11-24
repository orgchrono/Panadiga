/**
 * @file openos_main.c
 * @brief Panadiga OpenOS (`M0 -> M3` Bring-Up) `DIAG_PROG_RAM` / `MAIN_OS_RAM` Declarative Kernel Entrypoint.
 *
 * HARDWARE-INTEROPERABILITY PROTOTYPE -- NOT LEGALLY CLEAN-ROOM.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "drivers/openos_drivers.h"

#include <string.h>

typedef bool (*openos_cmd_handler_fn)(const char *arg);

typedef struct openos_shell_cmd {
    const char *name;
    bool has_arg;
    openos_cmd_handler_fn handler;
} openos_shell_cmd_t;

static bool openos_apply_standard_and_vfd(openos_tv_system_t sys, const char *vfd_label, const char *msg) {
    const bool ok = openos_aiof_apply_tbc_profile(sys, false, true);
    if (ok) {
        (void)openos_tbus_send_vfd_text(vfd_label);
        openos_uart0_puts(msg);
    }
    return ok;
}

static bool openos_probe_log_step(openos_board_video_ic_t ic, const char *addr8_label) {
    uint8_t detected_addr8 = 0u;
    if (!openos_iic_probe_ic3001(ic, &detected_addr8)) {
        return false;
    }
    if (ic == OPENOS_VIDEO_IC_C1AB2379_EC) {
        openos_uart0_puts("[OPENOS] IC3001_PROBE=C1AB2379_0x94\n");
    } else if (detected_addr8 == AM34_OPENOS_IC3001_BR_BH7629_ADDR8_H) {
        openos_uart0_puts("[OPENOS] IC3001_PROBE=BH7629KS2_0x92\n");
    } else {
        openos_uart0_puts(addr8_label);
    }
    return true;
}

static bool openos_provision_noop_writer(
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
    openos_uart0_puts("[OPENOS] HAL=AM34 IPL=RO_E05 VIDEO=AIOF+AVDD+IC3001 TBUS=BR_SAFE SATA=1000ms\n");
    return true;
}

static bool cmd_ntsc(const char *arg) {
    (void)arg;
    return openos_apply_standard_and_vfd(OPENOS_TV_NTSC, "NTSC", "[OPENOS] TV=NTSC TBC=ON\n");
}

static bool cmd_palm(const char *arg) {
    (void)arg;
    return openos_apply_standard_and_vfd(OPENOS_TV_PAL_M, "PAL-M", "[OPENOS] TV=PAL-M TBC=ON\n");
}

static bool cmd_pal(const char *arg) {
    (void)arg;
    return openos_apply_standard_and_vfd(OPENOS_TV_PAL, "PAL", "[OPENOS] TV=PAL TBC=ON\n");
}

static bool cmd_tbc_on(const char *arg) {
    (void)arg;
    const bool ok = openos_aiof_apply_tbc_profile(OPENOS_TV_NTSC, true, true);
    if (ok) {
        openos_uart0_puts("[OPENOS] TBC=NONSTD+MVAGC ON\n");
    }
    return ok;
}

static bool cmd_tbc_off(const char *arg) {
    (void)arg;
    const bool ok = openos_aiof_apply_tbc_profile(OPENOS_TV_NTSC, false, false);
    if (ok) {
        openos_uart0_puts("[OPENOS] TBC=BYPASS\n");
    }
    return ok;
}

static bool cmd_video_br(const char *arg) {
    (void)arg;
    const bool ok = openos_video_bringup_board(
        OPENOS_VIDEO_IC_BH7629KS2_BR_NA, OPENOS_TV_NTSC, false, true
    );
    if (ok) {
        openos_uart0_puts("[OPENOS] VIDEO=BH7629KS2_0x90 NTSC\n");
    }
    return ok;
}

static bool cmd_video_ec(const char *arg) {
    (void)arg;
    const bool ok = openos_video_bringup_board(
        OPENOS_VIDEO_IC_C1AB2379_EC, OPENOS_TV_PAL, false, true
    );
    if (ok) {
        openos_uart0_puts("[OPENOS] VIDEO=C1AB2379_0x94 PAL\n");
    }
    return ok;
}

static bool cmd_video_probe(const char *arg) {
    (void)arg;
    if (openos_probe_log_step(OPENOS_VIDEO_IC_BH7629KS2_BR_NA, "[OPENOS] IC3001_PROBE=BH7629KS2_0x90\n")) {
        return true;
    }
    if (openos_probe_log_step(OPENOS_VIDEO_IC_C1AB2379_EC, "[OPENOS] IC3001_PROBE=C1AB2379_0x94\n")) {
        return true;
    }
    openos_uart0_puts("[OPENOS] IC3001_PROBE=NONE\n");
    return false;
}

static bool cmd_dac_on(const char *arg) {
    (void)arg;
    const bool ok = openos_avdd_configure_output(OPENOS_TV_NTSC, true);
    if (ok) {
        openos_uart0_puts("[OPENOS] AVDD_DAC=ON\n");
    }
    return ok;
}

static bool cmd_dac_off(const char *arg) {
    (void)arg;
    const bool ok = openos_avdd_configure_output(OPENOS_TV_NTSC, false);
    if (ok) {
        openos_uart0_puts("[OPENOS] AVDD_DAC=MUTED\n");
    }
    return ok;
}

static bool cmd_colorbar_on(const char *arg) {
    (void)arg;
    const bool ok = openos_avdd_set_colorbar(true);
    if (ok) {
        openos_uart0_puts("[OPENOS] AVDD_COLORBAR=ON\n");
    }
    return ok;
}

static bool cmd_colorbar_off(const char *arg) {
    (void)arg;
    const bool ok = openos_avdd_set_colorbar(false);
    if (ok) {
        openos_uart0_puts("[OPENOS] AVDD_COLORBAR=OFF\n");
    }
    return ok;
}

static bool cmd_provision(const char *arg) {
    (void)arg;
    openos_storage_plan_t plan;
    openos_provision_stage_t last_stage = OPENOS_PROV_STAGE_M_FIRM;
    if (!openos_storage_plan_compute(488281250UL, 0x00645048UL, 0x32873975UL, &plan)) {
        return false;
    }
    if (!openos_storage_provision_execute(&plan, openos_provision_noop_writer, NULL, &last_stage)) {
        return false;
    }
    openos_uart0_puts("[OPENOS] PROVISION PLAN OK\n");
    return true;
}

static void openos_put_hex_u32(uint32_t val) {
    static const char k_hex[] = "0123456789ABCDEF";
    openos_uart0_puts("0x");
    for (int shift = 28; shift >= 0; shift -= 4) {
        (void)openos_uart0_putc(k_hex[(val >> (uint32_t)shift) & 0x0FUL]);
    }
}

static void openos_put_u16_dec(uint16_t val) {
    char buf[8];
    size_t idx = 0u;
    if (val == 0u) {
        (void)openos_uart0_putc('0');
        return;
    }
    uint16_t tmp = val;
    while ((tmp > 0u) && (idx < sizeof(buf))) {
        buf[idx++] = (char)('0' + (tmp % 10u));
        tmp = (uint16_t)(tmp / 10u);
    }
    while (idx > 0u) {
        (void)openos_uart0_putc(buf[--idx]);
    }
}

static bool cmd_sd(const char *arg) {
    (void)arg;
    openos_sdcd_status_t sd;
    if (!openos_sdcd_probe_status(&sd)) {
        return false;
    }
    openos_uart0_puts("[OPENOS] SDCD bus=");
    openos_uart0_puts(sd.bus_width_4bit ? "4BIT" : "1BIT");
    openos_uart0_puts(" ctrl=");
    openos_put_hex_u32(sd.ctrl_reg);
    openos_uart0_puts(" cfg=");
    openos_put_hex_u32(sd.bus_cfg_reg);
    openos_uart0_putc('\n');
    return true;
}

static bool cmd_bgo(const char *arg) {
    (void)arg;
    openos_unbrick_status_t st;
    if (!openos_nor_inspect_unbrick(&st)) {
        return false;
    }
    openos_uart0_puts("[OPENOS] BGO p1_sa=");
    if (st.bgo_p1_sa == 0xFFFFu) {
        openos_uart0_puts("NONE");
    } else {
        openos_put_u16_dec(st.bgo_p1_sa);
    }
    openos_uart0_puts(" p2=");
    openos_put_u16_dec(st.bgo_p2_count);
    openos_uart0_puts(" erased=");
    openos_put_u16_dec(st.bgo_erased_count);
    openos_uart0_puts(" sa11=");
    openos_uart0_puts(st.sa11_is_erased ? "FF" : (st.sa11_has_autoboot_capsule ? "NAUT" : "USED"));
    openos_uart0_puts(" eemul1284=");
    openos_uart0_puts(st.eemul_has_1284 ? "1" : "0");
    openos_uart0_puts(" nor_autoboot=");
    openos_uart0_puts(st.nor_autoboot_ready ? "READY\n" : "NO\n");
    return true;
}

static bool cmd_ssot(const char *arg) {
    (void)arg;
    openos_uart0_puts("[OPENOS] SSOT=" OPENOS_SSOT_CONTRACT_HASH_STR " IPL_RO=E05\n");
    return true;
}

static bool cmd_wdt(const char *arg) {
    (void)arg;
    const bool ok = openos_tbus_send_keepalive_255s();
    if (ok) {
        openos_uart0_puts("[OPENOS] TBUS 255s OK\n");
    }
    return ok;
}

static bool cmd_sata(const char *arg) {
    (void)arg;
    uint32_t waited_ms = 0u;
    const bool ok = openos_davis_reset_and_wait_ready(true, &waited_ms);
    if (ok && (waited_ms == OPENOS_SATA_BRIDGE_WAIT_MS)) {
        openos_uart0_puts("[OPENOS] DAVIS CH1 SATA OK\n");
        return true;
    }
    return false;
}

static bool cmd_vfd(const char *arg) {
    const bool ok = openos_tbus_send_vfd_text(arg);
    if (ok) {
        openos_uart0_puts("[OPENOS] VFD OK\n");
    }
    return ok;
}

static const openos_shell_cmd_t k_shell_cmds[] = {
    { "help",         false, cmd_help },
    { "status",       false, cmd_status },
    { "ntsc",         false, cmd_ntsc },
    { "palm",         false, cmd_palm },
    { "pal",          false, cmd_pal },
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
    { "wdt",          false, cmd_wdt },
    { "sata",         false, cmd_sata },
    { "vfd ",         true,  cmd_vfd },
};

static bool cmd_help(const char *arg) {
    (void)arg;
    openos_uart0_puts("[OPENOS] cmds:");
    for (size_t i = 0u; i < (sizeof(k_shell_cmds) / sizeof(k_shell_cmds[0])); ++i) {
        openos_uart0_putc(' ');
        openos_uart0_puts(k_shell_cmds[i].name);
        if (k_shell_cmds[i].has_arg) {
            openos_uart0_puts("<7ch>");
        }
    }
    openos_uart0_putc('\n');
    return true;
}

bool openos_shell_exec_line(const char *line) {
    if ((line == NULL) || (*line == '\0')) {
        openos_uart0_puts("[OPENOS] ERR unknown cmd\n");
        return false;
    }
    for (size_t i = 0u; i < (sizeof(k_shell_cmds) / sizeof(k_shell_cmds[0])); ++i) {
        const openos_shell_cmd_t *cmd = &k_shell_cmds[i];
        if (cmd->has_arg) {
            const size_t prefix_len = strlen(cmd->name);
            if (strncmp(line, cmd->name, prefix_len) == 0) {
                return cmd->handler(line + prefix_len);
            }
        } else if (strcmp(line, cmd->name) == 0) {
            return cmd->handler(NULL);
        }
    }
    openos_uart0_puts("[OPENOS] ERR unknown cmd\n");
    return false;
}

#define OPENOS_SHELL_IDLE_KEEPALIVE_POLLS 65536UL

void openos_shell_init(openos_shell_state_t *st) {
    if (st == NULL) {
        return;
    }
    memset(st, 0, sizeof(*st));
}

bool openos_shell_poll_once(openos_shell_state_t *st, bool *out_cmd_ran) {
    if (out_cmd_ran != NULL) {
        *out_cmd_ran = false;
    }
    if (st == NULL) {
        return false;
    }
    char ch = '\0';
    if (openos_uart0_try_getc(&ch)) {
        st->idle_ticks = 0UL;
        if ((ch == '\r') || (ch == '\n')) {
            openos_uart0_putc('\n');
            if (st->len > 0u) {
                st->line[st->len] = '\0';
                (void)openos_shell_exec_line(st->line);
                st->len = 0u;
                st->line[0] = '\0';
                if (out_cmd_ran != NULL) {
                    *out_cmd_ran = true;
                }
            }
            openos_uart0_puts("OPENOS> ");
        } else if ((ch == '\b') || ((uint8_t)ch == 0x7Fu)) {
            if (st->len > 0u) {
                st->len -= 1u;
                st->line[st->len] = '\0';
                openos_uart0_puts("\b \b");
            }
        } else if (((uint8_t)ch >= 0x20u) && ((uint8_t)ch <= 0x7Eu) &&
                   ((st->len + 1u) < OPENOS_SHELL_LINE_CAP)) {
            st->line[st->len] = ch;
            st->len += 1u;
            st->line[st->len] = '\0';
            (void)openos_uart0_putc(ch);
        }
        return true;
    }

    st->idle_ticks += 1UL;
    if (st->idle_ticks >= OPENOS_SHELL_IDLE_KEEPALIVE_POLLS) {
        st->idle_ticks = 0UL;
        if (openos_tbus_send_keepalive_255s()) {
            st->keepalives_sent += 1UL;
        }
    }
    return false;
}

void openos_kernel_entry(void) {
    openos_uart0_init();
    openos_uart0_puts("\n[OPENOS] Booting Panadiga OpenOS in SDRAM (SA0..SA10 IPL RO E05)\n");

    const bool tbus_ok = openos_tbus_send_keepalive_255s();
    const bool vfd_ok = openos_tbus_send_vfd_text("OPENOS");
    const bool video_ok = openos_video_bringup_board(
        OPENOS_VIDEO_IC_BH7629KS2_BR_NA, OPENOS_TV_NTSC, false, true
    );

    uint32_t ch1_wait_ms = 0u;
    const bool davis_ok = openos_davis_reset_and_wait_ready(true, &ch1_wait_ms);

    if (tbus_ok && vfd_ok && video_ok && davis_ok && (ch1_wait_ms == OPENOS_SATA_BRIDGE_WAIT_MS)) {
        openos_uart0_puts("[OPENOS] M0..M3 bring-up OK (TV=NTSC, SATA_WAIT=1000ms, IPL=RO_E05)\n");
    } else {
        openos_uart0_puts("[OPENOS] WARN: Peripheral bring-up incomplete\n");
    }
    openos_uart0_puts("OPENOS> ");
}
