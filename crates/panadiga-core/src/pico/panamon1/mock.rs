#![deny(unsafe_code)]

//! Deterministic offline [`Panamon1Transport`] mock for unit and integration tests.

use std::collections::{BTreeMap, VecDeque};
use std::time::Duration;

use anyhow::Result;

use super::protocol::{PANAMON1_FULL_PROMPT, Panamon1Transport};

#[derive(Debug, Clone)]
struct MockMemSeg {
    base: u32,
    data: Vec<u8>,
}

/// Deterministic offline mock implementing [`Panamon1Transport`] for cross-crate unit tests.
#[derive(Debug, Clone)]
pub struct MockPanamon1Transport {
    pub port_name: String,
    pub tx_cli_log: Vec<String>,
    pub tx_target_log: Vec<String>,
    pub recorded_sleeps: Vec<Duration>,
    pub bridge_active: bool,
    pub nor_unlocked: bool,
    pub status_reply: String,
    pub pincheck_reply: String,
    pub boottrace_reply: String,
    pub bridge_ctrl_edges: u32,
    pub bridge_ferr: u32,
    pub bridge_overflow: bool,
    pub simulate_first_write_syntax_err: bool,
    pub auto_bridge_ping_ready: bool,
    pub boottrace_enters_panamon1: bool,
    segments: Vec<MockMemSeg>,
    pub(super) jump_raw_replies: BTreeMap<u32, VecDeque<Vec<u8>>>,
    pub(super) jump_mem_effects: BTreeMap<u32, Vec<(u32, Vec<u8>)>>,
    pub(super) dumper_patch_vas: Option<(u32, u32, u32, u32)>,
}

impl Default for MockPanamon1Transport {
    fn default() -> Self {
        Self::new()
    }
}

impl MockPanamon1Transport {
    /// Creates a healthy mock Pico/AM34 bench session (`VDD: 3.24V (OK)`, `PINCHECK: OK`).
    pub fn new() -> Self {
        Self {
            port_name: "MOCK_PICO".to_string(),
            tx_cli_log: Vec::new(),
            tx_target_log: Vec::new(),
            recorded_sleeps: Vec::new(),
            bridge_active: false,
            nor_unlocked: false,
            status_reply: "[STANDBY] RXD=1 NSW5V=1 VDD=3.24V fault_latch=0 dropouts=0\r\nVDD: 3.24V (OK)\r\n@P7501_RES cmd=STATUS status=OK\r\nPANADIGA> ".to_string(),
            pincheck_reply: "PINCHECK: OK\r\nPANADIGA> ".to_string(),
            boottrace_reply: format!("BOOTTRACE: OK\r\n{PANAMON1_FULL_PROMPT}\r\n"),
            bridge_ctrl_edges: 0,
            bridge_ferr: 0,
            bridge_overflow: false,
            simulate_first_write_syntax_err: false,
            auto_bridge_ping_ready: true,
            boottrace_enters_panamon1: true,
            segments: Vec::new(),
            jump_raw_replies: BTreeMap::new(),
            jump_mem_effects: BTreeMap::new(),
            dumper_patch_vas: None,
        }
    }

    pub fn map_memory(&mut self, base: u32, data: Vec<u8>) {
        if let Some(seg) = self
            .segments
            .iter_mut()
            .find(|s| s.base == base && s.data.len() == data.len())
        {
            seg.data = data;
        } else {
            self.segments.push(MockMemSeg { base, data });
        }
    }

    fn locate_seg_idx(&self, addr: u32, len: usize) -> Option<(usize, usize)> {
        let start = u64::from(addr);
        let end = start + len as u64;
        self.segments.iter().enumerate().find_map(|(i, s)| {
            let lo = u64::from(s.base);
            (start >= lo && end <= lo + s.data.len() as u64).then(|| (i, (start - lo) as usize))
        })
    }

    pub fn read_memory_bytes(&self, addr: u32, len: usize) -> Option<Vec<u8>> {
        self.locate_seg_idx(addr, len)
            .map(|(i, off)| self.segments[i].data[off..off + len].to_vec())
    }

    pub fn write_memory_bytes(&mut self, addr: u32, bytes: &[u8]) -> bool {
        let Some((i, off)) = self.locate_seg_idx(addr, bytes.len()) else {
            return false;
        };
        self.segments[i].data[off..off + bytes.len()].copy_from_slice(bytes);
        true
    }

    pub fn set_jump_raw_reply(&mut self, va: u32, raw_reply: Vec<u8>) {
        self.jump_raw_replies
            .entry(va)
            .or_default()
            .push_back(raw_reply);
    }

    pub fn set_jump_mem_effect(&mut self, va: u32, writes: Vec<(u32, Vec<u8>)>) {
        self.jump_mem_effects.insert(va, writes);
    }

    pub fn enable_dynamic_nor_dumper(
        &mut self,
        entry_va: u32,
        start_imm_va: u32,
        end_imm_va: u32,
        block_size: u32,
    ) {
        self.dumper_patch_vas = Some((entry_va, start_imm_va, end_imm_va, block_size));
    }
}

impl Panamon1Transport for MockPanamon1Transport {
    fn port_label(&self) -> &str {
        &self.port_name
    }

    fn pico_cli_exchange(
        &mut self,
        cmd: &str,
        _stop_tokens: &[&str],
        _timeout: Duration,
    ) -> Result<String> {
        self.tx_cli_log.push(cmd.to_string());
        let head = cmd
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase();
        Ok(match head.as_str() {
            "PING" => "PONG\r\nPANADIGA> ".to_string(),
            "STATUS" => self.status_reply.clone(),
            "PINCHECK" => self.pincheck_reply.clone(),
            "ISOLATE" => {
                self.bridge_active = false;
                "ISOLATE: OK\r\n@P7501_RES cmd=ISOLATE status=OK\r\nPANADIGA> ".to_string()
            }
            "FAULTACK" => {
                "FAULTACK: OK\r\n@P7501_RES cmd=FAULTACK status=OK\r\nPANADIGA> ".to_string()
            }
            "BRIDGE" => {
                self.bridge_active = self.auto_bridge_ping_ready;
                "[BRIDGE] USB CDC <-> P7501 SC0 115200 PASSIVE active\r\n".to_string()
            }
            "BOOTTRACE" => {
                if self.boottrace_enters_panamon1 {
                    self.auto_bridge_ping_ready = true;
                }
                self.boottrace_reply.clone()
            }
            _ => "PANADIGA> ".to_string(),
        })
    }

    fn escape_bridge(&mut self, _timeout: Duration) -> Result<String> {
        self.tx_cli_log.push("~~~".to_string());
        let was_active = std::mem::replace(&mut self.bridge_active, false);
        if was_active {
            let ovf = if self.bridge_overflow { "YES" } else { "NO" };
            Ok(format!(
                "BRIDGE: OK (rx=128 tx=64 ferr={} ctrl_edges={} overflow={})\r\nPANADIGA>",
                self.bridge_ferr, self.bridge_ctrl_edges, ovf
            ))
        } else {
            Ok("PANADIGA>".to_string())
        }
    }

    fn target_exchange_bytes(
        &mut self,
        cmd: &str,
        _pace: Duration,
        _req: &[&[u8]],
        _to: Duration,
    ) -> Result<Vec<u8>> {
        self.tx_target_log.push(cmd.to_string());
        Ok(if self.bridge_active {
            self.dispatch_mock_target_cmd(cmd)
        } else {
            Vec::new()
        })
    }

    fn sleep(&mut self, duration: Duration) {
        self.recorded_sleeps.push(duration);
    }
}
