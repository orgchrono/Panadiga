use super::*;

#[test]
fn ssot_thresholds_match_firmware_and_platform_constants() {
    let spec = build_hardware_ssot_spec();
    assert!((spec.thresholds.min_vdd_threshold_volts - 2.50).abs() < f32::EPSILON);
    assert!((spec.thresholds.max_vdd_ceiling_volts - 3.45).abs() < f32::EPSILON);
    assert_eq!(spec.usb_ids.raspberry_pi_vid, 0x2E8A);
    assert_eq!(spec.usb_ids.rp2350_cdc_pid, 0x000F);
    assert_eq!(spec.pinout.len(), 6);
    assert_eq!(
        spec.pinout[1].signal_name,
        "XRESET Open-Drain (Nunca Push-Pull)"
    );
    assert_eq!(spec.pinout[2].signal_name, "UART TX -> AM34 RX via Q7514");
    assert_eq!(spec.pinout[3].manual_pin, 4);
    assert_eq!(spec.pinout[3].signal_name, "Terra 0.0Ω Chassi");
    assert_eq!(spec.breakin_modes.len(), 4);
    assert_eq!(spec.flash_geometries.len(), 4);
    assert!(spec.pico_commands.len() >= 35);
    assert!(spec.pico_commands.iter().any(|c| c.id == "PIOLOGIC"));
    assert!(spec.pico_commands.iter().any(|c| c.id == "DIAGWATCH"));
    assert!(
        spec.pico_commands
            .iter()
            .find(|c| c.id == "PANARSP")
            .is_some_and(|c| c.syntax.contains("XMEM:") && c.syntax.contains("DUMPNOR"))
    );
    assert!(
        spec.pico_commands
            .iter()
            .find(|c| c.id == "BRIDGE")
            .is_some_and(|c| c.description.contains("~~~"))
    );
    assert!(spec.indexed_manuals_count >= 15);
}
