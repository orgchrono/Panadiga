/**
 * @file openos_drivers.h
 * @brief Modular bare-metal & host-testable C11 driver interfaces for Panadiga OpenOS (`AM34 / MN103S`).
 *
 * HARDWARE-INTEROPERABILITY PROTOTYPE -- NOT LEGALLY CLEAN-ROOM.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PANADIGA_OPENOS_DRIVERS_H
#define PANADIGA_OPENOS_DRIVERS_H

#include "hal/am34_mmio.h"

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Initialize AM34 `SC0` UART0 (`115200 8N1`). */
void openos_uart0_init(void);

/** Transmit one character via `SC0` after polling `TXBUSY | TXBF` with bounded timeout. */
bool openos_uart0_putc(char ch);

/** Non-blocking single-byte receive from `SC0` RX FIFO (`SC0STR & 0x0010` -> `SC0RXB`). */
bool openos_uart0_try_getc(char *out_ch);

/** Transmit NUL-terminated string via `SC0` normalizing `\n` -> `\r\n`. */
void openos_uart0_puts(const char *str);

/** Compute `IC7501` `TBUS` 8-bit frame checksum: `(sum(body15[0..15]) - 0x5B) mod 256`. */
uint8_t openos_tbus_checksum15(const uint8_t body15[15]);

/** Build 16-byte `IC7501` 255-second hung-up WDT keepalive frame (`[0x8A, 0x02, ..., 0x30]`). */
void openos_tbus_build_wdt_255s_frame(uint8_t out16[16]);

/** Build 16-byte `IC7501` VFD 7-character ASCII display frame (`cmd = 0xA3`). */
void openos_tbus_build_vfd_frame(const char *ascii7, uint8_t out16[16]);

/** Return `false` for European-only TBUS opcodes rejected (`0x7F`) by Brazilian `IC7501`. */
bool openos_tbus_opcode_safe_for_br_ic7501(uint8_t opcode);

/** Send a prebuilt 16-byte TBUS frame after verifying checksum and Brazilian `IC7501` opcode policy. */
bool openos_tbus_send_frame_checked(const uint8_t frame16[16]);

/** Transmit up to 7 ASCII characters to the front-panel VFD display (`cmd = 0xA3`). */
bool openos_tbus_send_vfd_text(const char *ascii7);

/** Transmit the 255-second keepalive frame to `0xDC010010..0xDC01001C` and strobe `0xDC010000`. */
bool openos_tbus_send_keepalive_255s(void);

/** Map `[nfrom]` `config1 + 0x12F0` (`page 9 + 0x00F0`, `ID 0x0117`) value to `openos_tv_system_t`. */
openos_tv_system_t openos_tv_from_nfrom_id0117(uint32_t raw_id0117);

/** Configure `AIOF` video output standard (`OPENOS_TV_NTSC`, `OPENOS_TV_PAL_M`, `OPENOS_TV_PAL`). */
bool openos_aiof_select_tv_system(openos_tv_system_t sys);

/** Configure `AIOF` TV standard together with `FS0` (non-standard sync), `YC3` (MVAGC), and `CONT` IRE setup. */
bool openos_aiof_apply_tbc_profile(openos_tv_system_t sys, bool nonstandard_sync, bool mvagc_line_tbc);

/** Resolve 8-bit I2C write address for `IC3001` (`BH7629KS2` BR/NA `0x90`/`0x92` vs `C1AB00002379` EC `0x94`). */
uint8_t openos_ic3001_addr8(openos_board_video_ic_t ic, bool adr_high);

/** Probe `IC3001` on `IIC1` (`0x90`/`0x92` BR/NA or `0x94` EC) returning the responsive 8-bit address. */
bool openos_iic_probe_ic3001(openos_board_video_ic_t ic, uint8_t *out_addr8);

/** Write one register over verified AM34 `IIC0`/`IIC1` controller after MMIO safety barrier check. */
bool openos_iic_write_reg_checked(uint32_t iic_base, uint8_t slave_addr8, uint8_t reg, uint8_t val);

/** Program `AVDD` `NPALCTL0`, `VOUTMODE0`, and `DACOUT1` video output/mute registers. */
bool openos_avdd_configure_output(openos_tv_system_t sys, bool unmute_dac);

/** Toggle `AVDD` prototype colorbar pattern (`VOUTMODE0_COLORBAR` vs `VOUTMODE0_NORMAL`). */
bool openos_avdd_set_colorbar(bool enable);

/** Execute full `AIOF` + `AVDD` + `IC3001` (`IIC1`) board video bring-up for the selected hardware variant. */
bool openos_video_bringup_board(
    openos_board_video_ic_t ic,
    openos_tv_system_t sys,
    bool nonstandard_sync,
    bool mvagc_line_tbc
);

/** Assert/release `DAVIS` ATA reset and apply safe bridge wait (`1000 ms` for `CH1` `IDE2SAT2`). */
bool openos_davis_reset_and_wait_ready(bool is_ch1_sata_bridge, uint32_t *out_waited_ms);

/** Deterministic full-disk MEIHFS provisioning VFD stages (`M FIRM -> MAIN -> UPDATE -> UPDOK -> FINISH`). */
typedef enum openos_provision_stage {
    OPENOS_PROV_STAGE_M_FIRM = 0,
    OPENOS_PROV_STAGE_MAIN = 1,
    OPENOS_PROV_STAGE_UPDATE = 2,
    OPENOS_PROV_STAGE_UPDOK = 3,
    OPENOS_PROV_STAGE_FINISH = 4
} openos_provision_stage_t;

/** Pure deterministic full-storage layout plan computed from detected disk capacity (`total_lba`). */
typedef struct openos_storage_plan {
    uint32_t total_lba;
    uint32_t mbr_lba;
    uint32_t mbr_mirror_lba;
    uint32_t journal_lba;
    uint32_t primary_fw_lba;
    uint32_t mirror_fw_lba;
    uint32_t fw_sectors;
    uint32_t data_start_lba;
    uint32_t usable_blocks_2kb;
    uint32_t payload_crc32;
    bool valid;
} openos_storage_plan_t;

/** Sector write callback invoked by the thin storage provisioning shell (`stage_tag` = `openos_provision_stage_t`). */
typedef bool (*openos_storage_write_fn)(uint32_t start_lba, uint32_t sector_count, uint32_t stage_tag, void *ctx);

/** Return the canonical 7-char VFD label for `openos_provision_stage_t`. */
const char *openos_provision_stage_vfd(openos_provision_stage_t stage);

/** Pure FCIS planner auto-sizing MEIHFS geometry and mirror LBAs across full `total_lba` (`5 GB..2 TiB`). */
bool openos_storage_plan_compute(
    uint32_t total_lba,
    uint32_t payload_bytes,
    uint32_t payload_crc32,
    openos_storage_plan_t *out_plan
);

/** Execute `M FIRM -> MAIN -> UPDATE -> UPDOK -> FINISH` over `DAVIS CH1` with `IC7501` keepalive and fail-closed halt. */
bool openos_storage_provision_execute(
    const openos_storage_plan_t *plan,
    openos_storage_write_fn write_fn,
    void *ctx,
    openos_provision_stage_t *out_last_stage
);

/** Read-only `SDCD` host controller status (`0x89001800` + `0x89001850`). */
typedef struct openos_sdcd_status {
    uint32_t ctrl_reg;
    uint32_t bus_cfg_reg;
    bool bus_width_4bit;
} openos_sdcd_status_t;

/** Probe AM34 `SDCD` controller registers strictly read-only. */
bool openos_sdcd_probe_status(openos_sdcd_status_t *out);

/** Read-only Nor-FROM (`SA11`), BGO (`SA26..SA70`), and EEMUL (`SA134`) readiness summary. */
typedef struct openos_unbrick_status {
    uint16_t bgo_p1_sa;
    uint16_t bgo_p1_count;
    uint16_t bgo_p2_count;
    uint16_t bgo_erased_count;
    bool sa11_is_erased;
    bool sa11_has_p4zu;
    bool eemul_has_1284;
    bool nor_autoboot_ready;
} openos_unbrick_status_t;

/** Inspect live NOR flash `SA11`, `SA26..SA70` headers, and `EEMUL` tag strictly read-only. */
bool openos_nor_inspect_unbrick(openos_unbrick_status_t *out);

#define OPENOS_SHELL_LINE_CAP 64U

/** Bounded zero-heap state for the interactive `OPENOS>` UART0 shell (`SC0` RX). */
typedef struct openos_shell_state {
    char line[OPENOS_SHELL_LINE_CAP];
    size_t len;
    uint32_t idle_ticks;
    uint32_t keepalives_sent;
} openos_shell_state_t;

/** Initialize bounded interactive `OPENOS>` shell state. */
void openos_shell_init(openos_shell_state_t *st);

/** Poll `SC0` RX once, accumulate/echo line, dispatch command on CR/LF, and feed `TBUS` keepalive on idle. */
bool openos_shell_poll_once(openos_shell_state_t *st, bool *out_cmd_ran);

/** Execute one interactive `OPENOS>` command line (`help`, `status`, `ntsc`, `palm`, `pal`, `tbc on/off`, `video br/ec/probe`, `dac on/off`, `colorbar on/off`, `provision`, `sd`, `bgo`, `ssot`, `wdt`, `sata`, `vfd <txt>`). */
bool openos_shell_exec_line(const char *line);

/** Stage-1.5 / Stage-2 C kernel entry invoked by `crt0_am34.S`. */
void openos_kernel_entry(void);

#ifdef __cplusplus
}
#endif

#endif /* PANADIGA_OPENOS_DRIVERS_H */
