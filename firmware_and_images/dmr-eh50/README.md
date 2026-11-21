# Panasonic DMR-EH50 Firmware Assets

- **Model:** Panasonic DMR-EH50 (Gen 3 Early DIGA)
- **SoC Architecture:** Matsushita UniPhier MN2DS0016 (AM34 RISC)
- **Optical Container:** [`dmr-eh50-PANA_DVD.FRM`](https://github.com/orgchrono/Panadiga/releases/download/firmware-pack-v0.1.0/dmr-eh50-PANA_DVD.FRM) (Extracted from official [`UPDATE_08_07_2006_DMR_EH50.zip`](https://github.com/orgchrono/Panadiga/releases/download/firmware-pack-v0.1.0/dmr-eh50-UPDATE_08_07_2006_DMR_EH50.zip), 7.22 MB — [Firmware Pack `firmware-pack-v0.1.0`](https://github.com/orgchrono/Panadiga/releases/tag/firmware-pack-v0.1.0))
- **Cryptographic & TOC Characteristics:**
  - 64-bit ECB Matsushita cipher using key **`le2011_08e0`** (`08E03D859AF9F3EE`)
  - Verified Dvd48B modules: **`DRV J147`** (`0x1800`, `623,056 B` — note slot-0 stores offset `0x1800` at byte `+0x08` instead of `+0x0C`), **`PROG019`** (`0x999D0`, `196,608 B`, `05/07/19`), **`MAIN3360NA`** (`0xC99D0`, `6,400,636 B`, LZSS `0x5700_0400`, HW code **`M7820`** / **`NA-348`**)
  - Erased NOR Flash block: `79BB2C325AD4FD34` (4,107 occurrences)
  - Null padding block: `B3A25A0B9D864F08` (178 occurrences)
  - Global Header: `492F0560CEDE508CE3C08AFD49B64A9A` (16 bytes)
- **Usage:** Analyze with `panadiga frm analyze firmware_and_images/dmr-eh50/PANA_DVD.FRM`.
