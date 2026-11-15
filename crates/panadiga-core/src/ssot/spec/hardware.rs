//! Hardware SSOT builders: thresholds, USB VID/PIDs, P7501 pinout, break-in modes, and Flash geometry.

use crate::constants::{
    AM34_NVRAM_BASE, AM34_SRAM_LOADER_BASE, AM34_SYSTEM_MEMORY_REGIONS, DEFAULT_BAUD_RATE,
    DEFAULT_GDB_LISTEN_SOCKET,
};
use crate::pico::platforms::{
    PICO_BOOTSEL_PID, PICO_RP2040_CDC_PID, PICO_RP2040_W_CDC_PID, PICO_RP2350_CDC_PID,
    PICO_RP2350_RISCV_CDC_PID, PICO_RP2350_W_CDC_PID, RASPBERRY_PI_VID,
};

use super::dto::{
    BreakInModeSpecDto, FlashGeometrySpecDto, HardwareThresholdsDto, MemoryRegionDto,
    P7501PinMappingDto, PicoUsbIdentityDto, pin,
};

pub(super) fn build_thresholds() -> HardwareThresholdsDto {
    HardwareThresholdsDto {
        bus_freq_hz: 1_000_000,
        nominal_vdd_volts: 3.30,
        min_vdd_threshold_volts: 2.50,
        safe_operating_min_volts: 3.00,
        max_vdd_ceiling_volts: 3.45,
        default_baud_rate: DEFAULT_BAUD_RATE,
        default_gdb_socket: DEFAULT_GDB_LISTEN_SOCKET.to_string(),
        ram_loader_base_hex: format!("0x{:08X}", AM34_SRAM_LOADER_BASE),
        default_nvram_base_hex: format!("0x{:08X}", AM34_NVRAM_BASE),
    }
}

pub(super) fn build_usb_ids() -> PicoUsbIdentityDto {
    PicoUsbIdentityDto {
        raspberry_pi_vid: RASPBERRY_PI_VID,
        rp2040_cdc_pid: PICO_RP2040_CDC_PID,
        rp2040_w_cdc_pid: PICO_RP2040_W_CDC_PID,
        rp2350_cdc_pid: PICO_RP2350_CDC_PID,
        rp2350_w_cdc_pid: PICO_RP2350_W_CDC_PID,
        rp2350_riscv_cdc_pid: PICO_RP2350_RISCV_CDC_PID,
        bootsel_pid: PICO_BOOTSEL_PID,
    }
}

pub(super) fn build_memory_regions() -> Vec<MemoryRegionDto> {
    AM34_SYSTEM_MEMORY_REGIONS
        .iter()
        .map(|r| MemoryRegionDto {
            name: r.name.to_string(),
            start: format!("0x{:08X}", r.start),
            end: format!("0x{:08X}", r.end),
            size_kb: r.size_kb,
            perms: r.perms.to_string(),
        })
        .collect()
}

pub(super) fn build_pinout() -> Vec<P7501PinMappingDto> {
    vec![
        pin(
            1,
            1,
            "Pino 1 [P7501 SW_UART / XFWE]",
            "GP26 (ADC0 / Pin 31)",
            "Pino 31",
            "SW_UART & ADC0 Sense (~3.25V Hi-Z / ~2.69V PD)",
            "Target <-> Pico (Q7510/QR7503)",
            "3.3V LVTTL",
            Some(
                "LOW força serial boot via XFWE (corta Q7514 TX). Mantenha Hi-Z para TX normal e NUNCA alimente o Pico por este pino.",
            ),
        ),
        pin(
            2,
            2,
            "Pino 2 [P7501 XRESET]",
            "GP5 (Pin 7)",
            "Pino 7",
            "XRESET Open-Drain (Nunca Push-Pull)",
            "Pico Open-Drain",
            "3.3V Hi-Z / 0V",
            None,
        ),
        pin(
            3,
            3,
            "Pino 3 [P7501 UARTM2P_T / Q7514]",
            "GP4 (Pin 6)",
            "Pino 6",
            "UART TX -> AM34 RX via Q7514",
            "Pico -> Target (115200 8N1)",
            "3.3V LVTTL",
            Some(
                "Pull-up R7570 10kΩ para NSW5V. Requer SW_UART=Hi-Z para Q7514 conduzir até o SoC AM34.",
            ),
        ),
        pin(
            4,
            4,
            "Pino 4 [P7501 GND]",
            "GND (Pin 38)",
            "Pino 38",
            "Terra 0.0Ω Chassi",
            "Terra Comum",
            "0.0V (VSS)",
            Some(
                "Manual P7501 B6B-ZR-3R4 = PINO 4 (referência comum 0.0V). Confira continuidade 0.0Ω antes de ligar.",
            ),
        ),
        pin(
            5,
            5,
            "Pino 5 [P7501 NSW5V / IC1506]",
            "GP3 (Pin 5)",
            "Pino 5",
            "NSW5V Sense Hi-Z ONLY (~3.29V em repouso)",
            "Target -> Pico (Hi-Z Input Sem Pulls)",
            "~3.29V (IC1506 ~5.15V upstream)",
            Some(
                "CRÍTICO: No DMR-EH55 P7501 Pino 5 é barramento NSW5V (originado em IC1506 ~5.15V via D7510). SEMPRE Hi-Z Input sem pulls — NUNCA dirigir push-pull!",
            ),
        ),
        pin(
            6,
            6,
            "Pino 6 [P7501 UARTP2M_T / MAX TXD]",
            "GP2 (Pin 4)",
            "Pino 4",
            "UART RX <- AM34 TX 115200 8N1",
            "Target -> Pico (115200 8N1)",
            "3.3V LVTTL",
            Some("Única linha UART TX ativa observada no DMR-EH55 P7501 (R7572 10kΩ para NSW5V)."),
        ),
    ]
}

pub(super) fn build_breakin_modes() -> Vec<BreakInModeSpecDto> {
    vec![
        BreakInModeSpecDto {
            mode: 0,
            label: "Modo 0: Normal Hi-Z (SW_UART Aberto)".to_string(),
            badge: "Passivo".to_string(),
            badge_variant: "secondary".to_string(),
            description:
                "Mantém SW_UART (GP26) em Hi-Z durante pulso Open-Drain de XRESET; Q7514 permanece conduzindo UARTM2P_T -> AM34.".to_string(),
            signal_diagram: "SW_UART: Hi-Z | XRESET: 10ms Open-Drain Low".to_string(),
        },
        BreakInModeSpecDto {
            mode: 1,
            label: "Modo 1: Serial Pulse (XFWE Pré-Reset)".to_string(),
            badge: "FMU".to_string(),
            badge_variant: "secondary".to_string(),
            description:
                "Pulsa SW_UART LOW apenas durante a liberação de XRESET e libera GP26 em Hi-Z para habilitar TX bidirecional após handshake.".to_string(),
            signal_diagram: "SW_UART: Pulse Low -> Hi-Z | Q7514 Re-enabled".to_string(),
        },
        BreakInModeSpecDto {
            mode: 2,
            label: "Modo 2: JIG Hold LOW (SW_UART Retido)".to_string(),
            badge: "RECOMENDADO".to_string(),
            badge_variant: "success".to_string(),
            description:
                "Padrão JIGBOOT para captura UART do AM34: mantém SW_UART em LOW via Q7510 durante toda a janela de inicialização e retorna a Hi-Z.".to_string(),
            signal_diagram: "SW_UART: Hold Low -> Isolate Hi-Z | 115200 8N1 RX".to_string(),
        },
        BreakInModeSpecDto {
            mode: 3,
            label: "Modo 3: Hi-Z Estendido (Captura Longa)".to_string(),
            badge: "Seguro".to_string(),
            badge_variant: "warning".to_string(),
            description:
                "Executa reset Open-Drain em XRESET mantendo os pinos UART em escuta Hi-Z para inspeção completa de CDDIAG e DDR CHECK.".to_string(),
            signal_diagram: "XRESET: Open-Drain | GP2 RX + GP3 NSW5V Hi-Z".to_string(),
        },
    ]
}

pub(super) fn build_flash_geometries() -> Vec<FlashGeometrySpecDto> {
    vec![
        FlashGeometrySpecDto {
            size_mb: 2,
            size_bytes: 0x0020_0000,
            chip_name: "Spansion S29AL016J NOR Flash (2 MB / 16 Mbit)".to_string(),
            nvram_base_hex: "0x801F0000".to_string(),
            nvram_size_bytes: 0x0001_0000,
            microsector_range: "SA38..SA45".to_string(),
        },
        FlashGeometrySpecDto {
            size_mb: 4,
            size_bytes: 0x0040_0000,
            chip_name: "Spansion S29JL032H NOR Flash (4 MB / 32 Mbit)".to_string(),
            nvram_base_hex: "0x803F0000".to_string(),
            nvram_size_bytes: 0x0001_0000,
            microsector_range: "SA70..SA77".to_string(),
        },
        FlashGeometrySpecDto {
            size_mb: 8,
            size_bytes: 0x0080_0000,
            chip_name: "Spansion S29JL064H NOR Flash (8 MB / 64 Mbit)".to_string(),
            nvram_base_hex: "0x807F0000".to_string(),
            nvram_size_bytes: 0x0001_0000,
            microsector_range: "SA134..SA141".to_string(),
        },
        FlashGeometrySpecDto {
            size_mb: 16,
            size_bytes: 0x0100_0000,
            chip_name: "Spansion S29GL128P NOR Flash (16 MB / 128 Mbit)".to_string(),
            nvram_base_hex: "0x80FE0000".to_string(),
            nvram_size_bytes: 0x0002_0000,
            microsector_range: "SA254..SA269".to_string(),
        },
    ]
}
