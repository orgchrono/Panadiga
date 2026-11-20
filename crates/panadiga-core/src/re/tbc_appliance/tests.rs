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
