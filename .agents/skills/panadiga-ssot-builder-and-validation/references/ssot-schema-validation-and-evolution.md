# Referência: Arquitetura da SSOT Declarativa, Schemas & Regras Fail-Closed

## 1. Anatomia das Coleções `profiles/`

| Subdiretório | Esquema / Gerenciador | Função na SSOT |
| :--- | :--- | :--- |
| `profiles/soc_cores/` | `soc_core.schema.json` | Núcleos ISA (`MN10300/AM34`, SPARC, ARM) + invariante `3300 mV` |
| `profiles/socs/` | `soc.schema.json` | Janelas de barramento, `NSW5V Hi-Z only`, endereços letais (`0xC000100C`) |
| `profiles/flash_chips/` | `flash_chip.schema.json` | Geometria de bancos/setores (`S29JL064H` 8 MiB, `S29JL032H` 4 MiB Top-Boot), JEDEC ID |
| `profiles/eeprom_chips/` | `eeprom_chip.schema.json` | EEPROMs/NVRAMs auxiliares |
| `profiles/firmware_trains/` | `firmware_train.schema.json` | Loaders, `.FRM` TOC (`sd48`/`sd64`), `digadiag_spec`, `bgo_retarget_spec` |
| `profiles/nor_layouts/` | `nor_layout.schema.json` | Particionamento físico real `(train_ref × flash_ref)` com políticas por região |
| `profiles/media_formats/` | `media_format.schema.json` | Formatos `SD FAT16B`, `DVD ISO9660`, `HDD MEIHFS` e magics |
| `profiles/boards/` | `board.schema.json` | Mapas PCB (`VEP79138` UART vs `VEP79183` HDMI `+5.8V_AV`), headers e jumpers |
| `profiles/models/` | `model_manifest.schema.json` | `89` modelos ligados por FKs + aliases únicos + `claims` `T0..T4` |
| `profiles/schema/` | `12` arquivos Draft 2020-12 | Exportados por `panadiga profiles export-schema` e verificados no `build.rs` |
| `profiles/traps/` | `geometry_and_binary_traps.json` | Registro declarativo de armadilhas (`TRAP_BGO_4MB_VS_8MB`, etc.) |
| `profiles/symbols/` & `profiles/forensic_atlas/` | `panadiga re index-symbols` + `generate_firmware_atlas.py` | Dicionários determinísticos de funções, callers, hex, cabeçalhos/footers e bitwise |

## 2. Checagens Matemáticas em `validate_bgo.rs` e `validate_layout.rs`

1. **Alinhamento & Disjunção (`validate_single_nor_layout`):**
   - Cada região `[start_offset..=end_offset]` precisa alinhar exatamente com limites de setor retornados por `derive_sector_table(flash)` e não pode sobrepor regiões vizinhas.
   - `Ipl` em `0x000000` precisa ter `>= 256 KiB` (`MIN_CRITICAL_IPL_GUARD_BYTES`) e política `read_only_permanent`.
2. **Consistência `BGO` × `FlashChip` (`validate_bgo_retarget_spec` & `validate_digadiag_and_layout_pairing`):**
   - Extrai capacidade declarada (`_4MB_` -> `4 MiB`, `_8MB_` -> `8 MiB`, `_16MB_` -> `16 MiB`) e base VA (`_0X<HEX>`) do identificador de geometria (`BGO_35SEC_4MB_0X84200000` vs `BGO_45SEC_8MB_0X84130000`).
   - Impede que um `FirmwareTrainProfile` ou `bgo_retarget_spec` de `8 MiB` seja emparelhado a uma Flash de `4 MiB` (ou vice-versa), valida `word[12]` / `word[13]` / `word[31]` da tabela BGO e confere o tamanho esperado do `DIGADIAG.FRM` (`raw_256k_prog061` = `262144`, `extrhead_128k` = `131072`).
3. **Derivação Unificada (`panadiga ssot export-all`):**
   - Atualiza `scripts/profiles_types.py`, projeções TypeScript em `apps/desktop/src/` (incluindo `SSOT_CONTRACT_HASH_STR`), tabelas C11 em `firmware/pico-p7501/generated/` e `tools/ghidra/` em passo único determinístico.
