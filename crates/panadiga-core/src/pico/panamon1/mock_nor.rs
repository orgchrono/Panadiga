#![deny(unsafe_code)]

//! Mock Spansion NOR write/erase & dynamic NOR dumper stream helpers for [`MockPanamon1Transport`].

use std::collections::VecDeque;

use super::super::rsp_codec::policy::{NorWritePolicy, is_nor_write_blocked_with_policy};
use super::mock::MockPanamon1Transport;
use super::nor_guard::flash_offset_from_va;
use super::protocol::{NOR_MAGIC_FOOTER_DONE, NOR_MAGIC_HEADER, PANAMON1_FULL_PROMPT};
use crate::constants::{
    AM34_FLASH_IPL_SIZE, AM34_FLASH_SIZE, AM34_FLASH_UNCACHED_BASE,
    SPANSION_MAIN_SECTOR_SIZE_BYTES, SPANSION_MICROSECTOR_SIZE,
};

impl MockPanamon1Transport {
    fn current_nor_policy(&self) -> NorWritePolicy {
        if self.nor_unlocked {
            NorWritePolicy::UpperNorAppUnlocked
        } else {
            NorWritePolicy::NvramOnly
        }
    }

    pub(super) fn handle_nor_write(&mut self, va: u32, val: u8) -> &'static str {
        if is_nor_write_blocked_with_policy(va, 1, self.current_nor_policy()) {
            return "ERR:E05_IPL_RO";
        }
        let next = if flash_offset_from_va(va).is_some() {
            self.read_memory_bytes(va, 1)
                .map_or(val, |cur| cur[0] & val)
        } else {
            val
        };
        let _ = self.write_memory_bytes(va, &[next]);
        "OK:WRITE"
    }

    pub(super) fn handle_nor_erase(&mut self, va: u32) -> &'static str {
        if is_nor_write_blocked_with_policy(va, 1, self.current_nor_policy()) {
            return "ERR:E05_IPL_RO";
        }
        let Some(off) = flash_offset_from_va(va) else {
            return "ERR:E05_IPL_RO";
        };
        let (sec_off, sec_len) = if !(0x0001_0000..AM34_FLASH_IPL_SIZE).contains(&off) {
            (off & !0x1FFF, SPANSION_MICROSECTOR_SIZE)
        } else {
            (off & !0xFFFF, SPANSION_MAIN_SECTOR_SIZE_BYTES)
        };
        let _ = self.write_memory_bytes(va - off + sec_off, &vec![0xFF; sec_len]);
        "OK:ERASE"
    }

    pub(super) fn build_dynamic_nor_stream(
        &self,
        start_imm_va: u32,
        end_imm_va: u32,
        block_size: u32,
    ) -> Vec<u8> {
        let read_va_or = |addr: u32, default_va: u32| -> u32 {
            self.read_memory_bytes(addr, 4)
                .and_then(|b| b.try_into().ok())
                .map_or(default_va, u32::from_le_bytes)
        };
        let start_va = read_va_or(start_imm_va, AM34_FLASH_UNCACHED_BASE);
        let end_va = read_va_or(end_imm_va, AM34_FLASH_UNCACHED_BASE + AM34_FLASH_SIZE);
        let len = end_va.saturating_sub(start_va) as usize;
        let data = self
            .read_memory_bytes(start_va, len)
            .unwrap_or_else(|| vec![0xFF; len]);

        let mut out = Vec::with_capacity(len + 128);
        out.extend_from_slice(NOR_MAGIC_HEADER);
        out.extend_from_slice(&AM34_FLASH_UNCACHED_BASE.to_le_bytes());
        out.extend_from_slice(&AM34_FLASH_SIZE.to_le_bytes());
        out.extend_from_slice(&block_size.to_le_bytes());
        for chunk in data.chunks(block_size as usize) {
            let sum32 = chunk
                .iter()
                .fold(0u32, |acc, &b| acc.wrapping_add(u32::from(b)));
            out.extend_from_slice(chunk);
            out.extend_from_slice(&sum32.to_le_bytes());
        }
        out.extend_from_slice(NOR_MAGIC_FOOTER_DONE);
        out.extend_from_slice(format!("\r\n{PANAMON1_FULL_PROMPT}\r\n").as_bytes());
        out
    }

    pub(super) fn dispatch_mock_target_cmd(&mut self, cmd: &str) -> Vec<u8> {
        let t = cmd.trim();
        let prompt_reply = |body: &str| -> Vec<u8> {
            if body.is_empty() {
                format!("{t}\r\n{PANAMON1_FULL_PROMPT}\r\n").into_bytes()
            } else {
                format!("{t}\r\n{body}\r\n{PANAMON1_FULL_PROMPT}\r\n").into_bytes()
            }
        };
        if t == "?" {
            return prompt_reply("");
        }
        if let Some(rest) = t.strip_prefix("R ")
            && let Some((a, l)) = rest.split_once(',')
            && let (Ok(addr), Ok(len)) = (
                u32::from_str_radix(a.trim(), 16),
                usize::from_str_radix(l.trim(), 16),
            )
        {
            let data = self
                .read_memory_bytes(addr, len)
                .unwrap_or_else(|| vec![0xFF; len]);
            return prompt_reply(&hex::encode_upper(&data));
        }
        if let Some(rest) = t.strip_prefix("W ") {
            if self.simulate_first_write_syntax_err {
                self.simulate_first_write_syntax_err = false;
                return prompt_reply("?");
            }
            if let Some((a, b)) = rest.split_once(',')
                && let (Ok(addr), Ok(val)) = (
                    u32::from_str_radix(a.trim(), 16),
                    u8::from_str_radix(b.trim(), 16),
                )
            {
                return prompt_reply(self.handle_nor_write(addr, val));
            }
        }
        if let Some(rest) = t.strip_prefix("E ")
            && let Ok(addr) = u32::from_str_radix(rest.trim(), 16)
        {
            return prompt_reply(self.handle_nor_erase(addr));
        }
        if t == "U CONFIRM_UPPER_NOR" {
            self.nor_unlocked = true;
            return prompt_reply("OK:NOR_UNLOCK");
        }
        if t == "U LOCK" {
            self.nor_unlocked = false;
            return prompt_reply("OK:NOR_LOCK");
        }
        if let Some(rest) = t.strip_prefix("J ")
            && let Ok(va) = u32::from_str_radix(rest.trim(), 16)
        {
            if let Some(effects) = self.jump_mem_effects.get(&va).cloned() {
                for (addr, bytes) in effects {
                    let _ = self.write_memory_bytes(addr, &bytes);
                }
            }
            if let Some(raw) = self
                .jump_raw_replies
                .get_mut(&va)
                .and_then(VecDeque::pop_front)
            {
                return raw;
            }
            if let Some((entry_va, s_va, e_va, bs)) = self.dumper_patch_vas
                && va == entry_va
            {
                return self.build_dynamic_nor_stream(s_va, e_va, bs);
            }
            return prompt_reply("OK:DONE");
        }
        prompt_reply("?")
    }
}
