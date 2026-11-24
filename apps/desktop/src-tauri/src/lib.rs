//! Panasonic DMR-EH55 DIGA Engineering Suite - Cross-Platform Desktop & Android Backend Library

#![forbid(unsafe_code)]
#![cfg_attr(not(test), deny(clippy::print_stdout, clippy::print_stderr))]

use tauri::Manager;

pub mod commands;
pub mod pico;
pub mod state;

pub use commands::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    run_desktop_app();
}

macro_rules! build_invoke_handler {
    ($($android_cmd:path),* $(,)?) => {
        tauri::generate_handler![
            get_system_overview,
            clean_nvram,
            analyze_flash_dump,
            analyze_live_nvram_bytes,
            clean_nvram_command,
            patch_nvram_command,
            inspect_hdd,
            inspect_hdd_command,
            decode_meihdfs_allocations,
            decode_meihdfs_command,
            extract_partitions,
            extract_hdd_partitions_command,
            analyze_frm,
            analyze_frm_command,
            dump_frm_toc,
            dump_frm_toc_command,
            get_codebook,
            get_frm_codebook_command,
            list_frm_containers_command,
            pack_frm_command,
            unpack_extrhead_command,
            search_manuals,
            search_manuals_command,
            list_service_manuals,
            list_engineering_docs,
            read_engineering_doc,
            decode_engineering_log,
            read_hex_dump,
            get_diga_model_catalog,
            list_pico_serial_ports,
            pico_get_status,
            pico_set_breakin_mode,
            pico_inject_ram_loader,
            pico_first_contact_backup,
            pico_send_serial_command,
            pico_connect_and_test,
            pico_execute_cold_break_in,
            pico_execute_in_circuit_erase,
            pico_switch_to_gdb_mode,
            start_gdb_bridge_command,
            stop_gdb_bridge_command,
            get_gdb_bridge_status_command,
            get_am34_registers_command,
            get_gdb_telemetry_command,
            launch_external_tool_command,
            query_rag_engine,
            get_hardware_ssot_spec,
            pico_parse_diagnostic_output,
            pico_execute_diagnostic,
            pico_start_sc0_bridge,
            pico_send_bridge_input,
            pico_stop_sc0_bridge,
            pico_get_bridge_status,
            pico_capture_nvram_live,
            set_pico_telemetry_stream,
            pico_detect_bootsel_drives,
            pico_trigger_bootsel_reboot,
            pico_flash_firmware,
            list_physical_drives,
            list_hdd_drives,
            list_sd_card_drives,
            unmount_physical_drive,
            format_hdd_meihdfs,
            write_hdd_image,
            restore_hdd_mirrors,
            inspect_hdd_capacity_patch,
            apply_hdd_capacity_patch,
            inspect_unbrick_workbench,
            extract_frm_modules_command,
            patch_bgo_unbrick_command,
            patch_main_passthrough_command,
            prepare_sd_unbrick_kit_command,
            verify_sd_unbrick_media_command,
            inspect_sd_disk_command,
            build_nor_dumper_stub_command,
            run_matshita_boot_emu,
            inject_matshita_atapi_write,
            run_matshita_serial_cmd,
            verify_frm_unlock_command,
            pack_unlock_frm_command,
            inspect_region_calibration_command,
            apply_region_calibration_command,
            harden_sd_unbrick_kit_command,
            inspect_sd_card_targets_command,
            flash_sd_recovery_card_command,
            inspect_optical_burners_command,
            create_dvd_recovery_disc_command,
            list_hardware_profiles_command,
            get_resolved_model_profile_command,
            validate_hardware_profiles_command,
            authorize_profile_capability_command,
            fingerprint_nor_dump_command,
            vault_init_session_command,
            vault_list_sessions_command,
            vault_get_session_command,
            vault_ingest_file_command,
            vault_mark_step_command,
            vault_run_prewrite_emu_gate_command,
            apply_main_tbc_bypasses_command,
            inspect_safe_target_disks_command,
            flash_eh55_hdd_safe_command,
            flash_composed_hdd_safe_command,
            pico_eh55_nor_dump_command,
            pico_eh55_bgo_audit_command,
            pico_eh55_bgo_repair_command,
            pico_panamon1_clean_nvram_command,
            pico_probe_davis_ide_command,
            request_operation_nonce,
            get_bench_snapshot,
            get_runtime_settings,
            save_runtime_settings,
            reset_runtime_settings,
            log_frontend_event,
            install_desktop_shortcut,
            $($android_cmd,)*
        ]
    };
}

/// Initializes native desktop runtime plugins, event listeners, and hardware-safe defaults.
pub fn run_desktop_app() {
    let settings = panadiga_core::settings::load_runtime_settings();
    let initial_level = panadiga_core::logger::LogLevel::parse_lossy(&settings.workbench.log_level);
    panadiga_core::logger::init_telemetry(initial_level);
    tauri::Builder::default()
        .manage(state::PanadigaBackendState::new())
        .setup(|app| {
            #[cfg(not(target_os = "android"))]
            panadiga_core::launcher::ensure_linux_desktop_launcher_if_missing();
            let backend = app.state::<state::PanadigaBackendState>();
            crate::commands::hardware_diag::attach_bench_stream_sink(
                app.handle().clone(),
                backend.inner(),
            );
            crate::commands::hardware_diag::start_port_mutation_observer(
                app.handle().clone(),
                backend.inner(),
            );
            Ok(())
        })
        .invoke_handler({
            #[cfg(target_os = "android")]
            {
                build_invoke_handler!(
                    ir_catalog_stats,
                    ir_search_catalog,
                    ir_assemble_codeset,
                    ir_import_profile,
                    ir_export_irplus,
                    ir_prepare_transmission,
                    ir_get_verified_macros,
                )
            }
            #[cfg(not(target_os = "android"))]
            {
                build_invoke_handler!()
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
