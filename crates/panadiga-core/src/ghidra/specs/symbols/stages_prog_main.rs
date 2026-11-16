//! Stage 2 (`PROG061` / `DIGADIAG.FRM`) and Stage 3 (`MAIN` + `_dev_SERIAL` RSP) symbols.

use super::super::{
    MAIN_DAVIS_TOPOLOGY_VA, MAIN_LBA48_CLAMP_VA, PROG061_BGO_ERROR_EXIT_VA, PROG061_BGO_MARK_OK_VA,
    PROG061_CTRL_INFO_INIT_VA, PROG061_DAVIS_TOPOLOGY_VA, PROG061_FEISTEL_SBOX_VA, SymbolSpec,
};
use super::sym;
use crate::re::bgo_unbrick::{
    OFF_BGO_INIT_FAIL_HOOK, OFF_BGO_SUM_BEQ, OFF_DEVINFO_CHECK_BEQ, PROG061_BASE_VA,
    PROG061_NFROM_FORMAT_VA,
};
use crate::re::main_passthrough::{
    DEV_SERIAL_CALLBACK_VA, DEV_SERIAL_ISR_CTX_PTR_VA, DEV_SERIAL_STATE_BASE_VA, MAIN_BASE_VA,
    OFF_FPC_STATUS_CTRL, OFF_ODD_INIT_STATUS, OFF_ODD_STATE_BRANCH,
};
use crate::re::prog061_uart_monitor::PROG061_UART_MONITOR_VA;

pub(super) const SPECS: &[SymbolSpec] = &[
    // Stage 2: PROG061 / DIGADIAG.FRM (0x50400000)
    sym(
        PROG061_BASE_VA,
        "prog061_fvu_entry",
        "Stage 2 PROG061 FVU Entrypoint (DIGADIAG.FRM dst=0x50400000)",
    ),
    sym(
        PROG061_NFROM_FORMAT_VA,
        "prog061_nfrom_format",
        "Orphan factory NFROM/BGO formatter (0 static callers; invokable from 0x5040A2CD)",
    ),
    sym(
        PROG061_BASE_VA + OFF_DEVINFO_CHECK_BEQ as u32,
        "prog061_devinfo_check_beq",
        "Site 2 (0x50405097): DeviceInfo INFO guard (beq+21 -> bra+21)",
    ),
    sym(
        PROG061_BASE_VA + OFF_BGO_INIT_FAIL_HOOK as u32,
        "prog061_bgo_init_fail_hook",
        "Site 3 (0x5040A2CD): 20B BGO init failure branch -> AUTO_FORMAT_BGO / PANAMON1",
    ),
    sym(
        PROG061_CTRL_INFO_INIT_VA,
        "prog061_ctrl_info_init",
        "Control information init continuation point after BGO init",
    ),
    sym(
        PROG061_BGO_MARK_OK_VA,
        "prog061_bgo_mark_ok",
        "BGO bypass OK return point (d0 == 0)",
    ),
    sym(
        PROG061_BGO_ERROR_EXIT_VA,
        "prog061_bgo_error_exit",
        "BGO factory error return point (d0 == -1)",
    ),
    sym(
        PROG061_DAVIS_TOPOLOGY_VA,
        "prog061_davis_ide_topology",
        "Fixed DAVIS IDE topology table acStack_70 = {2,0,1,0} (Sec Master HDD)",
    ),
    sym(
        PROG061_BASE_VA + OFF_BGO_SUM_BEQ as u32,
        "prog061_bgo_sum_beq",
        "Site 1 (0x504230F0): bgo_sum_23058 N(N+1)/2 checksum branch (beq+10 -> bra+10)",
    ),
    sym(
        PROG061_FEISTEL_SBOX_VA,
        "prog061_feistel_sbox_256b",
        "Verified 256-byte Feistel S-Box substitution table in PROG061",
    ),
    sym(
        PROG061_UART_MONITOR_VA,
        "prog061_panamon1_uart_monitor",
        "Verified 0xFF slack entrypoint for PANAMON1 UART0 rescue monitor",
    ),
    // Stage 3: EH55_MAIN_3410PQ.bin (0x57000000) & _dev_SERIAL RSP
    sym(
        MAIN_BASE_VA,
        "main_app_base",
        "Stage 3 MAIN Application Base (EH55_MAIN_3410PQ.bin)",
    ),
    sym(
        MAIN_DAVIS_TOPOLOGY_VA,
        "main_davis_ide_topology",
        "Fixed MAIN DAVIS IDE topology acStack_70 = {2,0,1,0}",
    ),
    sym(
        MAIN_LBA48_CLAMP_VA,
        "main_hdd_cmd_lba48_clamp",
        "src/hdd_cmd.c >200 GB LBA48 capacity clamp / patch site",
    ),
    sym(
        MAIN_BASE_VA + OFF_ODD_STATE_BRANCH as u32,
        "main_odd_state_branch",
        "TBC Site 2 (0x574F7B8C): ODD state check -> jmp16 +0x00D0 to 0x574F7C5C",
    ),
    sym(
        MAIN_BASE_VA + OFF_ODD_INIT_STATUS as u32,
        "main_odd_init_status",
        "TBC Site 3 (0x574F7C5C): ODD init status forced ready (clr d0; nop x5)",
    ),
    sym(
        MAIN_BASE_VA + OFF_FPC_STATUS_CTRL as u32,
        "main_fpc_status_ctrl",
        "TBC Site 1 (0x5771ABFB): FPC status control -> mov 1, d0; ret",
    ),
    sym(
        DEV_SERIAL_STATE_BASE_VA,
        "_dev_SERIAL_state_base",
        "_dev_SERIAL UART0 GDB RSP protocol state struct (+0x60 = state 0..4)",
    ),
    sym(
        DEV_SERIAL_CALLBACK_VA,
        "_dev_SERIAL_callback_ptr",
        "_dev_SERIAL registered UART0 RSP callback pointer (+0x40)",
    ),
    sym(
        DEV_SERIAL_ISR_CTX_PTR_VA,
        "_dev_SERIAL_isr_ctx_ptr",
        "Pointer to saved ISR register frame (saved PC at +0x64)",
    ),
];
