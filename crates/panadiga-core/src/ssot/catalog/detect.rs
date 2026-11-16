//! NOR Flash geometry validation and ASCII signature model detection.

/// Pure check for valid Panasonic DIGA Parallel NOR Flash geometries (2 MB, 4 MB, 8 MB, 16 MB).
/// Explicitly excludes 100 MB+ HDD dumps and optical .FRM containers.
pub const fn is_valid_nor_flash_size(size_bytes: u64) -> bool {
    matches!(
        size_bytes,
        0x0020_0000 | 0x0040_0000 | 0x0080_0000 | 0x0100_0000
    )
}

/// Pure signature detector identifying the Panasonic DIGA recorder model from a binary slice or geometry.
/// Covers all Panasonic DIGA generations: E-Series (E55..E500H), ES-Series (ES10..ES35V),
/// EH-Series (EH50..EH80V), EX-Series (EX75..EX99V), and EZ-Series (EZ25..EZ49V).
pub fn detect_diga_model_from_slice(data: &[u8]) -> String {
    let known_models = [
        // Série DMR-EH (HDD + DVD Analog/Hybrid)
        "DMR-EH80V",
        "DMR-EH75V",
        "DMR-EH68",
        "DMR-EH67",
        "DMR-EH65",
        "DMR-EH60",
        "DMR-EH59",
        "DMR-EH58",
        "DMR-EH57",
        "DMR-EH56",
        "DMR-EH55",
        "DMR-EH54",
        "DMR-EH50",
        // Série DMR-ES (DVD-RAM / VHS Combo without HDD)
        "DMR-ES35V",
        "DMR-ES30V",
        "DMR-ES25",
        "DMR-ES20",
        "DMR-ES15",
        "DMR-ES10",
        // Série DMR-EX (HDD + DVD + DVB-T Digital Tuner)
        "DMR-EX99V",
        "DMR-EX98V",
        "DMR-EX95V",
        "DMR-EX89",
        "DMR-EX88",
        "DMR-EX87",
        "DMR-EX85",
        "DMR-EX79",
        "DMR-EX78",
        "DMR-EX77",
        "DMR-EX768",
        "DMR-EX75",
        // Série DMR-EZ (DVD + Digital Tuner / VHS Combo)
        "DMR-EZ49V",
        "DMR-EZ48V",
        "DMR-EZ47V",
        "DMR-EZ45V",
        "DMR-EZ37V",
        "DMR-EZ28",
        "DMR-EZ27",
        "DMR-EZ25",
        // Série DMR-E / DMR-ExxH (1ª e 2ª Geração HDD / DVD-RAM)
        "DMR-E500H",
        "DMR-E100H",
        "DMR-E95H",
        "DMR-E85H",
        "DMR-E80H",
        "DMR-E75V",
        "DMR-E65",
        "DMR-E60",
        "DMR-E55",
        "DMR-E53",
        "DMR-E50",
    ];
    for model in known_models {
        if data
            .windows(model.len())
            .any(|w| w.eq_ignore_ascii_case(model.as_bytes()))
        {
            return model.to_string();
        }
    }
    match data.len() {
        0x0020_0000 => {
            "Panasonic DIGA (2MB NOR Flash - Geração 2003/2004: DMR-E50/E55/E65 / DMR-E80H/E85H/E95H)".to_string()
        }
        0x0040_0000 => {
            "Panasonic DIGA (4MB NOR Flash - Geração 2005: DMR-ES10/ES20/ES30V / DMR-EH50/EH60 / DMR-E100H/E500H)".to_string()
        }
        0x0080_0000 => {
            "Panasonic DIGA (8MB NOR Flash - Geração 2006+: DMR-ES15/ES25/ES35V / DMR-EH55/EH56/EH65 / DMR-EZ25)".to_string()
        }
        0x0100_0000 => {
            "Panasonic DIGA (16MB NOR Flash - Geração DVB-T/HDMI: DMR-EX75/EX85/EX77/EX87 / DMR-EZ27/EZ45V/EZ48V)".to_string()
        }
        _ => "Panasonic DIGA (Auto-Detected Target)".to_string(),
    }
}
