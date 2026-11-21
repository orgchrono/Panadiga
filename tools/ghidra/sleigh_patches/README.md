# Ghidra `GhidraMn10300` SLEIGH Patch Set & 4-Pass 100% Zero-`??` Recovery Plan

> **Read-Only Guarantee:** Never mutate `C:\ghidra\projects\Panadiga_MN103S` in-place. Always clone both the `GhidraMn10300` language directory and the project database (`Panadiga_MN103S` -> `Panadiga_MN103S_ZeroUndef`) before re-analyzing.

## 1. Verified SLEIGH Patch (`am33_panadiga_zero_undef.patch`)

[`am33_panadiga_zero_undef.patch`](./am33_panadiga_zero_undef.patch) is a unified diff generated against **Ghidra 12.1.4 PUBLIC** (`Ghidra\Extensions\GhidraMn10300\data\languages\{mn103.sinc,am33.sinc}`) and **compile-verified cleanly with `sleigh.bat -u -f` (`exit 0`, zero constructor conflicts, zero new warnings)**.

### Defeitos Corrigidos / Patched Defects

| # | File & Line | Opcode / Mnemonic | Stock SLEIGH Defect | Verified Hardware Semantics Fix |
| :---: | :--- | :--- | :--- | :--- |
| **1** | `am33.sinc` (before L142) | `0xF9 0xEA` `:MOVHU (Rm0+), Rn2` | **Missing constructor** (root cause of `3,935` `0xF9` `??` sites in `MAIN` + unrolled memcpy loops in `Loader ver0.61` `0x440242b7..0x440245ab` & `PROG061`) | `Rn2 = zext(*:2 Rm0); Rm0 = Rm0 + 2;` |
| **2** | `am33.sinc:346` | `0xF0 0xC0` `:SYSCALL` & `0xFF` `:BREAK` | **Missing constructors** (`??` at `0x504313E6`, `0x4400159A`, `0x504009E4`, `0x57000D2C`) | `:SYSCALL is op8=0xf0; op8=0xc0 { syscall(0:1); }` and `:BREAK is op8=0xff { mn_break(); }` |
| **3** | `am33.sinc:184` | `0xFB 0x9C` `:SUB Rm2, Rn0, Rd2` (Trap #8) | Inverted operands: `Rm2 - Rn0` (`src - dst`) | `local val:4 = Rn0; update_sub(val, Rm2); Rd2 = val;` (`dst - src`) |
| **4** | `am33.sinc:186-188` | `0xFB/FD/FE 0x98` `:SUB SIMM8/24/32, Rn0` | Erroneously called `update_add(...)` instead of `update_sub(...)` | `update_sub(Rn0, ...)` |
| **5** | `am33.sinc:98` | `0xFE 0x1A` `:MOV Rm2, (IMM32, Rn0)` | Effective address computed from `Rm2 + IMM32` (source data!) instead of base register `Rn0` | `local addr:4 = Rn0 + IMM32; *addr = Rm2;` |
| **6** | `am33.sinc:130-135` | `0xFE 0x2A/3A/AA/BA/2E/3E` `:MOVBU` | `130-133` were `mn_todo()`; `134-135` accessed `*:2` / `Rm2:2` (16-bit instead of 8-bit) | 1-byte `*:1` / `Rm2:1` with `zext` |
| **7** | `am33.sinc:273` | `0xFC 0xFC` `:AND IMM32, "EPSW"` | Bitwise OR (`EPSW | IMM32`) instead of AND | `EPSW = EPSW & IMM32;` |
| **8** | `am33.sinc:293-297` | `0xF9 0x39` `:NOT Rn0` & `0xFB/FD/FE 0xE9` `:BTST SIMM, Rn0` (Trap #10) | Stubbed with `mn_todo()`, hiding bit-15 test loops in BGO FTL (`FUN_50423540`, `FUN_5042358b`) | `Rn0 = ~Rn0; update_ZN_S32(Rn0);` & `update_ZN_S32(Rn0 & ...);` |
| **9** | `am33.sinc:78-83, 177-181, 189-193, 300-321` | `MOV (Rm0+, disp)`, `ADDC`, `SUBC`, `ASR`, `ASL`, `ROR`, `ROL` | Stubbed with `mn_todo()` | Full P-Code semantics (`post-increment`, carry propagation, shifts & 33-bit rotates through `CF`) |
| **10** | `mn103.sinc:519-522` | `0xF0 0x9x` / `0xFA 0xFx` / `0xFE 0x81/01` `:BCLR` (Trap #2) | Used bitwise OR (`temp | ~mask`) instead of AND, turning `BCLR 0xff` into a bogus set-bits / no-op in decompiler | `*:1 addr = (temp & ~mask);` |
| **11** | `mn103.sinc:503` | `0xF2 0x3x` `:NOT Dn0` (Trap #9) | Modeled as `Dm1 ^ Dn0`, collapsing `NOT D0` (`f2 30`) to constant `0` | `:NOT Dn0 is op8=0xF2; op4=3 & op2m=0 & Dn0 { Dn0 = ~Dn0; update_ZN_S32(Dn0); }` |
| **12** | `mn103.sinc:367, 369, 427, 537-538` | `:EXTB`, `:EXTH`, `:SUBC`, `:ROR`, `:ROL` | `EXTB` wrote `MDR` instead of `Dn0`; `EXTH` had broken mask math; `SUBC` called `update_add`; `ROR`/`ROL` were `mn_todo()` | `sext(Dn0:1)`, `sext(Dn0:2)`, `update_sub(Dn0, Dm1 + zext($(CF)))`, 33-bit rotates through `CF` |

---

## 2. 4-Pass Headless Pipeline for 100% Zero-`??` Recovery

Execute the four deterministic passes against a **cloned** project (`Panadiga_MN103S_ZeroUndef`) covering all **6 firmware stages** (`/BOOT.BIN`, `/eh55_loader_ver061_256k.bin`, `/eh55_frm_PROG061_BOOT.bin`, `/DRV_decomp.bin`, `/EH55_DRV_S211.bin`, `/EH55_MAIN_3410PQ.bin`):

### Pass 1 — Isolated SLEIGH Compilation & Database Clone
1. Copy `C:\ghidra\ghidra_12.1.4_PUBLIC\Ghidra\Extensions\GhidraMn10300` into an isolated staging directory (or back up `data\languages\` before patching).
2. Apply the patch and compile `mn103.sla`:
   ```powershell
   git apply --directory=<staged_GhidraMn10300> tools/ghidra/sleigh_patches/am33_panadiga_zero_undef.patch
   C:\ghidra\ghidra_12.1.4_PUBLIC\support\sleigh.bat -u -f <staged_GhidraMn10300>\data\languages\mn103.slaspec
   ```
3. Clone `C:\ghidra\projects\Panadiga_MN103S` -> `C:\ghidra\projects\Panadiga_MN103S_ZeroUndef` and import `firmware_and_images/eh55_loader_ver061_256k.bin` at `0x44000000` alongside the existing 5 binaries.

### Pass 2 — Typed Data, Literal Pool & Padding Separation
Before sweeping orphan code islands, explicitly type non-code regions so linear disassembly never misinterprets data as broken instructions:
- **ASCII / Shift-JIS string tables**: e.g., `0x400E779A` (`0x34`-byte Shift-JIS mecha error table in `DRV`), `[nfrom]` / `[RTSC]` / `[DAVIS]` / `[SDDIAG]` string pools.
- **Structured dispatch & lookup tables**: `<8sIII>` CLI table (`0x400DF5C0`), 256-slot ATAPI CDB table, BGO `0x440337A4` table, Feistel S-Box (`0x44035D84`), jump/pointer tables (`0x44035EB0..0x44035EC7`).
- **Flash/section padding**: trailing `0xFF` runs (Spansion erased pages) and alignment `0x00`/`0xFF` pads between `.text` and `.rodata`, typed as `byte[N]` arrays.

### Pass 3 — Exhaustive Code Island Seeding & Trap #4 Overlap Annotation
1. Seed every valid MN103S code pointer harvested by `panadiga re audit --input <BIN>` (`orphan_code_pointers`, CLI handlers, ATAPI CDB handlers).
2. Sweep remaining undefined gaps `>= 8 B` for standard MN103S prologues (`0xCF <mask>` `MOVM [...],(SP)` followed by `0xF8 0xFE <neg_disp8>` `ADD -N,SP`, or leaf entry targets of `CALL`/`JMP`), disassembling and calling `createFunction()` on verified islands.
3. For **Trap #4** mid-instruction overlaps (`BRA -1` landing inside byte `+1` of an adjacent instruction, detected by `detect_mid_instruction_overlaps`, e.g., `0x5040EA2F` inside `0x5040EA2E`), record an explicit EOL comment + Ghidra bookmark so auditors know the overlap is a compiler density optimization rather than missing code.

### Pass 4 — Decompilation Normalization & Zero-`??` Verification Gate
1. Run [`tools/ghidra/ghidra_mn103_fix_decompilation.py`](../ghidra_mn103_fix_decompilation.py) and [`tools/ghidra/ghidra_auto_unbrick_audit.py`](../ghidra_auto_unbrick_audit.py) headless via `analyzeHeadless`:
   - Enforce `stackPurge = 0` on `CALL target, [], 0x00` callees (`100%` ABI match).
   - Mark MMIO / Flash CFI windows (`0x10xxxxxx`, `0x84xxxxxx`, `0x88xxxxxx`, `0xC0xxxxxx`, `0xD4xxxxxx`) as `volatile` (eliminates Trap #5 dead-code pruning).
   - Re-decompile previously `mn_todo()`-tainted functions (`FUN_50423540`, `FUN_5042358b`, `FUN_5771abfb`, `FUN_4400c696`, `FUN_4400d32f`, `FUN_440236ed`).
2. Run a read-only post-audit verifying for each of the 6 binaries:
   - `undefined_code_bytes == 0` (every initialized byte is either a decoded instruction, typed data/literal/padding, or an explicitly bookmarked Trap #4 mid-instruction alias),
   - `mn_todo_calls == 0` across all core bootloader, BGO FTL, DAVIS IDE, SDDIAG, and DRV/MAIN command paths.

---

## 3. Verified 100% Zero-`??` Results & Unified Rust + JSON SSOT Pipeline

All four passes were executed headlessly against the isolated clone `Panadiga_MN103S_ZeroUndef` using `C:\ghidra\staging\ghidra_12.1.4_PATCHED` (`mn103.sla` SHA-256 `F5BEB23DD91B0C3AD2AF62E40DE30A034B0800B07DD64DD5EED611CC8ABCF72F`). The stock `GhidraMn10300\data\languages\mn103.sla` (`514638E29C76BE3F34474C57FE6E00311952CBD82C6E8B8043168E77ADECD48C`) and `C:\ghidra\projects\Panadiga_MN103S` remained completely untouched.

### Before / After Metrics Across All 6 Stages (`baseline.json` -> `verify.json`)

- Baseline report: [`docs/audits/zero_undef/baseline.json`](../../../docs/audits/zero_undef/baseline.json)
- Final verification gate: [`docs/audits/zero_undef/verify.json`](../../../docs/audits/zero_undef/verify.json) (`exit 0`)

| Program | Undef Bytes (Before -> After) | Instructions (Before -> After) | Functions (Before -> After) | Defined Data (Before -> After) | Error Bookmarks | `mn_todo()` Calls |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| `/BOOT.BIN` | `66,627` -> **`0`** | `66,287` -> `76,337` | `844` -> `1,018` | `888` -> `1,123` | `21` -> **`0`** | **`0`** (`0` in funcs) |
| `/eh55_loader_ver061_256k.bin` | `262,144` -> **`0`** | `0` -> `70,058` | `0` -> `487` | `0` -> `817` | `0` -> **`0`** | **`0`** (`0` in funcs) |
| `/eh55_frm_PROG061_BOOT.bin` | `261,478` -> **`0`** | `240` -> `75,550` | `5` -> `888` | `0` -> `934` | `0` -> **`0`** | **`0`** (`0` in funcs) |
| `/DRV_decomp.bin` | `660,402` -> **`0`** | `273,659` -> `338,305` | `2,346` -> `2,870` | `4,539` -> `10,658` | `26` -> **`0`** | **`0`** (`0` in funcs) |
| `/EH55_DRV_S211.bin` | `1,434,179` -> **`0`** | `3,040` -> `353,745` | `15` -> `1,454` | `0` -> `6,189` | `0` -> **`0`** | **`0`** (`0` in funcs) |
| `/EH55_MAIN_3410PQ.bin` | `12,177,574` -> **`0`** | `284,156` -> `2,712,328` | `33` -> `38,317` | `0` -> `66,676` | `32` -> **`0`** | **`0`** (`0` in funcs) |
| **Total (6 stages)** | **`14,862,404` -> `0`** | **`627,382` -> `3,626,323`** | **`3,243` -> `45,034`** | **`5,427` -> `86,397`** | **`79` -> `0`** | **`0`** |

- **All 6 previously `mn_todo()`-tainted functions decompiled cleanly (`completed: true`, `bad_tokens: []`):** `0x4400C696` (`boot_fat_mount`, `81` lines), `0x4400D32F` (`FUN_4400d32f`, `266` lines), `0x440236ED` (`FUN_440236ed`, `60` lines), `0x50423540` (`19` lines), `0x5042358B` (`24` lines), and `0x5771ABFB` (`main_fpc_status_ctrl`, `50` lines).
- **Verified opcode spot-checks:** `/eh55_loader_ver061_256k.bin` (`0x440242B7 = MOVHU (A0+),D0`, `0x440242BA = MOVHU D0,(A1+)`, `0x4400159B = BREAK`); `/eh55_frm_PROG061_BOOT.bin` (`0x504313E6 = SYSCALL`, `0x504009E4 = BREAK`, `0x5040EA2C = MOVHU D0,(-0x6 ,A3)`, `0x5040EA2F = MOV (0x8 ,A3),D1`); `/EH55_MAIN_3410PQ.bin` (`0x57000D2C = BREAK`).

### Unified Rust SSOT Generator & JSON Map Architecture

The 4-pass pipeline and all memory/stage/model maps are now unified and generated directly from Rust SSOT:

1. **JSON SSOT Maps:**
   - Source schema: [`crates/panadiga-core/src/ghidra/specs/ghidra_maps_ssot.json`](../../../crates/panadiga-core/src/ghidra/specs/ghidra_maps_ssot.json) (`32` AM34 regions synchronized 1:1 with `regions_static.rs`, `5` volatile MMIO windows, `6` firmware stages, `6` tainted VAs, and `generic_stage_rules`).
   - Enriched runtime bundle (projected dynamically across all **`89` DIGA hardware profiles**): [`tools/ghidra/ghidra_ssot_maps.json`](../ghidra_ssot_maps.json).
2. **Rust Generator & CLI:**
   - Implementation: [`crates/panadiga-core/src/ghidra/render/zero_undef_pipeline/mod.rs`](../../../crates/panadiga-core/src/ghidra/render/zero_undef_pipeline/mod.rs)
   - Regenerate all 7 scripts & JSON maps:
     ```powershell
     cargo run -p panadiga-cli -- ghidra export-all --dir tools/ghidra
     # or individually:
     cargo run -p panadiga-cli -- ghidra zero-undef
     cargo run -p panadiga-cli -- ghidra ssot-maps
     ```
3. **Analyzing Arbitrary Future Firmware Binaries (`all` Stage):**
   ```powershell
   C:\ghidra\.venv\Scripts\python.exe tools\ghidra\ghidra_zero_undef_pipeline.py all `
     --binary C:\path\to\NEW_MODEL_MAIN.bin --model DMR-EH58 --project-name MyClonedProject
   ```
   When `--base` is omitted, `ghidra_zero_undef_pipeline.py` infers the load address from `ghidra_ssot_maps.json` (`model_memory_maps` + `generic_stage_rules`), imports the binary safely into the cloned project (still refusing `Panadiga_MN103S`), applies model-projected AM34/MMIO blocks, automatically seeds MN10300 reset-vector branches (`0xDC`/`0xDD`/`0xCA`/`0xCC`) and LE32 header vectors, and runs Passes 2, 3, 4, and the Zero-`??` verification gate end-to-end.
