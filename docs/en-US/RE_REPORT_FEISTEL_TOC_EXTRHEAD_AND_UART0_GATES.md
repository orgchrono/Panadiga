> 🌐 **Language / Idioma:** **English** | [Português (BR)](../pt-BR/RELATORIO_RE_FEISTEL_TOC_EXTRHEAD_E_GATES_UART0.md)
> 🧭 **Portal:** [Docs Index](../INDEX.md) | [Complete Audit](../../COMPLETE_AUDIT.md) | [Audits](../audits/README.md) | [Ghidra Pseudocode](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Debug Logs](../debug_logs/README.md)

# Reverse Engineering Report: Feistel Cipher (`FUN_4400a666`), `.FRM` Containers (`EXTRHEAD`), and UART0/Debug Gates (`DMR-EH55`)

> **Scope & Integrity:** This document persistently consolidates the empirical discoveries obtained from MN103S disassembly of `BOOT.BIN` (**Track A**) and `EH55_MAIN_3410PQ.bin` (**Track B**), as well as their modular materialization in Rust (`panadiga-core` / `panadiga-cli`) and Python (`scripts/re/pana_frm.py`). Every address, key, table, and invariant described here has been validated directly against `firmware_and_images/PANA_DVD.FRM` and the automated workspace test suite.

---

## 1. Track A — 64-Bit Feistel Cipher (`BOOT.BIN`)

The protection layer used in `PANA_DVD.FRM` and the HDD firmware partitions (`MEIHDFS`) employs a 64-bit Feistel block cipher in ECB mode with 16 rounds, whose main routine in `BOOT.BIN` resides at `FUN_4400a666`.

### 1.1 Extracted Cryptographic Artifacts

| Component | Origin in `BOOT.BIN` / `PROG061_BOOT` | Symbol in `panadiga-core::constants` | Value / Notes |
| :--- | :--- | :--- | :--- |
| Key Part 1 (`7 B`) | `.data` `0x50480278` (off `0x39AA8`) | `PANA_FEISTEL_KEY_PART1` | `72 EB CA 91 30 45 68` |
| Key Part 2 (`8 B`) | `.data` `0x5048027F` (off `0x39AAF`) | `PANA_FEISTEL_KEY_PART2` | `AD 99 DF 56 72 3A B8 10` |
| Master Key (`7 B`) | Reconstructed (`reconstruct_master_key_7b`) | `PANA_FEISTEL_MASTER_KEY_7B` | `DF 62 15 C7 52 7F D0` |
| Key Schedule IV (`4 B`) | `FUN_44011d42` | `PANA_FEISTEL_KEYSCHED_IV` | `35 7D FB 9A` |
| Expanded Round Key (`32 B`) | Output of `FUN_44011d42` | `PANA_FEISTEL_ROUND_KEY_32B` | `A3 E7 9C 01 ... 25 51 F3 15` |
| Official S-Box (`256 B`) | `0x44035D84` (`BOOT`) / `0x50435C0A` (`PROG061`) | `PANA_FEISTEL_SBOX_256` | 256-byte nonlinear permutation/substitution table |

### 1.2 Key Reconstruction, Key Schedule, and Feistel Round

1. **7-Byte Master Key Reconstruction (`reconstruct_master_key_7b` in [`crates/panadiga-core/src/crypto/feistel.rs`](../../crates/panadiga-core/src/crypto/feistel.rs)):**
   - For `i` in `0..7`: `master[i] = p2[i] ^ p1[i]`.
   - Then: `master[1] ^= p2[7]` and `master[4] ^= p2[7]`.
2. **F-Round Function (`FUN_44012191` / `feistel_f_round`):**
   - Mixes the 4-byte half-block `[b0, b1, b2, b3]` with 4 subkey bytes `[k0, k1, k2, k3]` using right rotations (`rotate_right(2)` and `rotate_right(5)`), bit shifts, and `PANA_FEISTEL_SBOX_256` lookups, adding `s.rotate_left(3..0)` modulo $2^8$ into the 4 outputs.
3. **Subkey Expansion (`FUN_44011d42` / `derive_round_key_32b`):**
   - Copies the 7 bytes of `master` to the start of buffer `rk[0..32]` and initializes `state = PANA_FEISTEL_KEYSCHED_IV`.
   - Runs 16 iterations indexing `off = (round & 7) * 4`, replacing `rk[off..off+4]` with the output of `feistel_f_round` and chaining `state`.
4. **8-Byte Block Encryption / Decryption (`FUN_44011ec0` / `FUN_44012029` / `FUN_4400a666`):**
   - Splits the block into `L[0..4]` and `R[0..4]`. Encryption loops `round = 0..16`; decryption runs in reverse order `round = 15 - step`.

### 1.3 Strided Adler-32 (`FUN_4401e5a3`)

The `EXTRHEAD` header validates decompressed payloads using a **strided Adler-32** variant, implemented at `FUN_4401e5a3` and ported to `adler32_panasonic_strided`:

- `MOD_ADLER = 65521`, `s1 = 1`, `s2 = 0`.
- Iterates through `data` with step `stride = step.max(1)` (default `PANA_EXTRHEAD_DEFAULT_ADLER_STEP = 16`):
  - `s1 = (s1 + byte) % 65521`
  - `s2 = (s2 + s1) % 65521`
- Returns `s1 | (s2 << 16)`.

---

## 2. Structure of `.FRM` Containers and `EXTRHEAD`/`EXTRFOOT`

### 2.1 `PANA_DVD.FRM` (Encrypted TOC in 48-Byte Slots)

The `"Firm list"` header region spans the initial `0x1800` bytes (6,144 B) encrypted with `FUN_4400a666`, composed of **48-byte (`0x30`)** slots validated by summation `list_sum` (`u16` LE over the first 46 bytes):

| Slot | TOC Offset | Module | Payload Offset | Size (`B`) | Hardware | Model | `list_sum` |
| :---: | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `[0]` | `0x0000` | `DRV S211` | `0x00001800` | `0x000B0000` (`720,896`) | `M6930` | `NP-309` | `0x0AE1` (`ok=true`) |
| `[1]` | `0x0030` | `PROG061 ` | `0x000B1800` | `0x00040000` (`262,144`) | `M6930` | `NP-309` | `0x08F9` (`ok=true`) |
| `[2]` | `0x0060` | `MAIN3060NP` | `0x000F1800` | `0x0063D4E0` (`6,542,560`) | `M6930` | `NP-309` | `0x0B26` (`ok=true`) |

Each entry also stores `fvu_checksum_u32` (byte sum of the module at `+0x24`), `version_u16`, `target_flag`, and `model_group`.

### 2.2 `PANA__SD.FRM` (`sd48` vs `sd64` Plaintext) and `DIGADIAG.FRM` (`RAW_VER061` vs `EXTRHEAD`)

- **`PANA__SD.FRM` (`sd48` EH55 vs `sd64` `/BOOT.BIN`):** Real **DMR-EH55 hardware (`Loader ver0.61` + `PROG061` `FUN_5040ebfe`/`FUN_5040ed7c`)** uses up to `128` plaintext slots of **`0x30` (`48` B)** (`PANA_SD48_TOC_REGION_SIZE = 0x1800` / `6,144` B, `list_sum` u16 at `+0x2E`). By contrast, **`/BOOT.BIN` `ver0.63`** (`FUN_4400a733` / `FUN_4400a6d0`, EZ485V/EH58) uses `128` plaintext slots of **`0x40` (`64` B)** (`PANA_SD_TOC_REGION_SIZE = 0x2000` / `8,192` B, `list_sum` u16 at `+0x3E`; `"_fmu_error: list sum is invalid"` on mismatch).
- **`DIGADIAG.FRM` (`RAW_VER061` vs `build_digadiag_frm` `EXTRHEAD`):** On real **DMR-EH55 hardware (`Loader ver0.61 [06/03/06]`)**, `SDDIAG` reads **exact `0x40000` (`262,144`) raw uncompressed bytes** directly to **`0x90400000`** (`JMP 0x50400000`). On **`Loader ver0.63 [07/10/24]` (`/BOOT.BIN`)**, it expects a fixed **`0x20000`-byte (`131,072 B`)** plaintext `EXTRHEAD`/`EXTRFOOT` container (`algo = 0`, zlib, destination `0x50400000`, staged at `0x93FC0000`).

### 2.3 `EXTRHEAD`/`EXTRFOOT` Encapsulation with Partial Encryption (`extrhead.rs`)

- **64-byte header (`0x40`):** magic `"EXTRHEAD"`, `tag` (4 B), `flag` (`u16`), `algo` (`u16`: `0` = zlib, `3` = LZMA in `FUN_4400b6c6`), `dst_len` (`+0x10`), `dst_addr` (`+0x14`), `src_len` (`+0x18`), `footer_offset` (`+0x20`), `header_checksum` (`+0x28`), `adler_step` (`+0x2C`), `build_name` (`+0x30..0x40`), followed by compressed stream + `"EXTRFOOT"` footer + 8-byte padding.
- **Partial encryption modes:**
  - `main` (`MAIN`): encrypts/decrypts only the prefix of `0x19000` bytes (`PANA_EXTRHEAD_MAIN_CRYPT_SPAN = 102,400 B`) and the aligned trailing suffix of `0x19000` bytes prior to `aligned_end`.
  - `drv` (`DRV`): encrypts/decrypts only the prefix of `0x2800` bytes (`PANA_EXTRHEAD_DRV_CRYPT_SPAN = 10,240 B`), leaving the rest in plaintext (including `"EXTRFOOT"`).
  - `plain`: no Feistel encryption (`DIGADIAG.FRM` and unencrypted images).

---

## 3. Modular Architecture in Rust and CLI (`panadiga-core` & `panadiga-cli`)

Initial Python prototypes were ported to focused Rust modules inside crate `panadiga-core`:

- [`crates/panadiga-core/src/crypto/feistel.rs`](../../crates/panadiga-core/src/crypto/feistel.rs) — master key, `feistel_f_round`, `derive_round_key_32b`, ECB block/buffer cipher, and `adler32_panasonic_strided`.
- [`crates/panadiga-core/src/crypto/codec.rs`](../../crates/panadiga-core/src/crypto/codec.rs) — binary helpers (`pad_to_block`, LE read/write, and ASCII strings).
- [`crates/panadiga-core/src/crypto/extrhead.rs`](../../crates/panadiga-core/src/crypto/extrhead.rs) — parser, `pack_extrhead`, and `unpack_extrhead` with `ExtrheadCryptMode::{Auto, Main, Drv, Plain}`.
- [`crates/panadiga-core/src/crypto/frm_pack.rs`](../../crates/panadiga-core/src/crypto/frm_pack.rs) — `build_digadiag_frm` and `pack_frm` (`FrmContainerMode::{Dvd, Sd}`).
- [`crates/panadiga-core/src/crypto/toc.rs`](../../crates/panadiga-core/src/crypto/toc.rs) — structural analysis, `decrypt_and_parse_frm_toc`, `parse_sd_frm_toc`, and `dump_toc`.
- [`crates/panadiga-cli/src/cli/handlers/frm_pack.rs`](../../crates/panadiga-cli/src/cli/handlers/frm_pack.rs) and [`args/crypto.rs`](../../crates/panadiga-cli/src/cli/args/crypto.rs) — subcommands `panadiga frm pack` (`--mode sd|dvd|digadiag`, `--out`, `--module NAME:PATH`, `--input`, `--dst-addr`, `--tag`, `--build`, `--hw`, `--model`, `--version-u16`, `--target-flag`, `--json`) and `panadiga frm unpack` (`<EXTRHEAD_FILE>`, `--out`, `--mode auto|main|drv|plain`, `--json`).

---

## 4. Track B — Hardware Gates and UART0/Debug Console (`EH55_MAIN_3410PQ.bin`)

Ghidra analysis of the main firmware binary `EH55_MAIN_3410PQ.bin` mapped the hardware pins, state variables, and RTOS tasks controlling serial input/output (`SC0TXB` / `SC0RXB` at `0xD4002000..0xD400200C`) and the exception monitor.

### 4.1 Matsushita GPIO MMIO Mapping (`0x57a52900..0x57a52c00`)

| String in Table | String Address | MMIO Register | Observed Role |
| :--- | :--- | :--- | :--- |
| `"P0MD"` | `0x57a52bd1` | `0xDB000000` | Port 0 pin mode / direction control |
| `"P0IN"` | `0x57a52bcc` | `0xDB000004` | Logical read of physical Port 0 pins (`bit 1 = P01`) |
| `"P0OUT"` | `0x57a52bc6` | `0xDB000008` | Port 0 output register |
| `"P0TMIO"` | `0x57a52bbf` | `0xDB00000C` | Port 0 Timer/IO control |
| `"P1MD"` | `0x57a52bba` | `0xDB000100` | Port 1 mode control |

Bootloader `BOOT.BIN` also references the table via `&PTR_P0MD_440356f0` (offsets `0x356f0`/`0x356f4`, configuring `P0MD`/`P0IN`).

### 4.2 Bootstrap Chain and UART0 Console Gates (`SC0`)

1. **`FUN_570007b4` (`0x570007b4`, 130 B, called from crt0/reset `0x5700042a`):**
   - Clears bits `[3:0]` of `P0MD` (`0xDB000000 &= 0xFFFFFFF0`), reads `P0IN` (`0xDB000004`), and tests `BTST 0x2, D3` (bit 1 = `P01`).
   - If `P01 == 0`, also executes a write probe to development RAM (`0x80000000 = 0xA5C30000`).
2. **`FUN_57000dfa` (`0x57000dfa`, 161 B) and flags at `0x934E2EAC`:**
   - Controls bits `0x18` and `0x20` of `uRam934e2eac` and signature `iRam934e3350 == 0xAA01`.
   - When `(in_D2 & 0x18) != 0` and `(in_D2 & 0x18) != 8`, calls `FUN_57001fee()` (debug / UART0 init routine at `0xD4002000` and `0xD4003002`). If `(uRam934e2eac & 0x20) == 0x20` and `iRam934e3350 != 0xAA01`, enters an infinite stall loop (`TM0MD = 1`).
3. **`FUN_574ef7b0` (`0x574ef7b0`, 34 B) → `FUN_574ef7d2` (`0x574ef7d2`):**
   - `FUN_574ef7b0`: clears bits `[3:2]` of `P0MD` (`Ramdb000000 &= 0xFFF3`), reads `P0IN`, and returns `((P0IN >> 1) ^ 1) & 1`.
   - `FUN_574ef7d2`: executes `bRam515b2e39 |= 0x0A` (`0b1010`), calls `FUN_574ef7b0()`, and writes **`uRam515b2e38 = (ret == 0)`**. In short: when **`P01` is HIGH (`1`)**, `FUN_574ef7b0` returns `0` and **`0x515b2e38 = 1` (UART console enabled)**; on retail boards where `P01` is grounded/LOW (`0`), `0x515b2e38 = 0`.
4. **`FUN_574efb97` (UART menu init) & `FUN_574efdda` (`printf`) / `FUN_574f00eb` (`getchar`):**
   - `FUN_574efb97` (called by `0x574f46ba`, `0x57010c09`, `0x570cd208`) sets `0x515b2e39 = 1`, calls `FUN_574ef7d2`, and checks `BTST 0x2, (0x515b2e39)` (bit 1 set via `|= 0x0A`, yielding `0x0B`), initializing the UART0 ring buffer at `0x5316A250..0x5316AE88` and writing `0xC085` to `0xD4002000`.
   - `FUN_574efdda` transmits only when `cRam522bbd54 == 0` (mute off), `(bRam515b2e39 & 3) == 3`, and (`cRam515b2e38 != 0` or `FUN_5770cb76(0) == 0`). `FUN_574f00eb` blocks if `(bRam515b2e39 & 3) != 3`.
5. **RTOS Task Descriptors & Debug Shell (`Check Module` and `ndb>`):**
   - **`0x577E1C30` → `FUN_570cd1fc` (`"Check Module ver1.02"` / `2005/12/16 DRS` at `0x5788F042` / `0x5788F0DF`):** character-driven interactive menu task (`id/prio 0x001F0013`), configured in the RTOS table with **`auto_start = 0`**.
   - **`0x577E22C0` → `FUN_574f469d` (`"Please Wait ... \n"` / `"IIC_init\n"`):** calls `FUN_574efb97` at `0x574f46ba` (`id/prio 0x00010059`), configured with **`auto_start = 1`**.
   - **`FUN_574ae7dc` (prompt `\x1b[31mndb> \x1b[m` at `0x57A53B89`):** low-level monitor (`[task_id] bt`, hex/register dump) invoked at `0x574aebcc` after the error/exception handler (`Unknown Error`, `NMICR:%08X, sISR:%08X`, `Register Dump:`), writing `0xFF` to `0x20000044` on exit. Flag `0x515b2e38` also gates `System assert!!` handlers at `0x570b03ab` and `0x570b03f5`.

---

## 5. Infrastructure Fixes, Headless Ghidra Daemon, and Quality Gates

- **Quality Gates Suite ([`scripts/validate.py`](../../scripts/validate.py)):** Verified at **9/9 Quality Gates PASS** as of historical commit `a561249` (Rust workspace/fmt/tests/Ghidra export, TypeScript `tsc`, Vitest, `jscpd` 0% clones, UX state lint, Playwright E2E, C11 host tests, Python `mypy --strict`, `pytest`), with cross-builds validated for RP2040/RP2350, macOS, Ubuntu, and Android APK.
- **Headless `ghidra-mcp` Server (`BOOT.BIN`):** Persistent daemon at `http://127.0.0.1:8081/mcp` (`C:\ghidra\start_ghidra_mcp_headless_boot.cmd`) kept operational over HTTP 200 SSE stream without lock contention on project `Panadiga_MN103S`.
