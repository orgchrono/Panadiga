//! Shared pure declarative `PatchSpec` descriptor and atomic patch engine for firmware images.
//!
//! Eliminates duplicate validation/classification/write loops between `bgo_unbrick` and
//! `main_passthrough`, and enforces explicit length and provenance checks before any mutation.

use anyhow::{Result, bail};

use crate::nvram::calculate_hashes;

/// Action applied against a verified `PatchSpec` site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PatchApplyAction {
    ApplyReplace,
    RestoreExpect,
}

/// Declarative patch site contract with VA, base VA, byte expectations, and optional image bounds/hash.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PatchSpec<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub va: u32,
    pub base_va: u32,
    pub expect: &'a [u8],
    pub replace: &'a [u8],
    pub required_len: Option<usize>,
    pub input_sha256: Option<[u8; 32]>,
}

/// Pure site classification produced without side effects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PatchSiteClassification {
    pub va: u32,
    pub file_offset: usize,
    pub state: &'static str,
    pub actual_bytes: Vec<u8>,
}

impl PatchSpec<'_> {
    /// Computes file offset `va - base_va`, rejecting underflows cleanly.
    pub(crate) fn file_offset(&self) -> Result<usize> {
        let off = self.va.checked_sub(self.base_va).ok_or_else(|| {
            anyhow::anyhow!(
                "Invalid PatchSpec '{}': VA 0x{:08X} < base VA 0x{:08X}",
                self.name,
                self.va,
                self.base_va
            )
        })?;
        Ok(off as usize)
    }
}

fn check_buffer_contract(buf: &[u8], spec: &PatchSpec<'_>) -> Result<(usize, usize)> {
    if spec.expect.len() != spec.replace.len() {
        bail!(
            "Invalid PatchSpec '{}': expect len ({}) != replace len ({})",
            spec.name,
            spec.expect.len(),
            spec.replace.len()
        );
    }
    if let Some(req_len) = spec.required_len
        && buf.len() != req_len
    {
        bail!(
            "Firmware binary size mismatch for '{}' (expected {req_len} B, got {} B)",
            spec.name,
            buf.len()
        );
    }
    if let Some(expected_sha) = spec.input_sha256 {
        let (_, sha_hex) = calculate_hashes(buf);
        let expected_hex = hex::encode(expected_sha);
        if sha_hex != expected_hex {
            bail!(
                "Firmware SHA-256 provenance mismatch for '{}': expected {expected_hex}, got {sha_hex}",
                spec.name
            );
        }
    }
    let start = spec.file_offset()?;
    let end = start.checked_add(spec.expect.len()).ok_or_else(|| {
        anyhow::anyhow!(
            "PatchSpec '{}' range overflow at offset 0x{start:X}",
            spec.name
        )
    })?;
    if end > buf.len() {
        bail!(
            "Binary too small ({} bytes) for '{}' at 0x{start:06X}..0x{end:06X}",
            buf.len(),
            spec.name
        );
    }
    Ok((start, end))
}

/// Inspects a single `PatchSpec` site in an immutable buffer.
pub(crate) fn classify_patch_site(
    buf: &[u8],
    spec: &PatchSpec<'_>,
    patched_label: &'static str,
) -> Result<PatchSiteClassification> {
    let (start, end) = check_buffer_contract(buf, spec)?;
    let slice = &buf[start..end];
    let state = if slice == spec.expect {
        "FACTORY"
    } else if slice == spec.replace {
        patched_label
    } else {
        "UNKNOWN_OR_MISMATCH"
    };
    Ok(PatchSiteClassification {
        va: spec.va,
        file_offset: start,
        state,
        actual_bytes: slice.to_vec(),
    })
}

/// Validates EVERY requested `PatchSpec` site first, then applies all mutations atomically.
pub(crate) fn apply_patch_specs_in_place(
    buf: &mut [u8],
    ops: &[(&PatchSpec<'_>, PatchApplyAction)],
) -> Result<()> {
    let mut ranges = Vec::with_capacity(ops.len());
    for &(spec, action) in ops {
        let (start, end) = check_buffer_contract(buf, spec)?;
        let slice = &buf[start..end];
        if slice != spec.expect && slice != spec.replace {
            bail!(
                "Unexpected bytes at {} (VA 0x{:08X}, offset 0x{start:06X}): got {} (expected {} or {})",
                spec.name,
                spec.va,
                hex::encode(slice),
                hex::encode(spec.expect),
                hex::encode(spec.replace),
            );
        }
        let target = match action {
            PatchApplyAction::ApplyReplace => spec.replace,
            PatchApplyAction::RestoreExpect => spec.expect,
        };
        ranges.push((start, end, target));
    }

    for (start, end, target) in ranges {
        buf[start..end].copy_from_slice(target);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length_bad_hash_and_mismatched_bytes_without_partial_mutation() {
        const ORIG_A: [u8; 2] = [0xC8, 0x0A];
        const NEW_A: [u8; 2] = [0xCA, 0x0A];
        const ORIG_B: [u8; 2] = [0xC8, 0x15];
        const NEW_B: [u8; 2] = [0xCA, 0x15];

        let spec_a = PatchSpec {
            name: "site_a",
            description: "first site",
            va: 0x5000_0004,
            base_va: 0x5000_0000,
            expect: &ORIG_A,
            replace: &NEW_A,
            required_len: Some(16),
            input_sha256: None,
        };
        let spec_b = PatchSpec {
            name: "site_b",
            description: "second site",
            va: 0x5000_0008,
            base_va: 0x5000_0000,
            expect: &ORIG_B,
            replace: &NEW_B,
            required_len: Some(16),
            input_sha256: None,
        };

        let short = vec![0u8; 8];
        assert!(classify_patch_site(&short, &spec_a, "PATCHED").is_err());

        // Valid site_a paired with invalid site_b must leave site_a untouched.
        let mut buf = vec![0u8; 16];
        buf[4..6].copy_from_slice(&ORIG_A);
        buf[8..10].copy_from_slice(&[0xDE, 0xAD]);
        assert!(
            apply_patch_specs_in_place(
                &mut buf,
                &[
                    (&spec_a, PatchApplyAction::ApplyReplace),
                    (&spec_b, PatchApplyAction::ApplyReplace),
                ]
            )
            .is_err()
        );
        assert_eq!(&buf[4..6], &ORIG_A);

        // Provenance hash check rejects altered buffer.
        let hash_guarded = PatchSpec {
            input_sha256: Some([0x11u8; 32]),
            ..spec_a
        };
        assert!(classify_patch_site(&buf, &hash_guarded, "PATCHED").is_err());
    }
}
