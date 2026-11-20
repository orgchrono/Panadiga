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
