use std::path::PathBuf;

use byteorder::{ByteOrder, LittleEndian};

use crate::constants::{AM34_FLASH_UNCACHED_BASE, AM34_MAIN_BASE_VA};
use crate::crypto::{ExtrheadCryptMode, unpack_extrhead};
use crate::nvram::calculate_hashes;
use crate::re::Mn103Vm;
use crate::re::ssot_patches::find_main_build_by_id;

pub(super) use super::tables::va_off;
use super::*;

mod vm_nfd_tests;
pub(crate) use vm_nfd_tests::prove_main_bgo_8mb_retarget_against_real_eh55_nor;

pub(super) const VA_BGO_TASK_5C_STACK_TOP: u32 = 0x5180_1300;
pub(super) const VA_MAIN_CALLER_STUB: u32 = 0x5080_F000;
pub(super) const VA_MAIN_STACK_TOP: u32 = 0x50FF_F800;
pub(super) const VA_MAIN_OUT_BUF: u32 = 0x50FE_0000;

#[derive(Clone, Copy)]
pub(crate) struct MainBgoVmProfile {
    build_id: &'static str,
    nfd_init: u32,
    nfd_read: u32,
    nfd_write: u32,
    bgo_init: u32,
    bgo_write_lock_va: u32,
    bgo_task_5c: u32,
    bgo_task_tid: u32,
    itron_set_flg: u32,
    itron_clr_flg: u32,
    itron_wai_flg: u32,
    itron_pol_flg: u32,
    itron_sta_tsk: u32,
    itron_dly_tsk: u32,
    itron_clr_mbf: u32,
    itron_snd_mbf: u32,
    itron_rcv_mbf: u32,
    ext_stubs: [u32; 9],
    eeprom_rd: u32,
    data_rom_start: usize,
    data_len: usize,
    extra_ram_maps: [(u32, usize); 5],
    flag_init_va: u32,
    mfr_id_va: u32,
    dev_id_va: u32,
}

pub(crate) const PROFILE_MAIN3060NP: MainBgoVmProfile = MainBgoVmProfile {
    build_id: "3060NP",
    nfd_init: 0x574A_68E9,
    nfd_read: 0x574A_6BA8,
    nfd_write: 0x574A_6E65,
    bgo_init: 0x574A_8B82,
    bgo_write_lock_va: 0x52FE_9204,
    bgo_task_5c: 0x574B_04A2,
    bgo_task_tid: 0x5C,
    itron_set_flg: 0x5772_A65F,
    itron_clr_flg: 0x5772_A726,
    itron_wai_flg: 0x5772_A76F,
    itron_pol_flg: 0x5772_A789,
    itron_sta_tsk: 0x5772_F088,
    itron_dly_tsk: 0x5772_F78F,
    itron_clr_mbf: 0x5772_A12B,
    itron_snd_mbf: 0x5772_A146,
    itron_rcv_mbf: 0x5772_A166,
    ext_stubs: [
        0x570B_97C5,
        0x574A_A7AC,
        0x5750_E2AD,
        0x5769_7915,
        0x5772_B531,
        0x5772_B626,
        0x5772_F6D7,
        0x5773_00D8,
        0x5773_018F,
    ],
    eeprom_rd: 0x5740_E2BA,
    data_rom_start: va_off(0x57C9_95D0),
    data_len: 0x3_4E2C,
    extra_ram_maps: [
        (0x5177_4000, 0x2000),
        (0x5180_0000, 0x2000),
        (0x52FE_6000, 0x7000),
        (0x5306_9000, 0x1000),
        (0x531B_8000, 0x4000),
    ],
    flag_init_va: 0x515B_34C0,
    mfr_id_va: 0x515B_34DC,
    dev_id_va: 0x515B_34DE,
};

pub(crate) const PROFILE_MAIN3410PQ: MainBgoVmProfile = MainBgoVmProfile {
    build_id: "3410PQ",
    nfd_init: 0x5749_EE33,
    nfd_read: 0x5749_F0F2,
    nfd_write: 0x5749_F3AF,
    bgo_init: 0x574A_10CC,
    bgo_write_lock_va: 0x530F_D6A4,
    bgo_task_5c: 0x574A_89EC,
    bgo_task_tid: 0x50,
    itron_set_flg: 0x5770_8D45,
    itron_clr_flg: 0x5770_8E0C,
    itron_wai_flg: 0x5770_8E55,
    itron_pol_flg: 0x5770_8E6F,
    itron_sta_tsk: 0x5770_D76E,
    itron_dly_tsk: 0x5770_DE75,
    itron_clr_mbf: 0x5770_8811,
    itron_snd_mbf: 0x5770_882C,
    itron_rcv_mbf: 0x5770_884C,
    ext_stubs: [
        0x570B_0377,
        0x574A_2CF6,
        0x5750_99D9,
        0x5768_FE5F,
        0x5770_9C17,
        0x5770_9D0C,
        0x5770_DDBD,
        0x5770_EB1A,
        0x5770_EBD1,
    ],
    eeprom_rd: 0x5740_0722,
    data_rom_start: va_off(0x57C1_5D00),
    data_len: 0x5_0B98,
    extra_ram_maps: [
        (0x5177_D000, 0x2000),
        (0x5180_0000, 0x2000),
        (0x530F_A000, 0x7000),
        (0x5318_4000, 0x1000),
        (0x534E_4000, 0x4000),
    ],
    flag_init_va: 0x515B_252C,
    mfr_id_va: 0x515B_2548,
    dev_id_va: 0x515B_254A,
};

fn build_synthetic_main3060np_bgo_fixture() -> Vec<u8> {
    let (_, build) = find_main_build_by_id("3060NP").expect("SSOT missing 3060NP");
    let s = build_main_bgo_retarget_spec_from_ssot(build).unwrap();
    let mut buf = vec![0xCBu8; s.expected_size];
    for site in &s.word_sites {
        LittleEndian::write_u32(&mut buf[site.offset..site.offset + 4], site.expect);
    }
    for site in &s.byte_sites {
        buf[site.offset] = site.expect;
    }
    for site in &s.slice_sites {
        buf[site.offset..site.offset + site.expect.len()].copy_from_slice(&site.expect);
    }
    let mut cursor = s.reloc_scan_start;
    for reloc in &s.ram_relocs {
        for _ in 0..reloc.expected_count {
            LittleEndian::write_u32(&mut buf[cursor..cursor + 4], reloc.from);
            cursor += 4;
        }
    }
    buf
}

pub(super) fn patch_ret0_stub(main_seg: &mut [u8], va: u32) {
    let off = va_off(va);
    // CLR D0; RET [], 0 -> returns 0 cleanly to CALL callers.
    main_seg[off..off + 4].copy_from_slice(&[0x00, 0xDF, 0x00, 0x00]);
}

pub(super) fn map_main_bgo_vm_with_policy(
    profile: &MainBgoVmProfile,
    main_img: &[u8],
    nor_8mb: &[u8],
    stub: Vec<u8>,
    allow_upper_nor_writes: bool,
) -> Mn103Vm {
    let mut main_seg = main_img.to_vec();
    for ext_va in profile.ext_stubs {
        patch_ret0_stub(&mut main_seg, ext_va);
    }
    let eeprom_off = va_off(profile.eeprom_rd);
    main_seg[eeprom_off..eeprom_off + 6].copy_from_slice(&[0x2C, 0x60, 0x09, 0xDF, 0x00, 0x00]);

    let mut vm = Mn103Vm::new();
    if allow_upper_nor_writes {
        vm.nor_write_policy = crate::pico::rsp_codec::policy::NorWritePolicy::UpperNorAppUnlocked;
    }
    vm.map_segment(AM34_MAIN_BASE_VA, main_seg, false);
    vm.map_segment(
        AM34_FLASH_UNCACHED_BASE,
        nor_8mb.to_vec(),
        allow_upper_nor_writes,
    );
    vm.map_segment(0xDB00_0000, vec![0u8; 0x1000], true);
    vm.map_segment(
        0x5159_8000,
        main_img[profile.data_rom_start..profile.data_rom_start + profile.data_len].to_vec(),
        true,
    );
    for &(ram_va, ram_len) in &profile.extra_ram_maps {
        vm.map_segment(ram_va, vec![0u8; ram_len], true);
    }
    vm.map_segment(VA_MAIN_OUT_BUF, vec![0u8; 0x4000], true);
    vm.map_segment(0x50FF_0000, vec![0u8; 0x1_0000], true);
    vm.map_segment(VA_MAIN_CALLER_STUB, stub, false);
    vm
}

#[derive(Clone, Default)]
struct FiberCtx {
    d: [u32; 4],
    a: [u32; 4],
    r: [u32; 8],
    sp: u32,
    pc: u32,
    mdr: u32,
    mdrq: u32,
    mcrh: u32,
    mcrl: u32,
    mcvf: u32,
    lir: u32,
    lar: u32,
    psw: u16,
    epsw: u32,
    psw_undef: u16,
}

impl FiberCtx {
    fn from_vm(vm: &Mn103Vm) -> Self {
        Self {
            d: vm.d,
            a: vm.a,
            r: vm.r,
            sp: vm.sp,
            pc: vm.pc,
            mdr: vm.mdr,
            mdrq: vm.mdrq,
            mcrh: vm.mcrh,
            mcrl: vm.mcrl,
            mcvf: vm.mcvf,
            lir: vm.lir,
            lar: vm.lar,
            psw: vm.psw,
            epsw: vm.epsw,
            psw_undef: vm.psw_undef,
        }
    }

    fn apply_to_vm(&self, vm: &mut Mn103Vm) {
        vm.d = self.d;
        vm.a = self.a;
        vm.r = self.r;
        vm.sp = self.sp;
        vm.pc = self.pc;
        vm.mdr = self.mdr;
        vm.mdrq = self.mdrq;
        vm.mcrh = self.mcrh;
        vm.mcrl = self.mcrl;
        vm.mcvf = self.mcvf;
        vm.lir = self.lir;
        vm.lar = self.lar;
        vm.psw = self.psw;
        vm.epsw = self.epsw;
        vm.psw_undef = self.psw_undef;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FiberState {
    Idle,
    Runnable,
    WaitFlg {
        flgid: u32,
        waiptn: u32,
        wfmode: u32,
        p_flgptn: u32,
    },
    WaitMbf {
        qid: u32,
        dst_ptr: u32,
    },
}

pub(super) struct ItronTwoTaskRunner {
    pub(super) vm: Mn103Vm,
    profile: MainBgoVmProfile,
    flags: std::collections::HashMap<u32, u32>,
    mbfs: std::collections::HashMap<u32, std::collections::VecDeque<[u8; 32]>>,
    active: usize,
    other: FiberCtx,
    states: [FiberState; 2],
    pub(super) nfrom_logs: Vec<String>,
}

impl ItronTwoTaskRunner {
    pub(super) fn new(vm: Mn103Vm, profile: MainBgoVmProfile) -> Self {
        Self {
            vm,
            profile,
            flags: std::collections::HashMap::new(),
            mbfs: std::collections::HashMap::new(),
            active: 0,
            other: FiberCtx::default(),
            states: [FiberState::Runnable, FiberState::Idle],
            nfrom_logs: Vec::new(),
        }
    }

    fn read_cstr(&mut self, mut va: u32) -> String {
        let mut out = Vec::new();
        for _ in 0..160 {
            match self.vm.read_u8(va) {
                Ok(0) | Err(_) => break,
                Ok(b) => out.push(b),
            }
            va = va.wrapping_add(1);
        }
        String::from_utf8_lossy(&out).trim().to_string()
    }

    fn flg_matches(cur: u32, waiptn: u32, wfmode: u32) -> bool {
        if (wfmode & 0x02) != 0 {
            (cur & waiptn) != 0
        } else {
            (cur & waiptn) == waiptn
        }
    }

    fn switch_fiber(&mut self) {
        let cur_saved = FiberCtx::from_vm(&self.vm);
        self.other.apply_to_vm(&mut self.vm);
        self.other = cur_saved;
        self.active = 1 - self.active;
    }

    fn format_err(&self, label: &str, ring: &std::collections::VecDeque<String>) -> String {
        let trace: Vec<String> = ring.iter().cloned().collect();
        format!(
            "{label} (build={}, fiber={}, pc=0x{:08X}, d0=0x{:X}, sp=0x{:08X}, flags={:?})\ntrace:\n  {}",
            self.profile.build_id,
            self.active,
            self.vm.pc,
            self.vm.d[0],
            self.vm.sp,
            self.flags,
            trace.join("\n  ")
        )
    }

    fn complete_flg_wait(
        &mut self,
        flgid: u32,
        wfmode: u32,
        p_flgptn: u32,
        cur: u32,
    ) -> Result<(), String> {
        if p_flgptn != 0 {
            self.vm
                .write_u32(p_flgptn, cur)
                .map_err(|e| format!("p_flgptn write err: {e:?}"))?;
        }
        if (wfmode & 0x01) != 0 {
            self.flags.insert(flgid, 0);
        }
        Ok(())
    }

    fn read_mbf_msg(&mut self, ptr: u32) -> Result<[u8; 32], String> {
        let mut msg = [0u8; 32];
        for (idx, slot) in msg.iter_mut().enumerate() {
            *slot = self.vm.read_u8(ptr.wrapping_add(idx as u32)).map_err(|e| {
                format!(
                    "mbf read err @ 0x{:08X}: {e:?}",
                    ptr.wrapping_add(idx as u32)
                )
            })?;
        }
        Ok(msg)
    }

    fn write_mbf_msg(&mut self, ptr: u32, msg: &[u8; 32]) -> Result<(), String> {
        for (idx, &byte) in msg.iter().enumerate() {
            self.vm
                .write_u8(ptr.wrapping_add(idx as u32), byte)
                .map_err(|e| {
                    format!(
                        "mbf write err @ 0x{:08X}: {e:?}",
                        ptr.wrapping_add(idx as u32)
                    )
                })?;
        }
        Ok(())
    }

    fn is_itron_svc(&self, pc: u32) -> bool {
        let p = &self.profile;
        pc == p.itron_set_flg
            || pc == p.itron_clr_flg
            || pc == p.itron_wai_flg
            || pc == p.itron_pol_flg
            || pc == p.itron_sta_tsk
            || pc == p.itron_dly_tsk
            || pc == p.itron_clr_mbf
            || pc == p.itron_snd_mbf
            || pc == p.itron_rcv_mbf
    }

    fn handle_itron_svc(&mut self) -> Result<Option<&'static str>, String> {
        let pc = self.vm.pc;
        let p = self.profile;
        let ret_pc = self
            .vm
            .read_u32(self.vm.sp)
            .map_err(|e| format!("ITRON stack ret read failed @ 0x{pc:08X}: {e:?}"))?;
        if pc == p.itron_sta_tsk {
            let tid = self.vm.d[0];
            self.vm.d[0] = 0;
            self.vm.pc = ret_pc;
            if tid == p.bgo_task_tid && self.states[1] == FiberState::Idle {
                self.other = FiberCtx {
                    pc: p.bgo_task_5c,
                    sp: VA_BGO_TASK_5C_STACK_TOP,
                    ..Default::default()
                };
                self.states[1] = FiberState::Runnable;
            }
            Ok(None)
        } else if pc == p.itron_dly_tsk {
            self.vm.d[0] = 0;
            self.vm.pc = ret_pc;
            if self.states[1 - self.active] == FiberState::Runnable {
                self.switch_fiber();
            }
            Ok(None)
        } else if pc == p.itron_set_flg {
            let flgid = self.vm.d[0];
            let setptn = self.vm.d[1];
            let cur = {
                let e = self.flags.entry(flgid).or_default();
                *e |= setptn;
                *e
            };
            let peer = 1 - self.active;
            if let FiberState::WaitFlg {
                flgid: w_id,
                waiptn,
                wfmode,
                p_flgptn,
            } = self.states[peer]
                && w_id == flgid
                && Self::flg_matches(cur, waiptn, wfmode)
            {
                self.complete_flg_wait(flgid, wfmode, p_flgptn, cur)?;
                self.other.d[0] = 0;
                self.states[peer] = FiberState::Runnable;
            }
            self.vm.d[0] = 0;
            self.vm.pc = ret_pc;
            Ok(None)
        } else if pc == p.itron_clr_flg {
            let flgid = self.vm.d[0];
            let clrptn = self.vm.d[1];
            let e = self.flags.entry(flgid).or_default();
            *e &= clrptn;
            self.vm.d[0] = 0;
            self.vm.pc = ret_pc;
            Ok(None)
        } else if pc == p.itron_pol_flg || pc == p.itron_wai_flg {
            let p_flgptn = self.vm.d[0];
            let flgid = self.vm.d[1];
            let waiptn = self
                .vm
                .read_u32(self.vm.sp.wrapping_add(0x0C))
                .map_err(|e| format!("waiptn read err: {e:?}"))?;
            let wfmode = self
                .vm
                .read_u32(self.vm.sp.wrapping_add(0x10))
                .map_err(|e| format!("wfmode read err: {e:?}"))?;
            let cur = *self.flags.get(&flgid).unwrap_or(&0);
            if Self::flg_matches(cur, waiptn, wfmode) {
                self.complete_flg_wait(flgid, wfmode, p_flgptn, cur)?;
                self.vm.d[0] = 0;
                self.vm.pc = ret_pc;
                Ok(None)
            } else if pc == p.itron_pol_flg {
                self.vm.d[0] = 0xFFFF_FFCE;
                self.vm.pc = ret_pc;
                Ok(None)
            } else {
                let peer = 1 - self.active;
                if self.states[peer] != FiberState::Runnable {
                    return Ok(Some("Deadlock: both ITRON fibers blocked on wai_flg"));
                }
                self.vm.pc = ret_pc;
                self.states[self.active] = FiberState::WaitFlg {
                    flgid,
                    waiptn,
                    wfmode,
                    p_flgptn,
                };
                self.switch_fiber();
                Ok(None)
            }
        } else if pc == p.itron_clr_mbf {
            let qid = self.vm.d[0] & 0xFFFF;
            self.mbfs.remove(&qid);
            self.vm.d[0] = 0;
            self.vm.pc = ret_pc;
            Ok(None)
        } else if pc == p.itron_snd_mbf {
            let qid = self.vm.d[0] & 0xFFFF;
            let msg = self.read_mbf_msg(self.vm.d[1])?;
            let peer = 1 - self.active;
            if let FiberState::WaitMbf {
                qid: w_qid,
                dst_ptr,
            } = self.states[peer]
                && w_qid == qid
            {
                self.write_mbf_msg(dst_ptr, &msg)?;
                self.other.d[0] = 0;
                self.states[peer] = FiberState::Runnable;
            } else {
                self.mbfs.entry(qid).or_default().push_back(msg);
            }
            self.vm.d[0] = 0;
            self.vm.pc = ret_pc;
            Ok(None)
        } else if pc == p.itron_rcv_mbf {
            let qid = self.vm.d[0] & 0xFFFF;
            let dst_ptr = self.vm.d[1];
            if let Some(msg) = self.mbfs.get_mut(&qid).and_then(|q| q.pop_front()) {
                self.write_mbf_msg(dst_ptr, &msg)?;
                self.vm.d[0] = 0;
                self.vm.pc = ret_pc;
                Ok(None)
            } else {
                let peer = 1 - self.active;
                if self.states[peer] != FiberState::Runnable {
                    return Ok(Some("Deadlock: both ITRON fibers blocked on rcv_mbf"));
                }
                self.vm.pc = ret_pc;
                self.states[self.active] = FiberState::WaitMbf { qid, dst_ptr };
                self.switch_fiber();
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub(super) fn run_caller_until(&mut self, stop_pc: u32, max_steps: u64) -> Result<(), String> {
        let start_steps = self.vm.steps;
        let mut ring = std::collections::VecDeque::with_capacity(48);
        while !(self.active == 0 && self.vm.pc == stop_pc) {
            if self.vm.steps - start_steps >= max_steps {
                return Err(self.format_err("StepLimit", &ring));
            }
            if self.is_itron_svc(self.vm.pc) {
                if ring.len() == 48 {
                    ring.pop_front();
                }
                ring.push_back(format!(
                    "f{} SVC pc=0x{:08X} d0=0x{:X} d1=0x{:X} sp=0x{:08X}",
                    self.active, self.vm.pc, self.vm.d[0], self.vm.d[1], self.vm.sp
                ));
                if let Some(deadlock_msg) = self.handle_itron_svc()? {
                    return Err(self.format_err(deadlock_msg, &ring));
                }
                continue;
            }
            if self.vm.pc == self.profile.ext_stubs[1] {
                let msg = self.read_cstr(self.vm.d[1]);
                if !msg.is_empty() {
                    let a1 = self.vm.read_u32(self.vm.sp.wrapping_add(0x0C)).unwrap_or(0);
                    let a2 = self.vm.read_u32(self.vm.sp.wrapping_add(0x10)).unwrap_or(0);
                    self.nfrom_logs.push(format!("{msg} [0x{a1:X}, 0x{a2:X}]"));
                }
            }
            let op = self.vm.read_u8(self.vm.pc).unwrap_or(0xFF);
            if ring.len() == 48 {
                ring.pop_front();
            }
            ring.push_back(format!(
                "f{} pc=0x{:08X} op=0x{:02X} sp=0x{:08X} d0=0x{:X} d1=0x{:X} d2=0x{:X} a0=0x{:08X}",
                self.active,
                self.vm.pc,
                op,
                self.vm.sp,
                self.vm.d[0],
                self.vm.d[1],
                self.vm.d[2],
                self.vm.a[0]
            ));
            if let Err(e) = self.vm.step() {
                return Err(self.format_err(&format!("{e:?}"), &ring));
            }
        }
        Ok(())
    }
}

pub(super) fn call_main_va_with_policy(
    profile: &MainBgoVmProfile,
    main_img: &[u8],
    nor_8mb: &[u8],
    fn_va: u32,
    (d0, d1, a0): (u32, u32, u32),
    allow_upper_nor_writes: bool,
) -> Result<(u32, String), String> {
    let mut stub = vec![0xCBu8; 0x100];
    let rel = fn_va.wrapping_sub(VA_MAIN_CALLER_STUB).to_le_bytes();
    stub[0..7].copy_from_slice(&[0xDD, rel[0], rel[1], rel[2], rel[3], 0x00, 0x00]);
    let mut vm =
        map_main_bgo_vm_with_policy(profile, main_img, nor_8mb, stub, allow_upper_nor_writes);

    vm.d[0] = d0;
    vm.d[1] = d1;
    vm.a[0] = a0;
    vm.sp = VA_MAIN_STACK_TOP;
    vm.pc = VA_MAIN_CALLER_STUB;
    let mut runner = ItronTwoTaskRunner::new(vm, *profile);
    runner.run_caller_until(VA_MAIN_CALLER_STUB + 7, 4_000_000)?;
    let (_, nor_sha) = calculate_hashes(
        runner
            .vm
            .segment_bytes(AM34_FLASH_UNCACHED_BASE)
            .ok_or_else(|| "Missing NOR segment".to_string())?,
    );
    Ok((runner.vm.d[0], nor_sha))
}

#[test]
fn main3060np_bgo_retarget_roundtrips_and_fails_closed_on_corruption() {
    let orig = build_synthetic_main3060np_bgo_fixture();
    let (_, orig_sha) = calculate_hashes(&orig);
    let mut work = orig;

    assert_eq!(
        inspect_main_bgo_geometry(&work).unwrap().state,
        MAIN_BGO_STATE_FACTORY_4MB
    );
    apply_main_bgo_retarget_in_place(&mut work, true).unwrap();
    // Idempotent reapply
    apply_main_bgo_retarget_in_place(&mut work, true).unwrap();
    let st8 = inspect_main_bgo_geometry(&work).unwrap();
    assert_eq!(st8.state, MAIN_BGO_STATE_8MB);
    assert_eq!(st8.bgo_flash_base_hex, "0x84130000");
    assert_eq!(st8.nfd_bss_end_hex, "0x531BC000");

    apply_main_bgo_retarget_in_place(&mut work, false).unwrap();
    assert_eq!(calculate_hashes(&work).1, orig_sha);

    // Corrupt a single word site -> fail-closed
    work[va_off(0x579E_D354)] ^= 0x01;
    assert_eq!(
        inspect_main_bgo_geometry(&work).unwrap().state,
        MAIN_BGO_STATE_MISMATCH
    );
    assert!(apply_main_bgo_retarget_in_place(&mut work, true).is_err());
}
