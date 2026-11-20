#![deny(unsafe_code)]

//! Matsushita AM34 `DAVIS` IDE (`CH0`/`CH1`) SDRAM Live Probe & `IDENTIFY DEVICE (0xEC)` Engine.

pub mod ata_identify;
pub mod engine;
pub mod stub_builder;

pub use ata_identify::{
    ATA_IDENTIFY_BYTES, AtaIdentifyInfo, DAVIS_IDENTIFY_PIO_BYTES,
    extract_ata_identify_512_from_pio_1024, parse_ata_identify_pio_buffer,
    parse_ata_identify_words,
};
pub use engine::{
    DavisHangClassification, DavisProbeConfig, DavisProbeEngine, DavisProbeReport,
    DavisTaskfileSnapshot, classify_davis_ch1, validate_safe_mmio_read_range,
};
pub use stub_builder::{
    CompiledProbeStub, DAVIS_CH0_CONTROL_REG_VA, DAVIS_CH1_CONNECT_IDE_VA,
    DAVIS_CH1_CONTROL_REG_VA, DAVIS_CH1_RESET_REG_VA, DAVIS_CH1_STATE_STRUCT_VA,
    DAVIS_FE_TIMER_IRQ_INIT_VAS, DAVIS_IDENTIFY_BUF_DEFAULT_VA, DAVIS_IDENTIFY_META_DEFAULT_VA,
    DAVIS_IDENTIFY_SENTINEL, DAVIS_STUB_DEFAULT_VA, build_ata_identify_ec_stub,
    build_ch0_probe_stub, build_ch1_call_stub, build_ch1_probe_stub,
    build_ch1_scratchpad_probe_stub, build_fe_release_stub, build_mmio_write_stub,
};
