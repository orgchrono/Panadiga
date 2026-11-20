#![deny(unsafe_code)]

//! Live `DAVIS CH0/CH1` & `0xEC` state-machine runner with `0xC000_100C` fail-closed guard.

use std::time::Duration;

use anyhow::{Result, bail};

use super::{
    DavisProbeConfig, DavisProbeReport, DavisTaskfileSnapshot, classify_davis_ch1,
    validate_safe_mmio_read_range,
};
use crate::constants::{
    AM34_DAVIS_MMIO_SIZE, AM34_DAVIS_PRI_MMIO_BASE, AM34_DAVIS_SEC_MMIO_BASE, AM34_SYS_STATUS_REG,
    AM34_WDBC_REG,
};
use crate::pico::panamon1::{Panamon1Client, Panamon1Transport};
use crate::re::davis_probe::ata_identify::parse_ata_identify_pio_buffer;
use crate::re::davis_probe::stub_builder::{
    CompiledProbeStub, DAVIS_CH1_CONNECT_IDE_VA, DAVIS_CH1_RESET_REG_VA, DAVIS_CH1_STATE_STRUCT_VA,
    DAVIS_FE_TIMER_IRQ_INIT_VAS, build_ata_identify_ec_stub, build_ch0_probe_stub,
    build_ch1_call_stub, build_ch1_probe_stub, build_ch1_scratchpad_probe_stub,
    build_fe_release_stub, build_mmio_write_stub,
};

/// Orchestrates live non-destructive `DAVIS CH0/CH1` & `0xEC` diagnostics over `PANAMON1>`.
pub struct DavisProbeEngine<'a, T: Panamon1Transport> {
    client: &'a mut Panamon1Client<T>,
    config: DavisProbeConfig,
}

impl<'a, T: Panamon1Transport> DavisProbeEngine<'a, T> {
    pub fn new(client: &'a mut Panamon1Client<T>, config: DavisProbeConfig) -> Self {
        Self { client, config }
    }

    /// Reads `[addr, addr + len)` only after verifying `AM34_FORBIDDEN_LETHAL_REG` (`0xC000_100C`) is excluded.
    pub fn safe_read_reg(&mut self, addr: u32, len: usize) -> Result<Vec<u8>> {
        validate_safe_mmio_read_range(addr, len)?;
        self.client.mem_read_bytes(addr, len)
    }

    fn upload_verify_and_run(&mut self, stub: &CompiledProbeStub) -> Result<()> {
        if stub.code.len() >= 0x1000 {
            bail!(
                "Probe stub '{}' ({} B) exceeds 4 KiB safety budget before scratch outputs",
                stub.name,
                stub.code.len()
            );
        }
        self.client
            .mem_write_bytes_burst(stub.load_va, &stub.code)?;
        let rb = self.safe_read_reg(stub.load_va, stub.code.len())?;
        if rb != stub.code {
            bail!(
                "SDRAM write-verify mismatch for stub '{}' at 0x{:08X}",
                stub.name,
                stub.load_va
            );
        }
        self.client.jump(stub.load_va)
    }

    fn reset_pulse_and_probe(
        &mut self,
        name: &str,
        assert_val: u32,
        deassert_val: u32,
        out_va: u32,
        sentinel: u32,
    ) -> Result<DavisTaskfileSnapshot> {
        let s_assert = build_mmio_write_stub(
            &format!("{name}_assert"),
            self.config.sdram_stub_va,
            &[(DAVIS_CH1_RESET_REG_VA, assert_val)],
        );
        self.upload_verify_and_run(&s_assert)?;
        self.client.sleep(Duration::from_millis(50));

        let s_rel = build_mmio_write_stub(
            &format!("{name}_release"),
            self.config.sdram_stub_va,
            &[(DAVIS_CH1_RESET_REG_VA, deassert_val)],
        );
        self.upload_verify_and_run(&s_rel)?;
        self.client.sleep(Duration::from_millis(2500));

        let probe = build_ch1_scratchpad_probe_stub(
            name,
            self.config.sdram_stub_va,
            AM34_DAVIS_SEC_MMIO_BASE,
            self.config.drive_sel,
            out_va,
            sentinel,
        )?;
        self.upload_verify_and_run(&probe)?;
        DavisTaskfileSnapshot::from_bytes(&self.safe_read_reg(out_va, 40)?)
    }

    /// Executes Steps A -> 1A -> 1B -> 1C -> 2A..2D -> C (`0xEC IDENTIFY DEVICE`).
    pub fn run_probe(&mut self) -> Result<DavisProbeReport> {
        let mut logs = Vec::new();
        let _ = self.client.ensure_panamon1_bridge(
            self.config.boot_mode.as_deref(),
            self.config.boot_timeout_ms,
        )?;
        logs.push("[Step A] PANAMON1> active over BRIDGE 115200 PASSIVE".to_string());

        // Step 1A: Safe read-only baseline (strictly 0xC000_1000..0xC000_100C, never 0xC000_100C)
        let _ = self.safe_read_reg(AM34_DAVIS_PRI_MMIO_BASE & 0xFFFF_F000, 4)?;
        let _ = self.safe_read_reg(
            AM34_DAVIS_SEC_MMIO_BASE,
            AM34_DAVIS_MMIO_SIZE.min(0x40) as usize,
        )?;
        let rst = self.safe_read_reg(AM34_WDBC_REG, 12)?;
        let c0001008_before = u16::from_le_bytes([rst[8], rst[9]]);
        logs.push(format!("[Step 1A] C0001008_before=0x{c0001008_before:04X}"));

        // Step 1B: F/E release + timer/IRQ/rtsd_hw_init VAs
        if (c0001008_before & 0x8000) == 0 {
            let fe_stub = build_fe_release_stub(self.config.sdram_stub_va);
            self.upload_verify_and_run(&fe_stub)?;
            self.client.sleep(Duration::from_millis(500));
        }
        for va in DAVIS_FE_TIMER_IRQ_INIT_VAS {
            self.client.jump(va)?;
        }
        let post_sys = self.safe_read_reg(AM34_SYS_STATUS_REG, 2)?;
        let c0001008_after = u16::from_le_bytes([post_sys[0], post_sys[1]]);
        let fe_released = (c0001008_after & 0x8000) != 0;
        logs.push(format!(
            "[Step 1B] C0001008_after=0x{c0001008_after:04X} fe_released={fe_released}"
        ));

        // Step 1C: CH0 positive control
        let scratch_base = self.config.sdram_stub_va & 0xFFFF_0000;
        let out_ch0 = scratch_base.wrapping_add(0x1000);
        let s1c = build_ch0_probe_stub(self.config.sdram_stub_va, out_ch0)?;
        self.upload_verify_and_run(&s1c)?;
        let ch0_taskfile = DavisTaskfileSnapshot::from_bytes(&self.safe_read_reg(out_ch0, 40)?)?;
        logs.push(format!(
            "[Step 1C CH0] status=0x{:08X}",
            ch0_taskfile.status
        ));

        // Step 2A: CH1 raw + Master + Slave scratchpad probe
        let out_raw = scratch_base.wrapping_add(0x1100);
        let out_m = scratch_base.wrapping_add(0x1140);
        let out_s = scratch_base.wrapping_add(0x1180);
        let s2a = build_ch1_probe_stub(
            self.config.sdram_stub_va,
            AM34_DAVIS_SEC_MMIO_BASE,
            out_raw,
            out_m,
            out_s,
        )?;
        self.upload_verify_and_run(&s2a)?;
        let ch1_raw = DavisTaskfileSnapshot::from_bytes(&self.safe_read_reg(out_raw, 40)?)?;
        let ch1_master = DavisTaskfileSnapshot::from_bytes(&self.safe_read_reg(out_m, 40)?)?;
        let ch1_slave = DavisTaskfileSnapshot::from_bytes(&self.safe_read_reg(out_s, 40)?)?;
        let mut ata_alive = ch1_raw.is_valid_ata(false)
            || ch1_master.is_valid_ata(true)
            || ch1_slave.is_valid_ata(true);

        // Step 2B/2C: Hard reset polarity pulses if silent
        let mut ch1_2b_reset = None;
        let mut ch1_2c_reset = None;
        for (idx, (name, assert_val, deassert_val, off, sentinel)) in [
            ("ch1_2b_pol1", 2u32, 0u32, 0x11C0u32, 0xCAFE_02B0u32),
            ("ch1_2c_pol2", 0u32, 2u32, 0x1200u32, 0xCAFE_02C0u32),
        ]
        .into_iter()
        .enumerate()
        {
            if ata_alive {
                break;
            }
            let tf = self.reset_pulse_and_probe(
                name,
                assert_val,
                deassert_val,
                scratch_base.wrapping_add(off),
                sentinel,
            )?;
            ata_alive = tf.is_valid_ata(true);
            if idx == 0 {
                ch1_2b_reset = Some(tf);
            } else {
                ch1_2c_reset = Some(tf);
            }
        }

        // Step 2D: Factory FUN_5041CD4D(1)
        self.client
            .mem_write_bytes_burst(DAVIS_CH1_STATE_STRUCT_VA, &[0u8; 4])?;
        let s2d = build_ch1_call_stub(DAVIS_CH1_CONNECT_IDE_VA, 1, self.config.sdram_stub_va);
        self.upload_verify_and_run(&s2d)?;
        let rc_va = s2d
            .meta_va
            .unwrap_or_else(|| scratch_base.wrapping_add(0x12FC));
        let fun_cd4d_rc = self.client.mem_read_u32(rc_va)?;
        let ch1_struct = self.safe_read_reg(DAVIS_CH1_STATE_STRUCT_VA, 0x30)?;
        let ch1_struct_hex = hex::encode_upper(&ch1_struct);
        let is_ata_m = ch1_struct.get(6).copied().unwrap_or(0);
        logs.push(format!(
            "[Step 2D] rc=0x{fun_cd4d_rc:08X} is_ata_m={is_ata_m}"
        ));

        // Step C: IDENTIFY DEVICE (0xEC)
        let mut identify_sentinel = None;
        let mut identify_post_altstat = None;
        let mut identify_info = None;
        if ata_alive || is_ata_m != 0 {
            let ec_stub = build_ata_identify_ec_stub(
                AM34_DAVIS_SEC_MMIO_BASE,
                self.config.drive_sel,
                self.config.identify_out_va,
            )?;
            self.upload_verify_and_run(&ec_stub)?;
            let meta_va = ec_stub.meta_va.unwrap_or(0x9050_13F4);
            let meta = self.safe_read_reg(meta_va, 8)?;
            identify_sentinel = Some(u32::from_le_bytes([meta[0], meta[1], meta[2], meta[3]]));
            identify_post_altstat = Some(u32::from_le_bytes([meta[4], meta[5], meta[6], meta[7]]));
            let pio_1024 = self.safe_read_reg(self.config.identify_out_va, 1024)?;
            identify_info = parse_ata_identify_pio_buffer(&pio_1024).ok();
        }

        let classification =
            classify_davis_ch1(&ch1_raw, &ch1_master, &ch1_slave, identify_info.as_ref());
        logs.push(format!("[Classification] {classification:?}"));

        Ok(DavisProbeReport {
            c0001008_before,
            c0001008_after,
            fe_released,
            ch0_taskfile,
            ch1_raw,
            ch1_master,
            ch1_slave,
            ch1_2b_reset,
            ch1_2c_reset,
            fun_cd4d_rc,
            ch1_struct_hex,
            identify_sentinel,
            identify_post_altstat,
            identify_info,
            classification,
            step_logs: logs,
        })
    }
}
