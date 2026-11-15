//! MCP JSON schema definitions for RE, unbrick, HDD capacity, and SD kit tools.

use serde_json::{Value, json};

pub(super) fn re_tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "panadiga_hdd_capacity",
            "description": "Inspects or patches Panasonic MAIN firmware (`src/hdd_cmd.c` MaxProductLBA clamp: 200 GB ceiling -> dynamic LBA48 or preset capacity) in RAW MAIN binaries, EXTRHEAD containers, or full HDD Partition-1 images.",
            "inputSchema": {
                "type": "object",
                "required": ["input_path"],
                "properties": {
                    "input_path": {
                        "type": "string",
                        "description": "Path to RAW MAIN firmware (`EH55_MAIN_3410PQ.bin`), EXTRHEAD container, or full HDD `.bin` image."
                    },
                    "action": {
                        "type": "string",
                        "enum": ["inspect", "patch"],
                        "description": "Operation mode: 'inspect' (default) reads slots/clamp opcode; 'patch' writes a non-destructive modified copy."
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["dynamic", "cap", "factory_200gb"],
                        "description": "Patch strategy when action='patch' (default 'dynamic')."
                    },
                    "target_gb": {
                        "type": "integer",
                        "description": "Target nominal capacity in GB when mode='cap' (e.g. 250, 320, 500)."
                    },
                    "output_path": {
                        "type": "string",
                        "description": "Optional non-destructive output file path (must differ from input_path)."
                    }
                }
            }
        }),
        json!({
            "name": "panadiga_nor_dumper_stub",
            "description": "Assembles a read-only MN103S UART NOR flash dumper stub (`PANADMP1..PANADONE`, guarded strictly to 0x80000000/0x84000000 windows) as a raw RSP RAM stub or 128 KiB DIGADIAG.FRM.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "start_addr": {
                        "type": "string",
                        "description": "Hex or decimal NOR start VA (default '0x84000000')."
                    },
                    "total_len": {
                        "type": ["string", "integer"],
                        "description": "Total dump byte length, exact multiple of block_size (default '0x800000' = 8 MiB)."
                    },
                    "block_size": {
                        "type": "integer",
                        "description": "Checksummed block size in bytes (default 4096)."
                    },
                    "wrap_digadiag": {
                        "type": "boolean",
                        "description": "Wrap stub inside a 131072-byte DIGADIAG.FRM (FVU / NOR_DMP, default load 0x50400000) instead of raw stub (default load 0x51800000)."
                    },
                    "load_addr": {
                        "type": "string",
                        "description": "MN103S SDRAM stub load address (defaults to '0x50400000' when wrap_digadiag=true, else '0x51800000')."
                    },
                    "output_path": {
                        "type": "string",
                        "description": "Optional path to write the assembled `.bin` stub or `DIGADIAG.FRM`."
                    }
                }
            }
        }),
        json!({
            "name": "panadiga_bgo_unbrick",
            "description": "Inspects or patches the 3 verified BGO unbrick/bypass sites (`0x504230F0`, `0x50405097`, and `0x5040A2CD` -> `nfrom_format 0x50401F3C`) in `PROG061` / `DIGADIAG.FRM` to recover `BGO driver init failed[1].` (`U99`).",
            "inputSchema": {
                "type": "object",
                "required": ["input_path"],
                "properties": {
                    "input_path": {
                        "type": "string",
                        "description": "Path to `eh55_frm_PROG061_BOOT.bin` or plaintext `EXTRHEAD`/`DIGADIAG.FRM`."
                    },
                    "action": {
                        "type": "string",
                        "enum": ["inspect", "patch"],
                        "description": "Operation mode: 'inspect' (default) or 'patch'."
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["bypass", "auto_format", "uart_monitor", "factory"],
                        "description": "Patch strategy when action='patch' (default 'bypass'; 'auto_format' requires allow_unsafe_bgo_format=true due to unverified nfrom_format RAM pointers; 'uart_monitor' embeds interactive PANAMON1 UART0 monitor @ 0x50439494)."
                    },
                    "allow_unsafe_bgo_format": {
                        "type": "boolean",
                        "description": "Explicitly unlock unsafe 'auto_format' mode (fail-closed false by default)."
                    },
                    "wrap_as_digadiag": {
                        "type": "boolean",
                        "description": "Wrap patched PROG061 in a bootable `DIGADIAG.FRM` (default true)."
                    },
                    "output_path": {
                        "type": "string",
                        "description": "Optional non-destructive output path (must differ from input_path)."
                    }
                }
            }
        }),
        json!({
            "name": "panadiga_main_passthrough",
            "description": "Inspects or patches the 3 verified MAIN (`EH55_MAIN_3410PQ.bin`, VA 0x57000000) AV TBC (`E-E`) passthrough sites (`0x5771ABFB`, `0x574F7B8C`, `0x574F7C5C`), or generates `_dev_SERIAL` GDB RSP `$M` frames (`rsp_plan`, no image required).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["inspect", "patch", "rsp_plan"],
                        "description": "Operation mode: 'inspect' (default), 'patch', or 'rsp_plan' (generates Pico/UART RSP frames without an input binary)."
                    },
                    "input_path": {
                        "type": "string",
                        "description": "Path to unpacked `EH55_MAIN_3410PQ.bin` (required for 'inspect' and 'patch')."
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["av_passthrough", "factory_original"],
                        "description": "Patch mode when action='patch' (default 'av_passthrough')."
                    },
                    "output_path": {
                        "type": "string",
                        "description": "Optional non-destructive output path (must differ from input_path)."
                    },
                    "callback_target_va": {
                        "type": ["string", "integer"],
                        "description": "Optional MN103S target VA for `_dev_SERIAL` callback pointer (`0x515B2E40`) in 'rsp_plan' mode."
                    }
                }
            }
        }),
        json!({
            "name": "panadiga_sd_unbrick_kit",
            "description": "Unified Pre-MAIN Unbrick workbench tool: inspects workspace status/hardware facts, extracts modules from PANA_DVD.FRM, builds verified DIGADIAG.FRM + PANA__SD.FRM, verifies SD root media, or runs the Matshita R6-MV4 ODD boot/ATAPI-write/serial/.FRM emulator.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["status", "extract_frm", "prepare_sd", "verify_sd", "matshita_emu"],
                        "description": "Operation mode: 'status' (default), 'extract_frm', 'prepare_sd', 'verify_sd', or 'matshita_emu'."
                    },
                    "emu_action": {
                        "type": "string",
                        "enum": ["boot", "inject_write", "serial_cmd", "verify_frm", "pack_unlock_frm"],
                        "description": "Sub-action when action='matshita_emu' (default 'boot')."
                    },
                    "scenario": {
                        "type": "string",
                        "description": "Boot scenario for emu_action='boot' (healthy_with_disc, unlock_disc_frm, tray_cannot_close_80045300, laser_apc_failure_80040904)."
                    },
                    "payload_hex": {
                        "type": "string",
                        "description": "Hex payload bytes for emu_action='inject_write' or 'pack_unlock_frm'."
                    },
                    "offset": {
                        "type": ["string", "integer"],
                        "description": "Byte/buffer offset for emu_action='inject_write'."
                    },
                    "mode": {
                        "type": ["string", "integer"],
                        "description": "ATAPI 0x3B WRITE BUFFER mode for emu_action='inject_write' (default 2)."
                    },
                    "use_vendor_ed": {
                        "type": "boolean",
                        "description": "Use vendor CDB 0xED instead of 0x3B for emu_action='inject_write'."
                    },
                    "target_va": {
                        "type": "string",
                        "description": "Target MN103S VA for vendor 0xED injection (default '0x50400000')."
                    },
                    "serial_cmd": {
                        "type": "string",
                        "description": "Factory serial CLI command for emu_action='serial_cmd' (e.g. 'VER', 'PASS 0x4d415453 0x48495441', 'WW 0x51800000 0xDEADBEEF')."
                    },
                    "unlock_pass": {
                        "type": "boolean",
                        "description": "Run secret PASS command before serial_cmd in emu_action='serial_cmd' (default true)."
                    },
                    "container_kind": {
                        "type": "string",
                        "enum": ["sd", "cd"],
                        "description": "Unlock .FRM container type for emu_action='pack_unlock_frm' ('sd'=EXTRHEAD or 'cd'=MATSHITA)."
                    },
                    "frm_path": {
                        "type": "string",
                        "description": "Path to .FRM file for emu_action='verify_frm'."
                    },
                    "output_path": {
                        "type": "string",
                        "description": "Optional output file path for emu_action='pack_unlock_frm'."
                    },
                    "output_dir": {
                        "type": "string",
                        "description": "Destination directory for 'prepare_sd' or 'extract_frm'."
                    },
                    "sd_root_dir": {
                        "type": "string",
                        "description": "SD root directory for 'verify_sd'."
                    },
                    "bgo_mode": {
                        "type": "string",
                        "enum": ["bypass", "auto_format", "uart_monitor", "factory"],
                        "description": "PROG061 BGO patch mode for 'prepare_sd' (default 'bypass'; 'auto_format' is blocked unless allow_unsafe_auto_format=true)."
                    },
                    "allow_unsafe_auto_format": {
                        "type": "boolean",
                        "description": "Explicitly unlock unsafe 'auto_format' mode in prepare_sd (default false)."
                    },
                    "passthrough_mode": {
                        "type": "string",
                        "enum": ["av_passthrough", "factory_original"],
                        "description": "MAIN TBC passthrough mode for 'prepare_sd' (default 'av_passthrough'; guarded by build identity verification)."
                    },
                    "expected_main_build": {
                        "type": "string",
                        "description": "Optional MAIN build tag guard (e.g. '3410PQ' or '3060NP')."
                    },
                    "allow_target_mismatch": {
                        "type": "boolean",
                        "description": "Explicitly override expected_main_build check if non-matching (default false)."
                    },
                    "include_pana_sd": {
                        "type": "boolean",
                        "description": "Generate PANA__SD.FRM alongside DIGADIAG.FRM in 'prepare_sd' (default true)."
                    },
                    "unlock_hdd_dynamic": {
                        "type": "boolean",
                        "description": "Unlock LBA48 HDD capacity ceiling in MAIN when building PANA__SD.FRM (default true)."
                    }
                }
            }
        }),
    ]
}
