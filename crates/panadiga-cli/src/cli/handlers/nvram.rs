#![deny(unsafe_code)]

//! CLI handler for Spansion NOR Flash NVRAM sanitization and guarded copy-on-write patching.

use anyhow::{Result, bail};
use panadiga_core::crypto::parse_u32_radix;
use panadiga_core::nvram::{
    NvramPolicyOptions, clean_flash_nvram_with_policy, patch_flash_nvram_with_policy,
    patch_flash_upper_nor_file_with_policy,
};
use panadiga_core::re::matshita_emu::parse_hex_payload;
use panadiga_core::ssot::{LAB_OVERRIDE_DISCLAIMER_ACK, LabOverride};

use super::super::args::{LabOverrideCliArgs, NvramActions, NvramArgs};

fn parse_offset_and_hex(offset: &str, hex: &str) -> Result<(usize, Vec<u8>)> {
    Ok((parse_u32_radix(offset)? as usize, parse_hex_payload(hex)?))
}

/// Converts shared CLI override flags into a validated `Option<LabOverride>`.
pub(super) fn build_lab_override(flags: &LabOverrideCliArgs) -> Result<Option<LabOverride>> {
    if !flags.lab_override_unverified {
        if flags.confirm_disclaimer.is_some() || flags.override_reason.is_some() {
            bail!(
                "Passed --confirm-disclaimer or --override-reason without --lab-override-unverified"
            );
        }
        return Ok(None);
    }
    let ack = flags.confirm_disclaimer.as_deref().unwrap_or("").trim();
    if ack != LAB_OVERRIDE_DISCLAIMER_ACK {
        bail!(
            "--lab-override-unverified requires exact --confirm-disclaimer {}",
            LAB_OVERRIDE_DISCLAIMER_ACK
        );
    }
    let reason = flags.override_reason.as_deref().unwrap_or("").trim();
    if reason.len() < 5 {
        bail!("--lab-override-unverified requires --override-reason (>= 5 chars)");
    }
    Ok(Some(LabOverride {
        disclaimer_ack: ack.to_string(),
        operator_reason: reason.to_string(),
    }))
}

pub fn handle_nvram(args: NvramArgs) -> Result<()> {
    match args.action {
        NvramActions::Clean {
            input,
            output,
            policy,
        } => {
            let ov = build_lab_override(&policy.lab_override)?;
            let opts = NvramPolicyOptions {
                model_id: policy.model.as_deref(),
                lab_override: ov.as_ref(),
            };
            clean_flash_nvram_with_policy(&input, output.as_deref(), &opts)?;
        }
        NvramActions::Patch {
            input,
            output,
            offset,
            hex,
            policy,
        } => {
            let ov = build_lab_override(&policy.lab_override)?;
            let (rel_off, bytes) = parse_offset_and_hex(&offset, &hex)?;
            let opts = NvramPolicyOptions {
                model_id: policy.model.as_deref(),
                lab_override: ov.as_ref(),
            };
            patch_flash_nvram_with_policy(&input, output.as_deref(), rel_off, &bytes, &opts)?;
        }
        NvramActions::PatchUpperNor {
            input,
            output,
            offset,
            hex,
            backup,
            allow_critical_ipl,
            policy,
        } => {
            let ov = build_lab_override(&policy.lab_override)?;
            let (flash_off, bytes) = parse_offset_and_hex(&offset, &hex)?;
            let opts = NvramPolicyOptions {
                model_id: policy.model.as_deref(),
                lab_override: ov.as_ref(),
            };
            patch_flash_upper_nor_file_with_policy(
                &input,
                &output,
                backup.as_deref(),
                flash_off,
                &bytes,
                allow_critical_ipl,
                &opts,
            )?;
        }
        NvramActions::AuditNorSectors { image, json } => {
            let raw = std::fs::read(&image)?;
            let rep = panadiga_core::re::nor_dumper::analyze_spansion_nor_sectors(&raw)?;
            println!(
                "Spansion NOR Audit: total={} sectors populated={} blank={} sha256={} sigs={:?}",
                rep.total_sectors,
                rep.populated_sectors,
                rep.blank_sectors,
                rep.sha256,
                rep.loader_signatures
            );
            super::json_io::emit_json_output(&rep, json.as_deref())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::cli::{Cli, Commands, dispatch};
    use clap::Parser;
    use panadiga_core::constants::{SPANSION_FLASH_SIZE_BYTES, SPANSION_MAIN_SECTOR_SIZE_BYTES};
    use panadiga_core::ssot::LAB_OVERRIDE_DISCLAIMER_ACK;

    #[test]
    fn cli_patch_upper_nor_enforces_policy_and_executes_cow() {
        let dir = std::env::temp_dir().join("panadiga_cli_patch_upper_nor_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let unlabeled_path = dir.join("nor_unlabeled_8mb.bin");
        let in_path = dir.join("nor_8mb.bin");
        let out_path = dir.join("nor_8mb_out.bin");
        let bak_path = dir.join("nor_8mb_bak.bin");
        let raw_unlabeled = vec![0x44u8; SPANSION_FLASH_SIZE_BYTES];
        std::fs::write(&unlabeled_path, &raw_unlabeled).unwrap();

        // 1. Unlabeled 8 MiB dump MUST be denied without --lab-override-unverified
        let denied_cli = Cli::try_parse_from([
            "panadiga",
            "nvram",
            "patch-upper-nor",
            "--input",
            unlabeled_path.to_str().unwrap(),
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
        assert!(dispatch(denied_cli.command.unwrap()).is_err());

        // 2. Unlabeled dump WITH valid --lab-override-unverified passes offline CoW
        let ov_cli = Cli::try_parse_from([
            "panadiga",
            "nvram",
            "patch-upper-nor",
            "--input",
            unlabeled_path.to_str().unwrap(),
            "--output",
            out_path.to_str().unwrap(),
            "--offset",
            "0x00400010",
            "--hex",
            "CA FE BE EF",
            "--backup",
            bak_path.to_str().unwrap(),
            "--lab-override-unverified",
            "--confirm-disclaimer",
            LAB_OVERRIDE_DISCLAIMER_ACK,
            "--override-reason",
            "CLI lab R&D synthetic flash test",
        ])
        .unwrap();
        dispatch(ov_cli.command.unwrap()).unwrap();
        assert_eq!(
            &std::fs::read(&out_path).unwrap()[0x0040_0010..0x0040_0014],
            &[0xCA, 0xFE, 0xBE, 0xEF]
        );

        // 3. Verified DMR-EH55 dump (`Loader ver0.61`) executes Upper NOR CoW without LabOverride
        let mut eh55_img = raw_unlabeled;
        let banner = b"Loader ver0.61 [06/03/06]";
        eh55_img[0x0003_F000..0x0003_F000 + banner.len()].copy_from_slice(banner);
        std::fs::write(&in_path, eh55_img).unwrap();

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
