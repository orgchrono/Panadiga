//! Granular Pico P7501 Async UART / FMU firmware command catalog (`cli/p7501_cli.c`).

use super::commands_a::build_pico_commands_part_a;
use super::commands_b::build_pico_commands_part_b;
use super::dto::PicoCommandSpecDto;

pub(super) fn build_pico_commands() -> Vec<PicoCommandSpecDto> {
    let mut commands = build_pico_commands_part_a();
    commands.extend(build_pico_commands_part_b());
    commands
}
