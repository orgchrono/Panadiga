# Security Policy & Physical Hardware Safety Disclosure

## Reporting Security or Hardware-Safety Vulnerabilities
Please report potential vulnerabilities (including Tauri IPC boundary bypasses, MCP path-jail escapes, RAG injection bypasses, or live Pico/NOR interlock regressions) **privately** via GitHub Security Advisories on `orgchrono/Panadiga` or by contacting the maintainers before opening a public issue.

Do **not** include private bench logs, secret keystores, or unredacted device serials in public reports.

## Supported Versions
| Component | Supported Line |
| :--- | :--- |
| Rust Core / CLI / MCP (`crates/`) | `main` (`0.2.x`) |
| Desktop & Android Workbench (`apps/desktop`) | `main` (`0.2.x`) |
| Pico Probe C11 Firmware (`firmware/pico-p7501`) | `main` (`P0_FW_RO` unified RP2040/RP2350) |

## Non-Negotiable Physical Bench Interlocks
Any bug that weakens the hardware safety gates below is treated as **Critical (P0)**:
- **LVTTL Voltage Cap & `NSW5V` (`P7501` Pin 5 / `GP3`)**: All logic lines operate at `<= 3.3V`; `GP3` must remain `Hi-Z Input` with pulls disabled (never driven push-pull), and `GP5` (`XRESET`) must remain Open-Drain.
- **Standby SMPS / Galvano-Isolation**: Mains SMPS (`Y`-capacitors 60–110 VAC) requires galvanic isolation (`ADuM3160`); live `P7501` writes are gated behind `STATUS` electrical checks (`VDD >= 2.50V`, `NSW5V=1`).
- **3-Zone Spansion NOR (`IC5001` 8 MiB / `SA0..SA141`)**:
  - Layer 1 IPL (`SA0..SA10`, `0x80000000..0x8003FFFF` / `0x84000000..0x8403FFFF`): permanently read-only (`E05`).
  - Layer 2 Upper NOR / BGO (`SA11..SA133`, `0x80040000..0x807EFFFF`): fail-closed (`E05`) unless backed up and explicitly unlocked (`CONFIRM_UPPER_NOR`).
  - Layer 3 NVRAM/EEMUL (`SA134..SA141`, `0x807F0000..0x807FFFFF`): sanitized for surgical `U99` recovery.
- **Lethal MMIO (`0xC000100C`)**: Reads and writes remain permanently blocked across host RSP, firmware, and RAG guardrails.

## Supply-Chain & Binary Verification
- Release archives, `.deb` packages, `.apk` builds, and Pico `.uf2` images are verified via POSIX `SHA256SUMS.txt`.
- Workspace crates enforce `deny.toml` (`cargo deny check`), RustSec (`cargo audit`), `gitleaks`, and pinned toolchain manifests (`rust-toolchain.toml`, `Cargo.lock`, `package-lock.json`).
