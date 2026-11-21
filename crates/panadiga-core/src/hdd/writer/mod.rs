//! Cross-Platform Physical HDD / SSD & SD Safe Block Discovery and Writer.

mod block_ops;
pub mod composed_flash;
mod discovery;
mod discovery_unix;
mod discovery_win;
pub mod eh55_flash;
pub mod raw_device;
mod types;
pub mod win_ioctl;

pub use block_ops::{
    DriveWriteReport, is_unconditional_os_boot_disk, open_verified_block_device,
    restore_meihdfs_from_mirrors, verify_safety_lockout, verify_safety_lockout_with_os_disks,
    write_meihdfs_procedural_to_drive, write_raw_image_to_drive,
};
pub use composed_flash::{
    ComposedHddFlashReport, ComposedHddFlashRequest, execute_composed_meihfs_hdd_flash,
};
pub use discovery::{
    list_drives_by_role, list_hard_disk_drives, list_physical_drives, list_sd_card_drives,
    resolve_mount_point_to_block_device, unmount_drive_partitions,
};
pub use eh55_flash::{
    EH55_BACKUP_SECTOR_COUNT, EH55_FLASH_CHUNK_BYTES, EH55_MIRROR_MAIN_BACKSTEP_LBAS,
    Eh55FlashProgress, Eh55FlashStage, Eh55HddFlashReport, Eh55HddFlashRequest, SafeTargetDiskInfo,
    compute_eh55_mirror_main_lba, enrich_safe_target_disks, execute_eh55_hdd_flash,
    inspect_safe_target_disks,
};
pub use raw_device::{
    BUS_TYPE_NVME, BlockFlashDevice, FileBackedMockFlashDevice, PhysicalFlashDevice,
    UnixPhysicalFlashDevice, WindowsPhysicalFlashDevice, open_physical_flash_device,
    readback_sha256_verify, write_chunked_bytes,
};
pub use types::{
    MountedPartitionInfo, PhysicalDriveInfo, StorageMediaRole, classify_drive_role,
    filter_drives_by_role,
};

#[cfg(test)]
mod tests {
    use super::discovery_unix::{
        find_linux_os_root_disks, parse_lsblk_blockdevices_json, parse_proc_mounts_entries,
        resolve_mountpoint_device_in_mounts,
    };
    use super::discovery_win::parse_cim_disk_drives_json_with_sys;
    use super::*;

    #[test]
    fn test_system_drive_lockout_blocks_physicaldrive0_and_default_sda() {
        assert!(
            verify_safety_lockout(r"\\.\PhysicalDrive0", "CONFIRM-PhysicalDrive0")
                .unwrap_err()
                .to_string()
                .contains("SAFETY INVARIANT VIOLATION")
        );
        for protected in [
            "/dev/sda",
            "/dev/sda1",
            "/dev/disk0",
            "/dev/rdisk0",
            "/dev/disk0s1",
            "/dev/rdisk0s2",
        ] {
            let token = format!("CONFIRM-{protected}");
            assert!(
                verify_safety_lockout_with_os_disks(protected, &token, None)
                    .unwrap_err()
                    .to_string()
                    .contains("SAFETY INVARIANT VIOLATION"),
                "Expected lockout for {protected}"
            );
        }
        assert!(
            verify_safety_lockout_with_os_disks("/dev/disk2", "CONFIRM-/dev/disk2", None).is_ok()
        );
        assert!(
            verify_safety_lockout_with_os_disks("/dev/rdisk2", "CONFIRM-/dev/rdisk2", None).is_ok()
        );
    }

    #[test]
    fn test_dynamic_os_lockout_allows_external_sda_when_os_is_nvme() {
        let os_disks = ["/dev/nvme0n1"];
        assert!(
            verify_safety_lockout_with_os_disks(
                "/dev/nvme0n1",
                "CONFIRM-/dev/nvme0n1",
                Some(&os_disks)
            )
            .is_err()
        );
        assert!(
            verify_safety_lockout_with_os_disks("/dev/sda", "CONFIRM-/dev/sda", Some(&os_disks))
                .is_ok()
        );
        // Sibling disk names must never be falsely locked out by prefix collision
        assert!(
            verify_safety_lockout_with_os_disks(
                "/dev/disk10",
                "CONFIRM-/dev/disk10",
                Some(&["/dev/disk1"])
            )
            .is_ok()
        );
        assert!(
            verify_safety_lockout_with_os_disks(
                "/dev/disk1s1",
                "CONFIRM-/dev/disk1s1",
                Some(&["/dev/disk1"])
            )
            .is_err()
        );
        assert!(
            verify_safety_lockout_with_os_disks(
                "/dev/sdaa",
                "CONFIRM-/dev/sdaa",
                Some(&["/dev/sda"])
            )
            .is_ok()
        );
    }

    #[test]
    fn test_token_and_restore_safety_guards() {
        assert!(
            verify_safety_lockout(r"\\.\PhysicalDrive1", "WRONG")
                .unwrap_err()
                .to_string()
                .contains("token mismatch")
        );
        assert!(verify_safety_lockout(r"\\.\PhysicalDrive2", "CONFIRM-PhysicalDrive2").is_ok());
        assert!(
            restore_meihdfs_from_mirrors(r"\\.\PhysicalDrive0", "CONFIRM-PhysicalDrive0").is_err()
        );
    }

    #[test]
    fn test_parse_cim_and_role_separation_hdd_vs_sd() {
        let json = r#"[
            {"Index":0,"Model":"Samsung SSD 970 EVO Plus 500GB","Size":500105249280,"InterfaceType":"SCSI","Partitions":[{"DriveLetter":"C:"}]},
            {"Index":1,"Model":"Hitachi PATA HDD 250GB USB","Size":"250059350016","InterfaceType":"USB","Partitions":[{"DriveLetter":"F:"}]},
            {"Index":2,"Model":"Generic SD/MMC Card Reader","Size":2002780160,"InterfaceType":"USB","MediaType":"Removable Media","Partitions":[{"DriveLetter":"E:","VolumeName":"DIGA_SD"}]}
        ]"#;
        let drives = parse_cim_disk_drives_json_with_sys(json, Some("C:"));
        assert_eq!(drives.len(), 3);
        assert_eq!(drives[0].drive_role, StorageMediaRole::SystemOs);
        assert_eq!(drives[1].drive_role, StorageMediaRole::HardDisk);
        assert_eq!(drives[2].drive_role, StorageMediaRole::SdCard);

        let hdds = filter_drives_by_role(drives.clone(), Some(StorageMediaRole::HardDisk));
        let sds = filter_drives_by_role(drives, Some(StorageMediaRole::SdCard));
        assert_eq!(hdds.len(), 1);
        assert_eq!(hdds[0].device_path, r"\\.\PhysicalDrive1");
        assert_eq!(sds.len(), 1);
        assert_eq!(sds[0].mount_points, vec!["E:\\"]);
    }

    #[test]
    fn test_parse_lsblk_roles_mounts_and_nvme_root() {
        let lsblk_json = r#"{
          "blockdevices": [
            {"name": "/dev/nvme0n1", "kname": "nvme0n1", "path": "/dev/nvme0n1", "model": "SN770", "size": 1000204886016, "type": "disk", "tran": "nvme", "rm": false, "children": [{"kname": "nvme0n1p2", "path": "/dev/nvme0n1p2", "size": 999000000000, "mountpoints": ["/"]}]},
            {"name": "/dev/sda", "kname": "sda", "path": "/dev/sda", "vendor": "Hitachi", "model": "HDT722525DLAT80", "size": 250059350016, "type": "disk", "tran": "usb", "rm": false, "children": [{"kname": "sda1", "path": "/dev/sda1", "size": 250050000000, "mountpoint": "/media/user/DIGA HDD"}]},
            {"name": "/dev/mmcblk0", "kname": "mmcblk0", "path": "/dev/mmcblk0", "model": "SD2GB", "size": 2002780160, "type": "disk", "tran": "", "rm": true, "children": [{"kname": "mmcblk0p1", "path": "/dev/mmcblk0p1", "size": 2000000000, "mountpoints": ["/run/media/user/PANASONIC"], "label": "PANASONIC"}]}
          ]
        }"#;
        let drives = parse_lsblk_blockdevices_json(lsblk_json, &[]);
        assert_eq!(drives[0].drive_role, StorageMediaRole::SystemOs);
        assert_eq!(drives[1].drive_role, StorageMediaRole::HardDisk);
        assert_eq!(drives[1].mount_points, vec!["/media/user/DIGA HDD"]);
        assert_eq!(drives[2].drive_role, StorageMediaRole::SdCard);
        assert_eq!(drives[2].bus_type, "MMC/SD");
    }

    #[test]
    fn test_proc_mounts_and_invalid_cim_json() {
        let sample =
            "/dev/nvme0n1p2 / ext4 rw 0 0\n/dev/sda1 /media/user/DIGA\\040SD vfat rw 0 0\n";
        let mounts = parse_proc_mounts_entries(sample);
        assert_eq!(
            find_linux_os_root_disks(&mounts),
            vec!["/dev/nvme0n1".to_string()]
        );
        assert_eq!(
            resolve_mountpoint_device_in_mounts("/media/user/DIGA SD", &mounts),
            Some("/dev/sda1".to_string())
        );
        assert!(parse_cim_disk_drives_json_with_sys("", None).is_empty());

        use super::discovery_unix::should_run_macos_diskutil;
        assert!(should_run_macos_diskutil("/dev/disk0").is_err());
        assert!(should_run_macos_diskutil("/dev/rdisk0s2").is_err());
        assert!(should_run_macos_diskutil("/dev/disk2").unwrap());
        assert!(!should_run_macos_diskutil("/var/folders/tmp/disk_meihdfs.bin").unwrap());
    }
}
