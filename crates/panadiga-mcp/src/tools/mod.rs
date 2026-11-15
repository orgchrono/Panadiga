//! Registry and unified dispatcher for all native Panadiga MCP tools.

pub mod forensics;
pub mod hardware;
pub mod knowledge;
pub mod unbrick;
pub mod visuals;

use anyhow::Result;
use serde_json::{Value, json};
use std::path::Path;

pub fn tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "panadiga_memory_map",
            "description": "Returns the official physical memory map of the Matsushita AM34 / MN103S processor.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filter_name": {
                        "type": "string",
                        "description": "Optional region name filter (e.g. NVRAM_U99, FLASH_NOR_IPL)"
                    }
                }
            }
        }),
        json!({
            "name": "panadiga_inspect_hdd",
            "description": "Inspects a Panasonic DMR-EH55 MEIHFS HDD binary dump: validates MBR tag 'SH', mirror sector at 28MB, and partition layout.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "image_path": {
                        "type": "string",
                        "description": "Path to the HDD dump image relative to workspace root."
                    }
                }
            }
        }),
        json!({
            "name": "panadiga_safety_check",
            "description": "Validates electrical voltage and memory write addresses against deterministic Panadiga hardware invariants.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "voltage": {
                        "type": "number",
                        "description": "Voltage value in Volts to test against the 3.3V LVTTL safety rail (absolute ceiling 3.35V)."
                    },
                    "address": {
                        "type": "string",
                        "description": "Hexadecimal memory bus address (e.g. '0x807F0000') to test for write permission."
                    }
                }
            }
        }),
        json!({
            "name": "panadiga_clean_nvram",
            "description": "Analyzes or performs surgical 64KB NVRAM sanitization (U99 unbrick) on a Spansion S29JL064H 8MB flash dump.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "input_file": {
                        "type": "string",
                        "description": "Path to original 8MB flash dump file."
                    },
                    "output_file": {
                        "type": "string",
                        "description": "Optional path where sanitized flash file should be saved with NVRAM reset to 0xFF."
                    }
                },
                "required": ["input_file"]
            }
        }),
        json!({
            "name": "panadiga_search_manuals",
            "description": "Searches all Panasonic/RP2040/Spansion PDF manuals and schematics by filename AND extracted page contents via the RAG PDF index.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search term, IC identifier, connector pin, or error code."
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of PDF page hits to return (default 8, max 25)."
                    }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "panadiga_rag_query",
            "description": "Runs a guarded query against the native Rust 6-layer Zero-Error RAG Engine indexing docs, PDF manuals, Rust/C11/TS source code, and Ghidra scripts.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Technical query regarding Panasonic DIGA engineering or codebase."
                    },
                    "include_corpus_stats": {
                        "type": "boolean",
                        "description": "Include breakdown of indexed docs, PDF pages, code chunks, and Ghidra chunks."
                    }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "panadiga_inspect_frm",
            "description": "Inspects PANA_DVD.FRM update container TOC, firmware modules, and 8-byte entropy characteristics.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "frm_path": {
                        "type": "string",
                        "description": "Path to PANA_DVD.FRM file (defaults to firmware_and_images/PANA_DVD.FRM)."
                    }
                }
            }
        }),
        json!({
            "name": "panadiga_read_hex",
            "description": "Reads a safe read-only hexadecimal + ASCII slice from any binary image or dump file.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Path to binary file relative to workspace root or absolute."
                    },
                    "offset": {
                        "type": "string",
                        "description": "Byte offset in hex (e.g. '0x800000' or '0x02000000') or decimal (default 0)."
                    },
                    "length": {
                        "type": "integer",
                        "description": "Number of bytes to dump (16..4096, default 256)."
                    }
                },
                "required": ["file_path"]
            }
        }),
        json!({
            "name": "panadiga_pico_catalog",
            "description": "Returns authoritative Hardware SSOT specs: P7501 wiring pinout, safety thresholds, erase sequence, and Pico P7501 Async UART serial command catalog.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "category": {
                        "type": "string",
                        "description": "Optional command category filter (e.g. diag, debug, flash, gdb)."
                    }
                }
            }
        }),
        json!({
            "name": "panadiga_ghidra_symbols",
            "description": "Returns SSOT Matsushita AM34/MN103S memory regions and Ghidra symbol directives generated from Rust.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filter": {
                        "type": "string",
                        "description": "Optional filter matching symbol or region name."
                    }
                }
            }
        }),
        json!({
            "name": "panadiga_search_code",
            "description": "Searches Rust crates, RP2040 C11/PIO firmware, React/TypeScript UI, and Ghidra Python scripts indexed by the RAG engine.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Function, struct, PIO state machine, Tauri command, or protocol symbol."
                    },
                    "scope": {
                        "type": "string",
                        "description": "Optional filter: 'code' (default), 'ghidra', 'docs', or 'all'."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum matches to return (default 8, max 25)."
                    }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "panadiga_search_visuals",
            "description": "Searches the 238 curated DMR-EH55 schematic, PCB foil, pinout, and CAD crop PNGs indexed in docs/assets/visual_manifest.json without reopening raw PDF manuals.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Component, signal, page or topic filter (e.g. 'P7501 Q7514', 'IC1506', 'FP50004', 'page_113')."
                    },
                    "edition": {
                        "type": "string",
                        "description": "Optional service manual edition filter (e.g. 'North America', 'Vol. 2', 'Vol. 1', 'EC/EE')."
                    },
                    "category": {
                        "type": "string",
                        "description": "Optional category filter ('Schematic & Circuit Zoom', 'PCB Layout & Mechanical', 'Pinout / Electrical Trace')."
                    },
                    "page": {
                        "type": "integer",
                        "description": "Optional manual PDF page number filter."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum entries to return (default 15, max 50)."
                    }
                }
            }
        }),
        json!({
            "name": "panadiga_project_status",
            "description": "Reports live Panadiga workspace topology, Cargo crates, service manual PDFs count, DIGA models catalog, and optional RAG index statistics.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "include_rag_stats": {
                        "type": "boolean",
                        "description": "Trigger/return full RAG corpus breakdown stats."
                    }
                }
            }
        }),
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
            "description": "Assembles a read-only MN103S UART NOR flash dumper stub (`PANADMP1..PANADONE`, guarded strictly to 0x80000000/0x84000000 windows) for Pico RAM injection ($M/$G) or DIGADIAG.FRM.",
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
                    "load_addr": {
                        "type": "string",
                        "description": "MN103S SDRAM stub load address (default '0x51800000')."
                    },
                    "output_path": {
                        "type": "string",
                        "description": "Optional path to write the assembled `.bin` stub."
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
            "description": "Unified Pre-MAIN Unbrick workbench tool: inspects workspace status/hardware facts, extracts modules from PANA_DVD.FRM, builds verified DIGADIAG.FRM + PANA__SD.FRM (BGO + MAIN AV TBC Passthrough + HDD LBA48), or verifies SD root media.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["status", "extract_frm", "prepare_sd", "verify_sd"],
                        "description": "Operation mode: 'status' (default), 'extract_frm', 'prepare_sd', or 'verify_sd'."
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

pub fn execute_tool(name: &str, args: &Value, workspace_root: &Path) -> Result<Value> {
    match name {
        "panadiga_memory_map" => Ok(hardware::handle_memory_map(args)),
        "panadiga_safety_check" => Ok(hardware::handle_safety_check(args)),
        "panadiga_pico_catalog" => Ok(hardware::handle_pico_catalog(args)),
        "panadiga_project_status" => Ok(hardware::handle_project_status(args, workspace_root)),
        "panadiga_inspect_hdd" => Ok(forensics::handle_inspect_hdd(args, workspace_root)),
        "panadiga_clean_nvram" => forensics::handle_clean_nvram(args, workspace_root),
        "panadiga_inspect_frm" => forensics::handle_inspect_frm(args, workspace_root),
        "panadiga_read_hex" => forensics::handle_read_hex(args, workspace_root),
        "panadiga_hdd_capacity" => forensics::handle_hdd_capacity(args, workspace_root),
        "panadiga_nor_dumper_stub" => forensics::handle_nor_dumper_stub(args, workspace_root),
        "panadiga_bgo_unbrick" => forensics::handle_bgo_unbrick(args, workspace_root),
        "panadiga_main_passthrough" => forensics::handle_main_passthrough(args, workspace_root),
        "panadiga_sd_unbrick_kit" => unbrick::handle_sd_unbrick_kit(args, workspace_root),
        "panadiga_search_manuals" => knowledge::handle_search_manuals(args, workspace_root),
        "panadiga_rag_query" => knowledge::handle_rag_query(args),
        "panadiga_search_code" => knowledge::handle_search_code(args),
        "panadiga_ghidra_symbols" => Ok(knowledge::handle_ghidra_symbols(args)),
        "panadiga_search_visuals" => Ok(visuals::handle_search_visuals(args, workspace_root)),
        _ => anyhow::bail!("Unknown tool: {}", name),
    }
}
