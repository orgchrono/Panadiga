//! Extended Reverse Engineering & Bench Integration MCP tool schemas (`panadiga_region_compat`,
//! `panadiga_hdd_flash_inspect`, `panadiga_tbc_appliance_plan`, `panadiga_nor_sector_audit`,
//! `panadiga_eh55_bgo`, `panadiga_davis_probe`).

use serde_json::{Value, json};

pub(super) fn re_ext_tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "panadiga_region_compat",
            "description": "CoW inspect/patch for MAIN <deviceinfo>, DEV.BIN/ifodev2 (tail 0x400..0x7FF strictly metadata_and_dev_oem_only read-only), .FRM TOC retargeting, and LBS/TV calibration bundle inspection.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["inspect_main", "patch_main", "inspect_dev", "patch_dev", "retarget_frm", "inspect_tv_bundle"],
                        "description": "Operation mode (default 'inspect_tv_bundle')."
                    },
                    "input_path": { "type": "string", "description": "Input MAIN, DEV.BIN/ifodev2, or .FRM path." },
                    "output_path": { "type": "string", "description": "Distinct Copy-on-Write output path for patch/retarget actions." },
                    "mode": { "type": "string", "enum": ["bypass", "factory_strict"], "description": "MAIN patch mode." },
                    "model": { "type": "string", "description": "Target 1..=8 ASCII model string for patch_dev (e.g. EH55EG)." },
                    "oem": { "type": "string", "description": "Optional single ASCII OEM character (e.g. M or E)." },
                    "hw": { "type": "string", "description": "Optional .FRM hardware family (e.g. M6930LB or M6930)." },
                    "model_group": { "type": "string", "description": "Optional .FRM model_group (e.g. NP-309)." },
                    "target_flag": { "type": "integer", "description": "Optional .FRM FormatFlag byte." },
                    "preset": { "type": "string", "enum": ["br-pal-m", "us-ntsc", "eu-pal", "custom"], "description": "Regional LBS preset." },
                    "tv_standard": { "type": "string", "enum": ["ntsc", "pal-m", "pal", "secam"], "description": "TV color standard." },
                    "dev_bin_path": { "type": "string", "description": "Optional DEV.BIN / ifodev2 path for bundle audit." },
                    "main_path": { "type": "string", "description": "Optional MAIN path for bundle audit." },
                    "frm_path": { "type": "string", "description": "Optional .FRM path for bundle audit." },
                    "nvram_path": { "type": "string", "description": "Optional NVRAM/8MB NOR path for bundle audit." }
                }
            }
        }),
        json!({
            "name": "panadiga_hdd_flash_inspect",
            "description": "STRICTLY READ-ONLY / DRY-RUN EH55 HDD flash planner. Audits safe target disks (hard-blocking PhysicalDrive0, OS disks, and BusTypeNvme 17), computes mirror_main_lba and bounded 2KB sector blocks. Never writes disks.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "target_gb": { "type": "integer", "description": "Target disk capacity in GB to validate 2KB block math (default 250)." },
                    "allow_extended_4tb": { "type": "boolean", "description": "Allow 2201..=4096 GB range check (default false)." },
                    "disk_bytes": { "type": "integer", "description": "Optional explicit byte length to compute mirror_main_lba." },
                    "mock_disks": {
                        "type": "array",
                        "description": "Optional offline simulated disk list for deterministic lockout planning.",
                        "items": { "type": "object" }
                    }
                }
            }
        }),
        json!({
            "name": "panadiga_tbc_appliance_plan",
            "description": "Generates declarative clean-room Diskless/TBC appliance plan (Stage-1.5 DIGADIAG TBC default; experimental_sd_ram_chainload gated).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "mode": { "type": "string", "enum": ["stage15_digadiag_tbc", "experimental_sd_ram_chainload"] },
                    "tv_standard": { "type": "string", "enum": ["ntsc", "pal-m", "pal", "secam"] },
                    "experimental_ram_chainload": { "type": "boolean", "description": "Explicit opt-in required for experimental_sd_ram_chainload." }
                }
            }
        }),
        json!({
            "name": "panadiga_nor_sector_audit",
            "description": "Audits all 142 sectors (SA0..SA141) of an 8 MiB Spansion S29JL064H IC5001 NOR dump offline (read-only).",
            "inputSchema": {
                "type": "object",
                "required": ["nor_path"],
                "properties": {
                    "nor_path": { "type": "string", "description": "Path to 8 MiB Spansion NOR dump." }
                }
            }
        }),
        json!({
            "name": "panadiga_eh55_bgo",
            "description": "Audits 45-sector EH55 BGO FTL (SA26..=SA70), builds CoW repair plans, or executes guarded live repair (requires confirm_upper_nor=true + backup_dir; Layer 1 256 KiB IPL SA0..SA10 permanently locked E05).",
            "inputSchema": {
                "type": "object",
                "required": ["action"],
                "properties": {
                    "action": { "type": "string", "enum": ["audit", "plan", "live_apply"] },
                    "nor_path": { "type": "string", "description": "8 MiB IC5001 NOR path (for audit/plan)." },
                    "reference_nor_path": { "type": "string", "description": "Optional healthy reference NOR for plan." },
                    "out_plan_path": { "type": "string", "description": "Optional JSON output path for BgoRepairPlan." },
                    "cow_out_path": { "type": "string", "description": "Optional distinct CoW 8 MiB repaired NOR output path." },
                    "port": { "type": "string", "description": "Pico serial port (required for live_apply)." },
                    "plan_path": { "type": "string", "description": "Saved BgoRepairPlan JSON path (required for live_apply)." },
                    "backup_dir": { "type": "string", "description": "Mandatory CoW backup dir (required for live_apply)." },
                    "confirm_upper_nor": { "type": "boolean", "description": "Explicit opt-in unlocking Upper NOR SA26..=SA70 (must be true for live_apply)." },
                    "hitl_token": { "type": "string", "description": "Required Human-In-The-Loop authorization token matching PANADIGA_MCP_HITL_TOKEN for live_apply." },
                    "boot_trace_ms": { "type": "integer", "description": "Optional post-repair BOOTTRACE window ms (default 15000)." },
                    "no_boottrace": { "type": "boolean", "description": "Skip post-repair BOOTTRACE check." }
                }
            }
        }),
        json!({
            "name": "panadiga_davis_probe",
            "description": "AM34 DAVIS CH0/CH1 IDE & 0xEC IDENTIFY probe tool: offline stub builder, offline IDENTIFY decoder, or live PANAMON1> probe guarding lethal 0xC000_100C.",
            "inputSchema": {
                "type": "object",
                "required": ["action"],
                "properties": {
                    "action": { "type": "string", "enum": ["build_stubs", "parse_ec", "run_live"] },
                    "sdram_stub_va": { "type": "string", "description": "Optional hex SDRAM stub VA (build_stubs/run_live)." },
                    "identify_out_va": { "type": "string", "description": "Optional hex SDRAM IDENTIFY output VA." },
                    "drive": { "type": "string", "enum": ["master", "slave"], "description": "CH1 drive selector (default master)." },
                    "input_path": { "type": "string", "description": "512B or 1024B PIO IDENTIFY binary file (for parse_ec)." },
                    "ec_hex": { "type": "string", "description": "Alternative hex-encoded 512B or 1024B IDENTIFY buffer (for parse_ec)." },
                    "port": { "type": "string", "description": "Pico serial port (required for run_live)." },
                    "hitl_token": { "type": "string", "description": "Required Human-In-The-Loop authorization token matching PANADIGA_MCP_HITL_TOKEN for run_live." }
                }
            }
        }),
        json!({
            "name": "panadiga_profiles_authorize",
            "description": "Evaluates fail-closed CapabilityGate policy for a DIGA model profile given runtime probe evidence and optional audited LabOverride.",
            "inputSchema": {
                "type": "object",
                "required": ["model_id", "capability"],
                "properties": {
                    "model_id": { "type": "string", "description": "DIGA model ID or exact alias (e.g. DMR-EH55, DMR-ES35V, DMR-EZ485V, DMR-ES10)." },
                    "capability": {
                        "type": "string",
                        "enum": [
                            "inspect_read_only",
                            "nvram_sanitize_offline_cow",
                            "nvram_write_live",
                            "upper_nor_patch_offline_cow",
                            "upper_nor_write_live",
                            "official_frm_pass_through",
                            "custom_recovery_media",
                            "critical_ipl_or_lethal_mmio"
                        ],
                        "description": "Target capability to evaluate against claim tiers and silicon invariants."
                    },
                    "dump_size_bytes": { "type": "integer", "description": "Observed NOR dump byte length." },
                    "jedec_mfg_id": { "type": ["string", "integer"], "description": "Optional live JEDEC manufacturer ID (e.g. '0x01' or 1)." },
                    "jedec_device_cycles": {
                        "type": "array",
                        "items": { "type": ["string", "integer"] },
                        "description": "Optional live JEDEC device ID words (e.g. ['0x227E', '0x220A', '0x2201'])."
                    },
                    "wp_pin_deasserted": { "type": "boolean", "description": "Verified WP#/ACC deasserted state for chips requiring top-sector protection check." },
                    "has_eemul_f1_marker": { "type": "boolean", "description": "True if EEMUL 0xF1 active marker was detected in top sectors." },
                    "loader_banner": { "type": "string", "description": "Optional observed IPL loader banner string." },
                    "frm_sha256": { "type": "string", "description": "Optional observed official .FRM SHA-256 hex digest." },
                    "requested_loader_profile": { "type": "string", "description": "Optional requested SD loader profile key (e.g. 'ver061')." },
                    "lab_override_disclaimer": { "type": "string", "description": "Optional operator-supplied LabOverride disclaimer acknowledgement for offline R&D override (never auto-filled by agents)." },
                    "lab_override_reason": { "type": "string", "description": "Operator justification (>= 5 chars) recorded for offline LabOverride." }
                }
            }
        }),
        json!({
            "name": "panadiga_symbol_lookup",
            "description": "Queries train-keyed MN103S symbol dictionary (profiles/symbols/<train>/<module>.json) returning VA, name, ABI (D0/D1/(0x0C,SP)), ret, callers[], callees[], string/MMIO xrefs, and destructive side_effects safety gate (writes_nor_bgo | writes_hdd | writes_nvram).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "train_id": { "type": "string", "description": "Firmware train ID filter (e.g. loader_ver061_toc48_main57; never model names)." },
                    "module": { "type": "string", "description": "Module filter (e.g. PROG061, BOOT, MAIN)." },
                    "va": { "type": "string", "description": "Exact hex virtual address lookup (e.g. 0x50409DF2)." },
                    "name": { "type": "string", "description": "Substring match against resolved symbol name." },
                    "caller_of": { "type": "string", "description": "Return functions that directly call this target VA." },
                    "callee_of": { "type": "string", "description": "Return functions called by this caller VA." },
                    "side_effect": { "type": "string", "enum": ["read_only", "writes_nor_bgo", "writes_hdd", "writes_nvram", "writes_mmio", "unknown"], "description": "Filter by classified hardware side effect." },
                    "string_xref": { "type": "string", "description": "Filter by embedded ASCII string reference." },
                    "mmio_xref": { "type": "string", "description": "Filter by referenced Flash/DAVIS/MMIO hex address." },
                    "limit": { "type": "integer", "description": "Max symbols to return (1..200, default 50)." }
                }
            }
        }),
    ]
}
