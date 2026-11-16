//! Unit test entry point for `emu_patch_suite`.

use super::*;

#[test]
fn nor_emu_end_to_end_patcher_suite_passes_deterministically() {
    let rep = run_nor_emu_patch_verification_suite(None, None)
        .expect("Offline NOR + MN103Vm patch verification suite should pass");
    assert!(rep.all_passed);
    assert_eq!(rep.checks.len(), 6);
    assert_eq!(rep.cfi_manufacturer_id_hex, "0x0001");
    assert_eq!(rep.cfi_device_id_hex, "0x227E");
}

#[test]
fn real_binaries_and_8mb_nor_jedec_suite_passes_when_present() {
    super::checks_real_nor::run_optional_real_binaries_integration()
        .expect("Real 8 MiB flash.bin + PROG061 + MAIN verification suite should pass");
}
