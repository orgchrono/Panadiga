# Third-Party Attributions & Legal Scope

This file records external works, SDKs, datasets, and vendor references present in or utilized by the Panadiga workspace. The MIT license in [`LICENSE`](./LICENSE) covers **only** original Panadiga code, schemas, and original engineering documentation (see [`NOTICE`](./NOTICE)).

## 1. Community & Open-Source Assets

| Asset / Component | Origin / Author | License / Terms | Notes |
| :--- | :--- | :--- | :--- |
| `firmware_and_images/Panasonic_DVD_TV.ir` | `Flipper-IRDB` (`emptythevoid`) | Public community IR capture database | Panasonic Kaseikyo IR frame reference used for `SYS_SERV` telemetry and IR Commander validation |
| Raspberry Pi Pico SDK (`RP2040` / `RP2350`) | Raspberry Pi Ltd | BSD-3-Clause | External build dependency for `firmware/pico-p7501/` |
| Ghidra Software Reverse Engineering Suite | National Security Agency (NSA) | Apache-2.0 | Processor sleigh patch (`tools/ghidra/sleigh_patches/`) and headless Jython analysis integration |
| Rust Crate Ecosystem (`Cargo.lock`) | Respective crate authors (crates.io) | Various OSI-approved (MIT / Apache-2.0 / BSD / MPL-2.0) | Pinned in `Cargo.lock` |
| Node / TypeScript Dependencies (`apps/desktop/package-lock.json`) | Respective npm authors | Various OSI-approved | Pinned in `apps/desktop/package-lock.json` |

## 2. Proprietary Vendor References (Not Licensed Under MIT)

- **Vendor Service Manuals & Silicon Datasheets (`manuals_and_schematics/`):** Copyrighted by their respective manufacturers (Panasonic Corporation, Matsushita Electric Industrial Co., Ltd., Spansion Inc., ROHM Semiconductor, Microchip Technology Inc., Raspberry Pi Ltd). Included solely as local engineering references and scheduled for clean-room externalization per [`docs/audits/GIT_FILTER_REPO_PLAN.md`](./docs/audits/GIT_FILTER_REPO_PLAN.md).
- **Vendor Binary Firmware Dumps & `.FRM` Containers (`firmware_and_images/`):** Proprietary Panasonic / Matsushita binaries preserved exclusively for archival forensics, cryptographic verification, and hardware recovery testing. Not redistributed under MIT; see [`docs/audits/GIT_FILTER_REPO_PLAN.md`](./docs/audits/GIT_FILTER_REPO_PLAN.md) for the planned Git history offload workflow.
