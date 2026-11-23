# Panadiga Auto-Generated Tool Arsenal Index

> Gerado deterministicamente por `python scripts/generate_tool_arsenal.py --write`. Verifique deriva com `--check`.
> **Arsenal Fingerprint (`SHA-256`):** `ddeade5b13ce78cf7f24462d42f78a197be38fbd325e307125700eba2a4d51cb`

## 1. Resumo Quantitativo

| Categoria | Total | Fonte de Verdade |
| :--- | ---: | :--- |
| Crates Rust Workspace | 11 | `Cargo.toml` |
| Subcomandos `panadiga` CLI | 25 | `crates/panadiga-cli/src/cli/args/mod.rs` |
| Tools MCP Rust | 26 | `crates/panadiga-mcp/src/tools/defs*.rs` |
| Famílias de Resources MCP | 9 | `crates/panadiga-mcp/src/resources.rs` |
| Prompts MCP | 4 | `crates/panadiga-mcp/src/prompts.rs` |
| Comandos IPC Tauri 2 | 122 | `apps/desktop/src-tauri/src/lib.rs` |
| Comandos Sonda Pico `P7501` | 67 | `crates/panadiga-core/src/ssot/hardware_catalog.json` |
| Scripts Python Operacionais | 52 | `scripts/**/*.py` |
| Scripts Ghidra Jython | 9 | `tools/ghidra/*.py` |
| Skills `.agents/skills/` | 35 | `.agents/skills/*/SKILL.md` |
| Subagentes `.agents/agents/` | 6 | `.agents/agents/*.md` |

## 2. Crates Rust Workspace

| Crate | Caminho |
| :--- | :--- |
| `panadiga-constants` | `crates/panadiga-constants` |
| `panadiga-bench` | `crates/panadiga-bench` |
| `panadiga-vault` | `crates/panadiga-vault` |
| `panadiga-crypto` | `crates/panadiga-crypto` |
| `panadiga-ir` | `crates/panadiga-ir` |
| `panadiga-profiles` | `crates/panadiga-profiles` |
| `panadiga-core` | `crates/panadiga-core` |
| `panadiga-rag` | `crates/panadiga-rag` |
| `panadiga-mcp` | `crates/panadiga-mcp` |
| `panadiga-cli` | `crates/panadiga-cli` |
| `panadiga-desktop` | `apps/desktop/src-tauri` |

## 3. CLI `panadiga` & MCP (`Tools`, `Resources`, `Prompts`)

- **Subcomandos CLI:** `desktop`, `gui`, `interactive`, `nvram`, `hdd`, `frm`, `headers`, `scan`, `manuals`, `pico`, `profiles`, `ssot`, `ghidra`, `rag`, `memory-map`, `verify-artifact`, `gdb-config`, `gdb-bridge`, `mcp`, `unbrick`, `sd`, `dvd`, `session`, `re`, `install-shortcut`
- **MCP Tools:** `panadiga_memory_map`, `panadiga_inspect_hdd`, `panadiga_safety_check`, `panadiga_clean_nvram`, `panadiga_search_manuals`, `panadiga_rag_query`, `panadiga_inspect_frm`, `panadiga_read_hex`, `panadiga_pico_catalog`, `panadiga_ghidra_symbols`, `panadiga_search_code`, `panadiga_search_visuals`, `panadiga_project_status`, `panadiga_hdd_capacity`, `panadiga_nor_dumper_stub`, `panadiga_bgo_unbrick`, `panadiga_main_passthrough`, `panadiga_sd_unbrick_kit`, `panadiga_region_compat`, `panadiga_hdd_flash_inspect`, `panadiga_tbc_appliance_plan`, `panadiga_nor_sector_audit`, `panadiga_eh55_bgo`, `panadiga_davis_probe`, `panadiga_profiles_authorize`, `panadiga_symbol_lookup`
- **MCP Resources:** `panadiga://invariants`, `panadiga://memory-map`, `panadiga://pinout-p7501-uart`, `panadiga://visual-catalog`, `panadiga://models-catalog`, `panadiga://pico-commands`, `panadiga://profiles/<model>`, `panadiga://symbols/<train_id>/<module>`, `panadiga://docs/<path>`
- **MCP Prompts:** `u99_nvram_triage`, `p7501_uart_diagnostics`, `cddiag_p7501_capture`, `frm_crypto_re`

## 4. Comandos IPC Tauri 2 & Catálogo Pico `P7501`

- **Tauri IPC (`generate_handler!`):** `get_system_overview`, `clean_nvram`, `analyze_flash_dump`, `analyze_live_nvram_bytes`, `clean_nvram_command`, `patch_nvram_command`, `inspect_hdd`, `inspect_hdd_command`, `decode_meihdfs_allocations`, `decode_meihdfs_command`, `extract_partitions`, `extract_hdd_partitions_command`, `analyze_frm`, `analyze_frm_command`, `dump_frm_toc`, `dump_frm_toc_command`, `get_codebook`, `get_frm_codebook_command`, `list_frm_containers_command`, `pack_frm_command`, `unpack_extrhead_command`, `search_manuals`, `search_manuals_command`, `list_service_manuals`, `list_engineering_docs`, `read_engineering_doc`, `decode_engineering_log`, `read_hex_dump`, `get_diga_model_catalog`, `list_pico_serial_ports`, `pico_get_status`, `pico_set_breakin_mode`, `pico_inject_ram_loader`, `pico_first_contact_backup`, `pico_send_serial_command`, `pico_connect_and_test`, `pico_execute_cold_break_in`, `pico_execute_in_circuit_erase`, `pico_switch_to_gdb_mode`, `start_gdb_bridge_command`, `stop_gdb_bridge_command`, `get_gdb_bridge_status_command`, `get_am34_registers_command`, `get_gdb_telemetry_command`, `launch_external_tool_command`, `query_rag_engine`, `get_hardware_ssot_spec`, `pico_parse_diagnostic_output`, `pico_execute_diagnostic`, `pico_start_sc0_bridge`, `pico_send_bridge_input`, `pico_stop_sc0_bridge`, `pico_get_bridge_status`, `pico_capture_nvram_live`, `set_pico_telemetry_stream`, `pico_detect_bootsel_drives`, `pico_trigger_bootsel_reboot`, `pico_flash_firmware`, `list_physical_drives`, `list_hdd_drives`, `list_sd_card_drives`, `unmount_physical_drive`, `format_hdd_meihdfs`, `write_hdd_image`, `restore_hdd_mirrors`, `inspect_hdd_capacity_patch`, `apply_hdd_capacity_patch`, `inspect_unbrick_workbench`, `extract_frm_modules_command`, `patch_bgo_unbrick_command`, `patch_main_passthrough_command`, `prepare_sd_unbrick_kit_command`, `verify_sd_unbrick_media_command`, `inspect_sd_disk_command`, `build_nor_dumper_stub_command`, `run_matshita_boot_emu`, `inject_matshita_atapi_write`, `run_matshita_serial_cmd`, `verify_frm_unlock_command`, `pack_unlock_frm_command`, `inspect_region_calibration_command`, `apply_region_calibration_command`, `harden_sd_unbrick_kit_command`, `inspect_sd_card_targets_command`, `flash_sd_recovery_card_command`, `inspect_optical_burners_command`, `create_dvd_recovery_disc_command`, `list_hardware_profiles_command`, `get_resolved_model_profile_command`, `validate_hardware_profiles_command`, `authorize_profile_capability_command`, `fingerprint_nor_dump_command`, `vault_init_session_command`, `vault_list_sessions_command`, `vault_get_session_command`, `vault_ingest_file_command`, `vault_mark_step_command`, `vault_run_prewrite_emu_gate_command`, `apply_main_tbc_bypasses_command`, `inspect_safe_target_disks_command`, `flash_eh55_hdd_safe_command`, `flash_composed_hdd_safe_command`, `pico_eh55_nor_dump_command`, `pico_eh55_bgo_audit_command`, `pico_eh55_bgo_repair_command`, `pico_panamon1_clean_nvram_command`, `pico_probe_davis_ide_command`, `request_operation_nonce`, `get_bench_snapshot`, `get_runtime_settings`, `save_runtime_settings`, `reset_runtime_settings`, `log_frontend_event`, `install_desktop_shortcut`, `android_cmd`, `ir_catalog_stats`, `ir_search_catalog`, `ir_assemble_codeset`, `ir_import_profile`, `ir_export_irplus`, `ir_prepare_transmission`, `ir_get_verified_macros`

| Comando `P7501` | Categoria | Sintaxe | Restrito |
| :--- | :--- | :--- | :--- |
| `PINCHECK` | `electrical` | `PINCHECK` | no |
| `BUSDIAG` | `electrical` | `BUSDIAG` | no |
| `SNIFF` | `electrical` | `SNIFF [ms]` | no |
| `ISOLATE` | `electrical` | `ISOLATE` | no |
| `ENGAGE` | `electrical` | `ENGAGE` | no |
| `RESET` | `electrical` | `RESET` | no |
| `IDENTIFY` | `ocd_flash` | `IDENTIFY [PASSIVE|NORMAL|JIG] [ms]` | no |
| `BREAK_IN` | `ocd_flash` | `BREAK_IN [0..3]` | no |
| `JOB` | `ocd_flash` | `JOB [STATUS|NVRAM_CLEAN CONFIRM|BGO_APPLY CONFIRM|VAULT_BACKUP]` | yes |
| `AUTODETECT` | `ocd_flash` | `AUTODETECT` | no |
| `CFI` | `ocd_flash` | `CFI` | no |
| `FLASH_SIZE` | `ocd_flash` | `FLASH_SIZE [2|4|8|16]` | no |
| `DUMP` | `ocd_flash` | `DUMP <hex_addr> <words>` | no |
| `BOOTCAP` | `uart_fmu` | `BOOTCAP [ms]` | no |
| `UARTMON` | `uart_fmu` | `UARTMON [ms]` | no |
| `JIGBOOT` | `uart_fmu` | `JIGBOOT [ms]` | no |
| `JIGPIN` | `uart_fmu` | `JIGPIN [LOW|HIZ|STATUS]` | no |
| `BAUD` | `uart_fmu` | `BAUD [{{DEFAULT_BAUD_RATE}}|57600|38400|19200|9600]` | no |
| `FRAMING` | `uart_fmu` | `FRAMING [8N1|8E1|8O1]` | no |
| `BAUDSCAN` | `uart_fmu` | `BAUDSCAN [ms] [NORMAL|JIG] [APPLY]` | no |
| `UARTSTREAM` | `uart_fmu` | `UARTSTREAM [ms] [NORMAL|JIG[:ms]|NOPULL]` | no |
| `HEXCAP` | `uart_fmu` | `HEXCAP [ms] [NORMAL|JIG[:ms]|NOPULL]` | no |
| `UARTTXHEX` | `uart_fmu` | `UARTTXHEX <hex> [BRK] [rx_ms]` | no |
| `WAITTX` | `uart_fmu` | `WAITTX <ms> <NORMAL|JIG[:ms]|NOPULL> <trigger> <hex|BRK> [post_ms]` | no |
| `PWRTRACE` | `electrical` | `PWRTRACE [ms] [NORMAL|JIG[:ms]|NOPULL]` | no |
| `BOOTTRACE` | `electrical` | `BOOTTRACE [ms] [NORMAL|JIG[:ms]|NOPULL]` | no |
| `PIOLOGIC` | `electrical` | `PIOLOGIC [ms] [NORMAL|JIG[:ms]|NOPULL] [khz] [CTRL]` | no |
| `SDSNIFF` | `electrical` | `SDSNIFF [ms] [NORMAL|JIG[:ms]|NOPULL] [LEN:64|512]` | no |
| `JIGSWEEP` | `electrical` | `JIGSWEEP [HOLD|DELAY|PREBOOT] [ms..]` | no |
| `PANARSP` | `uart_fmu` | `PANARSP [ms] [boot] [MAGIC] [WAIT:<trig>] <PROBESEQ|REGS|G|Z0|Z1|MEM:addr,len|XMEM:addr,len|DUMPNOR[:addr[,len]]|PASSTHROUGH|CALL:addr|HEX:..>` | no |
| `DIAGWATCH` | `uart_fmu` | `DIAGWATCH [ms] [NORMAL|JIG[:ms]|NOPULL] [AUTO|BYPASS|READ[:addr[,len]]|WRITE:<addr,val>|FORMAT|DUMP|JUMP:<addr>|ERASE:<nvram_addr>|QUIT|NONE] [RELEASE|HOLD] [WAKE[:ms]]` | no |
| `IRTX` | `uart_fmu` | `IRTX <RED|GREEN|YELLOW|BLUE|POWER|AUDIO|DISPLAY|SERV|ND2|0..9|sub:cmd> [GP6..9|16..22] [repeat]` | no |
| `DRVPANA` | `uart_fmu` | `DRVPANA [ms] [NORMAL|JIG[:ms]|NOPULL] [NOPASS] <cmd...>` | no |
| `BRIDGE` | `uart_fmu` | `BRIDGE [baud] [NORMAL|JIG[:ms]|NOPULL]` | yes |
| `FMUSCAN` | `uart_fmu` | `FMUSCAN [modo]` | no |
| `FMUTEST` | `uart_fmu` | `FMUTEST <hex>` | no |
| `FMUBATCH` | `uart_fmu` | `FMUBATCH [step]` | no |
| `UARTSEND` | `uart_fmu` | `UARTSEND <texto>` | no |
| `UARTPROBE` | `uart_fmu` | `UARTPROBE [0..8]` | no |
| `PING` | `probe` | `PING` | no |
| `STATUS` | `probe` | `STATUS` | no |
| `BANNER` | `probe` | `BANNER` | no |
| `WDT` | `probe` | `WDT` | no |
| `LED` | `probe` | `LED <modo>` | no |
| `BOOTSEL` | `probe` | `BOOTSEL` | yes |
| `SELFTEST` | `probe` | `SELFTEST [LOOPBACK]` | no |
| `TARGET_PROFILE` | `ocd_flash` | `TARGET_PROFILE [DMR-EH55|DMR-ES35V|DMR-EZ485V|DMR-ES10]` | no |
| `VAULT` | `ocd_flash` | `VAULT [STATUS|LIST|DUMP <1..3>|ERASE <1..3> CONFIRM]` | no |
| `BLACKBOX` | `probe` | `BLACKBOX [STATUS|DUMP|SNAP [msg]|CLEAR CONFIRM]` | no |
| `PROBE_CFG` | `probe` | `PROBE_CFG [STATUS|CAL_ADC <gain_ppm> [offset_mv]|RESET CONFIRM]` | no |
| `RECIPE` | `ocd_flash` | `RECIPE [STATUS|LIST|VERIFY]` | no |
| `BGO_INSPECT` | `ocd_flash` | `BGO_INSPECT` | no |
| `EEMUL_INSPECT` | `ocd_flash` | `EEMUL_INSPECT` | no |
| `IRRX` | `electrical` | `IRRX [ms] [GP6..9|16..22]` | no |
| `SERVMACRO` | `electrical` | `SERVMACRO <SERVICE_MODE|SELF_CHECK|OPTICAL_CAL|ERR_HISTORY|RELEASE> [GPn]` | no |
| `MMIO_SNAP` | `ocd_flash` | `MMIO_SNAP [DIFF]` | no |
| `PWROSC` | `electrical` | `PWROSC [ms]` | no |
| `TBUSSNIFF` | `uart_fmu` | `TBUSSNIFF [ms]` | no |
| `M0KEEPALIVE` | `uart_fmu` | `M0KEEPALIVE [ON [1000..30000ms]|OFF|STATUS]` | no |
| `I2C_SCAN` | `electrical` | `I2C_SCAN` | no |
| `EEPROM` | `ocd_flash` | `EEPROM <INFO|READ <addr> <off> <len>|WRITE <addr> <off> <hex> CONFIRM>` | no |
| `USBMODE` | `probe` | `USBMODE [SINGLE|DUAL_CDC|STATUS]` | no |
| `BLE` | `probe` | `BLE [STATUS|DISCONNECT]` | no |
| `BLE_NOR_AUTH` | `probe` | `BLE_NOR_AUTH <CONFIRM [1..16]|REVOKE|STATUS>` | yes |
| `NOR_MAP` | `ocd_flash` | `NOR_MAP` | no |
| `FAST_READ` | `ocd_flash` | `FAST_READ <hex_va> <hex_len_mult_100>` | no |
| `FAST_WRITE` | `ocd_flash` | `FAST_WRITE <hex_va> <even_hex_bytes> CONFIRM_UPPER_NOR` | yes |

## 5. Scripts Python, Ghidra Jython, Skills & Subagentes

- **Scripts Python:** `scripts/bench/boot_harness.py`, `scripts/bench/decoders.py`, `scripts/bench/nor_dumper.py`, `scripts/bench/protocol.py`, `scripts/bench/rsp_dump.py`, `scripts/bench/rsp_frames.py`, `scripts/bench/runner.py`, `scripts/build.py`, `scripts/build_cache/inventory.py`, `scripts/build_cache/steps.py`, `scripts/build_lanes/android.py`, `scripts/build_lanes/android_verify.py`, `scripts/build_lanes/concurrency.py`, `scripts/build_lanes/dag.py`, `scripts/build_lanes/desktop.py`, `scripts/build_lanes/pico.py`, `scripts/dev/cad_primitives.py`, `scripts/dev/generate_schematics_and_cad.py`, `scripts/dev/schematic_sheet.py`, `scripts/generate_firmware_atlas.py`, `scripts/generate_tool_arsenal.py`, `scripts/ops/proc.py`, `scripts/ops/toolchain.py`, `scripts/panadiga_cache.py`, `scripts/panadiga_cli_bridge.py`, `scripts/panadiga_ops.py`, `scripts/pico_heartbeat_ops.py`, `scripts/profiles_types.py`, `scripts/re/_discovery_common.py`, `scripts/re/fleet_decrypt/catalog.py`, `scripts/re/fleet_decrypt/discovery.py`, `scripts/re/fleet_decrypt/unpacker.py`, `scripts/re/fleet_decryptor.py`, `scripts/re/frm_train_harvester.py`, `scripts/re/ghidra_fleet/drift_smoke.py`, `scripts/re/ghidra_fleet/isa_resolver.py`, `scripts/re/ghidra_fleet/pyghidra_runner.py`, `scripts/re/ghidra_fleet/report_export.py`, `scripts/re/ghidra_fleet_orchestrator.py`, `scripts/re/manual_spec_extractor.py`, `scripts/re/mn10300.py`, `scripts/re/mn103_traps.py`, `scripts/re/nor_intake_analyzer.py`, `scripts/re/pana_frm.py`, `scripts/repo_sanitize_and_sync.py`, `scripts/tui.py`, `scripts/tui_plan.py`, `scripts/tui_term.py`, `scripts/validate.py`, `scripts/validate_c11.py`, `scripts/validate_gates.py`, `scripts/verify_geometry_traps.py`
- **Scripts Ghidra:** `tools/ghidra/ghidra_am34_memory_map.py`, `tools/ghidra/ghidra_auto_unbrick_audit.py`, `tools/ghidra/ghidra_conectar_debugger.py`, `tools/ghidra/ghidra_connect_debugger.py`, `tools/ghidra/ghidra_export_pseudocode_bundle.py`, `tools/ghidra/ghidra_import_symbols_panadiga.py`, `tools/ghidra/ghidra_importar_simbolos_panadiga.py`, `tools/ghidra/ghidra_mn103_fix_decompilation.py`, `tools/ghidra/ghidra_zero_undef_pipeline.py`
- **Skills:** `panadiga-adversarial-and-hallucination-auditor`, `panadiga-anti-trap-geometry-verification`, `panadiga-architecture-and-guardrails`, `panadiga-bench-automation-and-headless`, `panadiga-bench-operations`, `panadiga-code-compliance-and-layer-coherence`, `panadiga-contracts-stream-and-journal`, `panadiga-device-onboarding`, `panadiga-emulation-and-patching`, `panadiga-extract-partitions`, `panadiga-firmware-decrypto`, `panadiga-firmware-forensic-atlas`, `panadiga-forensic-archival`, `panadiga-ghidra-workflow`, `panadiga-git-cicd-and-releases`, `panadiga-hardware-forensics`, `panadiga-nvram-unbrick`, `panadiga-opensource-firmware-dev`, `panadiga-pico-flashing`, `panadiga-pico-heartbeat-and-guided-ops`, `panadiga-prompt-and-adversarial-audit`, `panadiga-rag-and-mcp-engineering`, `panadiga-repo-sanitization-and-commit-sync`, `panadiga-rust-backend-dev`, `panadiga-security-and-violation-guards`, `panadiga-skill-authoring`, `panadiga-skill-orchestrator`, `panadiga-ssot-builder-and-validation`, `panadiga-subagent-and-research`, `panadiga-technical-documentation`, `panadiga-testing-and-feature-delivery`, `panadiga-tool-arsenal-catalog`, `panadiga-typescript-frontend-dev`, `panadiga-ux-ui-and-layout`, `panadiga-validate`
- **Subagentes:** `panadiga-architect`, `panadiga-desktop-ui`, `panadiga-firmware-re`, `panadiga-hardware-embedded`, `panadiga-qa-auditor`, `panadiga-storage-forensics`
