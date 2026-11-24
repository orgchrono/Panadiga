> 🌐 **Language / Idioma:** **English (US)** · [Versão em Português (`pt-BR`)](../pt-BR/GUIA_HARDWARE_STARTECH_IDE2SAT2_E_CABO_FFC_TYPE_B.md) · [Documentation Index](../INDEX.md)

# Hardware Guide: `StarTech IDE2SAT2` SATA Bridge, `P58001` Interface & `Type B` FFC Cable (Case Lid Fitment)

> `[PROVEN ON REAL HARDWARE]` · `[EVD: EV-20261122-019 + EV-20261123-022 + P58001 + StarTech IDE2SAT2]`

This guide specifies the electrical, mechanical, and firmware timing requirements to replace the original PATA hard drive with a 2.5" SATA HDD/SSD in a **Panasonic DMR-EH55LB-S** (`PAL-M`, main digital PCB `VEP79138`, `M6930LB`, `8 MiB` Spansion `S29JL064H` NOR Flash `IC5001`, `Loader ver0.61 [06/03/06]`) using the **`StarTech IDE2SAT2`** bridge and Panasonic's blue **`N5EZZ0000003`** FFC-to-IDE adapter PCB — including replacing the stock `Type A` FFC with a longer **`Type B`** cable to flip the blue PCB `180°` and close the case lid cleanly.

---

## 1. SATA-to-PATA Bridge Compatibility Matrix

| Bridge / Controller | Main IC | Mandatory Jumper | Behavior on Real DMR-EH55 (`VEP79138`) | Verdict |
| :--- | :--- | :--- | :--- | :--- |
| **`StarTech IDE2SAT2`** | **`Marvell 88SA8052`** | **`MASTER`** | Proven on real hardware with a **`SAMSUNG HM251JI`** (`250 GB` SATA) once post-reset timing accommodates bridge readiness (`1000 ms`). | **APPROVED** |
| **Generic `JM20330` bridges** | `JMicron JM20330` | — | Fails DIGA IDE/DMA initialization or proprietary reset/identification timing. | **INCOMPATIBLE / AVOID** |

> [!IMPORTANT]
> You **MUST** set the jumper on the **`StarTech IDE2SAT2`** to **`MASTER`** before mating it with the 40-pin IDE male header on the blue **`N5EZZ0000003`** adapter PCB.

---

## 2. Physical Motherboard Interface (`VEP79138`): `P58001` & `P1503`

Per the official DMR-EH55 Vol. 2 schematic (`manuals_and_schematics/dmr-eh55/service/panasonic_dmr_eh55ec_service_manual_vol2_schematics.pdf`, cataloged in [`manuals_and_schematics/README.md`](../../manuals_and_schematics/README.md)), drive connectivity is split across a 40-pin FFC ATA signal connector and a discrete DC power header:

| Subsystem | Connector | Verified Pinout / Rails | Bench Measurements & Safety Rules |
| :--- | :--- | :--- | :--- |
| **ATA Signal Bus (40-pin FFC)** | **`P58001`** (`VEP79138`) ↔ **`N5EZZ0000003`** | Reversed mapping **`Pin k = ATA(41 - k)`** | `3.3 V` LVTTL signals. Bench DC measurements: **`IORDY = 3.34 V`**, control lines **`2.45–2.47 V`**. |
| **Orientation Diagnostic Mask** | **`P58001`** (`0°` vs `180°`) | Diagnostic signature **`0x00002004`** | Reversing `P58001` (`Pin k` swapped with `41 - k`) prevents drive detection and reports mask/signature `0x00002004`. |
| **Drive DC Power** | **`P1503`** (`VEP79138`) | **`DR5V`** (`+5 V`) & **`DR12V`** (`+12 V`) rails | Verify **`DR5V` / `DR12V`** polarity before power-on; 2.5" SATA drives + `IDE2SAT2` draw only `+5 V` from the bridge's power harness. |

See [`BLUEPRINT_BENCH_WIRING_AND_SCHEMATIC.md`](BLUEPRINT_BENCH_WIRING_AND_SCHEMATIC.md) for overall bench wiring and [`STORAGE_ARCHITECTURE_MEIHDFS_PARTITIONS.md`](STORAGE_ARCHITECTURE_MEIHDFS_PARTITIONS.md) for MEIHFS partition layout.

---

## 3. Case Closure Mechanical Tip: Swap `Type A` FFC for `Type B` to Flip `N5EZZ0000003`

### Why the factory `Type A` (`20.5 mm`) FFC prevents closing the top lid
Panasonic's factory FFC linking `P58001` on `VEP79138` to the blue **`N5EZZ0000003`** adapter PCB (service part **`VEK0K00`** / **`VEK0K15`**) is specified as:
- **`40-pin`**, **`0.5 mm`** pitch, **`UL AWM 20624`**
- **`20.5 mm`** total length (extremely short)
- **`Type A`** (exposed contacts on the **same side** at both ends)

When plugging the **`StarTech IDE2SAT2`** upright onto the 40-pin IDE header of `N5EZZ0000003` using the stiff `20.5 mm` factory cable, the vertical stack protrudes above internal chassis clearance and **prevents closing the DMR-EH55 upper case lid**.

### Validated mechanical fix: `Type B` (`100–150 mm`) FFC + `180°` Adapter Flip

| FFC Parameter | Factory Panasonic Cable | Recommended Case-Closure Replacement |
| :--- | :--- | :--- |
| **Pins & Pitch** | `40-pin`, `0.5 mm` (`UL AWM 20624`) | `40-pin`, **`0.5 mm`** (`UL AWM 20624` or equivalent `60 V` / `80 °C+`) |
| **Contact Orientation** | **`Type A`** (same-side contacts) | **`Type B`** (**opposite-side** / reverse contacts) |
| **Length** | `20.5 mm` (`VEK0K00` / `VEK0K15`) | **`100–150 mm`** |
| **Result on `N5EZZ0000003`** | Pins the blue PCB tight above `P58001` | Allows **flipping the blue `N5EZZ0000003` adapter PCB `180°`**, routing `IDE2SAT2 + 2.5" SATA drive` flat inside the chassis so **the DMR-EH55 case lid closes completely** |

> [!CAUTION]
> **MANDATORY `1:1` MULTIMETER CONTINUITY CHECK BEFORE APPLYING AC POWER:**
> Never rely blindly on a vendor's *"Type A"* or *"Type B"* listing or the visual orientation of the ZIF latch! Because **`Type B`** switches contact faces between ends and you are physically **flipping the `N5EZZ0000003` PCB `180°`**, a mismatched insertion mirrors all `40` pins (`Pin k` ↔ `41 - k`), triggering **`0x00002004`** or risking shorted ATA/GND rails.
>
> **Required check with AC mains disconnected:**
> 1. Seat the new **`100–150 mm` `Type B` FFC** between `P58001` (`VEP79138`) and the flipped **`N5EZZ0000003`**.
> 2. Using a multimeter in continuity (**beep**) mode, verify **`1:1`** pin correspondence from each **`Pin k`** on `P58001` to **`ATA(41 - k)`** on the 40-pin IDE header of `N5EZZ0000003` — confirming identical net connectivity to the stock `VEK0K00 Type A` assembly, while the `180°` board flip achieves flat horizontal clearance.
> 3. Confirm no pin-to-pin bridges or power-to-GND shorts before connecting `P1503` power or AC mains.

---

## 4. `Marvell 88SA8052` Reset Timing (`~555 ms` vs Stock `~2 ms`)

Even with verified `1:1` wiring, the **`Marvell 88SA8052`** requires significantly more post-reset settle time than legacy PATA hard drives:

1. **Measured post-reset timing on `88SA8052`:**
   - Asserts **`BSY`**: **`~32 ms`**
   - Ready with **`DRDY`**: **`~555 ms`**
2. **Stock firmware race condition (`~2 ms`):**
   - Stock **`MAIN`** firmware (`MAIN3060NP` and Brazilian `PAL-M` `MAIN3410PQ`) waits only **`~2 ms`** after IDE reset before probing the drive, racing ahead of `88SA8052` readiness during reset/re-init paths.
3. **Deterministic software fix (`1000 ms`):**
   - [`sata_bridge_fix.rs`](../../crates/panadiga-core/src/re/sata_bridge_fix.rs) extends the post-reset wait to **`1000 ms`** (`CALL rtsd_sleep_10ms(100)`), verified on both `MAIN3060NP` and Brazilian **`MAIN3410PQ`** (`VA 0x5710A9D0 -> CALL 0x574DF538`).
   - Full firmware reverse engineering, `4 MiB -> 8 MiB` BGO retargeting, and Validation Gates 0..4 are documented separately in [`RESEARCH_VALIDATION_GATES_SATA_STAGE2_AND_8MB_BGO_RETARGET.md`](RESEARCH_VALIDATION_GATES_SATA_STAGE2_AND_8MB_BGO_RETARGET.md).

---

## 5. Real-Hardware Proof (`SAMSUNG HM251JI`, `250 GB` SATA)

Live tests via `P7501` (`COM10`, `115200 8N1` — see [`BENCH_GUIDE_P7501_NO_DESOLDERING.md`](BENCH_GUIDE_P7501_NO_DESOLDERING.md)) proved end-to-end operation of **`VEP79138` (`P58001` / `P1503`) + `1:1`-verified FFC + `N5EZZ0000003` + `StarTech IDE2SAT2` (`MASTER`) + `SAMSUNG HM251JI`**:

- **Gate 2 (SD boot with active SATA bridge):** [`boottrace_sata_bridge_gate2_20261122.log`](../evidence/boottrace_sata_bridge_gate2_20261122.log) (`7,536 B`, SHA-256 `f8c1b5699986334f1a97ef004319eaeef880b3447dc4d80d04c1db82c92d6865`) and no-SD trace [`boottrace_sata_bridge_nosd_only_20261122.log`](../evidence/boottrace_sata_bridge_nosd_only_20261122.log) (`9,021 B`, SHA-256 `3120123e29115694e99b51eb1cf120daeb346683b36fc5ad5091026b2a34ac2e`).
- **Gate 3 (`2026-11-23` — MEIHFS mirror write & cold boot directly from SATA HDD):**
  - **Boot 1 (`FVU)` + dual mirror flash):** [`boottrace_sata_bridge_gate3_8mb_boot1_fvu_20261123.log`](../evidence/boottrace_sata_bridge_gate3_8mb_boot1_fvu_20261123.log) (`4,906 B`, SHA-256 `455a86083636cf4bcc2c49d33ffd55be1391e6e9c12edb7ebd15791dbdacd2d3`, `check_sum = 0x32836a32`, `Vefify OK` × 2).
  - **Boot 2 (Direct SATA cold boot behind `StarTech IDE2SAT2`):** [`boottrace_sata_bridge_gate3_8mb_boot2_main_20261123.log`](../evidence/boottrace_sata_bridge_gate3_8mb_boot2_main_20261123.log) (`55,917 B`, SHA-256 `acbee64c1710e015dc32f707bff4acd3ce25cb1224575ab38753a7b3ff95595a`, `39,652` telemetry bytes of `PiE-OS / 3060NP`, cleanly enumerating **`SAMSUNG HM251JI`**, `M6930LB`, VFD **`TM LI`**).

---

## 6. Cross References

- **Technical Research on Gates 0..4 & ~10h Root-Cause Analysis:** [`RESEARCH_VALIDATION_GATES_SATA_STAGE2_AND_8MB_BGO_RETARGET.md`](RESEARCH_VALIDATION_GATES_SATA_STAGE2_AND_8MB_BGO_RETARGET.md)
- **Vol. 2 Service Manual & Schematics:** [`manuals_and_schematics/README.md`](../../manuals_and_schematics/README.md) (`dmr-eh55/service/panasonic_dmr_eh55ec_service_manual_vol2_schematics.pdf`)
- **Project Status & Forensic Ledger:** [`STATUS.md`](../STATUS.md) · [`ledger.json`](../evidence/ledger.json)
