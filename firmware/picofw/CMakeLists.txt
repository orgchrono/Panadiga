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

project(picofw C CXX ASM)
set(CMAKE_C_STANDARD 11)
set(CMAKE_CXX_STANDARD 17)

# Initialize the Raspberry Pi Pico SDK (auto-configures architecture for RP2040 or RP2350)
pico_sdk_init()

message(STATUS "[PICOFW] Configuring P7501 UART Probe for Board: ${PICO_BOARD} (SOC: ${PICO_PLATFORM})")

# Define main firmware executable (modular architecture grouped by concern)
add_executable(picofw
    picofw_pico_main.c
    common/picofw_cmd_arena.c
    common/picofw_hex.c
    common/picofw_crc_csum.c
    common/picofw_probe_fsm.c
    common/picofw_push_stream.c
    common/picofw_spsc_ring.c
    common/picofw_wdt_logic.c
    crypto/picofw_feistel.c
    crypto/picofw_compress.c
    crypto/picofw_decomp.c
    crypto/picofw_crypto_cli.c
    hal/picofw_ble_nus.c
    hal/picofw_bootsel_btn.c
    hal/picofw_bootsel_logic.c
    hal/picofw_hw.c
    hal/picofw_led_logic.c
    hal/picofw_led.c
    hal/picofw_multicore.c
    hal/picofw_usb_dual_cdc.c
    hal/picofw_watchdog.c
    hal/picofw_wireless_codec.c
    gdb/gdb_rsp_codec.c
    gdb/gdb_rsp_memory.c
    gdb/gdb_rsp_cpu.c
    generated/picofw_ssot_tables.c
    flash/picofw_blackbox_codec.c
    flash/picofw_bus_policy.c
    flash/picofw_bus_tx_guard.c
    flash/picofw_flash_geo.c
    flash/picofw_fast_stream_codec.c
    flash/picofw_normap_codec.c
    flash/picofw_fast_stub_io.c
    flash/picofw_fast_stream.c
    flash/picofw_ftl_inspect.c
    flash/picofw_job_codec.c
    flash/picofw_job.c
    flash/picofw_payload_cli.c
    flash/picofw_payloads.c
    flash/picofw_trap_guard.c
    flash/picofw_probe_cfg_codec.c
    flash/picofw_qspi_hal.c
    flash/picofw_qspi_map.c
    flash/picofw_qspi_storage.c
    flash/picofw_sector_ops.c
    flash/picofw_target_bus.c
    flash/picofw_target_bus_panamon.c
    flash/picofw_target_bus_pio.c
    flash/picofw_flash.c
    flash/picofw_vault_codec.c
    flash/picofw_vault.c
    diag/picofw_diag_electrical.c
    diag/picofw_diag_bus.c
    diag/picofw_diag_busdiag.c
    diag/picofw_diag_pwrtrace.c
    diag/picofw_boottrace_codec.c
    diag/picofw_diag_boottrace.c
    diag/picofw_gps_cli.c
    diag/picofw_hotplug_codec.c
    diag/picofw_hotplug_map.c
    diag/picofw_i2c_codec.c
    diag/picofw_bitwise_search.c
    diag/picofw_hdd_unlock.c
    diag/picofw_math_gps.c
    diag/picofw_math_stats.c
    diag/picofw_meihfs_probe.c
    diag/picofw_microsurgery.c
    diag/picofw_mn103_asm.c
    diag/picofw_mn103_disasm.c
    diag/picofw_pieos_xray.c
    diag/picofw_surg_cli.c
    diag/picofw_surg_live.c
    diag/picofw_xray_cli.c
    diag/picofw_i2c_master.c
    diag/picofw_identify_codec.c
    diag/picofw_identify.c
    diag/picofw_mmio_snapshot_codec.c
    diag/picofw_mmio_snapshot.c
    diag/picofw_piologic_codec.c
    diag/picofw_diag_piologic.c
    diag/picofw_pwrosc_codec.c
    diag/picofw_pwrosc.c
    diag/picofw_sds_codec.c
    diag/picofw_diag_sdsniff.c
    diag/picofw_jigsweep_codec.c
    diag/picofw_diag_jigsweep.c
    diag/picofw_ir_codec.c
    diag/picofw_ir_kaseikyo.c
    diag/picofw_serv_macro_codec.c
    diag/picofw_serv_macro.c
    diag/picofw_tbus_codec.c
    diag/picofw_tbus.c
    flash/picofw_flash_cli.c
    uart/picofw_uart_autobaud.c
    uart/picofw_uart_codec.c
    uart/picofw_uart_args.c
    uart/picofw_uart_capture.c
    uart/picofw_fmu_codec.c
    uart/picofw_fmu_probe.c
    uart/picofw_fmu_batch.c
    uart/picofw_uart_interactive.c
    uart/picofw_uart_pio_engine.c
    uart/picofw_uart_pio_tx.c
    uart/picofw_pio_loop.c
    uart/picofw_uart_baud.c
    uart/picofw_uart_stream.c
    uart/picofw_uart_tx.c
    uart/picofw_pana_dumpnor.c
    uart/picofw_pana_dumpnor_verify.c
    uart/picofw_pana_jedec_stubs.c
    uart/picofw_pana_jedec_exec.c
    uart/picofw_pana_jedec_id.c
    uart/picofw_pana_rsp_codec.c
    uart/picofw_pana_rsp_guard.c
    uart/picofw_pana_rsp_parse.c
    uart/picofw_pana_rsp_tokens.c
    uart/picofw_pana_rsp_report.c
    uart/picofw_pana_rsp_exec.c
    uart/picofw_pana_rsp.c
    uart/picofw_diag_watch_codec.c
    uart/picofw_diag_watch_events.c
    uart/picofw_diag_watch.c
    uart/picofw_drv_pana_codec.c
    uart/picofw_drv_pana.c
    cli/picofw_cli_help.c
    cli/picofw_cli_sys.c
    cli/picofw_cli.c
)

# Header include paths
target_include_directories(picofw PRIVATE
    ${CMAKE_CURRENT_LIST_DIR}
)

# Zero soft-float printf bloat & hardened 4 KiB Core 0 stack + 512 B frame ceiling
target_compile_definitions(picofw PRIVATE
    PICO_PRINTF_SUPPORT_FLOAT=0
    PICO_PRINTF_SUPPORT_EXPONENTIAL=0
    PICO_STACK_SIZE=0x1000
)

target_compile_options(picofw PRIVATE
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

target_link_options(picofw PRIVATE
    -Wl,--gc-sections
)

# Assemble PIO UART program and generate C header
pico_generate_pio_header(picofw ${CMAKE_CURRENT_LIST_DIR}/picofw_uart_duplex.pio)
pico_generate_pio_header(picofw ${CMAKE_CURRENT_LIST_DIR}/picofw_logic_sampler.pio)
pico_generate_pio_header(picofw ${CMAKE_CURRENT_LIST_DIR}/picofw_sds_sampler.pio)

# Link essential Pico SDK and RP2040/RP2350 hardware drivers
target_link_libraries(picofw
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
    message(STATUS "[PICOFW] Enabling Bluetooth LE Nordic UART Service (NUS) for ${PICO_BOARD}")
    target_compile_definitions(picofw PRIVATE
        PICOFW_BLE_ENABLED=1
    )
    target_link_libraries(picofw
        pico_cyw43_arch_none
        pico_btstack_ble
    )
elseif(PICO_BOARD STREQUAL "pico_w" OR PICO_BOARD STREQUAL "pico2_w")
    message(STATUS "[PICOFW] ${PICO_BOARD}: pico_cyw43_arch_none / pico_btstack_ble targets unavailable (Pico SDK wireless submodules uninitialized); building wired USB CDC fallback.")
endif()

# Enable USB CDC serial console, disable UART0
pico_enable_stdio_usb(picofw 1)
pico_enable_stdio_uart(picofw 0)

# Generate UF2, ELF, BIN, HEX, and MAP outputs
pico_add_extra_outputs(picofw)
