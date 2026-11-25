# DigaOS (`AM34 / MN103S` Bare-Metal Target OS & Multi-SoC Interop Prototype)

> **Legal & Provenance Notice (`NOT LEGALLY CLEAN-ROOM`):**
> `firmware/digaos/` resides in the same repository alongside hardware forensics and
> disassembled firmware symbol maps. Under clean-room separation doctrine, this codebase is an
> **open-source hardware-interoperability & replacement OS implementation (NOT legally clean-room)**
> written from scratch in ISO C11 + GNU assembly with **zero copyrighted Matsushita binary blobs**.
> This codebase also absorbs the standalone `DIAG_PROG_RAM` TBC feasibility monitor previously
> hosted under the retired bare-metal prototype (`probe_tbc_aiof_baseline`,
> `DIGAOS_TV_SECAM`, and `SW_GUARDS`).

---

## Architecture & SDRAM Boot Model (`SA0..SA10` IPL Permanently Untouched)

`DigaOS` replaces proprietary `PiE-OS` strictly inside **volatile SDRAM** via
`Loader ver0.61` `DIAG_PROG_RAM` (`DIGADIAG.FRM` raw `262,144 B` at `0x5040_0000`) and `MAIN_OS_RAM`
(`0x5700_0000`, `16 MiB` SDRAM window):

| Memory Slot | Address Span | Role |
| :--- | :--- | :--- |
| **IPL Read-Only** | `0x8000_0000..0x8003_FFFF` / `0x8400_0000..0x8403_FFFF` (`SA0..SA10`) | **NEVER modified (`E05`)**; preserves stock `P7501` & SD recovery |
| **`DIAG_PROG_RAM`** | `0x5040_0000..0x5047_FFFF` (`512 KiB`) | `crt0_am34.S` -> `digaos_kernel_entry()` early bring-up & OS chainloader |
| **`MAIN_OS_RAM`** | `0x5700_0000..0x57FF_FFFF` (`16 MiB`) | Declarative SSOT drivers, filesystem, video TBC, and interactive `DIGAOS>` shell |

---

## Non-Negotiable Hardware Invariants

1. **Lethal MMIO Barrier:** Never read or write **`0xC000_100C..0xC000_100F`** (hard bus/WDT hang)
   or **`0xDC0F_0000..0xDC0F_0FFF`** (`SYSCON` danger aperture). Enforced in `hal/am34_mmio.h`.
2. **Permanent IPL Lockout:** Any write into CS0/CS1 `SA0..SA10` (`256 KiB`) is blocked by
   `am34_mmio_is_forbidden()` / `am34_mmio_write_allowed()`.
3. **`IC7501` Front-Panel TBUS Keepalive:** `0xDC01_0000..0xDC01_001F` transmits the `255 s`
   heartbeat frame (`[0x8A, 0x02, 0x00, 0xFF, ..., 0x30]`) before ~44 s elapses so standby MCU
   `IC7501` does not pull down the main SMPS rails.
4. **SATA Bridge Post-Reset Margin:** `DAVIS` CH1 (`0x8900_0C00`) uses an active-HIGH reset and
   waits `1000 ms` (`DIGAOS_SATA_BRIDGE_WAIT_MS`) before `BSY`/`DRDY` polling behind `IDE2SAT2`.
5. **Multi-SoC Fail-Closed HAL (`UNVERIFIED_MMIO`):** Executable bring-up runs on verified
   primary `mn10300_am34` (`MN2DS0009` / `DMR-EH55`). Extensible schema targets (`mn10300_am33_2`,
   `arm_cortex_a9_uniphier`, `arm64_cortex_a53_uniphier`, `sparc_v8_domino`) compile cleanly into
   `hal/digaos_soc_stubs.c` and fail closed with `UNVERIFIED_MMIO` until verified on hardware.
6. **Declarative SSOT (`profiles/digaos/default_digaos_config.json`):** Menus, RGB565 themes,
   hardware-power presets, and multi-SoC targets are projected into `generated/digaos_ssot.{h,c}`
   via `panadiga ssot export-all`.

---

## Interactive `DIGAOS>` Shell (`kernel/digaos_main.c`)

Driven by non-blocking `SC0` RX polling (`SC0STR 0xD400_200C & 0x0010` -> `SC0RXB 0xD400_2009`) in `digaos_shell_poll_once()`:

- **Core & SSOT:** `help`, `status`, `wdt`, `sata`, `vfd <7ch>`, `ssot`, `menus`, `themes`, `power`, `targets`
- **Read-Only Hardware Triage:** `sd`, `bgo`
- **Video & TBC:** `ntsc`, `palm`, `pal`, `secam`, `tbc on`, `tbc off`, `video br`, `video ec`, `video probe`, `dac on`, `dac off`, `colorbar on`, `colorbar off`
- **Full-Storage Auto-Provisioner:** `provision`
