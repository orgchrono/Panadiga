> 🌐 **Language / Idioma:** **English** | [Português (BR)](../pt-BR/HISTORICO_UNBRICK_EH55_BGO_CH1_DAVIS_E_REGIAO.md)
> 🧭 **Portal:** [Docs Index](../INDEX.md) | [Complete Audit](../../COMPLETE_AUDIT.md) | [Ghidra Pseudocode](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Debug Logs](../debug_logs/README.md)

# Historical & Forensic EH55 Dossier: BGO Recovery (`SA39`/`SA54`), Direct `PANAMON1>`, `DAVIS` Polarity, `P58001` / `IDE2SAT2` Pinout, Diskless Routes & `PAL-M` Region

> **Scope & Mandatory Epistemic Rule:** This dossier consolidates the chronological timeline and physical/static evidence captured live on the **Panasonic DMR-EH55 (`VEP79138`)** `COM10` bench, the Windows host (`\\.\PhysicalDrive1`), and MN103S disassembly. To prevent any overclaiming, **every technical claim explicitly carries one of the 4 canonical maturity badges**:
> - **`[PROVEN ON REAL HARDWARE]`** — Measured directly on the `VEP79138` board, `COM10` UART logs, DC multimeter, or `PhysicalDrive1` SHA-256 read-back.
> - **`[PROVEN IN Mn103Vm EMULATOR]`** — MN103S code / Spansion FSM deterministically verified in `panadiga-core::re::mn10300_emu`.
> - **`[GHIDRA STATIC EVIDENCE / STAGE 0]`** — Decompilation, addresses, data layouts, and clean-room plans not yet executed end-to-end on target silicon.
> - **`[UNDER INVESTIGATION / CH1 PHYSICAL BLOCK]`** — Physically blocked by the active electrical fault along `P58001`–`VEK0K00`–`N5EZZ0000003`–`StarTech IDE2SAT2`; **`MAIN` has NOT yet booted from HDD on real EH55 hardware**.

---

## 1. Master Maturity Matrix & Current Bench Reality

| Subsystem / Finding | Maturity Badge | Verified Evidence |
| :--- | :--- | :--- |
| **BGO `U99` cure in `Loader ver0.61` (Cycle 1 + Cycle 2 `SA54/SA41/SA29/SA39`)** | `[PROVEN ON REAL HARDWARE]` | [`verify_bgo_restored_boot_20261119.log`](../debug_logs/verify_bgo_restored_boot_20261119.log) · [`repair_bgo_sa39_sa54_com10_20261119.log`](../debug_logs/repair_bgo_sa39_sa54_com10_20261119.log) |
| **Direct `PANAMON1>` entry without JIG via `PANA__SD.FRM` (`cdcc2cdf...`, `0x214E681`)** | `[PROVEN ON REAL HARDWARE]` | [`probe_ch1_full_diag_20261119.log`](../debug_logs/probe_ch1_full_diag_20261119.log) · [`probe_ch1_live_v2_0deg_20261119.log`](../debug_logs/probe_ch1_live_v2_0deg_20261119.log) |
| **Asymmetric `DAVIS` reset polarity (`CH0` active-LOW vs `CH1` active-HIGH) + `CH0` `0x50`** | `[PROVEN ON REAL HARDWARE]` | [`probe_ch1_full_diag_20261119.log`](../debug_logs/probe_ch1_full_diag_20261119.log) (`J 504156AD` $\rightarrow$ `0x504F81D0=2`, `0x504F81D4=1`) |
| **Reversed `P58001` pinout (`Pin k = ATA (41 - k)`) & `IDE2SAT2` `180°` (`0x00002004`) vs `0°` (`0x00000000`) proof** | `[PROVEN ON REAL HARDWARE]` | `vol2_schematics.pdf` p. 113 + [`probe_ch1_full_diag_20261119.log`](../debug_logs/probe_ch1_full_diag_20261119.log) + [`probe_ch1_live_v2_0deg_20261119.log`](../debug_logs/probe_ch1_live_v2_0deg_20261119.log) |
| **`PhysicalDrive1` CoW write & SHA-256 read-back (`250 GB` + high-LBA `488266096` mirror)** | `[PROVEN ON REAL HARDWARE]` | [`hdd_unbrick_flash_250gb_20261119.log`](../debug_logs/hdd_unbrick_flash_250gb_20261119.log) (`4fde753c...` + `d932692c...`) |
| **`ifodev2` extraction (`2,048 B`, `"M6930LB"`, `f4f22dd0...`) from repaired NOR** | `[PROVEN ON REAL HARDWARE]` | Extracted from `eh55_nor_8mb_ic5001_repaired.bin` (FAT chain `[0x0001, 0x0002, 0x0055, 0x0004]`) |
| **`sd_unbrick_kit`, Spansion FSM, `davis_probe` stubs, `eh55_bgo::repair_plan`, `tv_standard`, `capacity_patch` (`5 GB..2 TB`)** | `[PROVEN IN Mn103Vm EMULATOR]` | Rust test suites in `crates/panadiga-core/` (`mn10300_emu` with `0xFC 0xE0..0xE3` and `0xEC` stubs) |
| **`ES15/ES10/ES35V` incompatibility, `0x84040000`/`PROG` `0x0B1800`/`VDM_Init` routes, `FRONT(L)`/`KEYIN1..3`/`IR7501` topology & `AIOF`/`AVDD` (`PAL-M`) plans** | `[GHIDRA STATIC EVIDENCE / STAGE 0]` | `vol1`/`vol2`/NA schematics + `Loader ver0.61` (`Ldr061_v2`) & `MAIN3410PQ` disassembly |
| **Live ATA read on `CH1` (`0x00000000`, `is_ata=0`), real `250 GB` HDD `MAIN` boot, `MAIN` bypasses & post-`MAIN` `PAL-M`/`TBC`** | `[UNDER INVESTIGATION / CH1 PHYSICAL BLOCK]` | `CH1` multimeter readings (`IORDY`/`DIOR-`/`DIOW- = 0V` when cabled) halt `Loader` prior to Partition 1 load |

---

## 2. Forensic Timeline: From `U99` to Two-Cycle BGO Recovery (`SA39`/`SA54`)

### 2.1 Cycle 1 — Initial Cure of the 45-Sector BGO FTL (`0x440337A4`) `[PROVEN ON REAL HARDWARE]`

Using the gold capture [`eh55_nor_8mb_ic5001_dump.bin`](../../firmware_and_images/README.md) (`8,388,608 B`, `SHA-256: c1faee798f60545853eefdba0d15f23678b471466dc2bc84f522bc972f15a02d`), `Loader ver0.61 [06/03/06]` disassembly proved that the DMR-EH55 uses configuration table **`0x440337A4`** defining **`45` (`0x2D`) 64 KiB sectors across `0x84130000..0x843FFFFF` (`SA26..SA70`)** — distinct from the 35-sector `0x84200000..0x843FFFFF` window hardcoded in `PROG061` (`0x50436E30`) / `Loader ver0.63`.

In Cycle 1 (`apply_bgo_repair_com10.log`), via `PANAMON1>` (`U CONFIRM_UPPER_NOR` $\rightarrow$ `U LOCK`, keeping **`SA0..SA3` strictly read-only `E05`**):
1. Formatted `SA70` (`0x843F0000`) with `256` zero bytes at `0x843F0100`;
2. Executed **`1,629` pure `1 -> 0` byte writes** across `13` spare sectors (`SA42..SA66`: `SA42, 43, 44, 45, 46, 51, 56, 58, 59, 61, 64, 65, 66`), restoring the `13` missing `0xF2xx` sectors (`0xF209`, `0xF21D..0xF228`), block 2 of `ifodev2` (`0x0055` @ `0x84250800`), and FAT pages `0x0F0A` / `0x0F19` (`0x8427A200` / `0x8427AE00`);
3. Executed **`6` pure `1 -> 0` writes at `SA41 + 0x14`** (`0x84220014..0x84220023`), restoring the `0x0004` $\rightarrow$ `SA29` migration link and matching offline image [`eh55_nor_8mb_ic5001_repaired.bin`](../../firmware_and_images/README.md) (`SHA-256: adb21d955ed96121249e7abeec58e1ce3951f511a595636d70ebb07a35ca4c9c`, `19/19` `[nfrom]` files, `996/996` L2P entries).

### 2.2 `[nfrom] file_no[17]` Stock-FVU Relapse & Cycle 2 (`SA54`/`SA41`/`SA29`/`SA39`) `[PROVEN ON REAL HARDWARE]`

When `CH1` subsequently failed on `Boot 2` (`[RTSD] HDD spin-up error (80000404)` / `IDE HOST ERROR (E8000000)` $\rightarrow$ `HDD_MAX_LBA get failed[0xE8000000].` $\rightarrow$ `mainsoft data is invalid, so set update flag.`), the bootloader mounted a legacy SD card carrying stock **`Fvu ver0.61 [05/11/01]`** (`checksum 0x2182DC9`, legacy 35-sector unpatched `PROG061`). Running against `0x84200000..0x843FFFFF`, **`[nfrom] file_no[17],skip(0),non_skip(1),erase(0),new_write(0).` erased `SA54` (`0x842F0000`)** and broke the BGO tree again.

**Surgical Cycle 2** (`repair_bgo_sa39_sa54_com10.py`) restored the BGO over `PANAMON1>` (`BRIDGE 115200 PASSIVE`) using:
* **Safe `?\r` ping** (avoiding bare `\r` which desynchronized the monitor line parser) and **single automatic retry with `300 µs` per-byte pacing** (`~183 B/s`, ~15× faster than previous byte-by-byte pacing);
* **Explicit `U CONFIRM_UPPER_NOR` + immediate `U LOCK`** (preserving permanent `E05` RO on `SA0..SA3` with 100% post-lock read-back verification):
  1. **`SA54` (`0x842F0000`):** `1 -> 0` restoration of header (`+0x000..+0x0FF`), tags (`+0x200..+0x3FF`, block ID `0xF20D`), and **21 active slots** (`1, 25..30, 71..77, 118..124`) matching `eh55_nor_8mb_ic5001_repaired.bin` (`adb21d95...`).
  2. **`SA41` (`0x84220000`, slot 9):** copied `live_sa39_slot8_0054.bin` (`512 B` @ `0x84221600`) + tag `0x54 0x00` (`LSN 0x0054`) at `0x84220224`.
  3. **`SA29` (`0x84160000`, L2P):** promoted page `0x51` (`tid=0x0005`, `t2=0x013F`, `entry[4]=0x0A10`) to page `0x64` slot `s=6` (`0x841664DC`, `t2=0x0140`, `entry[4]=0x0A12`), monotonic `1 -> 0` transition `0xC0C0C0FD -> 0x808080FD` at `0x84166400..6402`, and invalidated page `0x51` at `0x84165103` (`0xD3 -> 0xD2`, yielding `0x808080D2`).
  4. **`SA39` (`0x84200000`):** `E 84200000` (`OK:ERASE`), `0xFF` verification polling, and spare format (`256` zero bytes @ `0x84200100..0x842001FF`).

**Post-Cycle-2 Normal Boot Proof ([`verify_bgo_restored_boot_20261119.log`](../debug_logs/verify_bgo_restored_boot_20261119.log) & [`repair_bgo_sa39_sa54_com10_20261119.log`](../debug_logs/repair_bgo_sa39_sa54_com10_20261119.log)):**
Under `BOOTTRACE 15000 NOJIG`, `Loader ver0.61 [06/03/06]` executed `DDR CHECK: OK` $\rightarrow$ `***** normal mode *****` $\rightarrow$ `*** RTSC Initialize ***` **without `BGO driver init failed[1].` in `Loader`** (which only recurred later in the fallback stage when `CH1` failure loaded stock `Fvu` `0x2182DC9`, underscoring why the canonical kit below is mandatory).

---

## 3. Direct `PANAMON1>` Entry Without JIG (`PANA__SD.FRM` `cdcc2cdf...`) `[PROVEN ON REAL HARDWARE]`

In [`probe_ch1_full_diag_20261119.log`](../debug_logs/probe_ch1_full_diag_20261119.log) (`VDD: 3.26V`) and [`probe_ch1_live_v2_0deg_20261119.log`](../debug_logs/probe_ch1_live_v2_0deg_20261119.log) (`VDD: 3.25V`), using the canonical kit [`firmware_and_images/sd_unbrick_kit/PANA__SD.FRM`](../../firmware_and_images/sd_unbrick_kit/SHA256SUMS.txt):
* **Canonical `PANA__SD.FRM` (`sd48`, `7,181,224 B`, `SHA-256: cdcc2cdf4617c4f08d8af3f617db369cfd360d8db2601b33c84bae0e6f995d90`):** packs **6 x 48-byte TOC entries** aliasing the 3 payloads across both `M6930LB` (slots `0..2`) and `M6930` (slots `3..5`, `[0x18]=0x4D`), embedding `UART_MONITOR_PANAMON1` `PROG061` (**real checksum `0x214E681`**).
* When `Loader ver0.61` hits `HDD_MAX_LBA get failed[0xE8000000]` in `normal mode` (`NOJIG` / `NORMAL`), it mounts SD (`0x3c4000[LBA]`), reads `0xB1800` $\rightarrow$ `0x90400000` (`cheking for fvu program check-sum[0x214E681].`), and drops **straight into `PANAMON1>[B/R/W/D/F/J/E/U/Q]` without asserting `SW_UART` (JIG)**, enabling live RAM and `DAVIS` register probing!
* **Proven hazard of stock `Fvu` (`0x2182DC9`):** leaving unpatched `Fvu ver0.61 [05/11/01]` (`0x2182DC9`) on the SD card in normal mode re-triggers `BGO driver init failed[1].` and destructive `[nfrom] file_no[17]` `SA54` erasure.

---

## 4. `DAVIS` IDE Probe, Reversed `P58001` Pinout (`41 - k`) & `IDE2SAT2` (`0°` vs `180°`) Proof

### 4.1 Asymmetric `DAVIS` Reset Polarity `[PROVEN ON REAL HARDWARE]`

Executing `J 504156AD` (`rtsd_hw_init`) in [`probe_ch1_full_diag_20261119.log`](../debug_logs/probe_ch1_full_diag_20261119.log) confirmed:
* `DAVIS_TOP` (`0x89000000`) = `0x5963464C`.
* **`0x504F81D0 = 0x00000002` (`CH0` ODD active-LOW reset):** released via `0x89000420 = 0x02`/`0x03`. Positive control **`1C CH0`** passed live (`STATUS=0x50`, `ALTSTAT=0x50`, `DEVSEL=0xA0`, `SECCNT=0x03`, `LBAMID=0xFF`, `LBAHI=0xFF`, `DATA=0xFFFF`, `SENTINEL=0xCAFE0001`).
* **`0x504F81D4 = 0x00000001` (`CH1` HDD active-HIGH reset):** released via `0x89000C20 = 0x00`, matching an external inverting stage on `CH1` `RESET-`.
* **Lethal register guard:** `DavisProbeEngine` deterministically rejects reads targeting **`0xC000_100C`** (`AM34_FORBIDDEN_LETHAL_REG`, bus/WDT lockup observed in early probes; likewise `0xDC0F0000` stalled in `probe_davis_ide_ch1.log`).

### 4.2 Reversed `P58001` Pinout, `VEK0K00` / `N5EZZ0000003` Bridge & `0°` vs `180°` Proof

* **`vol2_schematics.pdf` p. 113 (`P58001` on Digital PCB) `[PROVEN ON REAL HARDWARE]`:** geometrically proves the 40-pin `P58001` FFC header follows the exact reversal law:
  $$\text{Pin}_{P58001}(k) \equiv \text{ATA}(41 - k)$$
  Directly verified pins: `Pin 1=GND`, `Pin 3=CS1-`, `Pin 4=CS0-`, `Pin 13=CSEL`, `Pin 14=IORDY`, `Pin 16=DIOR-`, `Pin 18=DIOW-`, `Pin 21=NC` (ATA Pin 20 key), `Pin 27=DD13`, `Pin 28=DD2`, `Pin 39=GND`, `Pin 40=RESET-`. FFC ribbon `VEK0K00` connects `P58001` to adapter board **`N5EZZ0000003`** (`item 13`, 40 continuous pins verified; only the `StarTech IDE2SAT2` male 40-pin header has pin 20 keyed).
* **Empirical `IDE2SAT2` `180°` (`0x00002004`) vs `0°` (`0x00000000`) proof `[PROVEN ON REAL HARDWARE]`:**
  * Rotating `StarTech IDE2SAT2` to **`180°`** (`probe_ch1_full_diag_20261119.log`, `2A MASTER`/`SLAVE`) returned **`0x00002004`** across all 9 `CH1` ATA registers = **`DD13` (`0x2000`, ATA 14)** sampling `ATA 27 (IORDY = 3.3V)` + **`DD2` (`0x0004`, ATA 13)** sampling `ATA 28 (CSEL = 3.3V)`! This **mathematically proves** Type B / `180°` flipped-contact equivalence and establishes **`0°` as the true physical orientation**.
  * At **`0°`** ([`probe_ch1_live_v2_0deg_20261119.log`](../debug_logs/probe_ch1_live_v2_0deg_20261119.log)), `CH1` stopped hanging the CPU on `R 89000D58,4`, yet returned **`0x00000000`** across `2A..2C` and `FUN_5041CD4D(1)` (`rc=0x00000000`, `struct_504F8234`: `is_init=1, is_atapi=0, is_ata=0`).
* **Active Electrical Isolation (`[UNDER INVESTIGATION / CH1 PHYSICAL BLOCK]`):** DC multimeter measurements between `P58001` and `IDE2SAT2` recorded `RESET- = 2.43V`, `CS0- = 3.02V`, `CS1- = 3.23V`, while **`IORDY`/`DIOR-`/`DIOW-` collapsed to `0V` when cabled** (rising to `IORDY = 3.29V` and `0..0.2V` on digital lines with FFC disconnected). **Consequence:** this physical continuity/short defect prevents `CH1` from executing `IDENTIFY (0xEC)`, so **`MAIN` has NOT yet booted from HDD on physical EH55 hardware**.

---

## 5. `250 GB` Preparation on `PhysicalDrive1` (`hdd_unbrick_flash_250gb_20261119.log`)

* **`[PROVEN ON REAL HARDWARE]` (Host Layer):** In [`hdd_unbrick_flash_250gb_20261119.log`](../debug_logs/hdd_unbrick_flash_250gb_20261119.log), `\\.\PhysicalDrive1` (`250,059,350,016 B`, `BusType=7`, `488,397,168 LBAs`) underwent:
  1. Pre-write CoW backup of `65,536 B` (`354d38bd...`), preserving the Windows x86 MBR `55aa` (`3db0e904...`) at `LBA 0` while confirming existing `LBA 24..31`, `LBA 16384`, `LBA 57344`, and `LBA 131072` matched `hdd_100.bin`;
  2. `IOCTL_DISK_DELETE_DRIVE_LAYOUT` + SHA-256 read-back verified flashing of `panasonic_dmr_eh55_hdd_250gb_patched.img` (`104,857,600 B`, `SHA-256: 4fde753c1f4aefb8847621e3bef16dcb9493dc8ef4c52a230f3bb8dfcd34c1c2`, `LBA 0[0:16]=0000000001005348628c6ef51b2c3666`, `LBA 0[16:512]` 100% zeros);
  3. Aligned Partition 1 mirror write and SHA-256 read-back (`6,173,696 B`, `SHA-256: d932692c2daca4af9fd86c030341c213774a6477d7ca3305add9122100f265ed`) at high **`LBA 488266096`** (`((disk_bytes / 512) & !3) - 0x20000`).
* **`[UNDER INVESTIGATION / CH1 PHYSICAL BLOCK]` (Target EH55 Layer):** Real `AM34` boot from this `250 GB` image awaits physical repair of the `CH1` link.

---

## 6. `ES15`/`ES10`/`ES35V` vs `EH55` Verdict, Diskless/TBC Routes & Front Panel/IR (`[GHIDRA STATIC EVIDENCE / STAGE 0]`)

1. **Irreconcilable `DMR-ES15`/`ES10`/`ES35V` Firmware Incompatibility on `EH55` `IC5001` (guaranteed brick):**
   * **`ES10` (`4 MiB` `VEP79104A`, `MN2DS0012-H` `AM33`, no `IC7404` EEPROM)**, **`ES15` (`8 MiB` `A-3 863ZES MP-1`, `MN2DS00151DB`, `24LC64` 8 KiB)**, and **`ES35V` (`8 MiB`, `Loader ver0.18 [06/03/13]`, `24C64` 8 KiB)** place XIP `MAIN` across **`0x130000..0x40FFFF`** (destroying EH55 BGO `SA26..SA70`) and LZMA `EXTRHEAD MAIN 3080GR` across `0x430000..0x6D42F0` (EH55 Upper EEMUL `"1284"`).
   * Furthermore, the 88-pin B2B interconnect `P7402 <-> P59001` (`DR12V` pins 14/16, `DR5V` pins 22/24, `X_SW_12V`/`D1R2V` pins 64..74 feeding `IC59501`+`Q59501`), I²C EEPROM (`RFKFM6016KT` / `C3EBFC000053` 2 KiB at `IC7404`/`IC7401`), and `IC7501` `TBUS` Syscon (`C2CBJG000713` EC / `MN67788RA` NA) are incompatible.
2. **Viable Diskless / TBC Routes on Native `DMR-EH55` Without Touching `SA0..SA3`:**
   * **Gate #1 — `PANA__SD.FRM` `PROG` slot (`0x0B1800` $\rightarrow$ `0x90400000`):** validated only by a 32-bit byte-sum (`func_0x4400c172`, no RSA), already reached automatically upon `HDD_MAX_LBA` failure (`FUN_4400877f` $\rightarrow$ `FUN_44008bc5` $\rightarrow$ `FUN_44009703`).
   * **Gate #2 — `Nor-FROM` slot at `0x84040000` (`SA11..SA14`):** `FUN_4400877f` checks BGO for `"fvu.exe"` (`0x504800AF`), copies `0x40000` bytes from `0x84040000` $\rightarrow$ `0x90400000`, and jumps to `0x50400000` without an SD card (pending `"update"` `"ATIHSTAM"` flag `0..9` persistence confirmation in `FUN_44007f0e`).
   * **Diskless `MAIN3410PQ` blockers:** the 3 existing `AV_PASSTHROUGH` patches (`0x5771ABFB`, `0x574F7B8C`, `0x574F7C5C`) do **not** bypass `VDM_Init()` (`0x57323EC3`, `"###### ILLEGAL HDD ######"` / `"HDD NG"` infinite loop @ `0x574F5D37`), `SDM_Init()` / `SDM_Mount("/dev/hda1")` (`FUN_574f5c6a`), or `FUN_574f469d` msgs `1`/`9` (`"Drive Err -> P-OFF!"` / `"HDD Err -> P-OFF!"` @ `0x574F569E`/`0x574F56F8` $\rightarrow$ `func_0x575097d3()`). Consequently, `re::tbc_appliance` separates safe bare-metal **`Stage15DigadiagTbc`** (`<= 256 KiB` via `DIGADIAG.FRM`) from gated **`ExperimentalSdRamChainload`**.
3. **Front Panel (`FRONT(L)`), `KEYIN1..3`, `IR7501` & Standby `SERV` vs `MAIN`:**
   * `FRONT(L) P.C.B.` (`JW7001 <-> JW7501` / `P7001 <-> P7505`) carries **ONLY `[POWER]` (`S7002`)** wired to `KEYIN3` (pin `88` EC / `18` NA). All other 8 tactile keys (`EXT_LINK`, `CH_DOWN`, `CH_UP`, `OPEN/CLOSE`, `SELECT`, `STOP`, `PLAY`, `REC`) and **`IR7501` (`PNA4618M09VT`)** sit directly on the **Main PCB**, powered by `+5V` (`X_SW_5R2V` via `R7648` `47 Ω` + `8.2 kΩ` pull-ups `KEYIN1..3` on pins `86..88` EC / `16..18` NA; NA boards also bridge `P7501` Pin 1 `SW_UART` via `K7502` `0 Ω` to `IC7501` pin `80` `IR_BUSY`, whereas EC leaves pin `80` `NC`).
   * `SERV` (`[CH UP]+[OPEN/CLOSE]+[REC]` 5s) and `TEST` (`[STOP]+[CH UP]+[OPEN/CLOSE]` 5s) are accepted by `IC7501` **only from true Standby (`Power OFF`, `DI_P_ON = 0V`)**; actual `SERV` (`[0][1]..[9][9]`), `R_SERVICEMODE` (`0x57A116CB`), and continuous `TBUS` key/IR polling (`FPC_ReciveTASK` @ `0x57060DDC`) execute inside `EH55_MAIN`, which never starts while `CH1` fails (triggering the `255s` `TBUS` WDT `U99`).

---

## 7. `PAL-M` / `LBS` / `ifodev2`, `capacity_patch` & Rust Centralization

* **`ifodev2` (`2,048 B`, `0x0055`, `SHA-256: f4f22dd0407d9f0bebf4621e938f2ca02e3b6c5f23b21ae972c5185704a77676`):** `[PROVEN ON REAL HARDWARE]` on repaired NOR inspection (`AA55FFFF`, `"M6930LB\0"`, `0x0C='M'`, `"PANA_DVD_REC"`) and extracted as [`eh55_ifodev2_m6930lb_2048b.bin`](../../firmware_and_images/README.md); its unverified trailing half (`0x400..0x7FF`) is explicitly tagged `metadata_and_dev_oem_only = true` inside `re::region_compat::tv_standard` to block silent mutation.
* **`AIOF` / `AVDD` / `IC3001` & Patches (`[GHIDRA STATIC EVIDENCE / STAGE 0]` / `[PROVEN IN Mn103Vm EMULATOR]`):** `AM34_AIOF_BASE = 0x8800_0000` (`AIOF_V +0x66 = 0x8001` `clock_start`, `AIOF_VMODESEL`: `0x8020` NTSC, `0x8021` `cps_pal_m`, `0x8027` PAL; `FS0 +0x2C`, `YC3 +0x64`, `CONT +0xA4`), `AM34_AVDD_BASE = 0x8980_0000` (`NPALCTL0 +0x0624`), `IC3001` A/V switch (`C1AB00002379` EC 7-bit I2C `0x4A`/`0x94` vs `C1AB00001979` NA), `region_compat` (`3410PQ @ 0x57498738`, `3060NP @ 0x574A01AA`), `M0` `keepalive` (`0xDC01_0010`, checksum `0x30`), and `hdd::capacity_patch` spanning **`5 GB` (`0x0024C388` `SV0511D`) through `2 TB` (`DYNAMIC_UNCLAMPED_MAX_BLOCKS_2KB = 0x3FFF_FFFF`, `250 GB = 0x07466528`)**.
* **Centralized Rust Modules (`panadiga-core`):** `pico::panamon1` (`Panamon1Client`, RAII `Panamon1NorUnlockGuard` emitting `U LOCK` on `Drop` + `E05` gate), `re::nor_dumper::{orchestrator, spansion_audit}` (`142` Spansion sectors), `re::eh55_bgo::{audit, repair_plan, live_apply}`, `re::davis_probe` (`DavisProbeEngine` + `0xEC`), `re::region_compat::tv_standard`, `re::tbc_appliance::{self, keepalive}`, and `hdd::writer::eh55_flash` (`64 KiB` CoW backup, `PhysicalDrive0`/`BusTypeNvme` lockout, SHA-256 read-back verification).

---

## 8. Pre-Registered `CH1` (`E8000000`) Bench Diagnostic Runbook — Multimeter + Adapters + Pico `[UNDER INVESTIGATION / CH1 PHYSICAL BLOCK]`

Supported bench kit: **DC/Continuity Multimeter**, **Alternative PATA/SATA/CF/SD Adapters**, and **Raspberry Pi Pico Probe (`COM10` / Door #1 `PANA__SD.FRM` `cdcc2cdf...`)**. See also [`STATUS.md`](../STATUS.md).

| Hypothesis | Deterministic Bench Test | Diagnostic Pass / Fault Criterion |
| :---: | :--- | :--- |
| **`H1`** (`Undervoltage / IDE2SAT2 Clamping`) | Using a DC multimeter at the `StarTech IDE2SAT2` Molex/Berg connector, measure **`DR5V` (`~5.0V`)** and **`DR12V` (`~12.0V`)** during `*** RTSC Initialize ***`; re-test powering the adapter from an external common-ground +5V/+12V PSU prior to powering EH55 and measure `IORDY` (`ATA Pin 27`) / `DIOR-` (`Pin 25`) / `DIOW-` (`Pin 23`). | If `IORDY`/`DIOR-`/`DIOW-` recover from `0V` to `~3.3V` under external PSU, the fault is Main PCB `IC1506` (`DR5V`) brownout/clamping; if `0V` persists across alternative adapters, isolate via `H2`/`H3`. |
| **`H2` / `H3`** (`P58001`–`VEK0K00`–`N5EZZ0000003` vs `IDE2SAT2`) | Remove AC, discharge rails, swap in an alternative adapter from the kit at verified **`0°` (`P58001 Pin k = ATA (41 - k)`)**, and run `panadiga pico davis-probe` / `PANAMON1>` stub on `CH1` (`0x8900_0C00..0C7F`). | Reading `STATUS = 0x50` at `0xE800_005C` and completing `IDENTIFY DEVICE (0xEC)` (`SENTINEL = 0xCAFE00EC`) confirms `CH1` physical link recovery. |
| **`H4`** (`DAVIS` / `SYS_STATUS` Bus State) | Via Door #1 (`PANAMON1>`), inspect `SYS_STATUS` (`0xC000_1008`) and safe read ranges `0x8900_0C00..0C7F` + `CH1_DEVCTL` (`0x8900_0D58`) — **NEVER read `0xC000_100C`**. Optionally capture `IORDY`/`CS0-`/`RESET-` transitions via Pico `PIOLOGIC`. | Distinguishes whether the `DAVIS CH1` controller observes stuck lines even with `N5EZZ0000003` disconnected vs only under adapter load. |
