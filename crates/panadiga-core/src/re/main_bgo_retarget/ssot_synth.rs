use anyhow::Result;
use byteorder::{ByteOrder, LittleEndian};

use super::sites::{NFD_4MB_TO_8MB_BYTE_SITES, NFD_4MB_TO_8MB_SLICE_SITES};
use super::tables::{NFD_4MB_TO_8MB_RAM_RELOCS, NFD_4MB_TO_8MB_RODATA_WORDS, va_off};
use super::{
    BytePatchSite, OwnedMainBgoRetargetSpec, OwnedSlicePatchSite, U32RelocSpec, WordPatchSite,
};
use crate::ssot::registry::MainBuildSpec;

const REF_NFD_TEXT_BASE_VA: u32 = 0x574A_68E9;
const REF_RODATA_TABLE_BASE_VA: u32 = 0x579E_D2CC;
const REF_BSS_NFD_CTRL_BASE_VA: u32 = 0x52FE_6CE0;
const REF_BSS_RESERVED_BASE_VA: u32 = 0x531B_8000;
const REF_CAVE_VA: u32 = 0x5700_0320;
const CAVE_JMP_INDICES: [usize; 20] = [
    8, 19, 24, 35, 50, 61, 73, 84, 96, 101, 113, 125, 138, 143, 155, 160, 173, 178, 191, 196,
];

fn shift_offset(base_off: usize, delta: i32, ctx: &str) -> Result<usize> {
    let shifted = base_off as isize + delta as isize;
    usize::try_from(shifted)
        .map_err(|_| anyhow::anyhow!("{ctx}: shifted offset {shifted} underflows zero"))
}

fn shift_i32_le(le_bytes: &mut [u8], delta: i32, ctx: &str) -> Result<()> {
    let old_disp = LittleEndian::read_i32(le_bytes);
    let new_disp = old_disp
        .checked_add(delta)
        .ok_or_else(|| anyhow::anyhow!("{ctx}: displacement overflow ({old_disp} + {delta})"))?;
    LittleEndian::write_i32(le_bytes, new_disp);
    Ok(())
}

/// Synthesizes an [`OwnedMainBgoRetargetSpec`] from SSOT [`MainBuildSpec::bgo_retarget_anchors`].
pub fn build_main_bgo_retarget_spec_from_ssot(
    build: &MainBuildSpec,
) -> Result<OwnedMainBgoRetargetSpec> {
    let anchors = build.bgo_retarget_anchors.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "MAIN{} lacks declarative bgo_retarget_anchors in SSOT",
            build.build_id
        )
    })?;
    let expected_size = build.unpacked_size as usize;
    let va_to_off = |va: u32, label: &str| -> Result<usize> {
        let off = va
            .checked_sub(build.base_va)
            .map(|d| d as usize)
            .ok_or_else(|| anyhow::anyhow!("MAIN{} {label} 0x{va:08X} < base", build.build_id))?;
        if off + 4 > expected_size {
            anyhow::bail!(
                "MAIN{} {label} 0x{va:08X} exceeds unpacked size",
                build.build_id
            );
        }
        Ok(off)
    };

    let code_delta = i32::try_from(anchors.nfd_text_base_va as i64 - REF_NFD_TEXT_BASE_VA as i64)?;
    let rodata_delta =
        i32::try_from(anchors.rodata_table_base_va as i64 - REF_RODATA_TABLE_BASE_VA as i64)?;
    let ram_delta = anchors
        .bss_nfd_ctrl_base_va
        .wrapping_sub(REF_BSS_NFD_CTRL_BASE_VA);
    let res_delta = anchors
        .bss_reserved_base_va
        .wrapping_sub(REF_BSS_RESERVED_BASE_VA);
    let cave_delta = i32::try_from(anchors.cave_va as i64 - REF_CAVE_VA as i64)?;
    let call_disp_delta = i32::try_from(cave_delta as i64 - code_delta as i64)?;
    let ret_disp_delta = i32::try_from(code_delta as i64 - cave_delta as i64)?;

    let bss_reserved_end = anchors
        .bss_reserved_base_va
        .checked_add(0x4000)
        .ok_or_else(|| anyhow::anyhow!("MAIN{} bss_reserved_base_va overflow", build.build_id))?;
    let bss_reserved_uncached_end = bss_reserved_end.wrapping_add(0x4000_0000);

    let mut word_sites = Vec::with_capacity(3 + NFD_4MB_TO_8MB_RODATA_WORDS.len());
    word_sites.push(WordPatchSite {
        offset: va_to_off(anchors.bss_cached_end_site_va, "bss_cached_end_site_va")?,
        expect: anchors.bss_cached_end_orig,
        replace: bss_reserved_end,
    });
    word_sites.push(WordPatchSite {
        offset: va_to_off(
            anchors.crt0_uncached_end_site_va,
            "crt0_uncached_end_site_va",
        )?,
        expect: anchors.bss_uncached_end_orig,
        replace: bss_reserved_uncached_end,
    });
    word_sites.push(WordPatchSite {
        offset: va_to_off(
            anchors.sbrk_uncached_end_site_va,
            "sbrk_uncached_end_site_va",
        )?,
        expect: anchors.bss_uncached_end_orig,
        replace: bss_reserved_uncached_end,
    });
    for s in NFD_4MB_TO_8MB_RODATA_WORDS {
        let is_ram_row = (s.expect & 0xFF00_0000) == 0x5200_0000;
        word_sites.push(WordPatchSite {
            offset: shift_offset(s.offset, rodata_delta, "word_site")?,
            expect: if is_ram_row {
                s.expect.wrapping_add(ram_delta)
            } else {
                s.expect
            },
            replace: if is_ram_row {
                s.replace.wrapping_add(res_delta)
            } else {
                s.replace
            },
        });
    }

    let mut byte_sites = Vec::with_capacity(NFD_4MB_TO_8MB_BYTE_SITES.len());
    for s in NFD_4MB_TO_8MB_BYTE_SITES {
        byte_sites.push(BytePatchSite {
            offset: shift_offset(s.offset, code_delta, "byte_site")?,
            expect: s.expect,
            replace: s.replace,
        });
    }

    let mut slice_sites = Vec::with_capacity(NFD_4MB_TO_8MB_SLICE_SITES.len());
    for (idx, s) in NFD_4MB_TO_8MB_SLICE_SITES.iter().enumerate() {
        let mut replace = s.replace.to_vec();
        if idx + 1 < NFD_4MB_TO_8MB_SLICE_SITES.len() {
            if replace.len() < 5 || replace[0] != 0xDC {
                anyhow::bail!("Unexpected call trampoline opcode at slice {idx}");
            }
            shift_i32_le(&mut replace[1..5], call_disp_delta, "slice_call")?;
            slice_sites.push(OwnedSlicePatchSite {
                offset: shift_offset(s.offset, code_delta, "slice_site")?,
                expect: s.expect.to_vec(),
                replace,
            });
        } else {
            for jmp_idx in CAVE_JMP_INDICES {
                if jmp_idx + 5 > replace.len() || replace[jmp_idx] != 0xDC {
                    anyhow::bail!("Unexpected cave JMP opcode at byte 0x{jmp_idx:X}");
                }
                shift_i32_le(
                    &mut replace[jmp_idx + 1..jmp_idx + 5],
                    ret_disp_delta,
                    "cave_jmp",
                )?;
            }
            slice_sites.push(OwnedSlicePatchSite {
                offset: shift_offset(s.offset, cave_delta, "cave_site")?,
                expect: s.expect.to_vec(),
                replace,
            });
        }
    }

    let mut ram_relocs = Vec::with_capacity(NFD_4MB_TO_8MB_RAM_RELOCS.len());
    for r in NFD_4MB_TO_8MB_RAM_RELOCS {
        let is_ram = (r.from & 0xFF00_0000) == 0x5200_0000;
        ram_relocs.push(U32RelocSpec {
            from: if is_ram {
                r.from.wrapping_add(ram_delta)
            } else {
                r.from
            },
            to: if is_ram {
                r.to.wrapping_add(res_delta)
            } else {
                r.to
            },
            expected_count: r.expected_count,
        });
    }

    Ok(OwnedMainBgoRetargetSpec {
        target_geometry_id: anchors.target_geometry_id.clone(),
        factory_geometry_id: anchors.factory_geometry_id.clone(),
        base_va: build.base_va,
        expected_size,
        reloc_scan_start: shift_offset(va_off(0x574A_6000), code_delta, "reloc_scan_start")?,
        reloc_scan_end: shift_offset(va_off(0x574B_4500), code_delta, "reloc_scan_end")?,
        table_detect_offset: shift_offset(va_off(0x579E_D354), rodata_delta, "table_detect")?,
        word_sites,
        byte_sites,
        slice_sites,
        ram_relocs,
    })
}
