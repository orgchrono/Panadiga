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
| **Live ATA read on `CH1` (`0x00000000`, `is_ata=0`), real `250 GB` HDD `MAIN` boot, `MAIN` bypasses & post-`MAIN` `PAL-M`/`TBC`** | `[UNDER INVESTIGATION]` | Verified DC voltages on `IDE2SAT2` (`Pin 27 IORDY = 3.34V`, `Pins 1/23/25/37/38 = 2.45–2.47V`); `CH1` ATA `0xEC` handshake under refinement |

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

### 2.3 Forensic Anatomy of the BGO Relapse (`HDDERR` $\rightarrow$ `DRV NG` / `STFAIL` $\rightarrow$ `U99`): `+0x14` Rollback (`SA29`), `0xF20A` Compaction (`SA26 -> SA29`) & `SA70` `8×8 KiB` Top-Boot Abort (`0x843FE100`) `[PROVEN ON REAL HARDWARE]`

When the DMR-EH55 booted in `normal mode` after BGO restoration (while `CH1` remained electrically blocked), the front-panel VFD displayed **`HDDERR`** followed by **`DRV NG` / `STFAIL`**, executing multiple `[nfrom]` NOR Flash write transactions before returning permanently to **`U99`** (`BGO driver init failed[1].`). Cross-correlating `Loader ver0.61` / `PROG061` disassembly with live `COM10` read-backs across `SA26..SA70` proved the exact 4-link causal chain:

1. **VFD State Progression and SD-less `normal mode` BGO Write Calls (`Loader ver0.61`) `[GHIDRA STATIC EVIDENCE / STAGE 0]`:**
   * **`FUN_44007f0e` (`0x44007f0e..0x44008298`):** on startup, if `[nfrom]` file `"update"` (`0x5048009D`, `file_no[17]`, `"ATIHSTAM"`) is partially active (`FUN_44009d9e()`), the bootloader prints `"Clear update data.\n"` (`0x44030710`) and calls **`FUN_44009bed()` + `FUN_44009c60()`** (**BGO Write #1**, rewriting `"update"` to `"ATIHSTAM\0\0..."`). Next it runs `*** RTSC Initialize ***` (`FUN_4400b1d5()`) and queries `HDD_MAX_LBA` (`FUN_4400b980()`). Because `CH1` fails (`IDE HOST ERROR (E8000000)` $\rightarrow$ `HDD_MAX_LBA get failed[0xE8000000].`), **`0x44008268..0x44008286`** prints `"mainsoft data is invalid, so set update flag.\n"` (`0x44030771`), calls **`FUN_44009d5b()` + `FUN_44009d83(2)`** (**BGO Write #2**, writing `"ATIHSTAM\0\xFF..."` to `"update"`), and branches into **`FUN_4400877f(1, 1)`**.
   * **`FUN_4400877f` (`0x4400877f..0x44008bc4`):** with `param_1 == 1`, **`0x440087FA..0x44008803`** invokes `FUN_44006c70(300, 0x5048024C /* "HDDERR" */)` (**VFD displays `HDDERR`**), confirms `"fvu.exe"` (`0x504800AF`) is absent from BGO, and calls **`FUN_44008bc5(1, 0)`**.
   * **`FUN_44008bc5` & `FUN_4400940b` (`0x4400940B..0x4400943E`):** when no SD card is inserted (`0x44008C83` $\rightarrow$ `0x44008CD6`: `"SD info get failed[-1]. But, DVD-RAM/CD verup check start.\n"`), **`0x44008CFE`** calls `FUN_4400b789(1)` (`startup_drive(1)`) to spin up optical drive `CH0` looking for an Update-Disc. On optical drive startup failure, `0x44008D09` calls **`FUN_4400940b`**, which prints `"error! Failed to Startup Drive.\n"` (`0x44030F7D`), invokes **`FUN_44009c60()` + `FUN_44009bed()`** (**BGO Writes #3/#4**, clearing `"update"` / `"fvu.exe"` / `"drvfirm"` in `[nfrom]`), and calls **`FUN_44009524("DRV NG", "STFAIL")`** (`0x4400942D..0x44009439`), alternating **`DRV NG` and `STFAIL` on the VFD**.

2. **Stage A — Dual `+0x14` (Uncommitted Rollback) vs `+0x5C` (Committed Forward-Erase) Semantics Erased `SA29` (`0x84160000`) `[PROVEN ON REAL HARDWARE]`:**
   * Every BGO FTL sector header maintains two 16-slot migration bitmaps: **`+0x14`** (`FUN_4401F536` in `Loader ver0.61` / `FUN_50422DDD` in `PROG061`, rollback journal to erase an uncommitted target sector) and **`+0x5C`** (`FUN_4401F4B5` / `FUN_50422D3C`, forward-completion journal written by `FUN_5042269e` on the new sector to erase the old sector).
   * Because Cycle 1 left an open slot at **`SA41 + 0x14`** pointing to **`0x0004` (`SA29`, `0x84160000`)** while `SA29 + 0x14` had no open slots (`FUN_44020647 / FUN_50423540 == -1`), mounting the volume via `ioctl(0)` (`FUN_4401EF82` $\rightarrow$ `FUN_4401F536`) saw `header(SA41) == header(SA29) == 0xF101`, classified `SA29` as an aborted migration target of `SA41`, **erased `SA29` (`0x84160000`) into a clean spare (`0xFFFF`, `+0x100=0x00`)**, and closed `SA41 + 0x14` (`0xFFFF`) — leaving **`SA41` (`0x84220000`) as the sole active `0xF101` sector** and placing **`SA29` (`0x84160000`) at the head of the spare pool**.

3. **Stage B — `5` `"ATIHSTAM"` Cycles Filled `0xF20A` to `126/126` Subpages and Migrated `SA26 -> SA29` (`0x84160000`) `[PROVEN ON REAL HARDWARE]`:**
   * Live `COM10` read-back of `SA29` (`0x84160000`) proved `SA29` now carries logical sector **`0xF20A`** (migrated verbatim from `SA26`): subpages `sp=0..86` (`+0x400..+0xB1FF`) match original `SA26` (with superseded tags `sp=65` `0x000D`, `sp=73` `0x002F`, `sp=81` `0x0F07`, `sp=84` `0x0F16`, `sp=85` `0x0F03`, `sp=86` `0x0F01` invalidated to `0x0000`, preserving `sp=2,3,5,6,8,9`), while **`sp=87..125` (`+0xB200..+0xFE00`, `39` subpages filling `126/126` slots)** hold **`5` sequential `[nfrom]` `"update"` transactions** (`"ATIHSTAM\0\xFF..."` / `"ATIHSTAM\0\0..."`, FAT `0x0F07`/`0x0F16`, Dir `0x0F03` `"fio_recv / sys_hdd / update"`, and Ctrl `0x0F01` `0x0000FFFF -> 0x00000000 -> 0x00FFFFFF`).
   * Upon exhausting subpage `125` of `0xF20A`, FTL sector replacement (`FUN_44021080` / `FUN_5042269e`) compacted `0xF20A` from `SA26` (`0x84130000`) into first spare **`SA29` (`0x84160000`)** (keeping identical subpage indices in OMAP) and erased **`SA26` (`0x84130000`)** into a spare (`0xFFFF`, `z100=0000`), while leaving **`SA27, SA28, SA30..SA38`** untouched (`0` tag diffs).

4. **Stage C — `PROG061` `ioctl(2)` Format (`FUN_504223f8`) Wiped `SA39..SA70` & Aborted on `SA70` `8×8 KiB` Top-Boot Mismatch (`0x843FE100`) `[PROVEN ON REAL HARDWARE]`:**
   * Because `PROG061` table `0x50436E30` configures base **`0x84200000` (`SA39`)** with Bank 1 = `31 × 64 KiB` (`0x84200000..0x843E0000` = `SA39..SA69`) and Bank 2 = **`8 × 8 KiB` Top-Boot** (`0x843F0000..0x843FE000` inside `SA70`), running `FUN_50421ebe(0)` $\rightarrow$ `FUN_504266e0(1,0)` $\rightarrow$ **`FUN_504223f8(1,0)`**:
     1. Left **`SA26..SA38`** untouched (`SA26=0xFFFF`, `SA27..SA38` = `12` `0xF2xx` sectors including `SA29=0xF20A`);
     2. Erased all **31 sectors `SA39..SA69` (`0x84200000..0x843E0000`)** — **destroying `SA41` (`0x84220000`), the sole surviving `0xF101` sector** — and wrote `256` zeros at `+0x100` of each;
     3. Looped `idx = 32..39` across **`SA70` (`0x843F0000..0x843FE000`)**. Because `SA70` in the 8 MiB `S29JL064H` is a **single uniform `64 KiB` sector** (actual `8×8 KiB` boot blocks reside at `0x847F0000..0x847FFFFF`), the 8th sector erase at `0x843FE000` **wiped all of `SA70` again**, leaving `0x843F0100..0x843FC100` as `0xFF` and `256` zero bytes only at **`0x843FE100`** (`SA70 + 0xE100`, verified live on `COM10`!);
     4. Next (`0x50422468..0x504224B6`), `FUN_50422af2` validated `0x843F0000..0x843FC000` with `FUN_504236b5` and **aborted with error `1`** because `+0x100` was `0xFF` rather than `0x00`, exiting **before writing any `0xF101` or `0xF2xx` headers**. Live result: **`0` `0xF101` sectors across all BGO**, `12` `0xF2xx` sectors in `SA27..SA38`, `31` clean spares in `SA39..SA69`, spare `SA26`, and `SA70` marked at `0x843FE100` — causing permanent **`U99` (`BGO driver init failed[1].`)**.

> [!IMPORTANT]
> **Mandatory EH55 Restoration & Relapse-Prevention Rules:**
> 1. When repairing `SA26..SA70`, maintain **exactly 1 active `0xF101` sector** (e.g. `SA29` with clean `0xFFFF` `+0x14` and `+0x5C` bitmaps) and format `SA41` as a clean `0xFFFF` spare (`+0x100=0x00`), avoiding any open `SA41 + 0x14 -> SA29` link that triggers `FUN_4401F536` rollback.
> 2. While `CH1` remains faulted, every full `normal mode` boot consumes `[nfrom]` subpages in `0xF20A`; use `JIG:2500` boots during diagnostics or remove any legacy stock-FVU SD card.
> 3. Never send raw GDB RSP frames (`$m...#B1`) while the target is already sitting at **`PANAMON1>`**: ASCII **`'B'`** (`0x42`) in `#B1` triggers `OK:BYPASS` and exits `PANAMON1>` back into post-`0x5040A2CD` `PROG061` execution. Always probe **`?\r`** first.

### 2.4 Permanent Fix, Live `COM10` Unbrick & Firmware Coverage Audit `[PROVEN ON REAL HARDWARE]`

1. **Structural Fixes in C11 Pico Probe & `eh55_bgo` Reconstructor:**
   * **`p7501_target_bus_pio.c` & `p7501_target_bus_panamon.c`:** added mandatory `?\r` pre-probe (`p7501_bus_panamon1_probe_open`) before emitting any GDB RSP frame (`$m`/`$?`) to eliminate accidental `'B'` (`OK:BYPASS`) exits, paired with `350 µs` per-byte TX pacing and immediate echo draining so MN103S 1-byte `SC0RXB` never overruns.
   * **Single-`0xF101` `SA29` Compaction & Full FAT Verification (`crates/panadiga-core/src/re/eh55_bgo/`):** compacted all `243` L2P slots (`0x0000..0x00F2`) into a single `0xF101` sector (`SA29`), formatted `SA41` as a clean `0xFFFF` spare (`+0x100=0x00`), recovered superseded `0x0F0A` from `SA36` slot `38` (restoring `224/224` pages of `fs2.dat` and `96/96` of `debug`), and enforced full FAT chain termination (`0xFFED`) across all `19/19` `[nfrom]` files.
2. **Live Post-Repair Bench Proof (`COM10`) & Full 8 MiB Healthy `IC5001` Capture:**
   * **Live boot & BGO stability verification:** pre-boot **`BGO_INSPECT`** returned `sectors=45 erased=4 valid=41 corrupt=0 dup_lbn=0 zmarker_bad=0 healthy=YES`; **`BOOTTRACE 15000 NORMAL`** booted `Loader ver0.61 [06/03/06]` $\rightarrow$ `***** normal mode *****` $\rightarrow$ `*** RTSC Initialize ***` with **ZERO `BGO driver init failed[1].`**, cleanly processed `[nfrom] file_no[17]` (`upd_flg`), and reached `Fvu ver0.61` `PANAMON1>`; post-boot **`BGO_INSPECT`** continued to report **`healthy=YES`** (`erased=4 valid=41 corrupt=0`).
   * **New 8 MiB live healthy dump (`0x84000000..0x847FFFFF`):** captured page-by-page with 8-bit checksums via the MN103S stub into [`eh55_nor_8mb_ic5001_repaired.bin`](../../firmware_and_images/README.md) (`8,388,608 B`, **`SHA-256: bd089ed1a062c48756c60407e6555463e8646cde094b0328b88a253e4f1d4121`**, `BRIDGE: OK rx=3616219 tx=9395 ferr=0 ovf=NO`, `317.8 s`).
   * **Per-region integrity vs. immutable pre-repair gold dump (`c1faee79...`):** **`0` diff bytes** across `SA0..SA10` (`Loader ver0.61`, `256 KiB`), `SA11..SA14` (`Nor-FROM`, `256 KiB` `0xFF`), `SA15..SA25` (`EXTRHEADDRV`, `704 KiB`, zlib-decompresses to `1,441,792 B` SHA-256 `6f2b61ede80ea9e3e9185d3d677813593f3d361446678e437c905a0037060243`, 100% matching `EH55_DRV_S211.bin`), `SA71..SA133` (`EEMUL 64K`, `4,032 KiB`), and `SA134..SA141` (`NVRAM 8×8K`, `64 KiB`); **`panadiga unbrick bgo-audit`** and **`cargo test -p panadiga-core eh55_bgo`** (`7/7` OK) verified `healthy=true`, `active_p1=1`, `p2_present=40`, `spares=[41, 67, 68, 70]`, `corrupt=[]`, `active_l2p_pages=996`, `nfrom_files=19`, `ifodev2="M6930LB"`.
3. **Honest Firmware Mapping & Coverage Matrix (Mapped vs. Open Gaps):**
   * **Physical `IC5001` (142 sectors, 8 MiB):** **100% partitioned and integrity-verified** (`SA0..SA10` IPL `ver0.61`, `SA11..SA14` erased `Nor-FROM` slot, `SA15..SA25` zlib-verified `EXTRHEADDRV`, `SA26..SA70` page/FAT-decoded BGO `[nfrom]`, `SA71..SA133` `0FF00000` + `"1284"` Gemstar/TV Guide/`sched.bin`/`resetdiag` headers, `SA134..SA141` NVRAM). **Open gap:** full wear-leveling FS extractor for Upper `EEMUL` (`"1284"`, `1,158` tags) is not yet implemented in Rust.
   * **Disassembly, symbols, command tables & callers ([`INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md`](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) & [`GHIDRA_UNDEF_AUDIT_REPORT.md`](../pseudocode/GHIDRA_UNDEF_AUDIT_REPORT.md)):**
     * Boot/unbrick/recovery subsystems are thoroughly mapped (`96` curated SSOT symbols, `25` MCP tools, `457` error translations across 7 domains, `64` Pico catalog commands, `89` DIGA v2 hardware profiles, `<8sIII>` DRV CLI at `0xD92CC`, ATAPI `0x3c`, `PROG061` 3 patch sites & `55` call sites, `Loader ver0.61`/`ver0.63` `SDCD`/FAT/BGO/DAVIS callers), yet **not 100% of every instruction in the full binaries**: raw Ghidra databases still hold `??` gaps (`BOOT.BIN` `25.60%`, `DRV_decomp` `46.20%`, `MAIN_3410PQ` `93.65%` — where `_dev_SERIAL`, `LBA48`, `FPC`, `AIOF`/`AVDD`/`TBC`, `VDM_Init`/`SDM_Init`, `region_compat`, and `SERV`/IR are mapped, while unrelated 13 MB UI/codec tasks and live `IC7501` runtime opcodes await unblocked `CH1` `MAIN` boot).

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
  * Rotating `StarTech IDE2SAT2` to **`180°`** **with the factory Type A FFC** (`probe_ch1_full_diag_20261119.log`, `2A MASTER`/`SLAVE`) returned **`0x00002004`** across all 9 `CH1` ATA registers = **`DD13` (`0x2000`, ATA 14)** sampling `ATA 27 (IORDY = 3.3V)` + **`DD2` (`0x0004`, ATA 13)** sampling `ATA 28 (CSEL = 3.3V)`! This **mathematically proves** the dual-row involution $\pi_{180^\circ}(k) = 41 - k$ (`0°` is required when keeping the stock Type A flat ribbon).
  * **Low-Profile Chassis-Closure Workaround (`Type B` FFC + `180°` Blue Adapter `N5EZZ0000003`):** because the upright `0°` assembly collides with the DMR-EH55 top lid, pairing a **`180°`**-rotated blue board with a **40-pin `Type B` FFC** (`0.5 mm` pitch, `20.5 mm` / `~2 cm` width, `0.30 mm` end thickness, OEM `VEK0K00`/`VEK0K15` — or carefully shaving the opposite-face polymer jacket at one end of the stock FFC using a **brand-new razor blade** and insulating the old face with Kapton) composes two involutions ($\pi_{180^\circ}(\pi_{\text{TypeB}}(k)) = 41 - (41 - k) = k$), restoring exact **1:1 identity** across all 40 pins — including routing **`DMARQ` (`ATA 21`)** through the unblocked full contact #20 on `N5EZZ0000003` (see full proof, continuity checklist, and bench photos in [`STORAGE_ARCHITECTURE_MEIHDFS_PARTITIONS.md §9.6`](./STORAGE_ARCHITECTURE_MEIHDFS_PARTITIONS.md#96-low-profile-chassis-closure-modification-40-pin-type-b-ffc-or-diy-strip--180-inverted-blue-panasonic-adapter-n5ezz0000003)).
  * At **`0°`** ([`probe_ch1_live_v2_0deg_20261119.log`](../debug_logs/probe_ch1_live_v2_0deg_20261119.log)), `CH1` stopped hanging the CPU on `R 89000D58,4`, yet returned **`0x00000000`** across `2A..2C` and `FUN_5041CD4D(1)` (`rc=0x00000000`, `struct_504F8234`: `is_init=1, is_atapi=0, is_ata=0`).
* **Verified `IDE2SAT2` Electrical Voltages (`[PROVEN ON REAL HARDWARE]`):** DC multimeter measurements on the corrected `StarTech IDE2SAT2` pin numbering confirmed zero short circuits: **`Pin 27 (IORDY) = 3.34 V`**, **`Pin 1 (RESET-) = 2.45 V`**, **`Pin 23 (DIOW-) = 2.47 V`**, **`Pin 25 (DIOR-) = 2.47 V`**, **`Pin 37 (CS0-) = 2.47 V`**, and **`Pin 38 (CS1-) = 2.45 V`** (the earlier `0 V` reading was simply a pin-order offset before accounting for the `41 - k` reversal).

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
| **`H1`** (`IDE2SAT2 DC Rails & Control Voltages`) | Using a DC multimeter at the `StarTech IDE2SAT2` Molex/Berg connector and corrected `41 - k` pin order, measure **`DR5V` (`~5.0V`)**, **`DR12V` (`~12.0V`)**, `IORDY` (`ATA Pin 27 = 3.34V`), and `RESET-`/`DIOR-`/`DIOW-`/`CS0-`/`CS1-` (`2.45–2.47V`). | Confirms healthy `DR5V`/`DR12V` power and idle voltages on `IDE2SAT2` (`IORDY = 3.34V`, controls `2.45–2.47V`), ruling out `IC1506` brownout and advancing to `H2`/`H4`. |
| **`H2` / `H3`** (`P58001`–`VEK0K00`–`N5EZZ0000003` vs `IDE2SAT2`) | Remove AC, discharge rails, swap in an alternative adapter from the kit at verified **`0°` (`P58001 Pin k = ATA (41 - k)`)**, and run `panadiga pico davis-probe` / `PANAMON1>` stub on `CH1` (`0x8900_0C00..0C7F`). | Reading `STATUS = 0x50` at `0xE800_005C` and completing `IDENTIFY DEVICE (0xEC)` (`SENTINEL = 0xCAFE00EC`) confirms `CH1` physical link recovery. |
| **`H4`** (`DAVIS` / `SYS_STATUS` Bus State) | Via Door #1 (`PANAMON1>`), inspect `SYS_STATUS` (`0xC000_1008`) and safe read ranges `0x8900_0C00..0C7F` + `CH1_DEVCTL` (`0x8900_0D58`) — **NEVER read `0xC000_100C`**. Optionally capture `IORDY`/`CS0-`/`RESET-` transitions via Pico `PIOLOGIC`. | Distinguishes whether the `DAVIS CH1` controller observes stuck lines even with `N5EZZ0000003` disconnected vs only under adapter load. |
