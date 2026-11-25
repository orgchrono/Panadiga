/**
 * @file digaos_drivers.h
 * @brief Modular bare-metal & host-testable C11 driver interfaces for `DigaOS` (`AM34 / MN103S`).
 *
 * HARDWARE-INTEROPERABILITY PROTOTYPE -- NOT LEGALLY CLEAN-ROOM.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PANADIGA_DIGAOS_DRIVERS_H
#define PANADIGA_DIGAOS_DRIVERS_H

#include "hal/am34_mmio.h"
#include "hal/digaos_soc_stubs.h"

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Initialize AM34 `SC0` UART0 (`115200 8N1`). */
void digaos_uart0_init(void);

/** Transmit one character via `SC0` after polling `TXBUSY | TXBF` with bounded timeout. */
bool digaos_uart0_putc(char ch);

/** Non-blocking single-byte receive from `SC0` RX FIFO (`SC0STR & 0x0010` -> `SC0RXB`). */
bool digaos_uart0_try_getc(char *out_ch);

/** Transmit NUL-terminated string via `SC0` normalizing `\n` -> `\r\n`. */
void digaos_uart0_puts(const char *str);

/** Compute `IC7501` `TBUS` 8-bit frame checksum: `(sum(body15[0..15]) - 0x5B) mod 256`. */
uint8_t digaos_tbus_checksum15(const uint8_t body15[15]);

/** Build 16-byte `IC7501` 255-second hung-up WDT keepalive frame (`[0x8A, 0x02, ..., 0x30]`). */
void digaos_tbus_build_wdt_255s_frame(uint8_t out16[16]);

/** Build 16-byte `IC7501` VFD 7-character ASCII display frame (`cmd = 0xA3`). */
void digaos_tbus_build_vfd_frame(const char *ascii7, uint8_t out16[16]);

/** Return `false` for European-only TBUS opcodes rejected (`0x7F`) by Brazilian `IC7501`. */
bool digaos_tbus_opcode_safe_for_br_ic7501(uint8_t opcode);

/** Send a prebuilt 16-byte TBUS frame after verifying checksum and Brazilian `IC7501` opcode policy. */
bool digaos_tbus_send_frame_checked(const uint8_t frame16[16]);

/** Transmit up to 7 ASCII characters to the front-panel VFD display (`cmd = 0xA3`). */
bool digaos_tbus_send_vfd_text(const char *ascii7);

/** Transmit the 255-second keepalive frame to `0xDC010010..0xDC01001C` and strobe `0xDC010000`. */
bool digaos_tbus_send_keepalive_255s(void);

/** Map `[nfrom]` `config1 + 0x12F0` (`page 9 + 0x00F0`, `ID 0x0117`) value to `digaos_tv_system_t`. */
digaos_tv_system_t digaos_tv_from_nfrom_id0117(uint32_t raw_id0117);

/** Configure `AIOF` video output standard (`DIGAOS_TV_NTSC`, `DIGAOS_TV_PAL_M`, `DIGAOS_TV_PAL`, `DIGAOS_TV_SECAM`). */
bool digaos_aiof_select_tv_system(digaos_tv_system_t sys);

/** Safe baseline AIOF TBC probe absorbed from retired `blackbox-fw` feasibility monitor. */
bool probe_tbc_aiof_baseline(digaos_tv_system_t std_mode);

/** Configure `AIOF` TV standard together with `FS0` (non-standard sync), `YC3` (MVAGC), and `CONT` IRE setup. */
bool digaos_aiof_apply_tbc_profile(digaos_tv_system_t sys, bool nonstandard_sync, bool mvagc_line_tbc);

/** Resolve 8-bit I2C write address for `IC3001` (`BH7629KS2` BR/NA `0x90`/`0x92` vs `C1AB00002379` EC `0x94`). */
uint8_t digaos_ic3001_addr8(digaos_board_video_ic_t ic, bool adr_high);

/** Probe `IC3001` on `IIC1` (`0x90`/`0x92` BR/NA or `0x94` EC) returning the responsive 8-bit address. */
bool digaos_iic_probe_ic3001(digaos_board_video_ic_t ic, uint8_t *out_addr8);

/** Write one register over verified AM34 `IIC0`/`IIC1` controller after MMIO safety barrier check. */
bool digaos_iic_write_reg_checked(uint32_t iic_base, uint8_t slave_addr8, uint8_t reg, uint8_t val);

/** Program `AVDD` `NPALCTL0`, `VOUTMODE0`, and `DACOUT1` video output/mute registers. */
bool digaos_avdd_configure_output(digaos_tv_system_t sys, bool unmute_dac);

/** Toggle `AVDD` prototype colorbar pattern (`VOUTMODE0_COLORBAR` vs `VOUTMODE0_NORMAL`). */
bool digaos_avdd_set_colorbar(bool enable);

/** Execute full `AIOF` + `AVDD` + `IC3001` (`IIC1`) board video bring-up for the selected hardware variant. */
bool digaos_video_bringup_board(
    digaos_board_video_ic_t ic,
    digaos_tv_system_t sys,
    bool nonstandard_sync,
    bool mvagc_line_tbc
);

/** Assert/release `DAVIS` ATA reset and apply safe bridge wait (`1000 ms` for `CH1` `IDE2SAT2`). */
bool digaos_davis_reset_and_wait_ready(bool is_ch1_sata_bridge, uint32_t *out_waited_ms);

/** Deterministic full-disk MEIHFS provisioning VFD stages (`M FIRM -> MAIN -> UPDATE -> UPDOK -> FINISH`). */
typedef enum digaos_provision_stage {
    DIGAOS_PROV_STAGE_M_FIRM = 0,
    DIGAOS_PROV_STAGE_MAIN = 1,
    DIGAOS_PROV_STAGE_UPDATE = 2,
    DIGAOS_PROV_STAGE_UPDOK = 3,
    DIGAOS_PROV_STAGE_FINISH = 4
} digaos_provision_stage_t;

/** Pure deterministic full-storage layout plan computed from detected disk capacity (`total_lba`). */
typedef struct digaos_storage_plan {
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
} digaos_storage_plan_t;

/** Sector write callback invoked by the thin storage provisioning shell (`stage_tag` = `digaos_provision_stage_t`). */
typedef bool (*digaos_storage_write_fn)(uint32_t start_lba, uint32_t sector_count, uint32_t stage_tag, void *ctx);

/** Return the canonical 7-char VFD label for `digaos_provision_stage_t`. */
const char *digaos_provision_stage_vfd(digaos_provision_stage_t stage);

/** Pure FCIS planner auto-sizing MEIHFS geometry and mirror LBAs across full `total_lba` (`5 GB..2 TiB`). */
bool digaos_storage_plan_compute(
    uint32_t total_lba,
    uint32_t payload_bytes,
    uint32_t payload_crc32,
    digaos_storage_plan_t *out_plan
);

/** Execute `M FIRM -> MAIN -> UPDATE -> UPDOK -> FINISH` over `DAVIS CH1` with `IC7501` keepalive and fail-closed halt. */
bool digaos_storage_provision_execute(
    const digaos_storage_plan_t *plan,
    digaos_storage_write_fn write_fn,
    void *ctx,
    digaos_provision_stage_t *out_last_stage
);

/** Read-only `SDCD` host controller status (`0x89001800` + `0x89001850`). */
typedef struct digaos_sdcd_status {
    uint32_t ctrl_reg;
    uint32_t bus_cfg_reg;
    bool bus_width_4bit;
} digaos_sdcd_status_t;

/** Probe AM34 `SDCD` controller registers strictly read-only. */
bool digaos_sdcd_probe_status(digaos_sdcd_status_t *out);

/** Read-only Nor-FROM (`SA11`), BGO (`SA26..SA70`), and EEMUL (`SA134`) readiness summary. */
typedef struct digaos_unbrick_status {
    uint16_t bgo_p1_sa;
    uint16_t bgo_p1_count;
    uint16_t bgo_p2_count;
    uint16_t bgo_erased_count;
    bool sa11_is_erased;
    bool sa11_has_autoboot_capsule;
    bool eemul_has_1284;
    bool nor_autoboot_ready;
} digaos_unbrick_status_t;

/** Inspect live NOR flash `SA11`, `SA26..SA70` headers, and `EEMUL` tag strictly read-only. */
bool digaos_nor_inspect_unbrick(digaos_unbrick_status_t *out);

#define DIGAOS_SHELL_LINE_CAP 64U

/** Bounded zero-heap state for the interactive `DIGAOS>` UART0 shell (`SC0` RX). */
typedef struct digaos_shell_state {
    char line[DIGAOS_SHELL_LINE_CAP];
    size_t len;
    uint32_t idle_ticks;
    uint32_t keepalives_sent;
} digaos_shell_state_t;

/** Initialize bounded interactive `DIGAOS>` shell state. */
void digaos_shell_init(digaos_shell_state_t *st);

/** Poll `SC0` RX once, accumulate/echo line, dispatch command on CR/LF, and feed `TBUS` keepalive on idle. */
bool digaos_shell_poll_once(digaos_shell_state_t *st, bool *out_cmd_ran);

/** Execute one interactive `DIGAOS>` command line. */
bool digaos_shell_exec_line(const char *line);

/** `DIAG_PROG_RAM` / `MAIN_OS_RAM` C kernel entry invoked by `crt0_am34.S`. */
void digaos_kernel_entry(void);

#ifdef __cplusplus
}
#endif

#endif /* PANADIGA_DIGAOS_DRIVERS_H */
