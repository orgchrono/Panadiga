#![deny(unsafe_code)]

//! Hot Bare-Metal Silicon Explorer (`ndb>` kernel debugger protocol over PicoFW `SC0` bridge).
//!
//! Provides ANSI stripping, little-endian `dd` word decoding, `eb`/`ew` echo parsing,
//! BCU `BCBERR` + PiE-OS task-dump abort detection, fail-closed safety barriers (`STANDBY_GATE_STOP`,
//! lethal MMIO `0xC000100C`/`0xDC0F0000..0xDC0F0FFF`, BCU fault `0xD4004000`, and NOR write
//! blocks), hot-diff comparison, transactional reverse-order rollback, and evidence log replay.

pub mod parser;
pub mod session;

#[cfg(test)]
mod tests;

pub use parser::{
    NdbAccessKind, NdbBcuFault, NdbDdResult, NdbLogReplaySummary, NdbWriteEcho, VolatileDiffEntry,
    detect_ndb_bcu_fault, diff_dd_snapshots, parse_ndb_dd_output, parse_ndb_write_echo,
    replay_ndb_evidence_log, strip_ndb_ansi_prompt, validate_ndb_access,
};
pub use session::{
    NdbSession, NdbTransport, RollbackReport, SerialNdbTransport, VolatileDiffReport,
    VolatilePatchOp, VolatilePatchWidth,
};
