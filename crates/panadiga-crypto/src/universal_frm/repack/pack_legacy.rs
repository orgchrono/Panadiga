#![deny(unsafe_code)]

//! Legacy DVD/SD 48B and 64B sub-container serialization via `frm_pack`.

use anyhow::Result;

use super::super::types::UniversalFrmSubContainer;
use super::options::{FrmRepackOptions, rebuild_modified_module_slot};
use crate::constants::{
    PANA_FEISTEL_ROUND_KEY_32B, PANA_FRM_TOC_REGION_SIZE, PANA_SD_TOC_REGION_SIZE,
};
use crate::frm_pack::{FrmContainerMode, FrmPackModule, pack_frm_with_key};

pub(super) fn pack_legacy_subcontainer(
    sub: &UniversalFrmSubContainer,
    mode: FrmContainerMode,
    opts: &FrmRepackOptions,
    modified_flags: &[bool],
) -> Result<Vec<u8>> {
    let mut modules = Vec::with_capacity(sub.modules.len());
    for (idx, mod_info) in sub.modules.iter().enumerate() {
        let slot_bytes = rebuild_modified_module_slot(
            mod_info,
            modified_flags[idx],
            &sub.feistel_round_key,
            opts,
        )?;
        let mut m = FrmPackModule::new(mod_info.name.clone(), slot_bytes);
        m.version_u16 = mod_info.version_u16;
        m.hardware_family = mod_info.hardware_family.clone();
        m.model_prefix = mod_info.oem;
        m.target_flag = mod_info.target_flag;
        m.model_group = mod_info.model_group.clone();
        if mod_info.raw_toc_entry.len() >= 0x2E {
            m.tail_meta
                .copy_from_slice(&mod_info.raw_toc_entry[0x28..0x2E]);
        }
        modules.push(m);
    }
    let region_sz = match mode {
        FrmContainerMode::Dvd64B | FrmContainerMode::Sd => Some(PANA_SD_TOC_REGION_SIZE),
        FrmContainerMode::Dvd | FrmContainerMode::Sd48 => Some(PANA_FRM_TOC_REGION_SIZE),
    };
    let rk = if sub.feistel_round_key == [0u8; 32] {
        PANA_FEISTEL_ROUND_KEY_32B
    } else {
        sub.feistel_round_key
    };
    let (bytes, _rep) = pack_frm_with_key(&modules, mode, region_sz, &rk)?;
    Ok(bytes)
}
