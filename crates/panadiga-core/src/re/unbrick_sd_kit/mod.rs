//! SD Recovery Kit Generator (`DIGADIAG.FRM` + `PANA__SD.FRM`) and Media Validator.
//!
//! Splits pure media verification (`verify`), pure artifact composition (`compose`),
//! and thin disk I/O (`io`).

pub mod compose;
pub mod io;
pub mod verify;

pub use compose::{SdUnbrickKitReport, SdUnbrickKitRequest};
pub use io::{prepare_sd_unbrick_kit, verify_sd_unbrick_media};
pub use verify::{SdMediaVerificationReport, SdUnbrickCheck, verify_sd_buffers};
