//! Cross-Platform Physical HDD / SSD Safe Block Writer with System Drive Lockout

mod block_ops;
mod discovery;

pub use block_ops::{
    DriveWriteReport, open_verified_block_device, restore_meihdfs_from_mirrors,
    verify_safety_lockout, write_meihdfs_procedural_to_drive, write_raw_image_to_drive,
};
pub use discovery::{PhysicalDriveInfo, list_physical_drives};

#[cfg(test)]
mod tests {
    use super::discovery::parse_cim_disk_drives_json;
    use super::*;

    #[test]
    fn test_system_drive_lockout_blocks_physicaldrive0() {
        let res = verify_safety_lockout(r"\\.\PhysicalDrive0", "CONFIRM-PhysicalDrive0");
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("SAFETY INVARIANT VIOLATION")
        );
    }

    #[test]
    fn test_system_drive_lockout_blocks_sda() {
        let res = verify_safety_lockout("/dev/sda", "CONFIRM-/dev/sda");
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("SAFETY INVARIANT VIOLATION")
        );
    }

    #[test]
    fn test_token_mismatch_rejection() {
        let res = verify_safety_lockout(r"\\.\PhysicalDrive1", "WRONG-TOKEN");
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("token mismatch"));
    }

    #[test]
    fn test_valid_token_allows_non_system_drive() {
        let res = verify_safety_lockout(r"\\.\PhysicalDrive2", "CONFIRM-PhysicalDrive2");
        assert!(res.is_ok());
    }

    #[test]
    fn test_restore_mirrors_blocks_system_drive() {
        let res = restore_meihdfs_from_mirrors(r"\\.\PhysicalDrive0", "CONFIRM-PhysicalDrive0");
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("SAFETY INVARIANT VIOLATION")
        );
    }

    #[test]
    fn test_parse_cim_disk_drives_single_and_array() {
        let single = r#"{"Index":1,"Model":"JMicron Generic SCSI Disk Device","Size":250059350016,"InterfaceType":"USB"}"#;
        let parsed_single = parse_cim_disk_drives_json(single);
        assert_eq!(parsed_single.len(), 1);
        assert_eq!(parsed_single[0].device_path, r"\\.\PhysicalDrive1");
        assert!(!parsed_single[0].is_system_drive);
        assert!(parsed_single[0].safe_to_write);

        let multi = r#"[
            {"Index":1,"Model":"JMicron Generic","Size":250059350016,"InterfaceType":"USB"},
            {"Index":0,"Model":"Samsung SSD 970 EVO Plus 500GB","Size":500105249280,"InterfaceType":"SCSI"}
        ]"#;
        let parsed_multi = parse_cim_disk_drives_json(multi);
        assert_eq!(parsed_multi.len(), 2);
        assert_eq!(parsed_multi[0].device_path, r"\\.\PhysicalDrive0");
        assert!(parsed_multi[0].is_system_drive);
        assert!(!parsed_multi[0].safe_to_write);
        assert_eq!(parsed_multi[1].device_path, r"\\.\PhysicalDrive1");
        assert!(parsed_multi[1].safe_to_write);
    }

    #[test]
    fn test_parse_cim_disk_drives_invalid_json_empty() {
        assert!(parse_cim_disk_drives_json("").is_empty());
        assert!(parse_cim_disk_drives_json("not-json").is_empty());
    }
}
