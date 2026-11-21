#![deny(unsafe_code)]

//! Closed-loop verification harness running MN103S routines inside `Mn103Vm` with invariant guards.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::constants::AM34_FLASH_UNCACHED_BASE;
use crate::pico::rsp_codec::policy::NorWritePolicy;
use crate::re::{Mn103Fault, Mn103Vm};

const STOP_SENTINEL_VA: u32 = 0x50FF_FFFC;
const STACK_BASE_VA: u32 = 0x5010_0000;
const STACK_SIZE_BYTES: usize = 0x4000;
const STACK_INITIAL_SP: u32 = STACK_BASE_VA + 0x3F00;
const IPL_GUARD_BYTES: usize = 0x4_0000;

/// Additional memory segment mapped into `Mn103Vm` for a closed-loop verification run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifySegment {
    pub base_va: u32,
    pub data: Vec<u8>,
    pub writable: bool,
}

/// Execution configuration and post-run invariants for [`run_verify_loop`].
#[derive(Debug, Clone)]
pub struct VerifyExpectation {
    pub stop_pc: Option<u32>,
    pub max_steps: u64,
    pub expected_d0: Option<u32>,
    pub expected_uart_contains: Vec<String>,
    pub initial_d: [u32; 4],
    pub initial_a: [u32; 4],
    pub nor_write_policy: NorWritePolicy,
    pub uart_rx: Vec<u8>,
}

impl Default for VerifyExpectation {
    fn default() -> Self {
        Self {
            stop_pc: None,
            max_steps: 20_000,
            expected_d0: None,
            expected_uart_contains: Vec::new(),
            initial_d: [0; 4],
            initial_a: [0; 4],
            nor_write_policy: NorWritePolicy::NvramOnly,
            uart_rx: Vec::new(),
        }
    }
}

/// Result of executing a routine in `Mn103Vm` under IPL and lethal-MMIO safety verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyLoopReport {
    pub passed: bool,
    pub executed_steps: u64,
    pub final_pc: u32,
    pub final_d0: u32,
    pub uart_tx: String,
    pub ipl_untouched: bool,
    pub fault: Option<String>,
    pub notes: Vec<String>,
}

fn hash_vm_ipl(vm: &mut Mn103Vm, len: usize) -> Result<Vec<u8>> {
    let mut raw = Vec::with_capacity(len);
    for off in 0..len {
        raw.push(
            vm.read_u8(AM34_FLASH_UNCACHED_BASE.wrapping_add(off as u32))
                .map_err(|e| anyhow::anyhow!("IPL readback fault at +0x{off:X}: {e:?}"))?,
        );
    }
    Ok(Sha256::digest(&raw).to_vec())
}

/// Runs `entry_pc` inside `Mn103Vm` and validates exit state, UART TX, IPL SHA-256, and lethal-MMIO safety.
pub fn run_verify_loop(
    nor_bytes: &[u8],
    segments: &[VerifySegment],
    entry_pc: u32,
    exp: &VerifyExpectation,
) -> Result<VerifyLoopReport> {
    let mut vm = Mn103Vm::new();
    let ipl_len = nor_bytes.len().min(IPL_GUARD_BYTES);
    let pre_ipl_sha = Sha256::digest(&nor_bytes[..ipl_len]).to_vec();

    if !nor_bytes.is_empty() {
        vm.map_segment(AM34_FLASH_UNCACHED_BASE, nor_bytes.to_vec(), true);
    }
    vm.map_segment(STACK_BASE_VA, vec![0u8; STACK_SIZE_BYTES], true);
    vm.map_segment(STOP_SENTINEL_VA, vec![0xCB; 4], false);
    for seg in segments {
        vm.map_segment(seg.base_va, seg.data.clone(), seg.writable);
    }

    let target_stop = exp.stop_pc.unwrap_or(STOP_SENTINEL_VA);
    vm.nor_write_policy = exp.nor_write_policy;
    vm.d = exp.initial_d;
    vm.a = exp.initial_a;
    vm.sp = STACK_INITIAL_SP;
    vm.pc = entry_pc;
    let _ = vm.write_u32(STACK_INITIAL_SP, target_stop);
    vm.enable_sc0_uart(&exp.uart_rx);

    let run_res = vm.run_until(target_stop, exp.max_steps);
    let uart_tx = vm.take_uart_tx_utf8();
    let post_ipl_sha = if ipl_len > 0 {
        hash_vm_ipl(&mut vm, ipl_len)?
    } else {
        pre_ipl_sha.clone()
    };
    let ipl_untouched = pre_ipl_sha == post_ipl_sha;

    let mut notes = Vec::new();
    let fault = match run_res {
        Ok(()) => None,
        Err(Mn103Fault::LethalRegisterRead(va)) => Some(format!(
            "Forbidden lethal MMIO access at 0x{va:08X} (0xC000100C guard)"
        )),
        Err(Mn103Fault::IplWriteProtected(va)) => {
            Some(format!("IPL write protection triggered at 0x{va:08X}"))
        }
        Err(err) => Some(format!("VM execution fault: {err:?}")),
    };

    if let Some(d0) = exp.expected_d0
        && vm.d[0] != d0
    {
        notes.push(format!("Expected D0=0x{d0:08X}, got 0x{:08X}", vm.d[0]));
    }
    for needle in &exp.expected_uart_contains {
        if !uart_tx.contains(needle) {
            notes.push(format!("UART output missing expected text '{needle}'"));
        }
    }
    if !ipl_untouched {
        notes.push("IPL checksum changed during execution!".to_string());
    }

    let passed = fault.is_none() && ipl_untouched && notes.is_empty();
    Ok(VerifyLoopReport {
        passed,
        executed_steps: vm.steps,
        final_pc: vm.pc,
        final_d0: vm.d[0],
        uart_tx,
        ipl_untouched,
        fault,
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::re::mn10300_emit::{
        MONITOR_PUTC_BYTES, emit_add_sp_imm8, emit_call16_placeholder, emit_mov_imm32, emit_rets,
        patch_s16_rel,
    };

    #[test]
    fn test_verify_loop_passes_valid_uart_routine_and_protects_ipl_and_lethal_mmio() {
        let code_va = 0x5040_0000u32;
        let mut stub = Vec::new();
        emit_add_sp_imm8(&mut stub, -4);
        let _ = emit_mov_imm32(&mut stub, 0xCF, b'O' as u32); // MOV 'O', D3
        let c1 = emit_call16_placeholder(&mut stub);
        let _ = emit_mov_imm32(&mut stub, 0xCF, b'K' as u32); // MOV 'K', D3
        let c2 = emit_call16_placeholder(&mut stub);
        let _ = emit_mov_imm32(&mut stub, 0xCC, 0x1234); // MOV 0x1234, D0
        emit_add_sp_imm8(&mut stub, 4);
        emit_rets(&mut stub);
        let putc_off = stub.len();
        stub.extend_from_slice(&MONITOR_PUTC_BYTES);
        patch_s16_rel(&mut stub, c1, putc_off, "CALL").unwrap();
        patch_s16_rel(&mut stub, c2, putc_off, "CALL").unwrap();

        let nor = vec![0xFFu8; 0x8_0000];
        let rep = run_verify_loop(
            &nor,
            &[VerifySegment {
                base_va: code_va,
                data: stub,
                writable: false,
            }],
            code_va,
            &VerifyExpectation {
                expected_d0: Some(0x1234),
                expected_uart_contains: vec!["OK".into()],
                ..Default::default()
            },
        )
        .unwrap();
        assert!(rep.passed, "notes={:?}, fault={:?}", rep.notes, rep.fault);
        assert_eq!(rep.uart_tx, "OK");
        assert!(rep.ipl_untouched);

        // Lethal register read 0xC000100C must fail cleanly: MOV (0xC000100C), D0
        let lethal = vec![0xFC, 0xAC, 0x0C, 0x10, 0x00, 0xC0, 0xF0, 0xFC];
        let rep_lethal = run_verify_loop(
            &nor,
            &[VerifySegment {
                base_va: code_va,
                data: lethal,
                writable: false,
            }],
            code_va,
            &VerifyExpectation::default(),
        )
        .unwrap();
        assert!(!rep_lethal.passed);
        assert!(
            rep_lethal
                .fault
                .as_deref()
                .unwrap_or("")
                .contains("0xC000100C")
        );

        // Write to IPL 0x84000010 must trigger IplWriteProtected and keep IPL untouched
        let mut ipl_atk = Vec::new();
        let _ = emit_mov_imm32(&mut ipl_atk, 0xCC, 0xDEAD_BEEF); // D0
        let _ = emit_mov_imm32(&mut ipl_atk, 0x81, AM34_FLASH_UNCACHED_BASE + 0x10); // MOV D0, (abs32)
        emit_rets(&mut ipl_atk);
        let rep_ipl = run_verify_loop(
            &nor,
            &[VerifySegment {
                base_va: code_va,
                data: ipl_atk,
                writable: false,
            }],
            code_va,
            &VerifyExpectation::default(),
        )
        .unwrap();
        assert!(!rep_ipl.passed);
        assert!(rep_ipl.ipl_untouched);
    }
}
