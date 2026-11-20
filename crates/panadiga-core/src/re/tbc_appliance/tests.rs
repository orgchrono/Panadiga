#![deny(unsafe_code)]

//! Unit tests for clean-room Standalone TBC Appliance planner & safety invariants.

use super::*;

#[test]
fn stage15_tbc_plan_preserves_sa0_to_sa3_and_forbids_lethal_reg() {
    let plan = build_tbc_appliance_plan(
        TbcApplianceMode::Stage15DigadiagTbc,
        TvStandardKind::PalM,
        false,
    )
    .unwrap();
    assert!(plan.nor_sa0_to_sa3_untouched);
    assert!(!plan.requires_experimental_flag);
    assert_eq!(plan.tv_standard, TvStandardKind::PalM);
    assert_eq!(plan.verified_main_passthrough_sites.len(), 3);
    assert!(
        plan.warnings
            .iter()
            .any(|w| w.contains("IC3001") && w.contains("IC7501 TBUS"))
    );
    assert!(
        plan.mmio_ops
            .iter()
            .all(|op| op.addr != AM34_FORBIDDEN_LETHAL_REG)
    );
}

#[test]
fn experimental_sd_ram_chainload_requires_explicit_opt_in() {
    let err = build_tbc_appliance_plan(
        TbcApplianceMode::ExperimentalSdRamChainload,
        TvStandardKind::Ntsc,
        false,
    )
    .unwrap_err();
    assert!(err.to_string().contains("experimental_ram_chainload"));

    let ok = build_tbc_appliance_plan(
        TbcApplianceMode::ExperimentalSdRamChainload,
        TvStandardKind::Ntsc,
        true,
    )
    .unwrap();
    assert!(ok.requires_experimental_flag);
    assert!(ok.nor_sa0_to_sa3_untouched);
    assert_eq!(
        "ram_chainload".parse::<TbcApplianceMode>().unwrap(),
        TbcApplianceMode::ExperimentalSdRamChainload
    );
}

#[test]
fn no_jig_pana_sd_prog_slot_and_stage0_constants_enforce_safety() {
    use crate::constants::{
        AIOF_CONT_SETUP_0IRE_LIGHTER, AIOF_CONT_SETUP_75IRE_DARKER, AIOF_FS0_NON_STANDARD,
        AIOF_FS0_STANDARD, AIOF_YC3_MVAGC_LINE_TBC_BIT, AVDD_DNRPARAM00_OFF, AVDD_MVMODEA_OFF,
        IC7501_TBUS_CHECKSUM_BIAS, IC7501_WDT_KEEPALIVE_CMD,
    };

    assert_eq!(
        "no_jig".parse::<TbcApplianceMode>().unwrap(),
        TbcApplianceMode::NoJigPanaSdProgSlot
    );
    assert_eq!(
        "porta1".parse::<TbcApplianceMode>().unwrap(),
        TbcApplianceMode::NoJigPanaSdProgSlot
    );
    let plan = build_tbc_appliance_plan(
        TbcApplianceMode::NoJigPanaSdProgSlot,
        TvStandardKind::Ntsc,
        false,
    )
    .unwrap();
    assert!(!plan.requires_experimental_flag);
    assert!(plan.nor_sa0_to_sa3_untouched);
    assert!(plan.steps[0].contains("Door #1"));
    assert!(
        plan.mmio_ops
            .iter()
            .all(|op| op.addr != AM34_FORBIDDEN_LETHAL_REG)
    );

    assert_eq!(AIOF_FS0_STANDARD, 0x40F0);
    assert_eq!(AIOF_FS0_NON_STANDARD, 0xC0F0);
    assert_eq!(AIOF_YC3_MVAGC_LINE_TBC_BIT, 0x0020);
    assert_eq!(AIOF_CONT_SETUP_0IRE_LIGHTER, 0x4000);
    assert_eq!(AIOF_CONT_SETUP_75IRE_DARKER, 0x8000);
    assert_eq!(AVDD_MVMODEA_OFF, 0x03CC);
    assert_eq!(AVDD_DNRPARAM00_OFF, 0x0754);
    assert_eq!(IC7501_TBUS_CHECKSUM_BIAS, 0x5B);
    assert_eq!(IC7501_WDT_KEEPALIVE_CMD, 0x8A);
}

#[test]
fn m0_ic7501_tbus_frames_and_stub_execute_cleanly_on_mn103vm() {
    use crate::constants::{
        AIOF_EMPHA_OFF, AIOF_RDACC_OFF, AIOF_RDDET_OFF, AIOF_RDDET1_OFF, AIOF_RDFCNT_OFF,
        AIOF_RDMV_OFF, AIOF_TEST1_DEFAULT, AIOF_TEST1_OFF, AIOF_TEST1_SOFT_RESET_MASK,
        AIOF_V_CLOCK_START, AIOF_V_DDR_STOP, AIOF_V_OFF, AM34_IC7501_TBUS_RX_OFF,
        AM34_IC7501_TBUS_TX_OFF, IC3001_I2C_ADDR_7BIT, IC3001_I2C_ADDR_8BIT_READ,
        IC3001_I2C_ADDR_8BIT_WRITE, IC7501_TBUS_ACK_STROBE_BIT, IC7501_VFD_TEXT_CMD,
        IC7501_WDT_255S_CHECKSUM,
    };
    use crate::re::Mn103Vm;

    assert_eq!(AIOF_RDACC_OFF, 0x80);
    assert_eq!(AIOF_RDMV_OFF, 0x82);
    assert_eq!(AIOF_RDDET_OFF, 0x84);
    assert_eq!(AIOF_RDDET1_OFF, 0x92);
    assert_eq!(AIOF_RDFCNT_OFF, 0x94);
    assert_eq!(AIOF_EMPHA_OFF, 0xB4);
    assert_eq!(AIOF_V_OFF, 0x66);
    assert_eq!(AIOF_V_CLOCK_START, 0x8001);
    assert_eq!(AIOF_V_DDR_STOP, 0x8039);
    assert_eq!(AIOF_TEST1_OFF, 0x6C);
    assert_eq!(AIOF_TEST1_DEFAULT, 0xB040);
    assert_eq!(AIOF_TEST1_SOFT_RESET_MASK, 0x7FFF);
    assert_eq!(IC3001_I2C_ADDR_7BIT, 0x4A);
    assert_eq!(IC3001_I2C_ADDR_8BIT_WRITE, 0x94);
    assert_eq!(IC3001_I2C_ADDR_8BIT_READ, 0x95);
    assert_eq!(AM34_IC7501_TBUS_TX_OFF, 0x10);
    assert_eq!(AM34_IC7501_TBUS_RX_OFF, 0x20);
    assert_eq!(IC7501_TBUS_ACK_STROBE_BIT, 0x10);
    assert_eq!(IC7501_VFD_TEXT_CMD, 0xA3);
    assert_eq!(IC7501_WDT_255S_CHECKSUM, 0x30);

    let wdt = build_ic7501_wdt_255s_frame();
    assert_eq!(
        wdt,
        [
            0x8A, 0x02, 0x00, 0xFF, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x30
        ]
    );
    let body15: [u8; 15] = wdt[..15].try_into().unwrap();
    assert_eq!(compute_ic7501_tbus_checksum(&body15), 0x30);

    let vfd = build_ic7501_vfd_text_frame("TBC ON ");
    assert_eq!(vfd[0], 0xA3);
    assert_eq!(&vfd[3..10], b"TBC ON ");
    let vfd15: [u8; 15] = vfd[..15].try_into().unwrap();
    assert_eq!(vfd[15], compute_ic7501_tbus_checksum(&vfd15));

    let stub = emit_m0_tbus_wdt_keepalive_stub();
    let stub_va = 0x5180_0000u32;
    let stack_va = 0x5181_0000u32;
    let ret_sentinel = 0x5040_BEEFu32;

    let mut vm = Mn103Vm::new();
    vm.map_segment(stub_va, stub, false);
    vm.map_segment(AM34_IC7501_TBUS_BASE, vec![0xFFu8; 0x100], true);
    vm.map_segment(stack_va, vec![0u8; 0x200], true);
    vm.sp = stack_va + 0x100;
    vm.write_u32(vm.sp, ret_sentinel).unwrap();
    vm.pc = stub_va;
    vm.run_until(ret_sentinel, 64).unwrap();

    assert_eq!(vm.d[0], u32::from(IC7501_WDT_255S_CHECKSUM));
    assert_eq!(vm.sp, stack_va + 0x104);
    assert_eq!(vm.read_u8(AM34_IC7501_TBUS_BASE).unwrap(), 0x00);
    let mut observed = [0u8; 16];
    for (idx, slot) in observed.iter_mut().enumerate() {
        *slot = vm
            .read_u8(AM34_IC7501_TBUS_BASE + AM34_IC7501_TBUS_TX_OFF + idx as u32)
            .unwrap();
    }
    assert_eq!(observed, wdt);
}
