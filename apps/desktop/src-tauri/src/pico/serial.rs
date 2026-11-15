//! Panasonic DMR-EH55 DIGA Suite - Pico USB Serial Driver

mod commands;
mod telemetry;
mod transport;

pub use commands::{
    pico_execute_cold_break_in, pico_send_raw_serial_command, pico_switch_to_gdb_mode,
};
pub use telemetry::{
    is_ram_loader_injected, pico_connect_and_test, pico_get_device_status_dto,
    set_ram_loader_injected, try_pico_get_device_status_dto,
};
pub use transport::{
    ensure_serial_port_unlocked, list_pico_serial_ports, lock_serial_io,
    read_serial_response_complete,
};
