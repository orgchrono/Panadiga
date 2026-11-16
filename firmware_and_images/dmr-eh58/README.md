# Panasonic DMR-EH58 Firmware Assets

- **Model / Region:** Panasonic DMR-EH58 (`EH58EE` — Gen 4 DIGA, Eastern Europe/Russia/CIS firmware train)
- **SoC Architecture:** Matsushita UniPhier Gen 2 (`MN2DS0018/22` / MN103S family)
- **Optical / Service Container:** `DVDRAM.7z` (**excluded from Git via `.gitignore` — proprietary firmware binaries are not redistributed in this MIT repository**)
- **Usage:** Local reference firmware container used for cross-model static reverse engineering (`BOOT.BIN`, `DRV.BIN`, `MAIN.BIN`).

## Verified `DVDRAM.7z` Manifest (Copyright-Excluded Local Artifact)

- **Size:** `7,179,299` bytes (`0x6D8C23`)
- **MD5:** `b24750771125d45e5672a4bd66d92aa7`
- **SHA-256:** `d58297d6581551a95912ea88b47a6f8c4c291a841b7dd7eb6e47e2b8dcbcea31`
- **`VERSION.TXT` Metadata:**
  - `MODEL,EH58EE`
  - `OEM,M`
  - `BOOT_VER,0.63`
  - `DEV_VER,3110PC`
  - `DRV_VER,@ 0F`
  - `MAIN_VER,3110PC`
- **Archive Contents:** `BOOT.BIN`, `BOOT.MD5`, `DEV.BIN`, `DEV.MD5`, `DRV.BIN`, `DRV.MD5`, `MAIN.BIN`, `MAIN.MD5`, `VERSION.TXT`

> **Region & Recovery Warning:** Do not cross-flash `EH58EE` firmware onto non-EE hardware variants (`EG`, `EB`, `EF`, `EP`, `GN`, etc.) without verifying regional PCB strapping and `DEV.BIN` model identifiers. For `U99` NVRAM corruption or optical/HDD update recovery workflows, see [Technical Inventory & Operations Manual](../../docs/en-US/TECHNICAL_INVENTORY_AND_OPERATIONS_MANUAL.md).
