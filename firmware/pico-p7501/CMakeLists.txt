# ==============================================================================
# Panasonic DMR-EH55 DIGA Engineering Suite
# Subsystem: P7501 Async UART (115200 8N1) & Discrete Boot Probe (RP2040 / RP2350)
# Target Hardware: Raspberry Pi Pico (RP2040) / Raspberry Pi Pico 2 (RP2350)
# ==============================================================================

cmake_minimum_required(VERSION 3.13)

# Default to standard RP2040 (pico) if not specified; supports pico, pico_w, pico2, pico2_w
if(NOT DEFINED PICO_BOARD)
    set(PICO_BOARD pico CACHE STRING "Target Pico Board: 'pico', 'pico_w', 'pico2', or 'pico2_w'")
endif()

# Include official Raspberry Pi Pico SDK import helper
include(pico_sdk_import.cmake)

project(panasonic_p7501_uart C CXX ASM)
set(CMAKE_C_STANDARD 11)
set(CMAKE_CXX_STANDARD 17)

# Initialize the Raspberry Pi Pico SDK (auto-configures architecture for RP2040 or RP2350)
pico_sdk_init()

message(STATUS "[PANADIGA] Configuring P7501 UART Probe for Board: ${PICO_BOARD} (SOC: ${PICO_PLATFORM})")

# Define main firmware executable (modular architecture grouped by concern)
add_executable(panasonic_p7501_uart
    p7501_pico_main.c
    common/p7501_cmd_arena.c
    common/p7501_hex.c
    common/p7501_crc_csum.c
    common/p7501_probe_fsm.c
    common/p7501_push_stream.c
    common/p7501_spsc_ring.c
    common/p7501_wdt_logic.c
    crypto/p7501_feistel.c
    crypto/p7501_compress.c
    crypto/p7501_decomp.c
    crypto/p7501_crypto_cli.c
    hal/p7501_ble_nus.c
    hal/p7501_bootsel_btn.c
    hal/p7501_bootsel_logic.c
    hal/p7501_hw.c
    hal/p7501_led_logic.c
    hal/p7501_led.c
    hal/p7501_multicore.c
    hal/p7501_usb_dual_cdc.c
    hal/p7501_watchdog.c
    hal/p7501_wireless_codec.c
    gdb/gdb_rsp_codec.c
    gdb/gdb_rsp_memory.c
    gdb/gdb_rsp_cpu.c
    generated/p7501_ssot_tables.c
    flash/p7501_blackbox_codec.c
    flash/p7501_bus_policy.c
    flash/p7501_bus_tx_guard.c
    flash/p7501_flash_geo.c
    flash/p7501_fast_stream_codec.c
    flash/p7501_normap_codec.c
    flash/p7501_fast_stub_io.c
    flash/p7501_fast_stream.c
    flash/p7501_ftl_inspect.c
    flash/p7501_job_codec.c
    flash/p7501_job.c
    flash/p7501_payload_cli.c
    flash/p7501_payloads.c
    flash/p7501_trap_guard.c
    flash/p7501_probe_cfg_codec.c
    flash/p7501_qspi_hal.c
    flash/p7501_qspi_map.c
    flash/p7501_qspi_storage.c
    flash/p7501_sector_ops.c
    flash/p7501_target_bus.c
    flash/p7501_target_bus_panamon.c
    flash/p7501_target_bus_pio.c
    flash/p7501_flash.c
    flash/p7501_vault_codec.c
    flash/p7501_vault.c
    diag/p7501_diag_electrical.c
    diag/p7501_diag_bus.c
    diag/p7501_diag_busdiag.c
    diag/p7501_diag_pwrtrace.c
    diag/p7501_boottrace_codec.c
    diag/p7501_diag_boottrace.c
    diag/p7501_gps_cli.c
    diag/p7501_hotplug_codec.c
    diag/p7501_hotplug_map.c
    diag/p7501_i2c_codec.c
    diag/p7501_bitwise_search.c
    diag/p7501_hdd_unlock.c
    diag/p7501_math_gps.c
    diag/p7501_math_stats.c
    diag/p7501_meihfs_probe.c
    diag/p7501_microsurgery.c
    diag/p7501_mn103_asm.c
    diag/p7501_mn103_disasm.c
    diag/p7501_pieos_xray.c
    diag/p7501_surg_cli.c
    diag/p7501_surg_live.c
    diag/p7501_xray_cli.c
    diag/p7501_i2c_master.c
    diag/p7501_identify_codec.c
    diag/p7501_identify.c
    diag/p7501_mmio_snapshot_codec.c
    diag/p7501_mmio_snapshot.c
    diag/p7501_piologic_codec.c
    diag/p7501_diag_piologic.c
    diag/p7501_pwrosc_codec.c
    diag/p7501_pwrosc.c
    diag/p7501_sds_codec.c
    diag/p7501_diag_sdsniff.c
    diag/p7501_jigsweep_codec.c
    diag/p7501_diag_jigsweep.c
    diag/p7501_ir_codec.c
    diag/p7501_ir_kaseikyo.c
    diag/p7501_serv_macro_codec.c
    diag/p7501_serv_macro.c
    diag/p7501_tbus_codec.c
    diag/p7501_tbus.c
    flash/p7501_flash_cli.c
    uart/p7501_uart_autobaud.c
    uart/p7501_uart_codec.c
    uart/p7501_uart_args.c
    uart/p7501_uart_capture.c
    uart/p7501_fmu_codec.c
    uart/p7501_fmu_probe.c
    uart/p7501_fmu_batch.c
    uart/p7501_uart_interactive.c
    uart/p7501_uart_pio_engine.c
    uart/p7501_uart_pio_tx.c
    uart/p7501_pio_loop.c
    uart/p7501_uart_baud.c
    uart/p7501_uart_stream.c
    uart/p7501_uart_tx.c
    uart/p7501_pana_dumpnor.c
    uart/p7501_pana_dumpnor_verify.c
    uart/p7501_pana_jedec_stubs.c
    uart/p7501_pana_jedec_exec.c
    uart/p7501_pana_jedec_id.c
    uart/p7501_pana_rsp_codec.c
    uart/p7501_pana_rsp_guard.c
    uart/p7501_pana_rsp_parse.c
    uart/p7501_pana_rsp_tokens.c
    uart/p7501_pana_rsp_report.c
    uart/p7501_pana_rsp_exec.c
    uart/p7501_pana_rsp.c
    uart/p7501_diag_watch_codec.c
    uart/p7501_diag_watch_events.c
    uart/p7501_diag_watch.c
    uart/p7501_drv_pana_codec.c
    uart/p7501_drv_pana.c
    cli/p7501_cli_help.c
    cli/p7501_cli_sys.c
    cli/p7501_cli.c
)

# Header include paths
target_include_directories(panasonic_p7501_uart PRIVATE
    ${CMAKE_CURRENT_LIST_DIR}
)

# Zero soft-float printf bloat & hardened 4 KiB Core 0 stack + 512 B frame ceiling
target_compile_definitions(panasonic_p7501_uart PRIVATE
    PICO_PRINTF_SUPPORT_FLOAT=0
    PICO_PRINTF_SUPPORT_EXPONENTIAL=0
    PICO_STACK_SIZE=0x1000
)

target_compile_options(panasonic_p7501_uart PRIVATE
    -Os
    -Wall
    -Wextra
    -Wformat=2
    -Wformat-security
    -fno-common
    -fstack-protector-strong
    -ffunction-sections
    -fdata-sections
    -Wframe-larger-than=512
)

target_link_options(panasonic_p7501_uart PRIVATE
    -Wl,--gc-sections
)

# Assemble PIO UART program and generate C header
pico_generate_pio_header(panasonic_p7501_uart ${CMAKE_CURRENT_LIST_DIR}/p7501_uart_duplex.pio)
pico_generate_pio_header(panasonic_p7501_uart ${CMAKE_CURRENT_LIST_DIR}/p7501_logic_sampler.pio)
pico_generate_pio_header(panasonic_p7501_uart ${CMAKE_CURRENT_LIST_DIR}/p7501_sds_sampler.pio)

# Link essential Pico SDK and RP2040/RP2350 hardware drivers
target_link_libraries(panasonic_p7501_uart
    pico_stdlib
    pico_multicore
    hardware_pio
    hardware_dma
    hardware_adc
    hardware_clocks
    hardware_flash
    hardware_i2c
    hardware_pwm
    hardware_sync
    hardware_watchdog
)

# Optional Bluetooth Low Energy (CYW43439 / BTstack) support for Pico W and Pico 2 W
if((PICO_BOARD STREQUAL "pico_w" OR PICO_BOARD STREQUAL "pico2_w") AND TARGET pico_cyw43_arch_none AND TARGET pico_btstack_ble)
    message(STATUS "[PANADIGA] Enabling Bluetooth LE Nordic UART Service (NUS) for ${PICO_BOARD}")
    target_compile_definitions(panasonic_p7501_uart PRIVATE
        PANADIGA_BLE_ENABLED=1
    )
    target_link_libraries(panasonic_p7501_uart
        pico_cyw43_arch_none
        pico_btstack_ble
    )
elseif(PICO_BOARD STREQUAL "pico_w" OR PICO_BOARD STREQUAL "pico2_w")
    message(STATUS "[PANADIGA] ${PICO_BOARD}: pico_cyw43_arch_none / pico_btstack_ble targets unavailable (Pico SDK wireless submodules uninitialized); building wired USB CDC fallback.")
endif()

# Enable USB CDC serial console, disable UART0
pico_enable_stdio_usb(panasonic_p7501_uart 1)
pico_enable_stdio_uart(panasonic_p7501_uart 0)

# Generate UF2, ELF, BIN, HEX, and MAP outputs
pico_add_extra_outputs(panasonic_p7501_uart)
