> 🌐 **Language / Idioma:** **English** | [Português (BR)](../pt-BR/COMPILADO_SCRIPTS_RE_STEPS_01_50.md)
> 🧭 **Portal:** [Docs Index](../INDEX.md) | [Complete Audit](../../COMPLETE_AUDIT.md) | [Audits](../audits/README.md) | [Ghidra Pseudocode](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Debug Logs](../debug_logs/README.md)

# Reference Compendium — RE Python Scripts (`Steps 1–50` and CAD/Bench Utilities)

> Permanent quick-reference index consolidating all **119 Python scripts** generated during the investigation (`54` progressive `step1..step50` scripts + `65` CAD/PDF/bench utilities), avoiding redundant re-execution or recreation of analyses.
> Reusable, CI-tested routines were promoted into permanent packages [`scripts/re/`](../../scripts/re/) and [`scripts/bench/`](../../scripts/bench/).

## 1. Promoted Production Modules (CI + `mypy --strict` + `pytest`)

| Permanent Module | Responsibility | Replaces / Consolidates |
| :--- | :--- | :--- |
| [`scripts/re/mn10300.py`](../../scripts/re/mn10300.py) | `PC=cur_insn+disp` decoder (`0xCA/CC/CD/DC/DD`), `0xFC <imm32>` scanner, `LE32` pointer helpers | Duplicated logic across `step10..step50` (`step15b`, `step41`, `step47`) |
| [`scripts/re/pana_frm.py`](../../scripts/re/pana_frm.py) | `DRV.BIN` decompression, `EXTRHEAD` parser, 62-cmd CLI (`<8sIII`), 256 ATAPI CDBs, Sense LUT `0x400de35c`, `0x34`-stride CP932 dictionary (`0x83A7`) | `step5`, `step8`, `step30..34`, `step44..46`, `step49` |
| [`scripts/bench/protocol.py`](../../scripts/bench/protocol.py) | Safe transport `SerialTransport` (`COM10`) / `MockBenchTransport`, non-destructive allowlist, `BenchSession` with guaranteed `ISOLATE` in `__exit__` | Ad-hoc `COM10` scripts / `test_jigboot_cd.py` |
| [`scripts/bench/decoders.py`](../../scripts/bench/decoders.py) | Parsers for `STATUS` (`GP26 0.02V` bug detection), `PINCHECK`, `BUSDIAG`, and AM34 UART logs (UTF-16LE/UTF-8, `[SDCD]`, `SENSE[.. 83 A7 ..]`) | Standalone parsers and manual `.log` inspection |
| [`scripts/bench/runner.py`](../../scripts/bench/runner.py) | CLI `python -m scripts.bench.runner` (`health`, `capture`, `decode-log`, `smoke-suite`) | Manual Pico bench sessions |

## 2. Complete Catalog of Progressive Scripts (`step1`–`step50`: 54 scripts)

| Original Script (`scratch/`) | Lines | Inspected Targets | Summary / Primary Finding |
| :--- | :---: | :--- | :--- |
| `step1b_targeted.py` | 163 | Service Manual PDF, Live Logs | Inspect `XFWE`, `I2C5`, `P59001`, `P50001`, `P58001` on vol2 p.113, p.115, p.117, p.122, p.125, p.126 |
| `step1_deep_verify.py` | 184 | Service Manual PDF, Live Logs | Search vol2 for relevant keywords across all pages |
| `step2_forensics.py` | 96 | Service Manual PDF | Extract `USER_EXPLICIT` inputs from step 12500 onward in `transcript_full.jsonl` |
| `step3_clean_and_bin.py` | 271 | `PANA_DVD.FRM`, Live Logs | PART A: Manuals & container hygiene |
| `step3_next1.py` | 185 | Live Logs | Locate PDF manuals and cross-reference logs |
| `step4_boot_drv_inspect.py` | 105 | `BOOT.BIN`, `DRV.BIN`, `COM10`/Pico, Service Manual PDF | Inspect all notable strings in EH58 `BOOT.BIN` |
| `step5_decomp_and_crypto.py` | 86 | `BOOT.BIN`, `DRV_decomp`, `DRV.BIN` | Decompress EH58 `DRV.BIN` |
| `step6_drv_boot_deep.py` | 86 | `BOOT.BIN`, `DRV_decomp` | Filter strings that represent real words/identifiers/messages (not raw opcodes) |
| `step7_drv_and_boot_deep.py` | 186 | `BOOT.BIN`, `DRV_decomp`, Service Manual PDF | Establish `DRV_decomp.bin` base address & Sense Table <-> Strings mapping |
| `step8_mecha_83a7_and_boot_re.py` | 151 | `BOOT.BIN`, `DRV_decomp` | Decode Shift-JIS & error tables in `DRV_decomp.bin` (`0x0e3400..0x0ea200` & `0x0de000..0x0de600`) |
| `step9_decode_83a7_and_boot_disasm.py` | 139 | `BOOT.BIN`, `DRV_decomp` | Inspect `DRV_decomp.bin` sections & headers (`0x00000..0x10000`) & pinpoint base address |
| `step10_table_0x83a7_and_boot_disasm.py` | 92 | `BOOT.BIN`, `DRV_decomp` | Dump full `uint16` error code + Shift-JIS string table around `0x0e7800..0x0e8ec0` |
| `step11_trace_83a7_and_boot_disasm.py` | 107 | `BOOT.BIN`, `DRV_decomp`, Service Manual PDF | MN10300 / AM33 structural disassembler & string annotator |
| `step12_toc_mn10300_disasm.py` | 253 | `BOOT.BIN`, `DRV_decomp`, Service Manual PDF | Instant TOC of schematic PDFs |
| `step13_real_disasm_and_pages.py` | 135 | `BOOT.BIN`, `DRV_decomp`, Service Manual PDF | Exact opcode lengths in MN10300/AM33 so 32-bit immediates never misalign |
| `step14_trace_sense56_and_boot_i2c.py` | 96 | `BOOT.BIN`, `DRV_decomp` | Inspect MN10300 opcode table pages 246..265 of cross-assembler manual PDF |
| `step15a_disasm_probe.py` | 140 | `BOOT.BIN`, `DRV_decomp` | Scan for `fc <subop> <u32>` candidates (exact `fc xx imm32` within valid opcode ranges) |
| `step15b_exact_trace.py` | 177 | `BOOT.BIN`, `DRV_decomp` | Table-driven MN10300 / AM33 instruction length+annotation helper for empirical checks |
| `step15c_deep_decode.py` | 167 | `BOOT.BIN`, `DRV_decomp` | Locate all callers of `0x40028ea7` & inspect `0x40028e50..0x40028f40` |
| `step16_finalize_trace.py` | 158 | `BOOT.BIN`, `DRV_decomp` | `DRV`: Sense table `0x400de35c` & MechaErr table around `0x400e8c1e` |
| `step17_finalize_links.py` | 134 | `BOOT.BIN`, `DRV_decomp` | Sanitize output text and finalize cross-references |
| `step18_complete_proof.py` | 111 | `BOOT.BIN`, `DRV_decomp` | Inspect `BOOT` strings across `0x44034100..0x44034f50` |
| `step19_prove_both.py` | 109 | `BOOT.BIN`, `DRV_decomp`, Live Logs | Scan all `.log` / `.txt` captures for `BGO` / `X(8002` / `normal mode` |
| `step20_exact_proof.py` | 118 | `BOOT.BIN`, `DRV_decomp` | `DRV`: Sense `0x38` usage at `0x40012826` & callers of `0x40014434` / `0x400154df` |
| `step21_final_proof.py` | 167 | `BOOT.BIN`, `DRV_decomp`, Live Logs | Inspect initial 35 lines of `normal_boot_90s.log` |
| `step22_verify_hypotheses.py` | 104 | `BOOT.BIN`, `DRV_decomp` | `BOOT`: BGO inner init `0x44001c40..0x44001d4d` & close tray / startup drive `0x44009ba0..0x44009dd0` |
| `step23_trace_proof.py` | 95 | `BOOT.BIN`, `DRV_decomp` | `BOOT`: pre-normal mode, ATAPI packet helper `0x44009b11`, tray/start retries `0x44009dd0..9ee0` |
| `step24_eeprom_and_laser.py` | 87 | `BOOT.BIN`, `DRV_decomp` | `DRV`: EEPROM byte helpers `0x400d71e3`/`0x400d7378`, `0x40085171`, `0x400c6fe0..0x400c7040` |
| `step25_audit_recovery_and_diag.py` | 100 | `BOOT.BIN`, `DRV_decomp` | `BOOT`: mode pin/switch checks, `FEonly` callee `0x44004b06`, pre-Loader F/E uncompress, `[nfrom]` recovery `0x44008bc0..` |
| `step26_modes_and_drive_diag.py` | 125 | `BOOT.BIN`, `DRV_decomp`, `PANA_DVD.FRM` | `BOOT`: `0x44006440..0x44006580`, `0x4400bb00..0x4400bc80`, `0x4400f5b0..0x4400f700` (GPIO) |
| `step27_audit_diag_and_modes.py` | 115 | `BOOT.BIN`, `DRV_decomp`, `PANA_DVD.FRM` | Workspace / `eh58` metadata and boot-mode path audit |
| `step28_deep_diag_map.py` | 129 | `BOOT.BIN`, `DRV_decomp`, `PANA_DVD.FRM`, Live Logs | Summarize `capturas/captura_live_20261112_144101.log` unique events |
| `step29_uart_and_bootmodes.py` | 108 | `BOOT.BIN`, `DRV_decomp` | `BOOT`: `0x44009340..0x44009480` (serial/conveyer boot check), `0x44007f80..0x440080d0` |
| `step30_complete_diag_map.py` | 167 | `BOOT.BIN`, `DRV_decomp` | Exhaustive `BOOT.BIN` boot/diagnostic branch checks |
| `step31_decode_all_gaps.py` | 147 | `BOOT.BIN`, `DRV_decomp` | `BOOT`: exact entry points of `"[SDCD]This SD Card"`, `"*** CD diag boot mode ***"`, and `"SD init failed"` |
| `step32_atapi_and_cli_entry.py` | 127 | `DRV_decomp` | Inspect `0x400df500..0x400df5e4` & full 20-byte CLI table `0x400df5d4..0x400dfa80` |
| `step33_uart_hw_and_cdb_lut.py` | 95 | `DRV_decomp` | Dump ATAPI CDB LUT `0x400de960..0x400deb90` |
| `step34_final_facts.py` | 111 | `BOOT.BIN`, `DRV_decomp` | Decode `CDB[0] -> LUT[cdb] -> 3B / 2B / tbl[idx]` |
| `step35_boot_fmu_sdcd.py` | 118 | `BOOT.BIN`, `DRV_decomp` | `BOOT` `wait 1 second` / `timer correspond initialize` (`0x4400b340..0x4400b420`) |
| `step36_rx_and_sdcd.py` | 77 | `BOOT.BIN` | Inspect UART RX & TX in `BOOT.BIN` (`0x44004760..0x44004860`) and callers of RX functions |
| `step37_verify_all_for_docs.py` | 107 | `BOOT.BIN`, `DRV_decomp`, `PANA_DVD.FRM` | `BOOT` RX wrapper & callers + `fvu.exe` + `fio_recv` |
| `step38_doc_facts_bundle.py` | 238 | `BOOT.BIN`, `DRV_decomp`, `DRV.BIN`, `COM10`/Pico, `MAIN.BIN`, Live Logs | SECTION 1: File hashes & container headers verification |
| `step39_exact_callers.py` | 96 | `BOOT.BIN`, `DRV_decomp`, `DRV.BIN`, `MAIN.BIN` | `BOOT.BIN` exact relative-32 callers |
| `step40_check_rx_and_fmu.py` | 62 | `BOOT.BIN`, `DRV_decomp` | Callers across `BOOT` `0x44004790..0x44004930` |
| `step41_verify_mn103_rel.py` | 98 | `BOOT.BIN`, `DRV_decomp` | Verify key `BOOT` targets using exact `MN10300 PC=cur_insn (va + disp)` |
| `step42_drv_exact_entries.py` | 96 | `BOOT.BIN`, `DRV_decomp` | Walk backward to `fc df` / `cf 10` / `0xf2 fc df` function prologues |
| `step43_final_micro_checks.py` | 82 | `BOOT.BIN`, `DRV_decomp` | Scan prologues (`cf <mask> 3f`, `cf <mask> f8 fe`, `df`/`fc df` boundaries) |
| `step44_sense_japanese_and_bootflags.py` | 74 | `BOOT.BIN`, `DRV_decomp` | `DRV` Error Code -> Shift-JIS table around `0x400e8900..0x400e8ed0` |
| `step45_error_dict_and_sdcd_start.py` | 77 | `BOOT.BIN`, `DRV_decomp` | Scan complete `DRV` Error Dictionary around `0x400e7e00..0x400e8ed0` |
| `step46_decode_remaining.py` | 62 | `BOOT.BIN`, `DRV_decomp`, `DRV.BIN`, `MAIN.BIN` | Inspect `DEV.BIN` & `MAIN.BIN` section structures |
| `step47_final_decode.py` | 157 | `BOOT.BIN`, `DRV_decomp` | `BOOT.BIN`: CD diag -> Drive start failed -> reboot & `FC CC 0x44035602` |
| `step48_verify_callers.py` | 100 | `BOOT.BIN`, `DRV_decomp` | `BOOT` UART RX callers (`0x4400479f`, `0x4400488a`), `0x44007e40` exit block & `0x4400bcc2`/`0x4400b52e` |
| `step49_final_check.py` | 82 | `BOOT.BIN`, `DRV_decomp` | `DRV` console I/O helpers |
| `step50_check.py` | 48 | `BOOT.BIN`, `DRV_decomp` | Exact callers of `0x40028c87` / `0x40028d01` (Sense LUT formatter) and `0x40028ea5` (MechaErr formatter) |

## 3. Vector CAD, PDF Rendering, Bench & Audit Utility Scripts (65 scripts)

> All valid PNGs generated by these crop and inspection scripts have already been curated under [`docs/assets/images/re_dmr_eh55/`](../assets/images/re_dmr_eh55/) and indexed in the MCP visual manifest [`visual_manifest.json`](../assets/visual_manifest.json).

| Utility Script (`scratch/`) | Lines | Inspected Targets | Purpose |
| :--- | :---: | :--- | :--- |
| `annotate_pcb.py` | 25 | Service Manual PDF | Overlay PCB grid coordinates |
| `check_CF_exact.py` | 35 | Service Manual PDF | Inspect p63 bottom half net connections |
| `check_bins_and_parts.py` | 44 | `PANA_DVD.FRM`, Service Manual PDF | Identify non-`FF`/non-`00` transitions and final 64 KB regions |
| `check_p63_p55_rails.py` | 49 | Service Manual PDF | Verify NA `p.55`/`p.63` rect coordinates and page rotation |
| `check_parts_list.py` | 9 | Service Manual PDF | Inspect Vol 1, Page 91 electrical parts list |
| `collect_metrics.py` | 54 | `BOOT.BIN`, `DRV_decomp`, `DRV.BIN`, `PANA_DVD.FRM`, `MAIN.BIN` | Non-destructive exact binary sizes and SHA-256 hashes |
| `crop_ic7404_pins.py` | 7 | Workspace/Scratch | Crop `IC7404` region (`x: 500..750, y: 150..400`) |
| `debug_pdf_vectors.py` | 54 | Service Manual PDF | Align unrotated PDF drawing vectors with text words |
| `deep_manual_and_bin_check.py` | 49 | `PANA_DVD.FRM`, Service Manual PDF | Locate `IC74xx` / `IC75xx` / EEPROM / `IC55xxx` / `TEST MODE` in manuals |
| `dump_ic7401_info.py` | 13 | Service Manual PDF | Dump Vol 1, Page 68 `IC7401` details |
| `dump_ide_drive_pages.py` | 11 | Service Manual PDF | Dump IDE/drive pages in Vol 1 |
| `dump_manual_diag_pages.py` | 11 | Service Manual PDF | Dump self-diagnostic pages in Vol 1 |
| `dump_p117.py` | 9 | Service Manual PDF | Extract Vol 2 Page 117 text |
| `extract_key_manual_pages.py` | 39 | Service Manual PDF | Render schematic/block diagram pages at high DPI for visual inspection |
| `find_digital_ics.py` | 16 | Workspace/Scratch | Enumerate digital ICs across manuals |
| `find_grid.py` | 10 | Service Manual PDF | Inspect text grid layout (`A..C`, `1..3`) on page 74 |
| `find_hw_facts.py` | 14 | Workspace/Scratch | Extract hardware facts from earlier trajectory steps |
| `find_pcb_layouts_and_zoom.py` | 26 | Service Manual PDF | Print TOC/section headers of Vol 1 & Vol 2 (`pp. 50..97`, `110..177`) |
| `generate_re_dossiers.py` | 920 | `BOOT.BIN`, `DRV_decomp`, `DRV.BIN`, `PANA_DVD.FRM`, `COM10`/Pico, `MAIN.BIN`, Live Logs | Generate clean inventory rows from real verified files |
| `guided_recovery_monitor.py` | 46 | `COM10`/Pico, Live Logs | Guided recovery monitor for `DMR-EH55` via Pico on `COM10` |
| `inspect_eh80v_and_wires.py` | 43 | Service Manual PDF | Inspect `eh80v` pages `14, 15, 28, 67, 75, 111, 120, 133` |
| `inspect_ez485v.py` | 15 | Workspace/Scratch | Auxiliary analysis helper |
| `inspect_pcbs_and_logs.py` | 75 | Service Manual PDF | Inspect Vol 2 p.96 Address Information for `P7501` / connectors |
| `inspect_prev_and_rails.py` | 36 | Service Manual PDF | Inspect rail details from planner responses `14500..15005` |
| `inspect_tail.py` | 26 | Workspace/Scratch | Auxiliary analysis helper |
| `investigate_state.py` | 39 | Live Logs | Inventory workspace and Panadiga files |
| `list_pngs.py` | 19 | Workspace/Scratch | Auxiliary image listing helper |
| `make_overview.py` | 29 | Service Manual PDF | Render page 74 (Section 1/4) at 150 DPI |
| `prepare_err_translations.py` | 459 | `DRV_decomp` | Verify all 431 contiguous `0x34` records (`0x400e3736..0x400e8e8e`) |
| `read_mid_hist.py` | 12 | Workspace/Scratch | Inspect user history slice |
| `read_pcb_and_history.py` | 59 | Service Manual PDF | Read exact text around `P7501` on NA `p.76`, `V2 p.96`, `NA p.62`/`p.64`, `V2 p.151/157/173` |
| `read_user_hist.py` | 10 | Workspace/Scratch | Auxiliary analysis helper |
| `render_b3.py` | 16 | Service Manual PDF | Render Grid B-3 (`x: 200..400, y: 320..480`) |
| `render_disasm.py` | 6 | Service Manual PDF | Auxiliary analysis helper |
| `render_exploded.py` | 7 | Service Manual PDF | Auxiliary analysis helper |
| `render_ic7404.py` | 16 | Service Manual PDF | Locate bounding box of `IC7404` |
| `render_ic7404_hires.py` | 12 | Service Manual PDF | High-res crop of `IC7404` |
| `render_map.py` | 7 | Service Manual PDF | Auxiliary analysis helper |
| `render_more_manuals.py` | 27 | Service Manual PDF | Auxiliary schematic rendering helper |
| `render_p32.py` | 7 | Service Manual PDF | Auxiliary page 32 renderer |
| `render_p74.py` | 7 | Service Manual PDF | Auxiliary page 74 renderer |
| `render_strip.py` | 9 | Service Manual PDF | Auxiliary PCB strip renderer |
| `render_timer_all_rails.py` | 33 | Service Manual PDF | Render NA p62 right side (`IC7501` pins 1..25 & top rail feeding `R7506`/`R7569`/`Q7513`/`Q7511`) |
| `render_zoom_ic7404.py` | 26 | Service Manual PDF | Render page 74 at 400 DPI for fine pin labels |
| `scan_all_bins.py` | 20 | Workspace/Scratch | Auxiliary binary scanner |
| `scan_all_service_manuals.py` | 14 | Workspace/Scratch | Auxiliary manual catalog scanner |
| `scan_ez485v_strings.py` | 44 | `PANA_DVD.FRM` | Scan `panasonic_dmr_ez485v_flash_dump_4mb.bin` and Panadiga binaries for strings |
| `scan_manuals_and_user.py` | 46 | Service Manual PDF | Extract `USER_INPUT` steps `12000..12700` verbatim from `transcript_full.jsonl` |
| `search_digital_pcb.py` | 19 | Service Manual PDF | Search Digital PCB PDF pages |
| `search_ic7404.py` | 19 | Service Manual PDF | Search occurrences of `IC7404` |
| `search_manuals_deep.py` | 22 | Service Manual PDF | Deep keyword search across all PDFs |
| `search_service_manuals.py` | 26 | Service Manual PDF | Scan service manuals by term |
| `slice_b3.py` | 15 | Workspace/Scratch | Slice grid B-3 into 2x2 tiles |
| `summarize_inventory.py` | 54 | Workspace/Scratch | Summarize script inventory |
| `summarize_steps_out.py` | 47 | Workspace/Scratch | Extract headings & key metrics from scratch outputs |
| `test_jigboot_cd.py` | 21 | `COM10`/Pico | Capture `JIGBOOT` output over COM port |
| `trace_fp50004.py` | 10 | Service Manual PDF | Locate bounding box of `FP50004` |
| `trace_ic7404_and_manuals.py` | 45 | Service Manual PDF | Extract IC lines from Vol 1 parts list (`pp. 89..97`) |
| `trace_net_coords.py` | 15 | Service Manual PDF | Trace vector lines & net labels on Vol 2 Main Net 1/4..4/4 and Timer 1/4..4/4 |
| `trace_nets_vector.py` | 90 | Service Manual PDF | Match `pg.get_text('words')` coordinates with `pg.get_drawings()` vectors |
| `trace_sw_uart_xreset_5r2v.py` | 50 | Service Manual PDF | Search all pages of NA manual for `X_SW_5R2V`, `SW_5R2V`, `5R2V`, `JC_REG5V`, `NSW5V`, `SW_UART` |
| `trace_tl7401.py` | 12 | Service Manual PDF | Search for `TL7401` |
| `vector_net_p117.py` | 67 | Service Manual PDF | Trace vector net segments originating at `P59001` pins (`x=88..105, y=260..315`) |
| `verify_all_facts_for_dossier.py` | 290 | `BOOT.BIN`, `DRV_decomp`, `DRV.BIN`, `PANA_DVD.FRM`, `COM10`/Pico, `MAIN.BIN`, Live Logs | File hashes & cross-generation correlation |
| `verify_part2.py` | 168 | `BOOT.BIN`, `DRV_decomp` | Verify all 62 `DRV` CLI commands via `<8sIII` struct layout |
