> 🌐 **Language / Idioma:** **English (US)** | [Português (BR)](../pt-BR/CATALOGO_E_AUDITORIA_PERFIS_HARDWARE_LINHA_DIGA.md)
> 🧭 **Portal:** [Docs Index](../INDEX.md) | [Complete Audit](../../COMPLETE_AUDIT.md) | [Ghidra Pseudocode](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Debug Logs](../debug_logs/README.md)

# Hardware Profiles v2 Catalog & Audit — Full Panasonic DIGA Lineup (89 Models, `T0` -> `T4`)

**Document:** `DIGA_LINEUP_HARDWARE_PROFILES_CATALOG_AND_AUDIT.md`  
**Scope:** Cross-stack audit of the entire Panasonic DIGA recorder lineup (**DMR-E**, **DMR-ES**, **DMR-EH**, **DMR-EX**, **DMR-EZ**, **DMR-XW**, **DMR-BS/XS**, **DMR-BW/BWT/BST**, and **DMR-UBC/UBS/UBT**), evidence-tier governance (`EvidenceTier`), archival of newly downloaded official Panasonic CDN packages and datasheets, physical dump / `DRV J147` parser fixes, and follow-up engineering improvements.

> **Mandatory Epistemic Rule (`CapabilityGate` & `EvidenceTier`):** No model outside the hardware-verified **`DMR-EH55` (`VEP79138`)** bench target receives live NVRAM/NOR write authorization (`nvram_write_live` / `upper_nor_write_live`). Every tier assignment strictly reflects observed evidence:
> - **`T0` (`community_unverified`)**: forum reports (`remont-aud`, `elektroda`), part numbers without primary schematics/dumps, or unresolved conflicts.
> - **`T1` (`service_manual_only`)**: SM Order Nos., `VEP`/`REP`/`RFKN`/`RFKB` board codes, EEPROMs, and IC specs directly verified in Panasonic Service Manuals or official semiconductor datasheets.
> - **`T2` (`official_frm_verified`)**: official `.FRM` / `.7z` firmware packages verified cryptographically by SHA-256, TOC decryption, and extracted payload inspection.
> - **`T3` (`physical_dump_verified`)**: full chip-off physical dumps verified by SHA-256 and exact byte probes (`DMR-ES35V` `8 MiB` Triton, `DMR-EZ485V` `4 MiB` Magnum/LSI).
> - **`T4` (`bench_verified`)**: reserved strictly for **`DMR-EH55` (`VEP79138`)**, empirically verified over `P7501` UART (`COM10`).

---

## 1. Quantitative Summary of `profiles/` & Archive Expansion

Following a 6-front verification pass (all 30 local PDFs, 24 local `.FRM` containers + `DVDRAM.7z` + `DMR-ES35V`/`DMR-EZ485V` dumps, plus official Service Manuals and semiconductor datasheets across DMR-E/ES/EH/EX/EZ and HD/Blu-ray UniPhier models), the relational `profiles/` registry passed **796 checks** (`panadiga profiles validate` exit code `0`):

| Domain (`profiles/`) | Before | After | Key Entities Added / Upgraded |
| :--- | :---: | :---: | :--- |
| **`soc_cores/`** | `3` | **`5`** | Added `arm_cortex_a9_uniphier` and `arm64_cortex_a53_uniphier` (`3300 mV` LVTTL invariant) |
| **`socs/`** | `6` | **`12`** | Added `mn2ds0010h`, `mn2ds0019` (`T2` proven verbatim in EH58/EX `MAIN.BIN`), `mn2ws0030`, `mn2ws0038`, `mn2ws0230`, `uniphier_ld20_pxs3`; updated `lsi_domino_zebra64` to `DM8635-CHZ` |
| **`flash_chips/`** | `8` | **`18`** | Upgraded `spansion_s29jl032h_4mb_top` (`002-01186` `0x0001/[0x227E,0x220A,0x2201]`) & `oki_mr27v802_1mb_otp` (`FEDR27V802F`) to `T1`; added 10 `T1` parallel NOR ICs (`AM29LV640D`, `S29GL064N` Top/Uniform, `S29GL256P`, `MX29LV160/640`, `MBM29LV160TE`, `M29W160ET/320ET/640GT`) |
| **`eeprom_chips/`** | `4` | **`6`** | Upgraded `rohm_br24l16_2kb` & `microchip_24lc64_optical_cal` to `T1` backed by local PDF datasheets; added `panasonic_c3ebjc000055_dif` (`IC37502` EH80V) & `panasonic_rfkfm4016kt_dif` (`IC9705` EX98V / `IC7404` EH58) |
| **`firmware_trains/`** | `8` | **`18`** | Updated `eh50_ecb64_frm` (`le2011_08e0`), `loader_ver063_bootbin` (`drv=0x9020_2000`, `main=0x5700_0000`) & `toc*`; added 10 `T2` trains (`prog061_mini`, `prog201` 4M/8M, `prog202`, `prog205`, `xw300`, `bs850`, `bs880`, `bs885`, `ex97 PANAEUSB`) |
| **`nor_layouts/`** | `3` | **`3`** | Corrected `ez485v_vxworks_on_mx29lv320b` (`ipl` `0x0000_0000..0x0035_FFFF`, `vendor_nvram` `0x0036_0000..0x003F_FFFF`); enriched `es35v_ver018_on_8mb` |
| **`fixtures/`** | `3` | **`3`** | Aligned all byte probes and SHA-256 digests in `dmr_eh55`, `dmr_es35v`, and `dmr_ez485v` to on-disk dumps |
| **`boards/`** | `6` | **`31`** | Updated all 6 existing boards (`vep79104` `P7501=false`, `vep79108e`, `vep79195` Main vs D-IF `RFKB71134A`, `es35v_digital_board` `VEPV0057A/RFKNES35V*`, `ez485v_lsi_board` `VEP79180/MSC-RFKB79180C`); added 25 `T1` boards |
| **`models/`** | `18` | **`89`** | Audited all 18 existing models + added **`71` new models** across DMR-E (11), DMR-ES (5), DMR-EH (8), DMR-EX (18), DMR-EZ (9), and HD/Blu-ray DIGA (20) |

---

## 2. Critical Physical Safety & Architectural Discoveries

### 2.1 Repurposed `P7501` Hazard (`DMR-EH58` / `DMR-EH59`) and `EX98V` UART Topology
- **Hardware Safety Hazard (`T1` in `DSD0802009CE` / `CHM0903007CE`):** On `VEP79183` Main PCBs (`DMR-EH58`, `DMR-EH59`, `DMR-EH68`), **`P7501`** was **repurposed into an 8-pin connector (`K1KA08A00427`)** carrying switched HDMI power/control (**`+5.8V_AV`** / `XSW+5.8V_AV` toward `IC56105`/`P56101`) — it is **NOT** a `3.3V` UART header! Attaching an RP2040/RP2350 probe to `P7501` on these models risks hardware damage. Both `rfkb79183_eh58` and `rfkb79183aa_eh59` strictly enforce `p7501_header_present: false`.
- **`DMR-ES10` (`VEP79104A`):** Exhaustive search of all 66 pages in [`panasonic_dmr_es10_service_manual_north_america.pdf`](../../manuals_and_schematics/dmr-es10/service/panasonic_dmr_es10_service_manual_north_america.pdf) (`DSD0502037C1`) confirms `P7501` does not exist on the board (`PP7401`/`P9001` 88-pin B2B only) and clears confusion with `VEP79118` (which belongs to `DMR-EX75`/`EX95V`/`EH75V`).
- **`DMR-EH80V` (`MAD0510009CE`):** `P7501` does not exist (`PP7501` is the 10-pin FL connector `K1KA10B00176`); UART lines originate on `IC37501` (`C2CBKH000211`) pins `39/40/15` across `VEP09136G` (`D-IF`) to `P39702`.
- **`DMR-EX98V` (`DSD0806033CE`):** `VEP79195A/B/C` is the **Analog/VHS Main PCB**, while the Digital PCB (`RFKNEX98V*`) is omitted as an exchange module and the `DIG.INTERFACE` board **`RFKB71134A/AT`** hosts `IC9705` (`RFKFM4016K/KT`) alongside the 6-pin JST ZR debug UART header **`P9705`** (`K1KA06AA0188`: `DGND`, `UARTP2M`, `UARTM2P`, `SW UART`, `XRESET`, `D3R3V`).

### 2.2 `DMR-EZ` Architecture Split (Magnum SPARC V8 vs. UniPhier AM34)
- **North America ATSC/NTSC (`DMR-EZ28`, `DMR-EZ48VP`, `DMR-EZ485VP`):** Uses board **`VEP79180`** (`MSC-RFKB79180C`), SoC **`DM8635-CHZ`** (`LSI DoMiNo / Zebra 64` SPARC V8 big-endian, VxWorks 1.3), and Flash `IC8651` (`MX29LV320` 4 MiB bottom-boot) with NV starting at **`0x0036_0000`** (`CLsiPMABS` + `CIncrementalABS` at `0x3D0000`/`0x3F0000`), fixed in [`ez485v_vxworks_on_mx29lv320b.json`](../../profiles/nor_layouts/ez485v_vxworks_on_mx29lv320b.json).
- **Europe DVB-T/PAL (`DMR-EZ49VEB/VEC/VEG`, `DMR-EZ48VEB`):** Runs on **Matsushita UniPhier AM34 (`MN103S`)** — proven via `DSD1002003CE` (`VEP79250A`), `DSD0806025CE`, and local Feistel64 `PANA_DVD.FRM` images (`ez49veb`/`ez49vec` + combined `V107` package shared with `EX86`/`EX83`/`EX773`/`EX99V`), using `PROG201 (8MB)` / `PROG202` distinct from `PROG201 (4MB)`.

### 2.3 NOR -> NAND Transition on HD (`DMR-XW300`) and Blu-ray (`DMR-BS/XS`) Lineups
- All 4 official CDN updates newly archived in [`dmr-xw_bd_series/README.md`](../../firmware_and_images/dmr-xw_bd_series/README.md) (`UPDATE_XW300GN_V17.exe`, `UPDATE_BS850_BS750_XS350EG_V16.exe`, `UPDATE_BS880_BS780_XS380EB_V141.exe`, `UPDATE_BS885_BS785_XS385EG_V140.exe`) decrypt their TOC under key **`le2011_08e0`** (`08E03D859AF9F3EE`).
- The `BS785/BS885/XS385EG` container prepends a 48-byte plaintext **`PANASONIC\0\0\0`** outer header ahead of the encrypted TOC at `0x30`, while module descriptors still store file-relative offsets (`0x2000` and `0x2A000`).
- Every decrypted `PROG0.00` module opens with the **MN103S reset vector (`0xCA 0x10 ... 0xFC 0xDC`)**, yet executes `[SWUP] NAND FLASH` (`nand_base.c`) instead of parallel TSOP-48 NOR — confirming NAND Flash architecture (and standard `zlib.adler32` verification on BS/XS `PROG0.00`).
- Both **`DMR-EX97`** USB packages (`UPDATE_EX97EB_V110.zip` and `UPDATE_EX97SEG_EX97CEG_V105.exe`, `PANAEUSB.FRM` tracked in [`dmr-ex_ez_series/README.md`](../../firmware_and_images/dmr-ex_ez_series/README.md)) use a non-Feistel cipher outside the 12 known `PANA_DVD` keys and remain accurately classified as `T2` container/SHA-256 verified.

### 2.4 Why Digital SoC & NOR Flash Are Omitted in Vol. 1 Service Manuals (`DMR-E85H`, `ES15`, `EH65`, `EX`)
- Scanning all 30 local PDFs returned **0 occurrences** of `MN2DS*`, `S29*`, `MR27V802`, or `C3FB*`: Panasonic treats the `RAM/Digital P.C.B.` (`RFKN*`) as a factory-exchange module in Vol. 1 manuals. Therefore, models such as `DMR-E85H` (unresolved `2 MB` vs `4 MB` flash conflict), `DMR-EH56` (`DSD0603003CE`), and `DMR-EH58`/`EX98V` hold **`T1` on `board`/`eeprom`**, **`T2` on `train`** where official `.FRM`s exist, and strictly retain **`flash_chip`/`nor_layout` at `T0` (`community_unverified`)** until physical dumps or **Vol. 2** schematics are archived.

---

## 3. Newly Archived Packages & SHA-256 Inventory

### 3.1 Official Firmware Archives (`firmware_and_images/`)
All SHA-256 hashes below are pinned in [`dmr-xw_bd_series/SHA256SUMS.txt`](../../firmware_and_images/dmr-xw_bd_series/SHA256SUMS.txt) and [`dmr-ex_ez_series/README.md`](../../firmware_and_images/dmr-ex_ez_series/README.md):
- `dmr-xw_bd_series/xw300gn/PANA_DVD.FRM` (`33,290,728 B`, `193fd4377a1312b4a46aabd60e2449c7349b18c1ed08b1fee228b7befa1c2633`)
- `dmr-xw_bd_series/bs850_bs750_xs350eg/PANA_DVD.FRM` (`45,049,232 B`, `a23cc1288222b2816fb1328c24613686ec890b775c9b44daa91085d56d8f913d`)
- `dmr-xw_bd_series/bs880_bs780_xs380eb/PANA_DVD.FRM` (`43,836,424 B`, `747b607f61dc20a180beac78969cbf2c32743e7636e269110d27a6033f351468`)
- `dmr-xw_bd_series/bs885_bs785_xs385eg/PANA_DVD.FRM` (`40,562,688 B`, `222f213c234c5ec17ba1e026ee6fe9575eade3b8557f257a73b80740d9b30b21`)
- `dmr-ex_ez_series/ex97eb/PANAEUSB.FRM` (`107,417,648 B`, `f62bc64672fe3931a1987ff8d76098ce755e1b4647ad6f3983874c154e892fb2`)
- `dmr-ex_ez_series/ex97seg_ceg/PANAEUSB.FRM` (`112,152,576 B`, `25131e36cd0e1912d7be4d3ce26c222b0670cacb289b197ce60119c0943a65f8`)

### 3.2 Silicon Datasheets (`manuals_and_schematics/datasheets/` — 32 Cataloged PDFs)
Tracked in [`datasheets/SHA256SUMS.txt`](../../manuals_and_schematics/datasheets/SHA256SUMS.txt) and indexed in [`manuals_and_schematics/README.md`](../../manuals_and_schematics/README.md):
- [`rohm_br24l16_datasheet.pdf`](../../manuals_and_schematics/datasheets/rohm_br24l16_datasheet.pdf) (`693,706 B`, `5811033bbd71708dfe2067878945da59ba8c19fa9ba9d1b0ba6fb72d6b3a149b`) — upgrades `rohm_br24l16_2kb` to `T1`.
- [`microchip_24lc64_datasheet.pdf`](../../manuals_and_schematics/datasheets/microchip_24lc64_datasheet.pdf) (`602,424 B`, `af7781c3d84bdf0fac62ed55db5711c55368e821262bd6e49b5be9be6497329e`) — upgrades `microchip_24lc64_optical_cal` to `T1`.

---

## 4. Code & Physical Probe Fixture Fixes Applied

1. **`DRV J147` Offset Fix (`DMR-EH50` `le2011_08e0`):** In `dmr-eh50/PANA_DVD.FRM`, TOC slot 0 (`DRV J147`) places the real payload offset `0x1800` at `+0x08` instead of `+0x0C` (`0xCDF58588`). Updated [`parse.rs`](../../crates/panadiga-core/src/crypto/toc/parse.rs) to normalize slot 0 when `+0x08 == 0x1800` lies within file bounds, verified by unit test `eh50_drv_j147_slot0_offset_quirk_resolves_to_0x1800`.
2. **Fixture Alignment Against On-Disk Dumps (`profiles/fixtures/`):**
   - [`dmr_eh55.fixture.json`](../../profiles/fixtures/dmr_eh55.fixture.json): verified byte probes in `eh55_nor_8mb_ic5001_dump.bin` (`0x0002_FFE7`, `0x0003_0000`, `0x0003_FFE0`, `0x0013_0000`, `0x007F_0000`).
   - [`dmr_es35v.fixture.json`](../../profiles/fixtures/dmr_es35v.fixture.json): fixed 3 stale probes (`0x0002_F6F0`, `0x0003_46AE` `DAVIS`, `0x0070_0000` `0AF2`, `0x0072_0604` `M6720EE`, `0x007F_4000` `01F1`).
   - [`dmr_ez485v.fixture.json`](../../profiles/fixtures/dmr_ez485v.fixture.json): added `expected_sha256` (`6ebb31f2...`) and verified `CLsiPMABS` (`0x0036_0010`) and `CIncrementalABS` (`0x003D_0010` / `0x003F_0010`).

---

## 5. Proposed Roadmap Improvements

1. **Extend `.FRM` Parser for `PANASONIC\0\0\0` 48B Outer Headers & `Adler-32` Verification:** Teach `panadiga-core::crypto::toc::parse` to detect the 48-byte `PANASONIC\0\0\0` outer wrapper used in `BS785/BS885/XS385EG` and validate `Adler-32` checksums on Blu-ray/HD UniPhier payloads.
2. **Schema v2.1 Extension for SLC NAND & eMMC (`flash_chip`):** Post-2009 UniPhier models (`XW300`, `BS/XS`, `BW/BWT`, `UBC/UBS`) use SLC NAND or eMMC instead of parallel NOR with uniform top/bottom boot banks. Adding page/OOB/erase-block metadata to `flash_chip.schema.json` will allow modeling NAND/eMMC while keeping parallel NOR write policies strictly read-only.
3. **`PANAEUSB.FRM` Cipher Reverse Engineering (`DMR-EX97`):** Inspect `libfmupre.so` / ARMv7 UniPhier USB updater loaders to document the AES/container key schedule protecting `PANAEUSB.FRM`.
4. **Priority Vol. 2 / Schematic PDF Acquisition:** Archive `DSD1006036CE` (`DMR-EX93CEG` Vol. 1 & Vol. 2), `panasonic_dmr-ex80seg.pdf` (`DMR-EX80SEG`), `CHM0802005CE` (`DMR-EZ48VP/EZ485VP`), `DSD1002003CE` (`DMR-EZ49V`), and `DSD0806025CE` (`DMR-EZ48VEB`) to expose internal `MN2DS*` / `C3FB*` / `MN2WS*` silicon BOMs.
