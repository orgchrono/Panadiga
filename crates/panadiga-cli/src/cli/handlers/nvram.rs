//! CLI handler for Spansion NOR Flash NVRAM sanitization and guarded copy-on-write patching.

use super::super::args::{NvramActions, NvramArgs};
use anyhow::Result;
use panadiga_core::crypto::parse_u32_radix;
use panadiga_core::nvram::{clean_flash_nvram, patch_flash_nvram};
use panadiga_core::re::matshita_emu::parse_hex_payload;

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
            let rel_off = parse_u32_radix(&offset)? as usize;
            let bytes = parse_hex_payload(&hex)?;
            patch_flash_nvram(&input, output.as_deref(), rel_off, &bytes)?;
        }
    }
    Ok(())
}
