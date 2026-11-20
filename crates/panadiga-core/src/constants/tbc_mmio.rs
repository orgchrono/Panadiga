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
/// `AIOF` Y/C separation register 0 offset (`+0x10`).
pub const AIOF_YC0_OFF: u32 = 0x10;
/// `AIOF` Automatic Color Control register 0 offset (`+0x14`).
pub const AIOF_ACC0_OFF: u32 = 0x14;
/// `AIOF` TBC sync slice register 0 offset (`+0x1C`).
pub const AIOF_TBCSS0_OFF: u32 = 0x1C;
/// `AIOF` Time Base Corrector control register offset (`+0x24`).
pub const AIOF_TBC_OFF: u32 = 0x24;
/// `AIOF` Frame Synchronizer register 0 offset (`+0x2C`).
pub const AIOF_FS0_OFF: u32 = 0x2C;
/// `AIOF_FS0` standard signal frame-sync preset (`0x40F0`).
pub const AIOF_FS0_STANDARD: u16 = 0x40F0;
/// `AIOF_FS0` non-standard (LaserDisc/VHS pause/240p) frame-sync preset (`0xC0F0`).
pub const AIOF_FS0_NON_STANDARD: u16 = 0xC0F0;
/// `AIOF` Sync detector status register offset (`+0x30`).
pub const AIOF_SDET_OFF: u32 = 0x30;
/// `AIOF` Video mode detection readback register offset (`+0x38`).
pub const AIOF_VMODDET_OFF: u32 = 0x38;
/// `AIOF` Automatic Gain Control register 1 offset (`+0x46`).
pub const AIOF_AGC1_OFF: u32 = 0x46;
/// `AIOF` Video output selector register offset (`+0x5C`).
pub const AIOF_VOUTSEL_OFF: u32 = 0x5C;
/// `AIOF` Y/C separation & chroma register 3 offset (`+0x64`).
pub const AIOF_YC3_OFF: u32 = 0x64;
/// `AIOF_YC3` bit 5 enabling MVAGC Line TBC (`0x0020`).
pub const AIOF_YC3_MVAGC_LINE_TBC_BIT: u16 = 0x0020;
/// `AIOF` Noise reduction register 0 offset (`+0x78`).
pub const AIOF_NR0_OFF: u32 = 0x78;
/// `AIOF` ACC readback telemetry register offset (`+0x7E`).
pub const AIOF_RDACC_OFF: u32 = 0x7E;
/// `AIOF` Macrovision/copy-guard detector readback register offset (`+0x80`).
pub const AIOF_RDMV_OFF: u32 = 0x80;
/// `AIOF` Frame counter telemetry register offset (`+0x92`).
pub const AIOF_RDFCNT_OFF: u32 = 0x92;
/// `AIOF` Contrast / luma control register offset (`+0xA4`).
pub const AIOF_CONT_OFF: u32 = 0xA4;
/// `AIOF_CONT` lighter black-level setup preset (`0 IRE` NTSC-J, `0x4000`).
pub const AIOF_CONT_SETUP_0IRE_LIGHTER: u16 = 0x4000;
/// `AIOF_CONT` darker black-level setup preset (`7.5 IRE` NTSC-M, `0x8000`).
pub const AIOF_CONT_SETUP_75IRE_DARKER: u16 = 0x8000;

/// `AVDD` Analog Video Decoder / DNR Register Block Base (`0x8980_0000`).
pub const AM34_AVDD_BASE: u32 = 0x8980_0000;
/// `AVDD` VBI control register offset (`+0x03B4`).
pub const AVDD_VBICTL_OFF: u32 = 0x03B4;
/// `AVDD` CGMS copy-generation register 0 offset (`+0x03C4`).
pub const AVDD_CGMS0_OFF: u32 = 0x03C4;
/// `AVDD` Analog copy-protection mode A offset (`+0x03CC`).
pub const AVDD_MVMODEA_OFF: u32 = 0x03CC;
/// `AVDD` Analog copy-protection mode B offset (`+0x03FC`).
pub const AVDD_MVMODEB_OFF: u32 = 0x03FC;
/// `AVDD` Video DAC output control 1 offset (`+0x0404`).
pub const AVDD_DACOUT1_OFF: u32 = 0x0404;
/// `AVDD` Video output mode register 0 offset (`+0x0484`).
pub const AVDD_VOUTMODE0_OFF: u32 = 0x0484;
/// `AVDD` BT.656 input configuration offset (`+0x050C`).
pub const AVDD_REC656IN_OFF: u32 = 0x050C;
/// `AVDD` NTSC/PAL encoder control 0 offset (`+0x0624`).
pub const AVDD_NPALCTL0_OFF: u32 = 0x0624;
/// `AVDD` 3DNR parameter bank start offset (`+0x0754`).
pub const AVDD_DNRPARAM00_OFF: u32 = 0x0754;
/// `AVDD` 3DNR parameter commit/valid register offset (`+0x077C`).
pub const AVDD_DNRPARAMVALID_OFF: u32 = 0x077C;
/// `AVDD` Video decoder control 1 offset (`+0x0858`).
pub const AVDD_VDECCTL1_OFF: u32 = 0x0858;

/// AM34 I2C Master Controller 0 MMIO Base (`0xD840_0000`).
pub const AM34_IIC0_BASE: u32 = 0xD840_0000;
/// AM34 I2C Master Controller 1 MMIO Base (`0xD840_1000`).
pub const AM34_IIC1_BASE: u32 = 0xD840_1000;

/// `IC7501` Front-Panel / Timer TBUS Bridge MMIO Base (`0xDC01_0000`).
pub const AM34_IC7501_TBUS_BASE: u32 = 0xDC01_0000;
/// `IC7501` TBUS frame checksum byte offset (`+0x10`, 15-byte packet).
pub const AM34_IC7501_TBUS_CHECKSUM_OFF: u32 = 0x10;
/// `IC7501` TBUS checksum bias subtracted from sum (`sum - 0x5B`).
pub const IC7501_TBUS_CHECKSUM_BIAS: u8 = 0x5B;
/// `IC7501` TBUS 255-second hung-up watchdog keepalive opcode (`0x8A`).
pub const IC7501_WDT_KEEPALIVE_CMD: u8 = 0x8A;

/// AM34 Watchdog / System Reset Control Register (`0xC000_1000`).
pub const AM34_WDBC_REG: u32 = 0xC000_1000;
/// AM34 System Status / F/E Reset Release Register (`0xC000_1008`, bit 15 `0x8000`).
pub const AM34_SYS_STATUS_REG: u32 = 0xC000_1008;
/// **FORBIDDEN LETHAL REGISTER (`0xC000_100C`)**: Reading this address halts the AM34 bus/WDT permanently.
pub const AM34_FORBIDDEN_LETHAL_REG: u32 = 0xC000_100C;
