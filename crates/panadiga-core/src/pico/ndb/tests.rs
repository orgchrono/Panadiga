#![deny(unsafe_code)]

use std::collections::BTreeMap;

use super::*;

#[test]
fn replays_both_live_com10_ndb_logs_with_exact_endianness_and_barriers() {
    let volatile_raw =
        include_str!("../../../../../docs/evidence/com10_live_volatile_patch_20261124.log");
    let v_summary = replay_ndb_evidence_log(volatile_raw, "com10_live_volatile_patch_20261124.log")
        .expect("replay volatile patch log");
    assert!(v_summary.bcu_fault.is_none());
    assert!(!v_summary.standby_gate_tripped);
    assert!(v_summary.dd_reads.len() >= 6);
    assert_eq!(v_summary.dd_reads[0].base_va, 0x570D_F348);
    assert_eq!(
        v_summary.dd_reads[0].bytes[2..7],
        [0xCF, 0xF2, 0xF8, 0xFE, 0xE8]
    );
    assert!(
        v_summary
            .write_echoes
            .iter()
            .any(|e| e.addr == 0x8800_002C && e.old_value == 0xC0F0 && e.new_value == 0x40F0)
    );

    let diag_raw =
        include_str!("../../../../../docs/evidence/com10_live_video_osd_diag_20261124.log");
    let d_summary = replay_ndb_evidence_log(diag_raw, "com10_live_video_osd_diag_20261124.log")
        .expect("replay video osd diag log");
    let fault = d_summary.bcu_fault.expect("expected BCU fault");
    assert_eq!(fault.bcberr_hex, "00002102");
    assert_eq!(fault.fault_address, 0xD400_4000);
    assert!(fault.task_dump_count >= 50);
    assert!(d_summary.standby_gate_tripped);
    assert!(
        d_summary
            .standby_gate_reason
            .as_deref()
            .unwrap_or("")
            .contains("STANDBY_GATE_STOP")
    );
}

struct InMemNdb {
    mem: BTreeMap<u32, u8>,
}

impl NdbTransport for InMemNdb {
    fn execute_command(&mut self, cmd: &str) -> anyhow::Result<String> {
        let p: Vec<&str> = cmd.split_whitespace().collect();
        match p.as_slice() {
            ["dd", a, l] => {
                let addr = u32::from_str_radix(a, 16)?;
                let len = l.parse::<usize>()?;
                let mut line = format!("{addr:08X}:");
                for i in 0..len.div_ceil(4) {
                    let b = addr + (i as u32) * 4;
                    let w = u32::from_le_bytes([
                        *self.mem.get(&b).unwrap_or(&0),
                        *self.mem.get(&(b + 1)).unwrap_or(&0),
                        *self.mem.get(&(b + 2)).unwrap_or(&0),
                        *self.mem.get(&(b + 3)).unwrap_or(&0),
                    ]);
                    line.push_str(&format!(" {w:08X}"));
                }
                Ok(format!("{line}\nndb> "))
            }
            ["eb", a, v] => {
                let addr = u32::from_str_radix(a, 16)?;
                let new_v = u8::from_str_radix(v, 16)?;
                let old_v = self.mem.insert(addr, new_v).unwrap_or(0);
                Ok(format!("{addr:08X}: {old_v:02X} -> {new_v:02X}\nndb> "))
            }
            _ => anyhow::bail!("unexpected cmd {cmd}"),
        }
    }
}

#[test]
fn hot_diff_and_rollback_verify_roundtrip() {
    let mut mem = BTreeMap::new();
    for (i, b) in [0xF0u8, 0x20, 0xCF, 0xF2, 0xF8, 0xFE, 0xE8, 0xF5]
        .into_iter()
        .enumerate()
    {
        mem.insert(0x570D_F348 + i as u32, b);
    }
    let mut sess = NdbSession::new(InMemNdb { mem });
    let diff = sess
        .apply_patches_with_diff(&[VolatilePatchOp::write_bytes(0x570D_F34A, vec![0x80, 0x00])])
        .expect("apply patch");
    assert_eq!(diff.changes.len(), 2);
    let rb = sess.rollback().expect("rollback");
    assert!(rb.verified_clean);
    assert_eq!(rb.restored_bytes, 2);
}
