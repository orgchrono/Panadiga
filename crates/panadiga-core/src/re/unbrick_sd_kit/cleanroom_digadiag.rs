#![deny(unsafe_code)]

//! Clean-room bare-metal `DIGADIAG.FRM` generator (`PANAMON1>` + `IC7501` TBUS WDT keepalive).
//!
//! Produces a self-contained Stage-1.5 SDDIAG payload with ZERO Panasonic proprietary binary blobs,
//! strictly verified by the `Mn103Vm` pre-write gate before returning.

use anyhow::{Result, bail};

use crate::constants::{PANA_DIGADIAG_FRM_SIZE, PANA_DIGADIAG_RAW_VER061_SIZE};
use crate::crypto::build_digadiag_frm;
use crate::nvram::calculate_hashes;
use crate::re::bgo_unbrick::PROG061_BASE_VA;
use crate::re::mn10300_emit::{emit_add_sp_imm8, emit_call32_abs, emit_mov_imm32, emit_rets};
use crate::re::prewrite_gate::{PreWriteGateReport, PreWriteTarget, require_prewrite_gate};
use crate::re::prog061_uart_monitor::{
    OFF_UART_MONITOR_SLACK, PROG061_UART_MONITOR_VA, build_prog061_uart_monitor_payload,
};
use crate::re::tbc_appliance::emit_m0_tbus_wdt_keepalive_stub;

use super::compose::SdLoaderProfile;

/// Clean-room identifier stamped at `+0x0010` inside synthesized `DIGADIAG.FRM`.
pub const CLEANROOM_DIGADIAG_MAGIC: &[u8] = b"PANADIGA_CLEANROOM_SDDIAG_V1\0";

const CLEANROOM_ENTRY_OFFSET: usize = 0x0040;
const CLEANROOM_WDT_STUB_OFFSET: usize = 0x0100;

/// Verified clean-room `DIGADIAG.FRM` artifact paired with its mandatory `Mn103Vm` report.
#[derive(Debug, Clone)]
pub struct CleanroomDigadiagArtifact {
    pub bytes: Vec<u8>,
    pub sha256: String,
    pub loader_profile: SdLoaderProfile,
    pub emu_gate: PreWriteGateReport,
}

fn assemble_cleanroom_ver061_raw() -> Result<Vec<u8>> {
    let mut img = vec![0xFFu8; PANA_DIGADIAG_RAW_VER061_SIZE];

    // Entry at 0x5040_0000: JMP32 +0x0000_0040 -> 0x5040_0040
    img[0] = 0xDC;
    img[1..5].copy_from_slice(&(CLEANROOM_ENTRY_OFFSET as u32).to_le_bytes());

    img[0x10..0x10 + CLEANROOM_DIGADIAG_MAGIC.len()].copy_from_slice(CLEANROOM_DIGADIAG_MAGIC);

    // WDT keepalive leaf stub at +0x0100 (VA 0x5040_0100), ending with RETS
    let wdt_stub = emit_m0_tbus_wdt_keepalive_stub();
    if CLEANROOM_WDT_STUB_OFFSET + wdt_stub.len() > 0x0400 {
        bail!("WDT stub overflowed reserved header region");
    }
    img[CLEANROOM_WDT_STUB_OFFSET..CLEANROOM_WDT_STUB_OFFSET + wdt_stub.len()]
        .copy_from_slice(&wdt_stub);

    // Interactive PANAMON1> UART0 monitor placed at verified slack offset 0x039494 (VA 0x5043_9494)
    let panamon1 = build_prog061_uart_monitor_payload(PROG061_UART_MONITOR_VA)?;
    if OFF_UART_MONITOR_SLACK + panamon1.len() > img.len() {
        bail!("PANAMON1 payload exceeds 256 KiB DIGADIAG slack");
    }
    img[OFF_UART_MONITOR_SLACK..OFF_UART_MONITOR_SLACK + panamon1.len()].copy_from_slice(&panamon1);

    // Entry stub at 0x0040: CALLS WDT keepalive -> allocate 4B stack slot -> CALL32 PANAMON1 -> RETS
    let mut entry = Vec::with_capacity(64);
    let _ = emit_mov_imm32(
        &mut entry,
        0xDC,
        PROG061_BASE_VA + CLEANROOM_WDT_STUB_OFFSET as u32,
    );
    entry.extend_from_slice(&[0xF0, 0xF0]); // CALLS (A0)
    emit_add_sp_imm8(&mut entry, -4);
    emit_call32_abs(
        &mut entry,
        PROG061_BASE_VA + CLEANROOM_ENTRY_OFFSET as u32,
        PROG061_UART_MONITOR_VA,
    );
    emit_add_sp_imm8(&mut entry, 4);
    emit_rets(&mut entry);

    if CLEANROOM_ENTRY_OFFSET + entry.len() > CLEANROOM_WDT_STUB_OFFSET {
        bail!("Clean-room entry routine exceeds pre-WDT slot");
    }
    img[CLEANROOM_ENTRY_OFFSET..CLEANROOM_ENTRY_OFFSET + entry.len()].copy_from_slice(&entry);

    Ok(img)
}

/// Builds a 100% clean-room bare-metal `DIGADIAG.FRM` (`ver0.61` 256 KiB raw or `ver0.63` 128 KiB `EXTRHEAD`),
/// immediately enforcing the mandatory `Mn103Vm` pre-write gate before returning.
pub fn build_cleanroom_digadiag(
    loader_profile: SdLoaderProfile,
) -> Result<CleanroomDigadiagArtifact> {
    let raw = assemble_cleanroom_ver061_raw()?;
    let bytes = match loader_profile {
        SdLoaderProfile::Ver061RawSd48 => raw,
        SdLoaderProfile::Ver063ExtrheadSd64 => {
            build_digadiag_frm(
                &raw,
                PROG061_BASE_VA,
                "FVU ",
                "PANADIG",
                4,
                PANA_DIGADIAG_FRM_SIZE,
            )?
            .0
        }
    };
    let (_, sha256) = calculate_hashes(&bytes);
    let emu_gate = require_prewrite_gate(PreWriteTarget::CleanroomDigadiag {
        bytes: &bytes,
        loader_profile,
    })?;
    Ok(CleanroomDigadiagArtifact {
        bytes,
        sha256,
        loader_profile,
        emu_gate,
    })
}
