> 🌐 **Language / Idioma:** **English** | [Português (BR)](../pt-BR/TEORIA_E_PRATICA_UNBRICK_U99_E_CRIPTOGRAFIA_MATSUSHITA.md)
> 🧭 **Portal:** [Docs Index](../INDEX.md) | [Complete Audit](../../COMPLETE_AUDIT.md) | [Audits](../audits/README.md) | [Ghidra Pseudocode](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Debug Logs](../debug_logs/README.md)

# Reverse Engineering Treatise: U99 Unbrick, Cryptanalysis & Matsushita Hexadecimal Cartography
## Semiconductor Physics Fundamentals, EEMUL Subsystem, Cryptographic Keys & Firmware Memory Architecture

**Document:** `THEORY_AND_PRACTICE_U99_UNBRICK_MATSUSHITA_CRYPTO.md`  
**Classification:** Deep Reverse Engineering Treatise & Physics-Level Reference  

---

## 1. Why Erase the 64 KB NVRAM (`SA134..SA141`) to `0xFF` Instead of "Patching Only the U99 Flag"?

A recurring reverse engineering question is: *would it not be more elegant and surgical to decompile the firmware, pinpoint the exact byte holding the U99 error flag in NVRAM, and zero only that flag while keeping 100% of all other original bytes untouched?*

The definitive engineering answer is **NO**. That approach is both physically unworkable and harmful due to **three fundamental barriers rooted in semiconductor physics, transactional filesystem architecture, and factory contingency routines**:

```text
+---------------------------------------------------------------------------------------------------+
|                        PHYSICAL PROGRAMMING BARRIER OF NOR FLASH MEMORY                           |
+---------------------------------------------------------------------------------------------------+
| Unidirectional Bit Transition (Write) :  1 ───> 0    (Hot-Carrier Electron Injection Programming) |
| Reverse Bit Transition (FORBIDDEN)    :  0 ───> 1    (Physically IMPOSSIBLE without Sector Erase!)|
| Matrix Cell Reset Mechanism           :  Entire 8 KB sector forced to 0xFF via JEDEC Sector Erase |
+---------------------------------------------------------------------------------------------------+
```

### 1.1 Semiconductor Physics of Spansion `S29JL064H` NOR Flash
* **State Transition Asymmetry:** In floating-gate Flash memory cells, transistors leave the factory in an uncharged state (logic `1`). Programming (*Byte/Word Program*) only injects electrons into the floating gate, transitioning bits strictly from `1` to `0`.
* **Impossibility of Isolated $0 \rightarrow 1$ Writes:** If any status byte already holds cleared bits (`0`), the on-chip controller is **physically incapable** of flipping individual bits back to `1`.
* **The JEDEC `Sector Erase` Command:** The only way to return bits to `1` is evacuating trapped electrons across all floating gates in an entire sector simultaneously via Fowler-Nordheim tunneling, resetting every byte of all 8,192 bytes in that micro-sector back to `0xFF`.
* Consequently, toggling even one byte from `0` back to `1` inside an already-written block **mandates erasing the entire micro-sector anyway**.

---

### 1.2 Error `U99` Is NOT a "Static Flag" — It Is a Transactional Integrity Trap (*Kernel Panic Trap*)
Matsushita firmware does **not** implement NVRAM as a flat memory struct where `offset 0x1234 = error_code`.
The top 64 KB of Spansion Flash (`0x807F0000` through `0x807FFFFF`, Top-Boot micro-sectors `SA134` through `SA141`) hosts the **`EEMUL` (EEPROM Emulation Engine)**:

```text
               DYNAMIC CIRCULAR STRUCTURE OF THE EEMUL SUBSYSTEM (SA134..SA141)
+-----------------------------------------------------------------------------------------+
| Block Header    | Record 1   | Record 2   | ... | Record N (Active)  | 0xFF 0xFF (Blank)|
| (Seq ID + CRC)  | (Obsolete) | (Obsolete) |     | (Current Values)   | (Append Buffer)  |
+-----------------------------------------------------------------------------------------+
       │                                                      │
       └─── Wear-Leveling: Records advance continuously ──────┘
```

1. **Sequential Append Logging:** To avoid wearing out Flash endurance (rated at ~100,000 erase cycles), Panasonic engineered a sequential transactional journal. Each updated calibration record appends at the next virgin `0xFF` slot.
2. **Dynamic Compaction (Garbage Collection):** Once an 8 KB micro-sector runs out of blank space, active records compact into the next micro-sector and JEDEC `Sector Erase` wipes the exhausted sector.
3. **True Root Cause of `U99`:**
   - Error `U99` occurs primarily due to **premature power loss** or high ripple on the $3.3\text{V}$ rail (common as secondary SMPS electrolytic capacitors gain ESR after 15+ years of service).
   - If power collapses mid-JEDEC compaction, the micro-sector enters a split hybrid state: truncated records, dangling linked pointers, and **header checksum mismatches (CRC16/Sum32)**.
   - At boot, the **IPL (Initial Program Loader)** scans `SA134..SA141`. Detecting checksum corruption and broken circular pointers, the kernel refuses to energize optical laser diodes or initialize IDE buses to prevent permanent hardware damage.
   - The system locks into a fail-safe panic loop and drives the front VFD panel to display **`U99`** (*Fatal Parameter / Drive Initialization Timeout*).

> [!IMPORTANT]
> **Why Manual "Patching" Inevitably Fails:**  
> Because `EEMUL` slides continuously via wear-leveling, no "flag" sits at a fixed byte offset. Manually reconstructing the journal would require decoding Matsushita's transaction B-tree, repairing broken pointers, and recomputing proprietary checksums across every sector — a single mismatched word causes IPL rejection and returns immediately to `U99`.

---

### 1.3 Panasonic's Built-In Cold-Factory Recalibration Mechanism (*Cold-Factory Recalibration*)
The definitive recovery path engineered by Matsushita's own architects relies on virgin erased Flash state:

```text
                  BOOTLOADER (IPL) INITIALIZATION FLOW
                              [ Power-On ]
                                   │
                                   ▼
                 [ Read Sectors SA134..SA141 (0x807F0000) ]
                                   │
                      What state are the 64 KB in?
                     ┌─────────────┴─────────────┐
       DATA WRITTEN WITH INVALID CHECKSUM      ALL 64 KB EQUAL 0xFF
                     │                                   │
                     ▼                                   ▼
              [ PANIC TRAP: U99 ]           [ AUTO-FACTORY MODE TRIGGERED ]
            (Freezes IDE/Laser Buses)       1. Detects virgin factory Flash
                                            2. Recalibrates ADCs & thermal sensors
                                            3. Sends ATA IDENTIFY DEVICE to HDD
                                            4. Rebuilds clean calibration tables
                                            5. Writes fresh valid EEMUL blocks
                                                          │
                                                          ▼
                                                [ 100% CLEAN BOOT ]
```

* **Behavior When All 64 KB Read `0xFF`:**  
  The IPL recognizes the factory *"Uninitialized Non-Volatile Memory"* signature.
* It immediately runs the self-recovery sequence:
  1. Clears residual fault counters.
  2. Auto-calibrates internal ADC references and SoC thermal sensors.
  3. Issues `ATA IDENTIFY DEVICE` (`0xEC`) over IDE to interrogate native drive geometry.
  4. Synthesizes clean calibration tables and writes a fresh valid `EEMUL` structure to Flash.
  5. Completes startup and advances into normal recorder operation.

Therefore, **erasing all 64 KB of NVRAM to `0xFF` is not destructive data loss; it is the official factory trigger that makes the recorder self-heal and match its exact installed physical hardware**.

---

### 1.4 Why Erasing ONLY Block 3 (`SA137`, 8 KB Panic Flags) Does NOT Resolve `U99`

A tempting hypothesis raised during early audits was:  
*"If Block 3 (`SA137`, 8 KB, physical offset `0x7F6000..0x7F7FFF`) stores fatal error records and Panasonic panic flags, would running JEDEC `Sector Erase` solely on that single 8 KB micro-sector while leaving the other 7 blocks (56 KB) untouched cure `U99`?"*

Technical analysis rooted in firmware internals and bench verification proves that **erasing Block 3 alone categorically fails to restore the recorder** due to three deterministic blockers:

#### 1. Cross-Sector Superblock Invalidation in `SA141` (Block 7)
Matsushita's `EEMUL` subsystem **does not treat the eight 8 KB blocks as independent partitions**. Block 7 (`SA141`, `0x807FE000..0x807FFFFF`) acts as the *Master Superblock* storing:
* Global compaction Generation ID.
* Active pointer table for each sector `SA134..SA140`.
* Structural cross-checksums (`CRC16`/`Sum32`) covering every micro-sector.

If you erase only Block 3 (`SA137`) to `0xFF`:
1. `SA137`'s digest collapses to the trivial $\text{CRC16}(0\text{xFF}\dots0\text{xFF})$.
2. At power-on, IPL self-check reads the master table in `SA141` and recomputes `SA137`'s CRC.
3. Comparing computed CRC against the committed checksum inside `SA141` triggers **`Cross-Sector Checksum Mismatch`**.
4. Confronted with transactional tree incoherence, the kernel halts parameter loading and immediately throws panic freeze **`U99`**.
5. Because NOR Flash forbids $0 \rightarrow 1$ writes without sector erase, updating `SA141` requires erasing `SA141` itself — destroying pointers to the remaining sectors!

#### 2. Confusing Cause with Symptom (Block 3 Is the Black Box, Not the Fault)
Block 3 (`SA137`) acts purely as the critical event flight recorder (*Crash Dump / Panic Logger*):
* It records the CPU snapshot at panic time: AM34 registers, IDE bus status, power sense, and fault code.
* The underlying fault **NEVER originated inside Block 3**. It originated in **Block 0 (`SA134` — laser focus/tracking calibration)** or **Block 6 (`SA140` — active transaction descriptors)** during a brownout or sudden power cut.
* Wiping only Block 3 erases the crash log while leaving the root corruption in `SA134` or `SA140` completely intact.
* Within microseconds of rebooting, the IPL reloads corrupted tables from `SA134`/`SA140`, hits another bus fault, logs a **fresh crash record into Block 3**, and locks back on **`U99`**.

#### 3. Atomic Boolean Check Inside `cold_factory_recovery`
Decompression and startup logic in the IPL validates factory initialization via a strict 64 KB loop:

$$\text{TriggerAutoFactory} = \bigwedge_{addr = 0x807F0000}^{0x807FFFFF} \big(\text{FlashRead8}(addr) == 0\text{xFF}\big)$$

* Only if **all 65,536 bytes** equal `0xFF` does the IPL recognize virgin Flash and invoke full factory recalibration.
* If Block 3 is `0xFF` while `SA134..SA136` and `SA138..SA141` retain bytes $\neq 0\text{xFF}$, the condition evaluates **FALSE**.
* The IPL skips factory recovery, boots down the normal operational path, trips over the corrupt tree, and locks on `U99`.

**Unequivocal Conclusion:** The only 100%-reliable recovery action is issuing JEDEC `Sector Erase` across all 8 NVRAM micro-sectors together (`SA134..SA141`, 64 KB total).

---

## 2. Matsushita Cryptanalysis: Where Keys Live, How Handshakes Work & Extraction Paths

### 2.1 Physical Residence of the Master Key
DMR-EH55 hardware lacks transparent external-bus decryption logic. For the AM34 CPU to fetch instructions starting at reset (`0x80000000`), external Spansion NOR Flash **must contain plaintext native MN103S machine instructions**.

Master cryptographic material resides in two hardware locations:
1. **IPL Bootloader Code (`SA0`, Spansion Flash `0x80000000`..`0x8000FFFF`):**
   - Embedded as immediate constants and static tables directly inside unencrypted machine code on the main PCB.
   - Any external programmer readout (XGecu T48 / SVOD 4) or uncached `0x80000000` capture contains the decryption routines and key constants.
2. **Internal Mask ROM of Matsushita AM34 (`MN103S`) Silicon:**
   - An on-die 16 KB Boot ROM validating primary IPL authenticity.

---

### 2.2 Three-Level Operational Handshake

Firmware ingestion flows through three distinct verification stages:

```text
+------------------------------------------------------------------------------------+
| LEVEL 1: OPTICAL ATAPI HANDSHAKE (DVD Drive MN102 <---> Main CPU AM34)             |
|    - User inserts CD-R. Optical DSP (MN102) reads 160-byte TOC.                    |
|    - Validates Matsushita Magic: EF DC 1C DA 13 88 1B AB                           |
|    - Validates Core Target ID:   74 A7 8A CD 78 9B 84 FF (AM34)                    |
|    - Front panel VFD cycles: LOAD ──> LD FVU ──> M_FIRM ──> L1 P01                 |
+------------------------------------------------------------------------------------+
                                         │
                                         ▼
+------------------------------------------------------------------------------------+
| LEVEL 2: HDD STORAGE HANDSHAKE (In-Drive Encrypted Transfer)                       |
|    - Main CPU does NOT decrypt firmware before writing to IDE HDD.                 |
|    - Transfers encrypted blocks DIRECTLY into reserved HDD partitions at           |
|      LBAs 65536 (32 MB) and 131072 (64 MB).                                        |
|    - Writes encrypted key check marker: E5 D1 F3 4A 51 8B 26 90                    |
+------------------------------------------------------------------------------------+
                                         │
                                         ▼
+------------------------------------------------------------------------------------+
| LEVEL 3: OPERATIONAL BOOT HANDSHAKE (Executed on Every Power-On)                   |
|    - IPL reads "SH" MBR at LBA 0 and 256 XOR 0xFF descriptors at LBA 24..31.       |
|    - DMAs HDD Partition 1 into Elpida SDRAM (0x40000000).                          |
|    - Runs 16-round Feistel decryption in RAM using ROM keys.                       |
|    - Decompresses kernel (GZIP/LZSS) and jumps to RTOS Entry Point.                |
+------------------------------------------------------------------------------------+
```

---

### 2.3 Forensic Phenomenon of the 64-Bit ECB Codebook

Operating in *Electronic Codebook* (ECB) mode with no inter-block chaining (zero IV) exposes structural plaintext patterns, letting us catalog the **Panasonic Codebook**:

$$\text{Plaintext } [0\text{xFF} \times 8] \xrightarrow{\text{Feistel 64-bit}} \mathbf{03\ 50\ 65\ C8\ 7B\ B8\ E0\ 90} \quad \text{(Virgin Erased Flash)}$$
$$\text{Plaintext } [0\text{x00} \times 8] \xrightarrow{\text{Feistel 64-bit}} \mathbf{E5\ D1\ F3\ 4A\ 51\ 8B\ 26\ 90} \quad \text{(Zero Padding / BSS)}$$

The simultaneous appearance of `E5D1F34A518B2690` inside optical container `PANA_DVD.FRM` and inside reserved HDD partition headers proves **both media channels share the exact same cryptographic master key**.

---

## 3. Complete Ecosystem Hexadecimal Cartography

### 3.1 Forensic MEIHFS Hard Disk Dump (`hdd_100.bin` — 104,857,600 Bytes)
* **SHA-256 Digest:** `9d292d5b46f190775f359e30af11d074021fb321f040bf701d0ecf0f2056db00`
* **Audited Span:** `100.00 MB` / `204,800` 512-byte LBA sectors.

```text
  LBA Sector      Physical Offset     Active Size     Signature / Magic        Technical & Forensic Description
─────────────   ───────────────────  ─────────────   ───────────────────────  ─────────────────────────────────────────────
  LBA 0           0x00000000..1FF      512 bytes     0000000001005348...      Primary Panasonic MBR (ASCII "SH" tag, GUID)
  LBA 1..23       0x00000200..2FFF      11.5 KB      Variable                 MEIHFS Partition Table & Volume Structures
  LBA 24..31      0x00003000..3FFF     4,096 bytes   XOR 0xFF (1's Comp.)     256 Bit-Inverted Extent Descriptors
  LBA 16640       0x00820000..95FF     1.25 MB       Transactional 0x5348     Transactional Journaling Pool (10 blocks)
  LBA 57344       0x01C00000..1FF      512 bytes     61162c24c9240408...      MIRROR MBR (Exact 28.00 MB / 100% identical)
  LBA 57368..75   0x01C03000..3FFF     4,096 bytes   4b604ad2a689bd39...      Identical Mirror of 256 XOR Descriptors
  LBA 65536       0x02000000         6,173,264 B     5187138D5B0AFB22         PARTITION 1: Main AM34 Kernel (32 MB mark)
  LBA 131072      0x04000000           720,896 B     5187138D5B0AFB22         PARTITION 2: Subsystem / Drivers (64 MB mark)
```

---

### 3.2 Optical Firmware Update Container (`PANA_DVD.FRM` — 7,531,744 Bytes)
* **Shannon Entropy:** `7.99958 bits/byte`.

```text
    Offset Range          Active Size       Signature / Target ID        Hardware Role & Payload Destination
──────────────────────   ──────────────   ──────────────────────────   ─────────────────────────────────────────────
  0x000000..0x00000F         16 bytes     492F0560CEDE508CE3C0...      Optical Container Global Header
  0x000010..0x00003F         48 bytes     EFDC1CDA... 74A78ACD...      TOC Entry 0: Module 1 (Target: AM34 Core)
  0x000040..0x00006F         48 bytes     EFDC1CDA... 3B7E7EDB...      TOC Entry 1: Module 2 (Target: MN102 DSP)
  0x000070..0x00009F         48 bytes     EFDC1CDA... 74A78ACD...      TOC Entry 2: Module 3 (Target: AM34 OS)
  0x0000A0..0x000AE480      713,696 B     Encrypted Payload            MODULE 1: Subsystem/OSD (Maps to HDD Partition 2)
  0x000AEFD0..0x000B17D0     10,240 B     4D3C5DBB0C357777 (1,280x)    Inter-Module Transition Descriptors
  0x000B1800..0x000EAC98    234,648 B     Contains 196x E5D1F34A...    MODULE 2: DVD Drive Firmware (MN102 DSP)
  0x000EAC98..0x000F17F8     27,488 B     035065C87BB8E090 (3,436x)    Virgin Flash Padding (0xFF x 8)
  0x000F1818..0x0072ECE0  6,542,536 B     Encrypted Payload            MODULE 3: DIGA Main Kernel (Maps to HDD Partition 1)
```

---

### 3.3 Physical NOR Flash Spansion `S29JL064H` (`IC5001` — 142 Sectors, 8,388,608 Bytes)

Audited directly against the physical dump captured from the live DMR-EH55 ([`eh55_nor_8mb_ic5001_dump.bin`](../../firmware_and_images/README.md), `SHA-256: c1faee798f60545853eefdba0d15f23678b471466dc2bc84f522bc972f15a02d`): Spansion `S29JL064H70TFI00` combines `8×8 KiB` bottom sectors (`SA0..SA7`) + `126×64 KiB` main sectors (`SA8..SA133`) + `8×8 KiB` top sectors (`SA134..SA141`), totaling exactly `142` sectors (`8,192 KiB`).

```text
   Uncached CS1 Window             File Offset Range        Sectors / Size      Attr    Real Architectural Role (DMR-EH55 ver0.61)
─────────────────────────────   ───────────────────────   ──────────────────   ──────   ────────────────────────────────────────────────────
  0x84000000 .. 0x8400FFFF       0x000000..0x00FFFF        SA0..SA7 (8×8K)       RO     Lower IPL Loader ver0.61 [06/03/06] (Plaintext)
  0x84010000 .. 0x8403FFFF       0x010000..0x03FFFF        SA8..SA10 (3×64K)     RO     Upper Loader ver0.61 + BGO table 0x440337A4 (256 KiB)
  0x84040000 .. 0x8407FFFF       0x040000..0x07FFFF        SA11..SA14 (4×64K)    RO     Erased space (100% 0xFF)
  0x84080000 .. 0x8412FFFF       0x080000..0x12FFFF        SA15..SA25 (11×64K)   RO     EXTRHEAD "DRV " container (0x84080040 -> 0x90202000)
  0x84130000 .. 0x843FFFFF       0x130000..0x3FFFFF        SA26..SA70 (45×64K)   RW*    Real EH55 BGO [nfrom] FTL (1 P1 + 40 P2 + 4 spares; 2,880 KiB)
  0x84400000 .. 0x847EFFFF       0x400000..0x7EFFFF        SA71..SA133 (63×64K)  RW*    Upper EEMUL / NOR FS ("1284", 0x0FF00000 headers)
  0x847F0000 .. 0x847F1FFF       0x7F0000..0x7F1FFF        SA134 (1×8 KiB)       RW     Upper EEMUL tail (6 active bytes 0ff00000002c on live EH55)
  0x847F2000 .. 0x847FFFFF       0x7F2000..0x7FFFFF        SA135..SA141 (7×8K)   RW     Top EEMUL micro-sectors (virgin 0xFF in captured dump)
```
*(Note: in abstract `64 KiB` Pico policy units `NorWritePolicy`, `SA0..SA3` spans the opening `256 KiB` `0x80000000..0x8003FFFF` = physical `SA0..SA10`, and `SA4..SA133` spans `0x80040000..0x807EFFFF` = physical `SA11..SA133`; the cached mirror resides at `0x80000000..0x807FFFFF`.)*

### 3.4 Bus Invariants Summary & Two-Layer NOR Write-Protection Model
* **Default-Protected Factory Region (99.21875%):** `0x80000000` to `0x807EFFFF` (8,323,072 bytes; uncached mirror `0x84000000..0x847EFFFF`) — Preserved (`immutable RO` across the opening `256 KiB` boot block `SA0..SA10` / policy `SA0..SA3`, and write-protected by default with `E05` across `0x80040000..0x807EFFFF` / `SA4..SA133`) throughout standard operation.
* **Targeted NVRAM Sanitization Window (`SA134..SA141`, 0.78125%):** `0x807F0000` to `0x807FFFFF` (`8` micro-sectors of `8 KiB`, `65,536` bytes) — Writable by default on the Pico probe. By contrast, FTL BGO metadata faults in **`SA26..SA70`** (`0x84130000..0x843FFFFF`) require explicit `CONFIRM_UPPER_NOR` opt-in and data-preserving surgical writes (pure `1 -> 0` byte programming without erasing live pages).

> [!NOTE]
> **Two-Layer Controlled NOR Write Model (`SA0..SA3` vs. `SA4..SA133` vs. `SA134..SA141`):**
> Unified policy enforcement via [`NorWritePolicy`](../../crates/panadiga-core/src/pico/rsp_codec/policy.rs) partitions the `8 MiB` Spansion `S29JL064H` flash into three operational tiers:
> 1. **Layer 1 — Critical Bootloader / IPL (`256 KiB`: `0x80000000..0x8003FFFF` / `0x84000000..0x8403FFFF`, policy `SA0..SA3` / physical `SA0..SA10`):** **permanently read-only (`immutable RO`, never writable on the Pico probe)** — never erase or overwrite on hardware; every probe write/erase attempt is unconditionally blocked with error **`E05`** to preserve the serial-boot Root of Trust.
> 2. **Layer 2 — BGO / Application Upper-NOR Region (`0x80040000..0x807EFFFF` / `0x84040000..0x847EFFFF`, policy `SA4..SA133` / physical `SA11..SA133`):** **write-protected by default (`NorWritePolicy::NvramOnly` → `E05`)**, yet **explicitly unlockable** (`NorWritePolicy::UpperNorAppUnlocked`) via `CONFIRM_UPPER_NOR` for surgical BGO FTL repair (`SA26..SA70`) or controlled Upper EEMUL research after SHA-256 backup verification.
> 3. **Layer 3 — Top NVRAM / EEMUL (`SA134..SA141`, `0x807F0000..0x807FFFFF`, `8 KiB` sectors):** writable by default.

### 3.5 Upper-NOR Unlock Governance, JEDEC SRAM Stubs (`0x51800000`) & `Mn103Vm` Emulation

| Surface | Mechanism / Command | Scope & Safety Invariant (`E05`) |
| :--- | :--- | :--- |
| **GDB RSP (`gdb_rsp_codec.c` / `gdb_rsp_memory.c`)** | `$qPanadigaNorUnlock:CONFIRM_UPPER_NOR#<xx>` / `$qPanadigaNorLock#<xx>` | Toggles `NvramOnly` ↔ `UpperNorAppUnlocked`; **never** unlocks the `256 KiB` IPL (`0x80000000..0x8003FFFF`, permanently `E05`). |
| **`PANAMON1` Monitor ([`jedec_emit.rs`](../../crates/panadiga-core/src/re/prog061_uart_monitor/jedec_emit.rs))** | `U CONFIRM_UPPER_NOR` / `U LOCK` | Serial command inside the AM34 resident monitor; `DIAGWATCH` dispatches a two-step `'U'` handoff prior to `'W'` / `'E'`. |
| **Pico `PANARSP` & `DIAGWATCH` ([`p7501_pana_rsp_parse.c`](../../firmware/pico-p7501/uart/p7501_pana_rsp_parse.c), [`p7501_diag_watch.c`](../../firmware/pico-p7501/uart/p7501_diag_watch.c))** | `NORUNLOCK:CONFIRM_UPPER_NOR` / `NORLOCK` + `JEDEC_ERASE:<addr>` / `JEDEC_PROG:<addr>,<hex>` | Uploads [`p7501_pana_jedec_stubs.c`](../../firmware/pico-p7501/uart/p7501_pana_jedec_stubs.c) SRAM stub at `0x51800000` (`1..=16` bytes per write), reporting `PANAJEDEC_OK` or `PANAJEDEC_TO`. Keeps the `256 KiB` IPL `immutable RO` (`E05`). |
| **Offline Copy-on-Write CLI ([`panadiga nvram patch-upper-nor`](../../crates/panadiga-cli/src/cli/handlers/nvram.rs))** | `panadiga nvram patch-upper-nor` with `--backup` ([`upper_nor.rs`](../../crates/panadiga-core/src/nvram/pure/upper_nor.rs), [`io/upper_nor.rs`](../../crates/panadiga-core/src/nvram/io/upper_nor.rs)) | Produces a *Copy-on-Write* output image (**never** mutates source dumps in-place) alongside a SHA-256 verified backup; blocks the opening `256 KiB` unless `--allow-critical-ipl` is passed offline. |

* **Real Spansion JEDEC FSM in `Mn103Vm` ([`spansion_nor.rs`](../../crates/panadiga-core/src/re/mn10300_emu/spansion_nor.rs)):**
  Faithfully simulates the `S29JL064H` state machine: **Autoselect `0x90`** (`0x0001` Manufacturer / `0x227E` Device), **6-cycle Sector Erase (`0x80 -> 0x30`)**, **4-cycle Word/Byte Program (`0xA0`, bitwise `&=` NOR semantics)**, and **Reset (`0xF0`)**.
* **Real-Binary Integration Verification ([`checks_real_nor.rs`](../../crates/panadiga-core/src/re/emu_patch_suite/checks_real_nor.rs)):**
  Tests end-to-end Upper-NOR unlocking, JEDEC programming/erasing, and verification against real firmware artifacts: 8 MiB `flash.bin`, `prog061_dec.bin`, `PANA_DVD.FRM` (`3410PQ`), `main3060np`, and `PANA__SD.FRM`.

---

## 4. Real FTL BGO Anatomy (`SA26..SA70`), `0x90400000` Coherence & Surgical `U99` Repair

### 4.1 Geometry Divergence: Real EH55 `Loader ver0.61` vs. `Loader ver0.63` / `PROG061`

1. **`Loader ver0.63 [07/10/24]` (4MB/TOP BOOT, `/BOOT.BIN` DMR-EZ485V) & `PROG061` (`0x50436E30`):**
   * Places BGO at **`0x84200000..0x843FFFFF`**, using control page `0x0801`, `8` FAT pages, and `4` directory pages (in `PROG061` table `0x50436E30`: `3` P1 blocks, `28`=`0x1C` P2 blocks, `35`=`0x23` sectors, `max_pid=0x0A1A`).
2. **Real DMR-EH55 (`Loader ver0.61 [06/03/06]`, geometry table at `0x440337A4`):**
   * The geometry table at **`0x440337A4`** (file offset `0x0337A4` in [`eh55_loader_ver061_256k.bin`](../../firmware_and_images/README.md)) configures **`45` (`0x2D`) 64 KiB sectors in `0x84130000..0x843FFFFF` (`SA26..SA70`)**:
     * **Partition 1 (L2P Mapping):** **`1`** block (`0xF101`, `SA29` at `0x160000` + migrating block `SA41` at `0x220000`).
     * **Partition 2 (`[nfrom]` Data):** **`40` (`0x28`)** logical blocks (`0xF201..0xF228`), each holding `126` 512-byte slots with slot tags at `+0x0200`.
     * **Spare Pool:** **`4`** formatted spare sectors (`0xFFFF`, `+0x00..+0xFF` all `0xFF`, `0x00*256` marker at `+0x0100..+0x01FF`).
     * **BGO Control & FAT:** max page ID **`0x0F24`**, control page **`0x0F01`**, directory **`0x0F02..0x0F04`**, primary FAT **`0x0F07..0x0F15`** (`15` pages × `256` `u16` entries), and backup FAT **`0x0F16..0x0F24`**.
   * **Upper EEMUL (`SA71..SA141`, `0x84400000..0x847FFFFF`):** Independent from `[nfrom]` BGO (`SA26..SA70`), holding `"1284"` (`0x0FF00000`) files such as `UEIdb-*`, `bitmaps_db*`, `gos_db*`, `config.bin`, and `resetdiag.bin`.

### 4.2 AM34 Uncached Coherence (`0x90400000`) & Real `S29JL064H` ~700 ms Erase Timing

* **Uncached SDRAM Mirror (`0x90400000` vs `0x50400000`):** On the AM34 (`MN103S`), writes to cached SDRAM (`0x50400000`) may remain in write-back D-cache and miss I-cache instruction fetch. Consequently, both the `PANADMP1` dumper (`MOV` bounds patched at **`0x9043950E`** $\rightarrow$ **`J 904394FA`**) and test stub **`DD3754BEB30000DF0000`** (`FUN_4401EF77` at **`0x90439B40`**) target the uncached `0x90400000` mirror directly.
* **Real `S29JL064H` (~700 ms) Erase vs `DQ7` Polling (`jedec_emit.rs`):** Physical 64 KiB sector erase takes ~**`700 ms`**. Because [`emit_jedec_poll_dq7_reset_sub`](../../crates/panadiga-core/src/re/prog061_uart_monitor/jedec_emit.rs) uses a bounded `0x00100000`-iteration DQ7 loop, `PANAMON1>` `E <addr>` returns `OK:ERASE` early while internal erase is still running; scripts must poll `R <addr>,20` for `0xFF*32` before issuing `W` program writes.

### 4.3 Surgical BGO FTL `U99` Repair & Live Normal Boot Proof (`COM10`)

In the pre-repair gold dump ([`eh55_nor_8mb_ic5001_dump.bin`](../../firmware_and_images/README.md), `SHA-256: c1faee798f60545853eefdba0d15f23678b471466dc2bc84f522bc972f15a02d`), `BGO driver init failed[1].` originated from: (a) `13` missing `0xF2xx` blocks (`0xF209`, `0xF21D..0xF228`), leaving `16` formatted spares; (b) `SA70` (`0x843F0000`) with mis-placed `0x00*256` at `+0xE100..+0xE1FF`; and (c) corrupted migration link header at `SA41 + 0x14` (`f8fff8fff8fffcffffff0400ffffffffffffffff`). Live repair via `PANAMON1>` (`OK:NOR_UNLOCK` $\rightarrow$ `OK:NOR_LOCK`) executed:

1. **Clean format of `SA70` (`0x843F0000`):** sector erase + programming `256` zero bytes at `0x843F0100` (`20.7 s`).
2. **`1,629` pure `1 -> 0` byte writes (zero populated sector erases):** across `13` spare sectors (`SA42=62`, `SA43=36`, `SA44=52`, `SA45=34`, `SA46=936`, `SA51=38`, `SA56=40`, `SA58=40`, `SA59=135`, `SA61=118`, `SA64=46`, `SA65=58`, `SA66=34`), restoring all `13` `0xF2xx` sectors, block 2 of `ifodev2` (`0x0055` @ `0x84250800` in `SA44`), and reconstructed FAT pages `0x0F0A` (`0x8427A200`) and `0x0F19` (`0x8427AE00` in `SA46`).
3. **`6` pure `1 -> 0` byte writes at `SA41 + 0x14` (`0x84220014..0x84220023`):** programming `F0` (`+14/+16/+18`), `F8` (`+1A`), `04` (`+22`), and `00` (`+23`), yielding `F0FFF0FFF0FFF8FFFFFF0400FFFF0400FFFFFFFF` (migration link `0x0004` $\rightarrow$ `SA29`). Offline verification confirmed `19/19` directory files and `996/996` L2P entries (`eh55_nor_8mb_ic5001_repaired.bin`, `SHA-256: adb21d955ed96121249e7abeec58e1ce3951f511a595636d70ebb07a35ca4c9c`).
4. **Live `BOOTTRACE 15000 NOJIG` verification (`COM10`):** following live RAM validation of `FUN_4401EF77` at `0x90439B40`, normal boot logged `DDR CHECK: OK` (+10 ms) $\rightarrow$ F/E decompressed from `0x84080040` (`0xACC01`) to `0x90202000` (`0x160000`, `HEADER/DECOMP:5627DD54`) $\rightarrow$ `F/E reset release finish ...` (+778 ms) $\rightarrow$ **`***** Loader ver0.61 [06/03/06] *****`** (+781 ms) $\rightarrow$ **`***** normal mode *****`** (+785 ms) $\rightarrow$ **`*** RTSC Initialize ***`** (+802 ms; `310 B`, `ferr=0 perr=0 ovf=NO`), proving `U99` eliminated and normal boot entering HDD `MAIN` loading via RTSC/DAVIS IDE.
5. **BGO Cycle 2 (`[PROVEN ON REAL HARDWARE]`):** a subsequent boot with a stock FVU SD card (`0x2182DC9`, legacy `PROG061` restricted to `0x84200000..0x843FFFFF`) triggered a relapse on `SA54` (`[nfrom] file_no[17]`, `BGO driver init failed[1].`); Cycle 2 surgical repair across `SA54` + `SA41` slot 9 + `SA29` (`0x51 -> 0x64`) + `SA39`, using `?\r` + `300 µs` retry and `U CONFIRM_UPPER_NOR`/`U LOCK`, permanently restored clean normal boot ([`verify_bgo_restored_boot_20261119.log`](../debug_logs/verify_bgo_restored_boot_20261119.log) · [`EH55_UNBRICK_BREAKTHROUGH_BGO_CH1_DAVIS_AND_REGION.md`](./EH55_UNBRICK_BREAKTHROUGH_BGO_CH1_DAVIS_AND_REGION.md)).
