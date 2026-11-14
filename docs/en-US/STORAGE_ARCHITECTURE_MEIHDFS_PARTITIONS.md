# MEIHDFS STORAGE ARCHITECTURE & PARTITIONING SPECIFICATION
## Reverse Engineering the Hard Disk & Firmware Structure of Panasonic DMR-EH55
### Matsushita UniPhier / AM34 Platform — DIGA Chassis (PCB: VEP79138)

---

## 1. System Overview & Storage Engineering Scope

The **Panasonic DMR-EH55** digital DVD/HDD recorder (part of Matsushita Electric Industrial Co., Ltd.'s DIGA product line) utilizes a non-conventional hybrid storage architecture. Unlike generic PC platforms or commodity Linux-based DVRs, the DMR-EH55 is powered by the proprietary **Matsushita AM34 / MN103S** SoC (`VEP79138` main/digital PCB) and delegates persistent storage across two complementary media:

1. **On-Board NOR Flash Memory (Spansion S29JL064H — 8 MBytes TSOP-48):**
   Contains the primary initial program loader (*IPL*), minimal hardware initialization routines, exception vectors, and non-volatile emulated calibration parameter blocks (`EEMUL` / NVRAM).
2. **Internal PATA/IDE Hard Disk Drive (80 GB to 500 GB):**
   Does not merely store user video recordings. The hard disk drive is an active, mandatory component of the boot subsystem: **it stores the operational executive firmware partitions (AM34 executive runtime), audio/video CODEC microcode, on-screen display (OSD) assets, peripheral microcodes, and the MEIHDFS filesystem metadata tables**.

If a blank or uninitialized hard disk drive is installed in a unit with a malfunctioning optical drive, the unit hangs during POST with front-panel error codes **`HDDERR`** or **`DRV NG`**, refusing to load the executive operating system or format the drive through user menus.

```text
                  PANASONIC DMR-EH55 HYBRID BOOT ARCHITECTURE
 ┌────────────────────────────────────────────────────────────────────────────────────────┐
 │                               MAIN BOARD (VEP79138)                                    │
 │                                                                                        │
 │   ┌──────────────────────────┐                  ┌──────────────────────────────────┐   │
 │   │  Matsushita AM34 / MN103S │   Initial Boot   │  IC5001 Spansion S29JL064H     │   │
 │   │  (UniPhier / AM34 Core)  │◄─────────────────│  (8 MB Flash NOR TSOP-48)        │   │
 │   │   108 MHz / 162 MHz      │   (Stage 1 IPL)  │  - Initial Program Loader (IPL)  │   │
 │   └────────────┬─────────────┘                  │  - Minimal Bootloader            │   │
 │                │                                │  - NVRAM / EEMUL (Calibrations)  │   │
 │                │ IDE / ATA Bus                  └──────────────────────────────────┘   │
 │                ▼                                                                       │
 │   ┌────────────────────────────────────────────────────────────────────────────────┐   │
 │   │                       HARD DISK DRIVE (PATA / IDE / SSD)                       │   │
 │   │                                                                                │   │
 │   │   [ LBA 0 ]                Panasonic Proprietary MBR ("SH", Disk GUID)         │   │
 │   │   [ LBA 24..31 ]           Low-Level Metadata & Allocation Table               │   │
 │   │   [ LBA 16,384..19,199 ]   Transactional Journal (10 x 128 KB Blocks)          │   │
 │   │   [ LBA 57,344 (28 MB) ]   Mirror MBR Table / Identical Backup to LBA 0        │   │
 │   │   [ LBA 65,536 (32 MB) ]   Firmware Partition 1 (0x5E3250 = ~5.89 MB, Stage 2) │   │
 │   │   [ LBA 131,072 (64 MB) ]  Firmware Partition 2 (0xB0000 = ~704 KB, OSD/Micro) │   │
 │   │   [ LBA 204,800+ ]         MEIHDFS Volume (VR_MOVIE.VRO / VR_MANGR.IFO)        │   │
 │   └────────────────────────────────────────────────────────────────────────────────┘   │
 └────────────────────────────────────────────────────────────────────────────────────────┘
```

### Forensic Reference Image File
* **Filename:** `firmware_and_images/panasonic DMR EH-55 Firmware hdd_100.bin`
* **Exact Size:** `104,857,600` bytes (exactly 100.00 MB)
* **Total LBA Sectors:** `204,800` sectors of 512 bytes
* **Global MD5:** `ef4453fc8651903b0a4b0c1873d950d4`
* **Global SHA-256:** `9d292d5b46f190775f359e30af11d074021fb321f040bf701d0ecf0f2056db00`

---

## 2. Global Memory Map & Sector Layout (LBAs & Offsets)

Exhaustive sector-by-sector analysis of the 100 MB forensic image reveals the exact storage topography engineered by Matsushita. The disk layout partitions the storage into guard bands, transactional metadata zones, mirror redundancy, and executable code partitions:

| LBA Sector Range | Hexadecimal Offset Range | Size (Bytes) | Size (MB) | Primary Role / Technical Description |
| :--- | :--- | :--- | :--- | :--- |
| **LBA 0** | `0x00000000` – `0x000001FF` | 512 B | 0.0005 MB | **Panasonic Proprietary MBR:** Signature `"SH"`, version 1.0, and Disk GUID. |
| **LBA 1 .. 23** | `0x00000200` – `0x00002FFF` | 11,776 B | 0.0112 MB | **Guard Band 1:** Zero-filled (`0x00`) pre-metadata isolation sectors. |
| **LBA 24 .. 31** | `0x00003000` – `0x00003FFF` | 4,096 B | 0.0039 MB | **Descriptor Table:** 8 sectors containing 16-byte physical mapping records. |
| **LBA 32 .. 16,383** | `0x00004000` – `0x007FFFFF` | 8,372,224 B | 7.9844 MB | **Intermediate Guard Band:** Zero-padded alignment space up to 8 MB boundary. |
| **LBA 16,384** | `0x00800000` – `0x008001FF` | 512 B | 0.0005 MB | **Journal Catalog Header:** Block count index (`0x0A` = 10 blocks). |
| **LBA 16,385 .. 16,639** | `0x00800200` – `0x0081FFFF` | 130,560 B | 0.1245 MB | **Pre-Allocation Guard:** Boundary alignment buffer. |
| **LBA 16,640 .. 19,199** | `0x00820000` – `0x0095FFFF` | 1,310,720 B | 1.2500 MB | **Transactional Journal Area:** 10 discrete 128 KB blocks (256 sectors each). |
| **LBA 19,200 .. 49,151** | `0x00960000` – `0x017FFFFF` | 15,335,424 B | 14.625 MB | **Post-Journal Guard Band:** Zero-padded space up to 24 MB boundary. |
| **LBA 49,152** | `0x01800000` – `0x018001FF` | 512 B | 0.0005 MB | **24 MB Transition Marker:** Format status flag and volume geometry boundary. |
| **LBA 49,153 .. 57,343** | `0x01800200` – `0x01BFFFFF` | 4,193,792 B | 3.9995 MB | **Pre-Mirror Guard Band:** Zero-padded isolation space up to 28 MB boundary. |
| **LBA 57,344** | `0x01C00000` – `0x01C001FF` | 512 B | 0.0005 MB | **Mirror MBR Table:** Exact bit-for-bit duplicate of LBA 0 at 28.00 MB. |
| **LBA 65,536** | `0x02000000` – `0x025E324F` | 6,173,264 B | ~5.89 MB | **Firmware Partition 1 (OS/Core):** AM34 Executive, CODECs, and drivers. |
| **LBA 131,072** | `0x04000000` – `0x040AFFFF` | 720,896 B | ~0.70 MB | **Firmware Partition 2 (OSD/Micro):** Graphics assets, fonts, sub-CPU microcodes. |
| **LBA 204,800+** | `0x06400000` onwards | Variable | Variable | **MEIHDFS Audio/Video Volume:** User media recordings and DVD-VR data structures. |

---

## 3. Forensic Analysis of LBA 0 (Panasonic MBR)

The primary Master Boot Record at LBA 0 does not follow IBM-PC partition table standards (`0x55AA` signature at offset `0x1FE` is intentionally absent or repurposed):

* **Magic Bytes:** Offset `0x0000` contains ASCII `"SH"` (`0x53 0x48`), identifying the volume as a Panasonic Matsushita Storage Device.
* **Format Revision:** Offset `0x0004` specifies version `0x00010000` (Version 1.0).
* **Volume GUID:** Offset `0x0010` contains a 16-byte unique identifier paired with the optical DVD subsystem.
* **Allocation Bitmap Bit-Inversion (XOR 0xFF):** Panasonic applies bit-inversion (`0xFF` XOR masking) across allocation bitmaps to prevent standard desktop operating systems (Windows, macOS) from attempting destructive automatic file-system repairs or writing volume headers over firmware partitions.

---

## 4. Panadiga CLI Storage Commands

```bash
# Decode partition descriptors and LBA layout
panadiga hdd decode-descriptors "firmware_and_images/panasonic DMR EH-55 Firmware hdd_100.bin"

# Extract binary firmware partitions to working directory
panadiga hdd extract --outdir firmware_and_images/extracted_partitions "firmware_and_images/panasonic DMR EH-55 Firmware hdd_100.bin"

# Verify forensic integrity against SHA-256 audited manifest
panadiga hdd verify-hashes "firmware_and_images/panasonic DMR EH-55 Firmware hdd_100.bin"

# Inspect or unlock the src/hdd_cmd.c 200 GB HDD capacity ceiling (250 GB to 2 TB+)
panadiga hdd inspect-capacity -i firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin
panadiga hdd patch-capacity -i firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin --mode dynamic
```

---

## 5. Dynamic HDD Capacity Unlock (`src/hdd_cmd.c` + MEIHDFS1.0) — 80 GB to 2 TB+

By factory default, Panasonic DIGA's MEIHDFS/LBA48 driver supports full physical drive geometry at the ATA layer, but the executive **`MAIN`** firmware enforces an artificial software cap inside a function compiled from **`src/hdd_cmd.c`**. It compares the measured LBA48 physical block count (`PhysicalBlocks2KB`) against a RAM ceiling variable (`MaxProductLBA`) and clamps larger drives (such as 250 GB, 320 GB, 500 GB, 1 TB, or 2 TB SSDs/HDDs) down to `MaxProductLBA - 1` (~200 GB on standard DMR-EH55 retail firmware).

### 5.1 MN103S Byte-by-Byte Clamp Anatomy Across `MAIN` Builds

Disassembly and binary pattern matching across both unpacked `MAIN` binaries localized a **117-byte basic block (`+0x00..+0x74`)**, matched deterministically via a **`WIN_LEN = 0x72` (`114`-byte)** structural signature starting with `FD AA 00 8A 00 00 FB E9 00 80 C8 1D FC CC`:

| Symbol / Offset | `MAIN3060NP` (`main3060np_unpacked.bin`) | `EH55_MAIN_3410PQ` (`EH55_MAIN_3410PQ.bin`) | Notes for On-Call Engineers |
| :--- | :--- | :--- | :--- |
| **Unpacked Size / SHA-256** | `13,427,852 B` / `075f552e...61ddcca7` | `13,003,052 B` / `c3275d39...5dc991b1` | Base virtual address `0x57000000` |
| **Clamp Window Start** | `0x574E499C` (`file +0x004E499C`) | `0x574DD1FC` (`file +0x004DD1FC`) | Anchors `WIN_LEN = 0x72` (`114` B) |
| **`MaxProductLBA` SDRAM VA** | `0x5300750C` | `0x5311B9AC` | 32-bit global ceiling (in 2 KB blocks) |
| **`PhysicalBlocks2KB` SDRAM VA** | `0x530077E0` | `0x5311BC80` | 32-bit physical LBA48 count / 4 |
| **RTSD Option Strings** | `0x579F9F13` / `0x579F9F2D` | `0x57A5E957` / `0x57A5E971` | `"[RTSD] M7920(HDD 400GB) \n"` / `"M7920(HDD 200GB) \n"` |

Instruction breakdown (`+0x00..+0x74`):
* **`+0x00..+0x0B`** (`FD AA 00 8A 00 00 FB E9 00 80 C8 1D`): Reads halfword `DevInfo(option)+0x8A`, masks bit `0x8000` (`M7920` 400 GB flag), and branches `beq +29` (`-> +0x27`) if zero.
* **`+0x0C..+0x26`**: 400 GB branch — loads `slot_400gb` (`0x0BA43B74` at `+0x0E`), stores into `MaxProductLBA`, logs `400GB`, and jumps `bra +41` (`-> +0x4E`).
* **`+0x27..+0x41`**: Default 200 GB branch — loads `slot_200gb` (`0x05D21DBA` at `+0x29`), stores into `MaxProductLBA`, logs `200GB`, and jumps `bra +14` (`-> +0x4E`).
* **`+0x42..+0x4D`**: Fallback path — writes `slot_fallback` at `+0x44` (`0x05D0DDBA` in `3060NP` / `0x04A6D7C8` in `3410PQ`).
* **`+0x4E..+0x5A`**: Loads `d1 = (MaxProductLBA)`, `d0 = (PhysicalBlocks2KB)`, and compares **`cmp d1, d0` (`0xA1`)**.
* **`+0x5B..+0x5C` (`0xC6 0x0C`, `bls +12`)**: Branches to `+0x67` when `d0 <= d1`. **In `dynamic` mode, Panadiga patches `+0x5B` from `0xC6` (`bls`) to `0xCA` (`bra`)**, forcing execution unconditionally to `+0x67` (`d0 - 1 -> (PhysicalBlocks2KB)`) so 100% of any LBA48 HDD capacity (up to `0x3FFFFFFF` x 2 KB blocks) is retained without truncation.
* **`+0x5D..+0x66`**: Original clamp block (`d1 - 1 -> (PhysicalBlocks2KB); bra +10`) that truncated drives > 200 GB.
* **`+0x67..+0x74`**: Clean full-capacity writeback (`28 FF FC 81 <phys_va> FC A4 <phys_va>`).

### 5.2 Native RTSD `Change HDD Capacity` Table (`0x574CA054` / `0x574C267B`)

The service menu `Change HDD Capacity` handler contains 9 hardcoded RTSD entries (`#1` 400 GB to `#9` 80 GB). Note entry **`#3`** officially defines **250 GB as `0x07466528`** 2 KB blocks (`122,053,928` blocks = `488,215,712` LBA 512 B sectors):

| RTSD Index | Preset Label | 2 KB Blocks (Hex) | 2 KB Blocks (Dec) | 512 B LBA Sectors |
| :---: | :--- | :---: | :---: | :---: |
| `#1` | `400GB HDD (RTSD)` | `0x0BA3FB74` | `195,296,116` | `781,184,464` |
| `#2` | `300GB HDD (RTSD)` | `0x08BAEC97` | `146,467,991` | `585,871,964` |
| **`#3`** | **`250GB HDD (RTSD)`** | **`0x07466528`** | **`122,053,928`** | **`488,215,712`** |
| `#4` | `200GB HDD (RTSD)` | `0x05D1DDBA` | `97,639,866` | `390,559,464` |
| `#5` | `160GB HDD (RTSD)` | `0x04A7D7C8` | `78,108,616` | `312,434,464` |
| `#6` | `137GB HDD (RTSD)` | `0x03FC7AC3` | `66,878,147` | `267,512,588` |
| `#7` | `120GB HDD (RTSD)` | `0x037DD1D6` | `58,577,366` | `234,309,464` |
| `#8` | `100GB HDD (RTSD)` | `0x02E8CEDD` | `48,811,741` | `195,246,964` |
| `#9` | ` 80GB HDD (RTSD)` | `0x0253CBE4` | `39,046,116` | `156,184,464` |

### 5.3 Cryptographic Repackaging (`EXTRHEAD` / `EXTRFOOT`) & Mandatory `MASTER` Hardware Rule

* **`panadiga-core::hdd::capacity_patch`** handles `RAW_MAIN_BINARY`, `EXTRHEAD_CONTAINER` (`hdd_partition_firmware_0x02000000.bin`), and `FULL_HDD_IMAGE_P1` (`panasonic DMR EH-55 Firmware hdd_100.bin` — sourced from Internet Archive, not a physical bench drive). Container repacking automatically recomputes deflate payload, strided Adler-32 (`step=16`), `EXTRFOOT`, and 16-round 64-bit Feistel ECB encryption (`unpack_extrhead` round-trip verified), while preserving `zeros_after_header_clean` on factory dumps and only syncing LBA 0 / LBA 57,344 sector counts on procedural `mkfs` images.

> [!CAUTION]
> **Mandatory Hardware Rule (`StarTech IDE2SAT2` Jumper = `MASTER`, NEVER `CS` and NEVER `Slave`):**
> On the **`DMR-EH55`**, schematics (`vol2_schematics.pdf` pp. 46, 52, 113, 125–126) and real firmware disassembly (`eh55_frm_PROG061_BOOT.bin` `FUN_504141df` / `FUN_50414ce8` / `FUN_50414908`, `EH55_MAIN_3410PQ.bin` `FUN_574d1487` / `FUN_574d21ee` / `FUN_574d1de8`) prove that **ODD and HDD are on separate IDE buses** (the `DAT_505733d4` single-cable remapping belongs only to **`DMR-EH58` `BOOT.BIN`**, not `DMR-EH55`):
> - **Primary Channel (`ch = 0`, MMIO `0x89000500`, `Pri`):** DVD-RAM ODD (`acStack_70[0] = 2`, Primary Master) attached internally to the Front-End (`FEP`).
> - **Secondary Channel (`ch = 1`, MMIO `0x89000D00`, `Sec`):** HDD (`acStack_70[2] = 1`, **Secondary MASTER**; `acStack_70[3] = 0`, Secondary Slave unused) on its own dedicated 40-pin `Extension FFC`.
>
> Setting a `StarTech IDE2SAT2` or HDD jumper to **`Slave` (`dev = 1`)** places the disk at unused slot `acStack_70[3] == 0`. Leaving it on **`CS` (`Cable Select`, Pin 28 `CSEL`)** can trigger indeterminate `DEV` selection, `0x8000040d` bus collision, or `MultiWord DMA MODE2` lockups in `FUN_5041f61b` / `FUN_5041f782` (`FUN_5710c6c3` / `FUN_5710c82a`). Power cycling during an IDE hang can corrupt NOR **BGO sectors `0x84200000..0x843FFFFF`**, producing `BGO driver init failed[1].` and `U99`. **Always jumper the HDD / `StarTech IDE2SAT2` explicitly as `MASTER` (`Device 0`) — never `CS` and never `Slave`.**
