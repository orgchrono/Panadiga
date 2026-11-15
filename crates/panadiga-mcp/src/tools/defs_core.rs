//! MCP JSON schema definitions for core, hardware, forensics, and RAG tools.

use serde_json::{Value, json};

pub(super) fn core_tool_definitions() -> Vec<Value> {
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
            "description": "Analyzes, sanitizes, or performs guarded copy-on-write patches in the 64KB NVRAM sector (0x7F0000..0x800000) of a Spansion S29JL064H 8MB flash dump (IPL stays strictly read-only).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["clean", "patch"],
                        "description": "Operation: 'clean' resets 64KB NVRAM to 0xFF (default), 'patch' writes hex bytes into [nvram_offset..nvram_offset+N] copy-on-write."
                    },
                    "input_file": {
                        "type": "string",
                        "description": "Path to original 8MB flash dump file."
                    },
                    "output_file": {
                        "type": "string",
                        "description": "Optional copy-on-write path where sanitized/patched flash file should be saved (never overwrites input_file)."
                    },
                    "nvram_offset": {
                        "type": ["string", "integer"],
                        "description": "Relative byte offset inside the 64KB NVRAM sector (0..65536) when action='patch' (default 0)."
                    },
                    "patch_hex": {
                        "type": "string",
                        "description": "Hex payload bytes to write at nvram_offset when action='patch'."
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
    ]
}
