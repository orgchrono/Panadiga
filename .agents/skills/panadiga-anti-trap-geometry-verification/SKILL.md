---
name: panadiga-anti-trap-geometry-verification
description: >-
  Prevenção obrigatória contra armadilhas de tamanho e layout Flash NOR/BGO (especialmente
  4 MiB vs 8 MiB, 0x84200000 vs 0x84130000, 35 vs 45 setores), XIP vs BGO, DIGADIAG 256K vs
  128K, P7501 HDMI +5.8V_AV e 10 armadilhas MN103S. Use antes de gravar, emular, editar BGO
  ou criar/validar mapas Flash/BIN.
---

# Verificação Anti-Armadilha de Geometria NOR/BGO (`4 MiB` vs `8 MiB`)

Esta skill impede confusões silenciosas entre mapas Flash/BGO de diferentes modelos e trens Panasonic DIGA antes que causem patches fora de limite ou bricks irreversíveis.

## 1. Quando Usar & Handoffs
- Ative sempre que manipular layouts BGO FTL, scripts/patches `PROG061`/`MAIN`/`Loader`, kits SD (`DIGADIAG.FRM`/`PANA__SD.FRM`), comparações entre séries `EH55` (`8 MiB`), `EH58` (`ver0.63`), `ES35V` ou `EZ485V` (`4 MiB`).
- Para editar regras/mapas em `profiles/`, use `.agents/skills/panadiga-device-onboarding/SKILL.md`.
- Para consultar constantes de pseudocódigo já auditadas, use `.agents/skills/panadiga-firmware-forensic-atlas/SKILL.md`.
- Tabela comparativa completa em [bgo-4mb-vs-8mb-and-geometry-traps.md](./references/bgo-4mb-vs-8mb-and-geometry-traps.md).

## 2. A Armadilha Crítica (`BGO 4 MiB` vs `8 MiB`) & Regra Fail-Closed
1. **Por que a armadilha ocorre:** `/BOOT.BIN` (`Loader ver0.63 [07/10/24]`, tabela `0x44036520`) e `PROG061` stock de fábrica (`0x50436E30`) configuram BGO a partir de **`0x84200000..0x843FFFFF`** (compatível com Flash **4 MiB** / Top-Boot `35` ou `29` descritores). Já o hardware físico **DMR-EH55 (`IC5001 S29JL064H` de 8 MiB, `Loader ver0.61` tabela `0x440337A4`)** usa **`45` setores de 64 KiB em `0x84130000..0x843FFFFF`** (`SA26..SA70`) e reserva `0x84400000..0x847FFFFF` para Upper EEMUL.
2. **Proibição de Inferência por Tamanho:** Nunca assuma `BGO = 4 MiB` nem `BGO = 8 MiB` nem infira layout somente pelo tamanho do arquivo; valide sempre a tupla `(train_ref × flash_ref × nor_layout)` via SSOT (`profiles/traps/geometry_and_binary_traps.json`).
3. **Bloqueio Duplo (Compile-Time Rust + Pre-Flight Python):**
   - `crates/panadiga-profiles/src/validate_bgo.rs` cruza `_4MB_`/`_8MB_`/`_16MB_` e base `0x...` contra `flash.total_bytes`, `bgo_table_words[12..13, 31]` e `nor_layout`.
   - `scripts/verify_geometry_traps.py` audita todas as referências e pode inspecionar dumps `.bin` somente leitura antes de qualquer patch.

## 3. Execução Rápida (`scripts/verify_geometry_traps.py`)
```bash
# Validar catálogo SSOT de armadilhas e todos os layouts Flash/Train
python scripts/verify_geometry_traps.py --check --json -

# Pre-flight somente leitura de um dump NOR físico antes de patch/unbrick
python scripts/verify_geometry_traps.py --nor-dump <DUMP.bin> --layout eh55_ver061_on_s29jl064h --json -

# Validador relacional nativo Rust
cargo run -q -p panadiga-cli -- profiles validate
```
- Wrapper local na skill: `.agents/skills/panadiga-anti-trap-geometry-verification/scripts/check_traps.py`.

## 4. Verificação
```bash
python -m mypy --strict scripts
python -m pytest scripts/tests/test_geometry_traps.py scripts/tests/test_profiles_ssot.py
```
