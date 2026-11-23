> 🌐 **Idioma / Language:** **Português (Brasil)** · [English Version (`en-US`)](../en-US/ANTI_TRAP_BGO_GEOMETRY_AND_SSOT_FORENSIC_ATLAS_GUIDE.md) · [Índice de Documentação](../INDEX.md)

# Guia Anti-Armadilhas de Geometria NOR/BGO (`4 MiB` vs `8 MiB`), SSOT Fail-Closed & Atlas Forense `100%` Ghidra

> `[PROVADO EM HARDWARE REAL]` · `[EVD: Panadiga_MN103S_ZeroUndef + profiles/ + validate_bgo.rs]`

Este dossiê consolida as lições de engenharia reversa sobre diferenças físicas de tamanho/geometria **Flash NOR / BGO FTL** (`4 MiB` vs `8 MiB`), documenta as novas checagens fail-closed em Rust e Python na SSOT (`profiles/`) e apresenta o **Atlas Forense Auto-Atualizável** para os binários `100%` disassemblados pelo Ghidra MN103S.

---

## 1. Post-Mortem da Armadilha Crítica: Geometria BGO `4 MiB` vs `8 MiB`

Durante o retargeting de código de atualização de fábrica (`PROG061` / `MAIN` / `/BOOT.BIN`) para o **DMR-EH55 real (`IC5001 Spansion S29JL064H` de `8 MiB`)**, emergiu a armadilha recorrente de assumir que toda tabela BGO utiliza os mesmos endereços ou a mesma contagem de setores/capacidade de chip Flash:

| Parâmetro | `Loader ver0.61` Real (`EH55`) | `PROG061` Stock (`0x50436E30`) | `/BOOT.BIN ver0.63` (`0x44036520`) |
| :--- | :--- | :--- | :--- |
| **Flash Alvo** | `S29JL064H` (`8 MiB`, `SA0..SA141`) | `4 MiB` Top-Boot donor | `S29JL032H` (`4 MiB`, Top-Boot `SA0..SA70`) |
| **Base BGO (`word[12]`)** | **`0x84130000`** (`SA26`) | `0x84200000` | `0x84200000` |
| **Teto BGO (`word[13]`)** | **`0x84400000`** (exclusivo) | `0x843F0000` + Bank 2 `8×8K` | `0x843F0000` |
| **Setores Físicos (`word[31]`)** | **`45` (`0x2D`) × `64 KiB`** (`2.880 KiB`) | `31 × 64 KiB` + `8 × 8 KiB` (`35` lóg.) | `29` (`0x1D`) |
| **`EEMUL` NVRAM** | `SA134..SA141` (`0x847F0000..0x847FFFFF`) | Upper top-boot | Top-boot boundary (`0x3F0000`) |

### Outras Armadilhas Relacionadas Catalogadas em [`profiles/traps/geometry_and_binary_traps.json`](../../profiles/traps/geometry_and_binary_traps.json)
1. **`TRAP_TOP_BOOT_NON_UNIFORM_SECTORS`:** `S29JL032H` (`63×64K + 8×8K`) vs `S29JL064H` (`8×8K + 126×64K + 8×8K`) exigem derivação exata por `derive_sector_table(flash)`.
2. **`TRAP_ES35V_MAIN_XIP_VS_EH55_BGO`:** Em `DMR-ES35V`, `0x130000..0x40FFFF` é código **`MAIN_XIP`** (não BGO); BGO fica em `0x700000..0x7EFFFF`.
3. **`TRAP_EZ485V_4MB_SPARC_VENDOR_NVRAM`:** `DMR-EZ485V` (`4 MiB` MX29LV320 bottom-boot SPARC) usa `vendor_nvram CLsiPMABS` em `0x360000..0x3FFFFF`.
4. **`TRAP_DIGADIAG_256K_RAW_VS_128K_EXTRHEAD` & `TRAP_PANA_SD_TOC_SD48_VS_SD64`:** `Loader ver0.61` exige `DIGADIAG.FRM` cru (`262.144 B`) + `PANA__SD.FRM sd48` (`0x1800`), enquanto `ver0.63` usa `EXTRHEAD` (`131.072 B`) + `sd64` (`0x2000`).
5. **`TRAP_EH58_P7501_5V8_HDMI`:** Na placa `VEP79183` (`EH58/EH59/EH68`), `P7501` é header HDMI de `+5.8V_AV`, não UART `3.3V`.
6. **`TRAP_MN103S_SLEIGH_10_CLASSES` & `TRAP_PROG061_BGO_DESTRUCTIVE_CALLERS`:** Sempre auditar `SUB fb 9c`, `NOT f2 3x`, `BTST mn_todo()` e bloquear chamadas diretas a mutadores BGO (`0x50409DF2` / `0x50409E65`).

---

## 2. Novos Validadores & Mapas SSOT Fail-Closed (`profiles/` + `panadiga-profiles`)

1. **Mapa de Referência `4 MiB` Top-Boot ([`loader_ver063_4mb_top_bgo35.json`](../../profiles/nor_layouts/loader_ver063_4mb_top_bgo35.json)):** Vincula `loader_ver063_bootbin` a `spansion_s29jl032h_4mb_top` (`4 MiB`), com IPL RO em `0x0000_0000..=0x0003_FFFF`, janela `Bgo` em `0x0020_0000..=0x003E_FFFF` e top-boot `0x003F_0000..=0x003F_FFFF`.
2. **Catálogo Declarativo de Armadilhas ([`geometry_and_binary_traps.json`](../../profiles/traps/geometry_and_binary_traps.json)):** Formaliza cada armadilha com `wrong_assumption`, `verified_ground_truth`, `affected_entities`, `automated_guard` e `evidence_refs`.
3. **Validação Compile-Time & CLI ([`validate_bgo.rs`](../../crates/panadiga-profiles/src/validate_bgo.rs)):**
   - Confere `_4MB_`/`_8MB_`/`_16MB_` e base `0x...` em `bgo_retarget_spec` / `digadiag_spec.bgo_geometry_target` contra `flash.total_bytes`, `bgo_table_words[12..13, 15, 31]` e o span `Bgo` de cada `nor_layout`.
   - Rejeita em tempo de compilação (`crates/panadiga-core/build.rs`) e CLI (`panadiga profiles validate`) qualquer mistura `4 MiB` × `8 MiB`, contagem errada de setores (`35` vs `45`) ou tamanho incorreto de `DIGADIAG.FRM`.
4. **Pre-Flight CLI (`scripts/verify_geometry_traps.py`):** Valida `profiles/traps/` e confere dumps binários read-only (`--nor-dump <DUMP.bin> --layout <ID>`).

---

## 3. Atlas Forense Auto-Atualizável (`100%` Ghidra) & Ecossistema de Skills (`35 Skills`)

- **Atlas Forense Determinístico (`scripts/generate_firmware_atlas.py`):** Compila automaticamente cobertura `53/53` zero-undef de [`fleet_verify.json`](../audits/zero_undef/fleet_verify.json), funções + callers + callees + constantes hexadecimais categorizadas + operações bitwise das capturas `docs/pseudocode/raw/doc_*.txt`, dicionários `profiles/symbols/` e cabeçalhos/footers (`EXTRHEAD`/`EXTRFOOT`/`MainListHeader`/`MEIHFS`/`FAT16B`/`ISO9660`/`EEMUL`) para [`firmware_forensic_atlas.json`](../../profiles/forensic_atlas/firmware_forensic_atlas.json) e [`FORENSIC_ATLAS_INDEX.md`](../../.agents/skills/panadiga-firmware-forensic-atlas/references/FORENSIC_ATLAS_INDEX.md).
- **Novas Skills Especialistas Integradas (`.agents/skills/`):**
  - [`panadiga-ssot-builder-and-validation`](../../.agents/skills/panadiga-ssot-builder-and-validation/SKILL.md) — Construção & validação fail-closed da SSOT em `profiles/`.
  - [`panadiga-anti-trap-geometry-verification`](../../.agents/skills/panadiga-anti-trap-geometry-verification/SKILL.md) — Prevenção contra armadilhas `4 MiB` vs `8 MiB` e layouts cruzados.
  - [`panadiga-firmware-forensic-atlas`](../../.agents/skills/panadiga-firmware-forensic-atlas/SKILL.md) — Indexação automática do disassembly `100%` Ghidra (`--write` / `--check`).
