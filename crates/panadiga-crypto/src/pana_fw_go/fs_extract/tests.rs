use std::io::Write;

use flate2::Compression;
use flate2::write::ZlibEncoder;

use super::*;
use crate::pana_fw_go::checksum_affs;

fn make_synthetic_mn103_elf() -> Vec<u8> {
    let mut elf = vec![0u8; 64];
    elf[0..6].copy_from_slice(b"\x7fELF\x01\x01");
    elf[16..18].copy_from_slice(&2u16.to_le_bytes());
    elf[18..20].copy_from_slice(&0x59u16.to_le_bytes());
    elf[24..28].copy_from_slice(&0x0040_1234u32.to_le_bytes());
    elf
}

fn build_rich_romfs() -> Vec<u8> {
    fn pad16_str(s: &[u8]) -> Vec<u8> {
        let mut out = s.to_vec();
        out.push(0);
        while !out.len().is_multiple_of(16) {
            out.push(0);
        }
        out
    }

    let elf = make_synthetic_mn103_elf();
    let sym_target = b"/bin/busybox";

    let mut img = vec![0u8; 16];
    img[0..8].copy_from_slice(PANA_ROMFS_MAGIC);
    img.extend_from_slice(&pad16_str(b"panatest"));

    let hdr0 = img.len();
    img.extend_from_slice(&[0u8; 16]);
    img.extend_from_slice(&pad16_str(b"app.elf"));
    let data0_off = img.len();
    let mut data0 = elf.clone();
    while !data0.len().is_multiple_of(16) {
        data0.push(0);
    }
    img.extend_from_slice(&data0);

    let hdr1 = img.len();
    img.extend_from_slice(&[0u8; 16]);
    img.extend_from_slice(&pad16_str(b"sh"));
    let mut sdata = sym_target.to_vec();
    while !sdata.len().is_multiple_of(16) {
        sdata.push(0);
    }
    img.extend_from_slice(&sdata);

    let hdr2 = img.len();
    img.extend_from_slice(&[0u8; 16]);
    img.extend_from_slice(&pad16_str(b"README.txt"));
    img.extend_from_slice(&pad16_str(b"hello_panadiga"));

    let total_sz = img.len() as u32;
    img[8..12].copy_from_slice(&total_sz.to_be_bytes());
    img[hdr0..hdr0 + 4].copy_from_slice(&((hdr1 as u32) | 2).to_be_bytes());
    img[hdr0 + 8..hdr0 + 12].copy_from_slice(&(elf.len() as u32).to_be_bytes());
    let _ = data0_off;
    img[hdr1..hdr1 + 4].copy_from_slice(&((hdr2 as u32) | 3).to_be_bytes());
    img[hdr1 + 8..hdr1 + 12].copy_from_slice(&(sym_target.len() as u32).to_be_bytes());
    img[hdr2..hdr2 + 4].copy_from_slice(&2u32.to_be_bytes());
    img[hdr2 + 8..hdr2 + 12].copy_from_slice(&(14u32).to_be_bytes());

    let sum_len = img.len().min(512) & !3;
    let csum = checksum_affs(&img[..sum_len]).unwrap();
    img[12..16].copy_from_slice(&csum.to_be_bytes());
    img
}

fn build_rich_cramfs() -> Vec<u8> {
    let mut img = vec![0u8; 76];
    img[0..4].copy_from_slice(&PANA_CRAMFS_MAGIC.to_le_bytes());
    img[4..8].copy_from_slice(&65536u32.to_le_bytes()); // deliberate dummy v1 size!
    img[16..32].copy_from_slice(PANA_CRAMFS_SIGNATURE);
    img[48..58].copy_from_slice(b"Compressed");

    // Root inode at 64: dir pointing to child list right at 76
    img[64..66].copy_from_slice(&0o040755u16.to_le_bytes());
    let root_children_off = img.len();
    let elf = make_synthetic_mn103_elf();

    let mut enc_elf = ZlibEncoder::new(Vec::new(), Compression::default());
    enc_elf.write_all(&elf).unwrap();
    let z_elf = enc_elf.finish().unwrap();

    // 1 child inode inside root directory: name = "bin\0" (4B) -> 16 bytes
    let elf_inode_off = root_children_off;
    img.resize(root_children_off + 16, 0);
    let blk_table_off = img.len();
    let blk_end = blk_table_off + 4 + z_elf.len();
    img.extend_from_slice(&(blk_end as u32).to_le_bytes());
    img.extend_from_slice(&z_elf);

    let w0 = 0o100755u32;
    let w1 = elf.len() as u32;
    let w2 = 1u32 | (((blk_table_off >> 2) as u32) << 6); // namelen/4 = 1 (4 bytes "a\0\0\0")
    img[elf_inode_off..elf_inode_off + 4].copy_from_slice(&w0.to_le_bytes());
    img[elf_inode_off + 4..elf_inode_off + 8].copy_from_slice(&w1.to_le_bytes());
    img[elf_inode_off + 8..elf_inode_off + 12].copy_from_slice(&w2.to_le_bytes());
    img[elf_inode_off + 12..elf_inode_off + 16].copy_from_slice(b"a.so");

    // Update root inode size = 16, offset = root_children_off >> 2
    img[68..72].copy_from_slice(&16u32.to_le_bytes());
    img[72..76].copy_from_slice(&(((root_children_off >> 2) as u32) << 6).to_le_bytes());
    img
}

#[test]
fn sanitize_path_rejects_traversal_and_normalizes_ntfs_hazards() {
    assert!(sanitize_fs_rel_path("../etc/passwd").is_err());
    assert!(sanitize_fs_rel_path("bin/\0bad").is_err());
    assert_eq!(
        sanitize_fs_rel_path("/bin/CON.txt").unwrap(),
        "bin/_CON.txt"
    );
    assert_eq!(sanitize_fs_rel_path("dir/a:b*c. ").unwrap(), "dir/a_b_c");
}

#[test]
fn extract_romfs_and_cramfs_handle_elfs_and_symlinks_safely_on_windows() {
    let dir = std::env::temp_dir().join(format!("panadiga_fs_extract_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);

    let romfs = build_rich_romfs();
    let rom_rep = extract_partition_fs(&romfs, Some(&dir.join("romfs_elf")), true).unwrap();
    assert_eq!(rom_rep.fs_kind, "RomFS");
    assert_eq!(rom_rep.regular_files, 2);
    assert_eq!(rom_rep.symlinks_skipped_on_disk, 1);
    assert_eq!(rom_rep.elf_count, 1);
    assert!(dir.join("romfs_elf").join("app.elf").is_file());
    assert!(!dir.join("romfs_elf").join("README.txt").exists());
    assert!(!dir.join("romfs_elf").join("sh").exists());

    let cramfs = build_rich_cramfs();
    let cram_rep = extract_partition_fs(&cramfs, Some(&dir.join("cramfs")), false).unwrap();
    assert_eq!(cram_rep.fs_kind, "CramFS");
    assert_eq!(cram_rep.elf_count, 1);
    let ent = cram_rep.entries.first().unwrap();
    assert_eq!(ent.elf.as_ref().unwrap().arch_label, "mn10300");
    assert!(dir.join("cramfs").join("a.so").is_file());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn extract_partition_fs_real_bs850_partitions_if_present() {
    let path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../firmware_and_images/dmr-xw_bd_series/bs850_bs750_xs350eg/PANA_DVD.FRM"
    ));
    if !path.is_file() {
        return;
    }
    let raw = std::fs::read(path).unwrap();
    let keyring = crate::FrmKeyring::builtin();
    let unpacked = crate::unpack_universal_frm(&raw, &keyring).unwrap();
    let main = unpacked.sub_containers[0]
        .modules
        .iter()
        .find(|m| m.name.starts_with("MAIN"))
        .unwrap();
    let parts = crate::split_main_partitions(&main.payload).unwrap();
    let mut total_elfs = 0usize;
    for part in &parts {
        if matches!(
            part.fs_kind,
            crate::PartitionFsKind::RomFs(_) | crate::PartitionFsKind::CramFs(_)
        ) {
            let rep = extract_partition_fs(&part.data, None, true).unwrap();
            assert!(rep.regular_files > 0);
            total_elfs += rep.elf_count;
        }
    }
    assert!(
        total_elfs >= 200,
        "Expected >200 ELFs across bs850 fma5/fma6/fma7, got {total_elfs}"
    );
}
