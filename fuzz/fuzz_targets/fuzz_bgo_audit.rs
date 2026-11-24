#![no_main]

use libfuzzer_sys::fuzz_target;
use panadiga_core::re::{
    audit_eh55_bgo_nor_8mb, eval_bgo_migration_index, is_active_bgo_p1_sector, parse_eh55_bgo_l2p,
};

fuzz_target!(|data: &[u8]| {
    let _ = audit_eh55_bgo_nor_8mb(data);
    let _ = parse_eh55_bgo_l2p(data);
    let _ = is_active_bgo_p1_sector(data, 0);
    let _ = eval_bgo_migration_index(data, 0);
});
