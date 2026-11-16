# Panasonic DMR-EZ485V Flash Dump Assets

- **Model:** Panasonic DMR-EZ485V (Gen 4 Combo VHS/DVD DIGA)
- **Flash Memory:** 4 MB (32 Mbit) Parallel NOR Flash (`MX29LV320` TSOP-48) — [`dmr-ez485v-4mb-flash-pack.zip`](https://github.com/orgchrono/Panadiga/releases/download/firmware-pack-v0.1.0/dmr-ez485v-4mb-flash-pack.zip) ([Firmware Pack `firmware-pack-v0.1.0`](https://github.com/orgchrono/Panadiga/releases/tag/firmware-pack-v0.1.0))
- **Dump Size:** 4,194,304 bytes (`panasonic_dmr_ez485v_flash_dump_4mb.bin`)
- **NVRAM Geometry:**
  - Base offset: `0x003F0000` (length - 64 KB)
  - End offset: `0x003FFFFF`
- **Verification:** Verified with `panadiga nvram clean` — surgical sanitization to `0xFF` preserves bootloader at `0x00000000..0x003EFFFF` and clears corruption.
