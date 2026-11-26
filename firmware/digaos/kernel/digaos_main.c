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
    const char *arg_hint;
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

static bool digaos_parse_hex_u32(const char *s, uint32_t *out_val, const char **out_rest) {
    if ((s == NULL) || (out_val == NULL)) {
        return false;
    }
    while (*s == ' ') {
        ++s;
    }
    if ((s[0] == '0') && ((s[1] == 'x') || (s[1] == 'X'))) {
        s += 2;
    }
    uint32_t acc = 0UL;
    size_t digits = 0u;
    while (*s != '\0' && *s != ' ') {
        const char ch = *s;
        uint32_t nib = 0UL;
        if ((ch >= '0') && (ch <= '9')) {
            nib = (uint32_t)(ch - '0');
        } else if ((ch >= 'a') && (ch <= 'f')) {
            nib = 10UL + (uint32_t)(ch - 'a');
        } else if ((ch >= 'A') && (ch <= 'F')) {
            nib = 10UL + (uint32_t)(ch - 'A');
        } else {
            return false;
        }
        if (digits >= 8u) {
            return false;
        }
        acc = (acc << 4u) | nib;
        ++digits;
        ++s;
    }
    if (digits == 0u) {
        return false;
    }
    *out_val = acc;
    if (out_rest != NULL) {
        while (*s == ' ') {
            ++s;
        }
        *out_rest = s;
    }
    return true;
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

static bool cmd_video_gpio(const char *arg) {
    (void)arg;
    const bool ok = digaos_video_bringup_board(
        DIGAOS_VIDEO_IC_BH7629KS2_GPIO_M6930LB, DIGAOS_TV_NTSC, false, true
    );
    if (ok) {
        digaos_uart0_puts("[DIGAOS] VIDEO=BH7629KS2_GPIO_M6930LB NTSC_525_60 XTAL=3579545Hz\n");
    }
    return ok;
}

static bool cmd_bootmap(const char *arg) {
    (void)arg;
    digaos_uart0_puts("[DIGAOS] BOOTMAP TIER1=SD_DIGADIAG(0x50400000) TIER2=NOR_SA11_SA14(0x84040000,256K,NO_SD_UART=ON) TIER3=SATA_HDD(0x57000000,DUALBOOT)\n");
    digaos_uart0_puts("[DIGAOS] M6930LB XTAL_NTSC=3579545Hz XTAL_PALM=3575611Hz P7501=115200_8N1 NSW5V=HIZ_INPUT\n");
    return true;
}

static bool cmd_video_status(const char *arg) {
    (void)arg;
    digaos_video_status_t vst;
    if (!digaos_video_inspect_status(&vst)) {
        return false;
    }
    digaos_uart0_puts("[DIGAOS] VIDEO_STATUS aiof=");
    digaos_put_hex_u16(vst.aiof_vmodesel);
    digaos_uart0_puts(" cont=");
    digaos_put_hex_u16(vst.aiof_cont);
    digaos_uart0_puts(" clk=");
    digaos_put_hex_u16(vst.avdd_clk);
    digaos_uart0_puts(" dac=");
    digaos_put_hex_u16(vst.avdd_dacout1);
    digaos_uart0_puts(" size=");
    digaos_put_hex_u32(vst.avdd_active_size);
    digaos_uart0_puts(vst.dac_unmuted ? " state=UNMUTED\n" : " state=MUTED\n");
    return true;
}

static bool cmd_odd(const char *arg) {
    (void)arg;
    digaos_davis_status_t st;
    if (!digaos_davis_probe_channel(false, &st)) {
        return false;
    }
    digaos_uart0_puts("[DIGAOS] ODD=");
    if (st.state == DIGAOS_DAVIS_READY) {
        digaos_uart0_puts("READY");
    } else if (st.degraded_odd_bypass) {
        digaos_uart0_puts("DEGRADED_BYPASS");
    } else {
        digaos_uart0_puts("FAULT");
    }
    digaos_uart0_puts(" status=");
    digaos_put_hex_u16(st.status_word);
    digaos_uart0_putc('\n');
    return true;
}

static bool cmd_fpc(const char *arg) {
    (void)arg;
    digaos_uart0_puts("[DIGAOS] FPC=MN67788RA TX_BLOCKED=13(0xE3,0xF9,0xA9,0xFA) RX_0X7F=NON_FATAL WAKEUP=NORMAL_UION HAZARD=0x70 WDT=255s\n");
    return true;
}

static bool cmd_peek(const char *arg) {
    uint32_t addr = 0UL;
    const char *rest = NULL;
    if (!digaos_parse_hex_u32(arg, &addr, &rest) || (*rest != '\0')) {
        digaos_uart0_puts("[DIGAOS] ERR syntax: peek <addr_hex>\n");
        return false;
    }
    if (am34_mmio_is_forbidden(addr)) {
        digaos_uart0_puts("[DIGAOS] ERR BARRIER_LOCKED\n");
        return false;
    }
    if ((addr & 3UL) == 0UL) {
        uint32_t v32 = 0UL;
        if (!am34_mmio_read32(addr, &v32)) {
            digaos_uart0_puts("[DIGAOS] ERR BARRIER_LOCKED\n");
            return false;
        }
        digaos_uart0_puts("[DIGAOS] PEEK ");
        digaos_put_hex_u32(addr);
        digaos_uart0_putc('=');
        digaos_put_hex_u32(v32);
        digaos_uart0_putc('\n');
        return true;
    }
    if ((addr & 1UL) == 0UL) {
        uint16_t v16 = 0U;
        if (!am34_mmio_read16(addr, &v16)) {
            digaos_uart0_puts("[DIGAOS] ERR BARRIER_LOCKED\n");
            return false;
        }
        digaos_uart0_puts("[DIGAOS] PEEK ");
        digaos_put_hex_u32(addr);
        digaos_uart0_putc('=');
        digaos_put_hex_u16(v16);
        digaos_uart0_putc('\n');
        return true;
    }
    uint8_t v8 = 0U;
    if (!am34_mmio_read8(addr, &v8)) {
        digaos_uart0_puts("[DIGAOS] ERR BARRIER_LOCKED\n");
        return false;
    }
    digaos_uart0_puts("[DIGAOS] PEEK ");
    digaos_put_hex_u32(addr);
    digaos_uart0_putc('=');
    digaos_put_hex_u16((uint16_t)v8);
    digaos_uart0_putc('\n');
    return true;
}

static bool digaos_poke_checked(const char *arg, bool width32) {
    uint32_t addr = 0UL;
    uint32_t val = 0UL;
    const char *rest = NULL;
    const char *tail = NULL;
    if (!digaos_parse_hex_u32(arg, &addr, &rest) ||
        !digaos_parse_hex_u32(rest, &val, &tail) ||
        (*tail != '\0')) {
        digaos_uart0_puts("[DIGAOS] ERR syntax: poke <addr_hex> <val_hex>\n");
        return false;
    }
    if ((width32 && ((addr & 3UL) != 0UL)) || (!width32 && (((addr & 1UL) != 0UL) || (val > 0xFFFFUL)))) {
        digaos_uart0_puts("[DIGAOS] ERR ALIGN_OR_RANGE\n");
        return false;
    }
    const bool ok = width32 ? am34_mmio_write32(addr, val) : am34_mmio_write16(addr, (uint16_t)val);
    if (!ok) {
        digaos_uart0_puts("[DIGAOS] ERR BARRIER_LOCKED\n");
        return false;
    }
    digaos_uart0_puts("[DIGAOS] POKE OK\n");
    return true;
}

static bool cmd_poke16(const char *arg) {
    return digaos_poke_checked(arg, false);
}

static bool cmd_poke32(const char *arg) {
    return digaos_poke_checked(arg, true);
}

static const digaos_shell_cmd_t k_shell_cmds[] = {
    { "help",         NULL,              cmd_help },
    { "status",       NULL,              cmd_status },
    { "bootmap",      NULL,              cmd_bootmap },
    { "fpc",          NULL,              cmd_fpc },
    { "ntsc",         NULL,              cmd_ntsc },
    { "palm",         NULL,              cmd_palm },
    { "pal",          NULL,              cmd_pal },
    { "secam",        NULL,              cmd_secam },
    { "tbc on",       NULL,              cmd_tbc_on },
    { "tbc off",      NULL,              cmd_tbc_off },
    { "video br",     NULL,              cmd_video_br },
    { "video ec",     NULL,              cmd_video_ec },
    { "video gpio",   NULL,              cmd_video_gpio },
    { "video probe",  NULL,              cmd_video_probe },
    { "video status", NULL,              cmd_video_status },
    { "dac on",       NULL,              cmd_dac_on },
    { "dac off",      NULL,              cmd_dac_off },
    { "colorbar on",  NULL,              cmd_colorbar_on },
    { "colorbar off", NULL,              cmd_colorbar_off },
    { "provision",    NULL,              cmd_provision },
    { "sd",           NULL,              cmd_sd },
    { "bgo",          NULL,              cmd_bgo },
    { "ssot",         NULL,              cmd_ssot },
    { "menus",        NULL,              cmd_menus },
    { "themes",       NULL,              cmd_themes },
    { "power",        NULL,              cmd_power },
    { "targets",      NULL,              cmd_targets },
    { "wdt",          NULL,              cmd_wdt },
    { "sata",         NULL,              cmd_sata },
    { "odd",          NULL,              cmd_odd },
    { "peek ",        "<addr>",          cmd_peek },
    { "poke16 ",      "<addr> <u16>",    cmd_poke16 },
    { "poke32 ",      "<addr> <u32>",    cmd_poke32 },
    { "vfd ",         "<7ch>",           cmd_vfd },
};

static bool cmd_help(const char *arg) {
    (void)arg;
    digaos_uart0_puts("[DIGAOS] cmds:");
    for (size_t i = 0u; i < (sizeof(k_shell_cmds) / sizeof(k_shell_cmds[0])); ++i) {
        (void)digaos_uart0_putc(' ');
        digaos_uart0_puts(k_shell_cmds[i].name);
        if (k_shell_cmds[i].arg_hint != NULL) {
            digaos_uart0_puts(k_shell_cmds[i].arg_hint);
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
        if (cmd->arg_hint != NULL) {
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

    digaos_board_video_ic_t boot_ic = DIGAOS_VIDEO_IC_BH7629KS2_GPIO_M6930LB;
    const bool tbus_ok = digaos_tbus_send_keepalive_255s();
    const bool vfd_ok = digaos_tbus_send_vfd_text("DIGAOS");
    const bool video_ok = digaos_video_auto_bringup(DIGAOS_TV_NTSC, &boot_ic);

    uint32_t ch1_wait_ms = 0u;
    const bool davis_ok = digaos_davis_reset_and_wait_ready(true, &ch1_wait_ms);

    if (tbus_ok && vfd_ok && video_ok && davis_ok && (ch1_wait_ms == DIGAOS_SATA_BRIDGE_WAIT_MS)) {
        digaos_uart0_puts(
            (boot_ic == DIGAOS_VIDEO_IC_BH7629KS2_GPIO_M6930LB)
                ? "[DIGAOS] M0..M3 bring-up OK (TV=NTSC, IC3001=GPIO_M6930LB, SATA_WAIT=1000ms, IPL=RO_E05)\n"
                : "[DIGAOS] M0..M3 bring-up OK (TV=NTSC, SATA_WAIT=1000ms, IPL=RO_E05)\n"
        );
    } else {
        digaos_uart0_puts("[DIGAOS] WARN: Peripheral bring-up incomplete\n");
    }
    digaos_uart0_puts(DIGAOS_PROMPT " ");
}
