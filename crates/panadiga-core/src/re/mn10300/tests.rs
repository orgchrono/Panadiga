use super::*;

#[test]
fn test_mn103s_call32_and_fc_string_xrefs() {
    let base = 0x5040_0000u32;
    let mut buf = vec![0xFFu8; 0x80];
    // At 0x5040_0040: call32 targeting 0x5040_0010 (disp = -0x30)
    buf[0x40] = 0xDD;
    buf[0x41..0x45].copy_from_slice(&(-0x30i32).to_le_bytes());
    // At 0x5040_0020: FC CC <ptr 0x5040_0060> -> "BGO OK\n"
    buf[0x20] = 0xFC;
    buf[0x21] = 0xCC;
    buf[0x22..0x26].copy_from_slice(&(base + 0x60).to_le_bytes());
    buf[0x60..0x68].copy_from_slice(b"BGO OK\n\0");

    let callers = find_callers_to(&buf, base, &[0x5040_0010]);
    assert_eq!(callers.len(), 1);
    assert_eq!(callers[0].va, 0x5040_0040);
    assert_eq!(callers[0].mnemonic, "call32");

    let loads = find_fc_immediate_loads(&buf, base, 0x5040_0018, 0x5040_0030);
    assert_eq!(loads.len(), 1);
    assert_eq!(loads[0].c_string.as_deref(), Some("BGO OK\n"));

    let ptrs = find_pointer_xrefs(&buf, base, base + 0x60);
    assert_eq!(ptrs, vec![0x5040_0022]);
}

#[test]
fn test_call_frames_bcc_and_setlb_loops() {
    let base = 0x5700_0000u32;
    let mut buf = vec![0xCBu8; 0x80];
    // call16 at +0x40 -> +0x10 (disp = -0x30), regs=0x98 (d2,a3,other), frame=0x24
    buf[0x40] = 0xCD;
    buf[0x41..0x43].copy_from_slice(&(-0x30i16).to_le_bytes());
    buf[0x43] = 0x98;
    buf[0x44] = 0x24;
    // beq +0x08 at +0x50 -> +0x58
    buf[0x50] = 0xC8;
    buf[0x51] = 0x08;
    // SETLB at +0x60, 4 bytes body (NOPs), LNE at +0x65
    buf[0x60] = 0xDB;
    buf[0x65] = 0xD9;

    let frames = decode_call_frames(&buf, base, None, None, 0x1000);
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].target_va, base + 0x10);
    assert_eq!(frames[0].frame_bytes, 0x24);
    assert_eq!(frames[0].saved_groups, vec!["d2", "a3", "other"]);

    let bccs = decode_bcc_refs(&buf, base, Some(base + 0x4E), Some(base + 0x56));
    assert_eq!(bccs.len(), 1);
    assert_eq!(bccs[0].mnemonic, "beq");
    assert_eq!(bccs[0].target_va, base + 0x58);

    let loops = decode_setlb_loops(&buf, base, None, None, 32);
    assert_eq!(loops.len(), 1);
    assert_eq!(loops[0].setlb_va, base + 0x60);
    assert_eq!(loops[0].loop_body_va, base + 0x61);
    assert_eq!(loops[0].lcc_va, base + 0x65);
    assert_eq!(loops[0].mnemonic, "lne");
}
