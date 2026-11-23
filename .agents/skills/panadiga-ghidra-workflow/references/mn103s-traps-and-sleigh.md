# Catálogo das 10 Armadilhas MN103S, Patch SLEIGH & Errata 3 Vias

## 1. As 10 Armadilhas MN103S / Ghidra

| # | Armadilha | Causa / Evidência Empírica | Resolução / Detector |
| :-: | :--- | :--- | :--- |
| **1** | **Funções Órfãs & Tabelas Indiretas** | CLI `<8sIII>` (`DRV_decomp` `0x400DF5C0`, `S211` `0x400D92A4`), ATAPI CDB e tabelas de ponteiros (`0x5040EA30`) sem xref direta | `panadiga re drv-cli`, `re pana-frm`, `scan_code_pointer_tables` (filtra stub `0x40014559`) |
| **2** | **`BCLR 0xff` / `BSET` Degenerado** | `mn103.sinc:519-522` usava `temp \| ~mask` em vez de `temp & ~mask` (`*param_3 = *param_3;` em `0x5771ac87`/`0x5041f64b`, oculta `0x0CD8`/`0x0D78`) | Patch `am33_panadiga_zero_undef.patch` corrige para `& ~mask` |
| **3** | **Post-Call Block Pruning (`Removing unreachable block`)** | `stackPurge` não modelado esconde ramos reais (ex.: `FUN_574dd15b` esconde `[RTSD] M7920(HDD 400GB)` mostrando só `200GB`) | `ghidra_mn103_fix_decompilation.py` normaliza `stackPurge = 0` |
| **4** | **Mid-Instruction Jump Overlaps (`??`)** | `0x5040ea52: BRA 0x5040ea2f` salta 1 byte antes de `0x5040ea30: MOVHU D1,(0x8408)` (prólogo real `0x5040ea1c`) | Auditor `detect_mid_instruction_overlaps` (`panadiga re traps --kind overlaps`) |
| **5** | **CFI Spansion Eliminado como Dead-Code** | Decompiler colapsa sequências MMIO `0x84200554`/`0x84200AAA`/`0x84200000` em `FUN_440236ed` se não forem voláteis | `ghidra_am34_memory_map.py` marca MMIO/Flash + `Mn103Vm` `CfiStage` |
| **6** | **Loops `SETLB`..`Lcc` & Frames `A3` Negativos** | Laço `SETLB (0xDB)..Lcc (in_LAR - 4)` e acesso de pilha com offset negativo em `A3` (`0x50404F12`) | Detector `panadiga re mn103 --kind setlb` |
| **7** | **Drift Cross-Revision & Shift-JIS (`0x83A7`)** | `PASS` em `0x4013420A` (`DRV_decomp`) vs `0x40130EFA` (`S211`); bytes `a7 83` em `0x4006f5e2` são `CALL`, erro real vive em `Err05mWTable` (`0x400E8C1E`, `0x34` B) | `panadiga re drv-errors` & `re traps --kind revision` |
| **8** | **Inversão `SUB Rm, Rn, Rd` (`fb 9c`)** | `am33.sinc:184` emitia `Rm2 - Rn0` (`src - dst`) em vez de **`Rd = Rn - Rm` (`dst - src`)** em FAT/SD (`FUN_4400c696` etc.) e BGO (`FUN_50423540`) | Patch SLEIGH + `PanadigaMn103sTrapAudit.java` (`--scope=exec`) |
| **9** | **`NOT Dm1, Dn0` (`0xF2 0x3x`) como XOR** | `mn103.sinc:503` gerava XOR; quando `Dm1 == Dn0` (`0x50423550`), `D0 ^ D0 = 0` zera scans de bitmap BGO | Patch SLEIGH corrige para negação bit-a-bit `~` |
| **10** | **`BTST SIMM, Rn0` & `NOT Rn0` com `mn_todo()`** | Ex.: `BTST 0x8000, R0` em `0x5042355e`/`0x504235a7` quebrava P-code | Patch SLEIGH implementa sem `mn_todo()` |

## 2. Patch SLEIGH de 12 Defeitos & Subcomandos `tools/ghidra/`
- **`tools/ghidra/sleigh_patches/am33_panadiga_zero_undef.patch`:** Corrige os 12 defeitos em `mn103.sinc` + `am33.sinc` (inclui `:MOVHU (Rm0+), Rn2` `0xF9 0xEA` que gerava `3.935` `??` no `MAIN`, `:SYSCALL 0xF0 0xC0`, `:BREAK 0xFF`, `MOV Rm2,(IMM32,Rn0)`, `MOVBU *:1`, `SUB` 3-op/`SIMM`, `AND IMM32,EPSW`, `BCLR`, `NOT`, `EXTB/EXTH`, `SUBC`, `ADDC`/`ASR`/`ASL`/`ROR`/`ROL`). Recompilar em cópia isolada via `sleigh.bat -u -f`.
- **Pipeline (`tools/ghidra/ghidra_zero_undef_pipeline.py`):** 4 passadas (`decrypt-runtime`, `import-loader`, `baseline`, `pass2`, `pass3`, `pass4`, `verify`, `all`) exclusivas em projetos clonados.
- **Analistas Java Headless (`tools/ghidra/java/`):** `Mn103sAuditSupport.java`, `PanadigaBootFatSdDecompiler.java`, `PanadigaMn103sDisasmDump.java`, `PanadigaMn103sTrapAudit.java` (executar sempre com `-noanalysis -readOnly`).
- **Geradores SSOT (`panadiga ghidra`):** `map`, `symbols`, `audit`, `fix-mn103`, `zero-undef`, `ssot-maps` (`ghidra_ssot_maps.json` com 32 regiões AM34, 5 janelas MMIO voláteis, 6 estágios, 89 modelos) e `export-all`.

## 3. Errata de Correspondência 3 Vias (`ver0.61` × `PROG061` × `ver0.63`)
| Aspecto | `Loader ver0.61` (EH55 Real `IC5001`) | `PROG061` (`PANA_DVD.FRM`) | `Loader ver0.63` (`/BOOT.BIN` EH58) |
| :--- | :--- | :--- | :--- |
| **Geometria BGO** | `0x440337A4` · **`45` setores (`SA26..SA70`)**, `max_pid=0x0F24` | `0x50436E30` · `35` setores (`0x84200000..0x843FFFFF`) — requer retarget! | `0x44036520` · `35` setores, `24` P2, `0x0A01` |
| **Kit SD Esperado** | `DIGADIAG` `262.144 B` raw (`0x90400000 -> 0x50400000`) + `sd48` (`0x1800`) | Depende do Loader ativo | `DIGADIAG` `131.072 B EXTRHEAD` + `sd64` (`0x2000`) |
| **Falsos `+5B` (NÃO funções)** | `0x44008020` | `0x504238a5` | — |
