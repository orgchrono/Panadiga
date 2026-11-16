//! CLI handler for Spansion NOR Flash NVRAM sanitization and guarded copy-on-write patching.

use super::super::args::{NvramActions, NvramArgs};
use anyhow::Result;
use panadiga_core::crypto::parse_u32_radix;
use panadiga_core::nvram::{clean_flash_nvram, patch_flash_nvram, patch_flash_upper_nor_file};
use panadiga_core::re::matshita_emu::parse_hex_payload;

fn parse_offset_and_hex(offset: &str, hex: &str) -> Result<(usize, Vec<u8>)> {
    Ok((parse_u32_radix(offset)? as usize, parse_hex_payload(hex)?))
}

pub fn handle_nvram(args: NvramArgs) -> Result<()> {
    match args.action {
        NvramActions::Clean { input, output } => {
            clean_flash_nvram(&input, output.as_deref())?;
        }
        NvramActions::Patch {
            input,
            output,
            offset,
            hex,
        } => {
            let (rel_off, bytes) = parse_offset_and_hex(&offset, &hex)?;
            patch_flash_nvram(&input, output.as_deref(), rel_off, &bytes)?;
        }
        NvramActions::PatchUpperNor {
            input,
            output,
            offset,
            hex,
            backup,
            allow_critical_ipl,
        } => {
            let (flash_off, bytes) = parse_offset_and_hex(&offset, &hex)?;
            patch_flash_upper_nor_file(
                &input,
                &output,
                backup.as_deref(),
                flash_off,
                &bytes,
                allow_critical_ipl,
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::cli::{Cli, Commands, dispatch};
    use clap::Parser;
    use panadiga_core::constants::{SPANSION_FLASH_SIZE_BYTES, SPANSION_MAIN_SECTOR_SIZE_BYTES};

    #[test]
    fn cli_patch_upper_nor_parses_and_executes_cow() {
        let dir = std::env::temp_dir().join("panadiga_cli_patch_upper_nor_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let in_path = dir.join("nor_8mb.bin");
        let out_path = dir.join("nor_8mb_out.bin");
        let bak_path = dir.join("nor_8mb_bak.bin");
        std::fs::write(&in_path, vec![0x44u8; SPANSION_FLASH_SIZE_BYTES]).unwrap();

        let cli = Cli::try_parse_from([
            "panadiga",
            "nvram",
            "patch-upper-nor",
            "--input",
            in_path.to_str().unwrap(),
            "--output",
            out_path.to_str().unwrap(),
            "--offset",
            "0x00400010",
            "--hex",
            "DE AD BE EF",
            "--backup",
            bak_path.to_str().unwrap(),
        ])
        .unwrap();
        let Some(Commands::Nvram(nvram_args)) = cli.command else {
            panic!("Expected Nvram command");
        };
        dispatch(Commands::Nvram(nvram_args)).unwrap();

        let patched = std::fs::read(&out_path).unwrap();
        assert_eq!(
            &patched[0x0040_0010..0x0040_0014],
            &[0xDE, 0xAD, 0xBE, 0xEF]
        );
        assert_eq!(
            std::fs::read(&bak_path).unwrap().len(),
            SPANSION_MAIN_SECTOR_SIZE_BYTES
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
