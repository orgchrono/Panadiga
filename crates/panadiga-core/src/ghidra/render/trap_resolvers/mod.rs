//! Decomposed Jython 2.7 / Python 3 resolver block for MN103S Traps 1..7 + AM33 Sleigh gaps.

mod helpers;
mod passes;

/// Returns the Jython 2.7 / Python 3 compatible source implementing all MN103S Trap & AM33 resolvers.
pub(crate) fn render_mn103_seven_traps_python() -> String {
    format!("{}\n{}", helpers::PYTHON_HELPERS, passes::PYTHON_PASSES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trap_resolver_block_contains_all_resolvers_and_remains_jython_safe() {
        let src = render_mn103_seven_traps_python();
        for needle in [
            "seed_orphan_tables",
            "annotate_bitops_and_traps",
            "recover_unreachable_and_todo_blocks",
            "resolve_mid_instruction_overlaps",
            "enforce_volatile_mmio_and_cfi_blocks",
            "annotate_setlb_and_frame_traps",
            "resolve_revision_shifts_cp932_and_false_consts",
            "annotate_am33_sleigh_gaps",
            "decode_am33_f9_ea_postinc_load",
            "0x44035EB0",
            "resolve_all_seven_traps",
        ] {
            assert!(src.contains(needle), "Missing {needle}");
        }
        assert!(!src.contains("pathlib"));
        assert!(!src.contains("from typing import"));
    }
}
