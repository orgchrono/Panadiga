#![deny(unsafe_code)]

//! Pure CoW NOR autoboot capsule (`Nor-FROM` self-extracting `SA11..SA12` + `update` `case 3 = 0xFF`) installer.
//!
//! Compresses SATA HDD autoboot `PROG061` with `zlib` behind a 256 B MN103S self-unpacker into `SA11..=SA12`
//! (`0x8404_0000..0x8405_FFFF`), leaving `SA13..=SA14` (`0x8406_0000..0x8407_FFFF`) 100% virgin
//! `0xFF` for `MAIN` `NFD_NMI` power-fail recovery and keeping `SA0..=SA10` strictly untouched.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};

use anyhow::{Result, bail};
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use serde::{Deserialize, Serialize};

use super::audit_helpers::{
    collect_f2_bases, find_nfd_dir_entry, p1_slot_state, read_bgo_page, read_le_u16,
    read_primary_fat, resolve_nfd_file_page_pid,
};
use super::{
    BgoSectorGeometry, Eh55BgoAuditReport, Eh55BgoSectorKind, audit_bgo_nor_with_geometry,
    parse_bgo_l2p_with_geometry,
};
use crate::constants::{AM34_FLASH_CRITICAL_IPL_SIZE, AM34_FLASH_SIZE};
use crate::nvram::calculate_hashes;
use crate::re::bgo_unbrick::PROG061_EXPECTED_SIZE;
use crate::re::mn10300_emit::{emit_add_sp_imm8, emit_call32_abs, emit_cmp_imm, emit_mov_imm32};
use crate::re::region_compat::TvStandardKind;
use crate::re::sata_bridge_fix::{
    SATA_BRIDGE_STATE_WAIT_1000MS, inspect_prog061_sata_bridge_reset_site,
};
use crate::re::sata_hdd_autoboot::{
    PROG061_HDD_AUTOBOOT_STATE_ENABLED, inspect_prog061_hdd_autoboot_site,
};

const SA11_12_OFFSET: usize = 0x4_0000;
const SA11_12_END: usize = 0x6_0000;
const SA13_14_OFFSET: usize = 0x6_0000;
const SA13_14_END: usize = 0x8_0000;
const SELF_UNPACK_HEADER_BYTES: usize = 0x100;
const TRAMPOLINE_OFFSET: usize = 0xA0;
const TRAMPOLINE_MAX_BYTES: usize = 0x5C;

/// Exact byte length of the SDRAM self-unpacker prefix preceding the zlib stream (`0x100` / `256 B`).
pub const NOR_AUTOBOOT_CAPSULE_HEADER_BYTES: usize = SELF_UNPACK_HEADER_BYTES;
/// Byte offset within the 256 B NOR autoboot self-unpacker header carrying [`NOR_AUTOBOOT_CAPSULE_MAGIC_LE`].
pub const NOR_AUTOBOOT_CAPSULE_MAGIC_OFFSET: usize = 0xFC;
/// Canonical 4-byte little-endian signature (`"NAUT"`, `0x5455_414E`) stamped at [`NOR_AUTOBOOT_CAPSULE_MAGIC_OFFSET`] (`+0xFC`).
pub const NOR_AUTOBOOT_CAPSULE_MAGIC_LE: u32 = u32::from_le_bytes(*b"NAUT");

const VA_FVU_ENTRY: u32 = 0x5040_0000;
const VA_FVU_UNCACHED_DST: u32 = 0x9040_0000;
const VA_AUTOBOOT_UNPACK_DST: u32 = 0x9100_0000;
const VA_AUTOBOOT_TRAMP_DST: u32 = 0x9104_0000;
const VA_RAM_TRAMP_SRC: u32 = 0x9040_00A0;
const VA_RAM_ZLIB_SRC: u32 = 0x9040_0100;
const VA_LOADER_UNCOMPRESS: u32 = 0x4402_834D;
const VA_LOADER_MEMCPY: u32 = 0x4402_A34A;
const VA_LOADER_MEDIA_GATE: u32 = 0x4400_8BC5;
const VA_AM34_CHCTR: u32 = 0xC000_0070;

/// Summary report emitted by [`build_nor_autoboot_cow_image`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NorAutobootInstallReport {
    pub source_nor_sha256: String,
    pub target_nor_sha256: String,
    pub ipl_sha256: String,
    pub payload_sha256: String,
    pub fvu_marker_pid: u16,
    pub update_pid: u16,
    pub config1_page9_pid: Option<u16>,
    pub sa11_12_needed_erase: bool,
    pub sa11_12_compressed_bytes: usize,
    pub sa13_14_preserved_blank: bool,
    pub written_bgo_pids: Vec<u16>,
    pub post_audit: Eh55BgoAuditReport,
}

fn assemble_sa11_12_self_unpacker(zlen: usize) -> Result<[u8; SELF_UNPACK_HEADER_BYTES]> {
    let zlen32 = u32::try_from(zlen)?;
    let mut hdr = [0xFFu8; SELF_UNPACK_HEADER_BYTES];

    let mut p1 = Vec::with_capacity(TRAMPOLINE_OFFSET);
    emit_add_sp_imm8(&mut p1, -0x20);
    let _ = emit_mov_imm32(&mut p1, 0xCC, PROG061_EXPECTED_SIZE as u32);
    p1.extend_from_slice(&[0x42, 0x18]);
    let _ = emit_mov_imm32(&mut p1, 0xCC, VA_RAM_ZLIB_SRC);
    p1.extend_from_slice(&[0x42, 0x0C]);
    let _ = emit_mov_imm32(&mut p1, 0xCC, zlen32);
    p1.extend_from_slice(&[0x42, 0x10]);
    p1.extend_from_slice(&[0x3C, 0x20, 0x18, 0xF1, 0xD1]);
    let _ = emit_mov_imm32(&mut p1, 0xCC, VA_AUTOBOOT_UNPACK_DST);
    emit_call32_abs(&mut p1, VA_FVU_ENTRY, VA_LOADER_UNCOMPRESS);
    emit_cmp_imm(&mut p1, 0, 0);
    let bne1 = p1.len();
    p1.extend_from_slice(&[0xC9, 0x00]);
    p1.extend_from_slice(&[0x58, 0x18]);
    let _ = emit_mov_imm32(&mut p1, 0xC8, PROG061_EXPECTED_SIZE as u32);
    let bne2 = p1.len();
    p1.extend_from_slice(&[0xC9, 0x00]);
    p1.extend_from_slice(&[0x80, TRAMPOLINE_MAX_BYTES as u8, 0x42, 0x0C]);
    let _ = emit_mov_imm32(&mut p1, 0xCC, VA_AUTOBOOT_TRAMP_DST);
    let _ = emit_mov_imm32(&mut p1, 0xCD, VA_RAM_TRAMP_SRC);
    emit_call32_abs(&mut p1, VA_FVU_ENTRY, VA_LOADER_MEMCPY);
    let _ = emit_mov_imm32(&mut p1, 0xDC, VA_AUTOBOOT_TRAMP_DST);
    p1.extend_from_slice(&[0xF0, 0xF4]);
    let fail = p1.len();
    p1[bne1 + 1] = u8::try_from(i8::try_from(fail - bne1)?)?;
    p1[bne2 + 1] = u8::try_from(i8::try_from(fail - bne2)?)?;
    p1.extend_from_slice(&[0x00, 0x04]);
    let _ = emit_mov_imm32(&mut p1, 0xDC, VA_LOADER_MEDIA_GATE);
    p1.extend_from_slice(&[0xF0, 0xF4]);
    if p1.len() > TRAMPOLINE_OFFSET {
        bail!(
            "SA11..SA12 unpacker Phase 1 ({} B) exceeds 0x{TRAMPOLINE_OFFSET:X}",
            p1.len()
        );
    }
    hdr[..p1.len()].copy_from_slice(&p1);

    let mut p2 = Vec::with_capacity(TRAMPOLINE_MAX_BYTES);
    let _ = emit_mov_imm32(&mut p2, 0xCC, PROG061_EXPECTED_SIZE as u32);
    p2.extend_from_slice(&[0x42, 0x0C]);
    let _ = emit_mov_imm32(&mut p2, 0xCC, VA_FVU_UNCACHED_DST);
    let _ = emit_mov_imm32(&mut p2, 0xCD, VA_AUTOBOOT_UNPACK_DST);
    emit_call32_abs(&mut p2, VA_AUTOBOOT_TRAMP_DST, VA_LOADER_MEMCPY);
    let _ = emit_mov_imm32(&mut p2, 0xDC, VA_AM34_CHCTR);
    p2.extend_from_slice(&[0xF0, 0x60, 0xFA, 0xE0, 0xFC, 0xFF, 0xF0, 0x70]);
    p2.extend_from_slice(&[0xF0, 0x60, 0xF8, 0xE0, 0x0C, 0xC9, 0xFB]);
    p2.extend_from_slice(&[0xF0, 0x60, 0xF8, 0xE4, 0x30, 0xF0, 0x70]);
    p2.extend_from_slice(&[0xF0, 0x60, 0xF8, 0xE0, 0x0C, 0xC9, 0xFB]);
    emit_add_sp_imm8(&mut p2, 0x20);
    let _ = emit_mov_imm32(&mut p2, 0xDC, VA_FVU_ENTRY);
    p2.extend_from_slice(&[0xF0, 0xF4]);
    if p2.len() > TRAMPOLINE_MAX_BYTES {
        bail!(
            "SA11..SA12 unpacker Phase 2 ({} B) exceeds 0x{TRAMPOLINE_MAX_BYTES:X}",
            p2.len()
        );
    }
    hdr[TRAMPOLINE_OFFSET..TRAMPOLINE_OFFSET + p2.len()].copy_from_slice(&p2);
    hdr[NOR_AUTOBOOT_CAPSULE_MAGIC_OFFSET..SELF_UNPACK_HEADER_BYTES]
        .copy_from_slice(&NOR_AUTOBOOT_CAPSULE_MAGIC_LE.to_le_bytes());
    Ok(hdr)
}

fn clone_bgo_page(
    nor: &[u8],
    l2p: &BTreeMap<u16, u16>,
    f2_bases: &BTreeMap<u16, usize>,
    pid: u16,
    label: &str,
) -> Result<[u8; 512]> {
    let pg = read_bgo_page(nor, l2p, f2_bases, pid)
        .ok_or_else(|| anyhow::anyhow!("Missing BGO {label} page PID 0x{pid:04X}"))?;
    let mut out = [0u8; 512];
    out.copy_from_slice(pg);
    Ok(out)
}

fn select_target_f2_sector(
    cow: &[u8],
    f2_bases: &BTreeMap<u16, usize>,
    needed_slots: usize,
) -> Result<(u16, usize, usize)> {
    let mut best: Option<(bool, u16, usize, usize, usize)> = None;
    for (&fid, &base) in f2_bases {
        let is_open = cow.get(base + 0xFC..base + 0x100) == Some(&[0x00, 0x00, 0xFF, 0xFF]);
        let max_used = (0..126usize).rfind(|&s| read_le_u16(cow, base + 0x200 + s * 4) != 0xFFFF);
        let next = max_used.map_or(0, |s| s + 1);
        let holes = max_used
            .is_some_and(|m| (0..m).any(|s| read_le_u16(cow, base + 0x200 + s * 4) == 0xFFFF));
        let avail = 126usize.saturating_sub(next);
        let clean = (next..126).all(|s| {
            cow[base + 0x200 + s * 4..base + 0x200 + s * 4 + 4] == [0xFF; 4]
                && cow[base + 0x400 + s * 0x200..base + 0x400 + (s + 1) * 0x200]
                    .iter()
                    .all(|&b| b == 0xFF)
        });
        if holes || avail < needed_slots || !clean {
            continue;
        }
        if best.is_none_or(|(bo, _, _, _, ba)| (is_open && !bo) || (is_open == bo && avail > ba)) {
            best = Some((is_open, fid, base, next, avail));
        }
    }
    best.map(|(_, fid, base, next, _)| (fid, base, next))
        .ok_or_else(|| {
            anyhow::anyhow!("No contiguous F2 sector has {needed_slots} trailing blank slots")
        })
}

fn queue_fvu_marker_and_fat_updates(
    cow: &[u8],
    l2p: &BTreeMap<u16, u16>,
    f2_bases: &BTreeMap<u16, usize>,
    queued: &mut BTreeMap<u16, [u8; 512]>,
) -> Result<u16> {
    let mut marker = [0xFFu8; 512];
    marker[..4].copy_from_slice(b"STG2");
    if let Some(existing) = find_nfd_dir_entry(cow, l2p, f2_bases, "fvu.exe") {
        queued.insert(existing.start_pid, marker);
        return Ok(existing.start_pid);
    }

    let fat = read_primary_fat(cow, l2p, f2_bases)?;
    let free_pid = (1u16..0x0F00)
        .find(|&p| {
            read_le_u16(&fat, usize::from(p - 1) * 2) == 0xFFFF
                && l2p.get(&p).is_none_or(|&w| w == 0xFFFF)
        })
        .ok_or_else(|| anyhow::anyhow!("No free BGO logical PID available for 'fvu.exe'"))?;
    queued.insert(free_pid, marker);

    let fat_pg_idx = (free_pid - 1) / 256;
    let in_off = usize::from((free_pid - 1) % 256) * 2;
    for base_pid in [0x0F07u16, 0x0F16u16] {
        let mut pg = clone_bgo_page(cow, l2p, f2_bases, base_pid + fat_pg_idx, "FAT")?;
        pg[in_off..in_off + 2].copy_from_slice(&0xFFEDu16.to_le_bytes());
        queued.insert(base_pid + fat_pg_idx, pg);
    }

    let (dir_pid, slot, mut dir_pg) = [0x0F03u16, 0x0F04, 0x0F02]
        .into_iter()
        .find_map(|dpid| {
            let pg = clone_bgo_page(cow, l2p, f2_bases, dpid, "dir").ok()?;
            let s = (0..16usize).find(|&i| pg[i * 16] == 0xFF && pg[0x100 + i * 16] == 0xFF)?;
            Some((dpid, s, pg))
        })
        .ok_or_else(|| anyhow::anyhow!("BGO NFD directory pages 0x0F02..0x0F04 are full"))?;
    let mut entry = [0xFFu8; 16];
    entry[..8].copy_from_slice(b"fvu.exe\0");
    entry[8..12].copy_from_slice(&4u32.to_le_bytes());
    entry[12..14].copy_from_slice(&free_pid.to_le_bytes());
    dir_pg[slot * 16..(slot + 1) * 16].copy_from_slice(&entry);
    dir_pg[0x100 + slot * 16..0x100 + (slot + 1) * 16].copy_from_slice(&entry);
    queued.insert(dir_pid, dir_pg);

    let mut super_pg = clone_bgo_page(cow, l2p, f2_bases, 0x0F01, "superblock")?;
    let next_file_cnt = read_le_u16(&super_pg, 6).saturating_add(1);
    let next_total_pgs = read_le_u16(&super_pg, 8).saturating_add(1);
    super_pg[6..8].copy_from_slice(&next_file_cnt.to_le_bytes());
    super_pg[8..10].copy_from_slice(&next_total_pgs.to_le_bytes());
    queued.insert(0x0F01, super_pg);
    Ok(free_pid)
}

fn commit_bgo_pages_to_cow(
    cow: &mut [u8],
    geom: &BgoSectorGeometry,
    p1_sa: usize,
    f2_bases: &BTreeMap<u16, usize>,
    l2p: &mut BTreeMap<u16, u16>,
    queued: &BTreeMap<u16, [u8; 512]>,
) -> Result<()> {
    let (target_fid, target_f2_base, start_slot) =
        select_target_f2_sector(cow, f2_bases, queued.len())?;
    let mut updated_tids = BTreeSet::new();
    for (idx, (&pid, payload)) in queued.iter().enumerate() {
        let slot = start_slot + idx;
        if let Some(&old_w) = l2p.get(&pid).filter(|&&w| w != 0xFFFF)
            && let Some(&old_base) = f2_bases.get(&(old_w >> 8))
        {
            let old_slot = usize::from(old_w & 0xFF) >> 1;
            cow[old_base + 0x200 + old_slot * 4..old_base + 0x200 + old_slot * 4 + 2]
                .copy_from_slice(&[0x00, 0x00]);
        }
        cow[target_f2_base + 0x400 + slot * 0x200..target_f2_base + 0x400 + (slot + 1) * 0x200]
            .copy_from_slice(payload);
        cow[target_f2_base + 0x200 + slot * 4..target_f2_base + 0x200 + slot * 4 + 2]
            .copy_from_slice(&pid.to_le_bytes());
        l2p.insert(pid, (target_fid << 8) | ((slot as u16) * 2));
        updated_tids.insert(pid / 16);
    }

    let p1_base = geom.sector_offset(p1_sa)?;
    let mut blank_sps = Vec::new();
    for sp in 2..256usize {
        let off = p1_base + sp * 0x100;
        if cow[off..off + 0x100].iter().all(|&b| b == 0xFF) {
            blank_sps.push(off);
            continue;
        }
        let b = [cow[off], cow[off + 1], cow[off + 2], cow[off + 3]];
        for s in 0..7usize {
            let soff = off + 4 + s * 0x24;
            let tid = read_le_u16(cow, soff);
            if p1_slot_state(&b, s, read_le_u16(cow, soff + 2)) == 8 && updated_tids.contains(&tid)
            {
                cow[off + 3] &= !(1u8 << s);
            }
        }
    }

    let tids: Vec<u16> = updated_tids.into_iter().collect();
    if tids.len() > blank_sps.len() * 7 {
        bail!(
            "Insufficient blank P1 subpages in SA{p1_sa}: need {} slots, have {}",
            tids.len(),
            blank_sps.len() * 7
        );
    }
    for (idx, tid) in tids.into_iter().enumerate() {
        let sp_off = blank_sps[idx / 7];
        let s = idx % 7;
        for b_i in 0..3usize {
            cow[sp_off + b_i] &= !(1u8 << s);
        }
        let soff = sp_off + 4 + s * 0x24;
        cow[soff..soff + 2].copy_from_slice(&tid.to_le_bytes());
        for i in 0..16u16 {
            let w = l2p.get(&(tid * 16 + i)).copied().unwrap_or(0xFFFF);
            cow[soff + 4 + usize::from(i) * 2..soff + 6 + usize::from(i) * 2]
                .copy_from_slice(&w.to_le_bytes());
        }
    }
    Ok(())
}

/// Packs a validated pre-FVU SATA HDD autoboot `PROG061` into a universal 256 KiB
/// NOR autoboot SDRAM self-unpacking container (bootable identically via `PANA__SD.FRM` or `SA11..SA12`),
/// leaving the trailing 128 KiB (`0x2_0000..0x4_0000` / `SA13..SA14`) 100% `0xFF`.
pub fn build_nor_autoboot_capsule(autoboot_prog061: &[u8]) -> Result<(Vec<u8>, usize)> {
    if autoboot_prog061.len() != PROG061_EXPECTED_SIZE {
        bail!(
            "Invalid SATA HDD autoboot PROG061 buffer size: {} B (expected {} B)",
            autoboot_prog061.len(),
            PROG061_EXPECTED_SIZE
        );
    }
    if inspect_prog061_sata_bridge_reset_site(autoboot_prog061)?.state
        != SATA_BRIDGE_STATE_WAIT_1000MS
        || inspect_prog061_hdd_autoboot_site(autoboot_prog061)?.state
            != PROG061_HDD_AUTOBOOT_STATE_ENABLED
    {
        bail!("PROG061 payload must carry both 1000ms SATA reset fix and SATA HDD autoboot hook");
    }

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(autoboot_prog061)?;
    let compressed = encoder.finish()?;
    let sa11_12_compressed_bytes = compressed.len();
    if SELF_UNPACK_HEADER_BYTES + sa11_12_compressed_bytes > SA11_12_END - SA11_12_OFFSET {
        bail!(
            "Compressed PROG061 ({} + {} B) exceeds SA11..SA12 128 KiB window",
            SELF_UNPACK_HEADER_BYTES,
            sa11_12_compressed_bytes
        );
    }
    let hdr = assemble_sa11_12_self_unpacker(sa11_12_compressed_bytes)?;
    let mut out = vec![0xFFu8; PROG061_EXPECTED_SIZE];
    out[..SELF_UNPACK_HEADER_BYTES].copy_from_slice(&hdr);
    out[SELF_UNPACK_HEADER_BYTES..SELF_UNPACK_HEADER_BYTES + sa11_12_compressed_bytes]
        .copy_from_slice(&compressed);
    Ok((out, sa11_12_compressed_bytes))
}

/// If `buf` is a NOR autoboot self-unpacking 256 KiB `PROG061` container, decompresses the inner payload.
#[must_use]
pub fn unpack_nor_autoboot_capsule(buf: &[u8]) -> Option<Vec<u8>> {
    if buf.len() != PROG061_EXPECTED_SIZE {
        return None;
    }
    if buf.get(NOR_AUTOBOOT_CAPSULE_MAGIC_OFFSET..SELF_UNPACK_HEADER_BYTES)?
        != NOR_AUTOBOOT_CAPSULE_MAGIC_LE.to_le_bytes()
    {
        return None;
    }
    let window = SA11_12_END - SA11_12_OFFSET;
    if !buf[window..].iter().all(|&b| b == 0xFF) {
        return None;
    }
    let mut decoder = ZlibDecoder::new(&buf[SELF_UNPACK_HEADER_BYTES..window]);
    let mut out = Vec::with_capacity(PROG061_EXPECTED_SIZE);
    decoder.read_to_end(&mut out).ok()?;
    (out.len() == PROG061_EXPECTED_SIZE).then_some(out)
}

/// Builds a verified CoW 8 MiB NOR image with self-extracting NOR autoboot (`SA11..SA12` + `fvu.exe` + `update` `case 3 = 0xFF`).
pub fn build_nor_autoboot_cow_image(
    nor_8mb: &[u8],
    autoboot_prog061: &[u8],
    tv_standard: Option<TvStandardKind>,
) -> Result<(Vec<u8>, NorAutobootInstallReport)> {
    if nor_8mb.len() != AM34_FLASH_SIZE as usize {
        bail!(
            "Invalid NOR input buffer size: {} B (expected {} B)",
            nor_8mb.len(),
            AM34_FLASH_SIZE
        );
    }
    let (self_unpack_256k, sa11_12_compressed_bytes) =
        build_nor_autoboot_capsule(autoboot_prog061)?;

    let (_, source_nor_sha256) = calculate_hashes(nor_8mb);
    let (_, ipl_sha256) = calculate_hashes(&nor_8mb[..AM34_FLASH_CRITICAL_IPL_SIZE as usize]);
    let (_, payload_sha256) = calculate_hashes(autoboot_prog061);
    let mut cow = nor_8mb.to_vec();
    let sa11_12_needed_erase = !cow[SA11_12_OFFSET..SA11_12_END].iter().all(|&b| b == 0xFF);
    let window = SA11_12_END - SA11_12_OFFSET;
    cow[SA11_12_OFFSET..SA11_12_END].copy_from_slice(&self_unpack_256k[..window]);

    let sa13_14_preserved_blank = cow[SA13_14_OFFSET..SA13_14_END].iter().all(|&b| b == 0xFF);
    if !sa13_14_preserved_blank {
        bail!(
            "NOR autoboot CoW safety violation: SA13..SA14 NFD_NMI window must remain 0xFF blank"
        );
    }

    let geom = BgoSectorGeometry::eh55_default();
    let pre_audit = audit_bgo_nor_with_geometry(&cow, &geom)?;
    if !pre_audit.healthy {
        bail!("Refusing NOR autoboot CoW build on unhealthy BGO NOR (heal BGO FTL first)");
    }
    let p1_sa = pre_audit
        .sectors
        .iter()
        .find(|s| s.kind == Eh55BgoSectorKind::Partition1Active)
        .map(|s| s.sa)
        .ok_or_else(|| anyhow::anyhow!("No active BGO P1 sector found"))?;
    let mut l2p = parse_bgo_l2p_with_geometry(&cow, &geom)?;
    let f2_bases = collect_f2_bases(&cow, &geom)?;
    let mut queued: BTreeMap<u16, [u8; 512]> = BTreeMap::new();

    let fvu_marker_pid = queue_fvu_marker_and_fat_updates(&cow, &l2p, &f2_bases, &mut queued)?;
    let update_pid = find_nfd_dir_entry(&cow, &l2p, &f2_bases, "update")
        .ok_or_else(|| anyhow::anyhow!("BGO NFD 'update' entry not found"))?
        .start_pid;
    let mut update_pg = [0xFFu8; 512];
    update_pg[..8].copy_from_slice(b"ATIHSTAM");
    update_pg[8..48].fill(0x00);
    update_pg[8 + 2] = 0xFF; // `case 3 = 0xFF` only (`case 2/16/17` remain 0x00)
    queued.insert(update_pid, update_pg);

    let mut config1_page9_pid = None;
    if let Some(std_kind) = tv_standard {
        let tv_byte = match std_kind {
            TvStandardKind::Ntsc => 0x01u8,
            TvStandardKind::PalM => 0x00u8,
            other => bail!(
                "NOR autoboot config1 TV system supports 'ntsc' or 'pal-m', got '{}'",
                other.as_str()
            ),
        };
        let cfg_pid = resolve_nfd_file_page_pid(&cow, &geom, "config1", 9)?;
        let mut cfg_pg = clone_bgo_page(&cow, &l2p, &f2_bases, cfg_pid, "config1[page=9]")?;
        cfg_pg[0x0EC] = tv_byte;
        queued.insert(cfg_pid, cfg_pg);
        config1_page9_pid = Some(cfg_pid);
    }

    let written_bgo_pids: Vec<u16> = queued.keys().copied().collect();
    commit_bgo_pages_to_cow(&mut cow, &geom, p1_sa, &f2_bases, &mut l2p, &queued)?;

    if calculate_hashes(&cow[..AM34_FLASH_CRITICAL_IPL_SIZE as usize]).1 != ipl_sha256 {
        bail!("NOR autoboot CoW safety violation: SA0..SA10 IPL hash mutated");
    }
    let post_audit = audit_bgo_nor_with_geometry(&cow, &geom)?;
    if !post_audit.healthy
        || !post_audit.corrupt_sas.is_empty()
        || post_audit.l2p_tag_mismatches != 0
        || post_audit.fat_mirror_mismatches != 0
    {
        bail!("NOR autoboot CoW post-audit failed");
    }
    let (_, target_nor_sha256) = calculate_hashes(&cow);
    Ok((
        cow,
        NorAutobootInstallReport {
            source_nor_sha256,
            target_nor_sha256,
            ipl_sha256,
            payload_sha256,
            fvu_marker_pid,
            update_pid,
            config1_page9_pid,
            sa11_12_needed_erase,
            sa11_12_compressed_bytes,
            sa13_14_preserved_blank,
            written_bgo_pids,
            post_audit,
        },
    ))
}
