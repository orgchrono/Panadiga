#![deny(unsafe_code)]

//! Clean-room Matsushita AM34 / UniPhier Time-Base Corrector (`AIOF`, `AVDD`, `IIC0/1`, `IC7501`)
//! and System Control (`WDBC`, `SYS_STATUS`) memory-mapped register SSOT.

/// `AIOF` Video Output / TBC Control Register Block Base (`0x8800_0000`).
pub const AM34_AIOF_BASE: u32 = 0x8800_0000;
/// `AIOF` Video Mode Select register offset (`+0x00`).
pub const AIOF_VMODESEL_OFF: u32 = 0x00;
/// `AIOF_VMODESEL` value for `NTSC` (`0x8020`).
pub const AIOF_VMODESEL_NTSC: u16 = 0x8020;
/// `AIOF_VMODESEL` value for Brazilian `PAL-M` (`0x8021`).
pub const AIOF_VMODESEL_PAL_M: u16 = 0x8021;
/// `AIOF_VMODESEL` value for `PAL` (`0x8027`).
pub const AIOF_VMODESEL_PAL: u16 = 0x8027;
/// `AIOF` Frame Synchronizer register 0 offset (`+0x2C`).
pub const AIOF_FS0_OFF: u32 = 0x2C;
/// `AIOF` Y/C separation & chroma register 3 offset (`+0x64`).
pub const AIOF_YC3_OFF: u32 = 0x64;
/// `AIOF` Contrast / luma control register offset (`+0xA4`).
pub const AIOF_CONT_OFF: u32 = 0xA4;

/// `AVDD` Analog Video Decoder / DNR Register Block Base (`0x8980_0000`).
pub const AM34_AVDD_BASE: u32 = 0x8980_0000;

/// AM34 I2C Master Controller 0 MMIO Base (`0xD840_0000`).
pub const AM34_IIC0_BASE: u32 = 0xD840_0000;
/// AM34 I2C Master Controller 1 MMIO Base (`0xD840_1000`).
pub const AM34_IIC1_BASE: u32 = 0xD840_1000;

/// `IC7501` Front-Panel / Timer TBUS Bridge MMIO Base (`0xDC01_0000`).
pub const AM34_IC7501_TBUS_BASE: u32 = 0xDC01_0000;

/// AM34 Watchdog / System Reset Control Register (`0xC000_1000`).
pub const AM34_WDBC_REG: u32 = 0xC000_1000;
/// AM34 System Status / F/E Reset Release Register (`0xC000_1008`, bit 15 `0x8000`).
pub const AM34_SYS_STATUS_REG: u32 = 0xC000_1008;
/// **FORBIDDEN LETHAL REGISTER (`0xC000_100C`)**: Reading this address halts the AM34 bus/WDT permanently.
pub const AM34_FORBIDDEN_LETHAL_REG: u32 = 0xC000_100C;
