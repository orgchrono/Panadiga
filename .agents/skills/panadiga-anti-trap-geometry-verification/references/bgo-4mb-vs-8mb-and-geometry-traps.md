# Referência Anti-Armadilha: Geometrias Flash NOR, BGO `4 MiB` vs `8 MiB`, `.FRM` & Hardware

## 1. Comparativo Exato de Geometria BGO (`Loader ver0.61` × `PROG061` × `Loader ver0.63`)

| Parâmetro | `Loader ver0.61` Real (`EH55`) | `PROG061` Stock (`0x50436E30`) | `/BOOT.BIN ver0.63` (`0x44036520`) |
| :--- | :--- | :--- | :--- |
| **Flash Alvo Típica** | `S29JL064H` (`8 MiB`, `SA0..SA141`) | `4 MiB` Top-Boot donor | `4 MiB` Top-Boot (`/BOOT.BIN`) |
| **Tabela `.rodata`** | `0x440337A4` (`+0x6c`) | `0x50436E30` (`+0x6c`) | `0x44036520` (`+0x6c`) |
| **Base BGO (`word[12]`)** | `0x84130000` (`SA26`) | `0x84200000` | `0x84200000` |
| **Teto BGO (`word[13]`)** | `0x84400000` (excl.) | `0x843F0000` + Bank2 `8×8K` | `0x843F0000` |
| **Setores Físicos (`word[31]`)** | **`45` (`0x2D`) × `64 KiB`** (`2.880 KiB`) | `31 × 64 KiB` + `8 × 8 KiB` | `29` (`0x1D`) |
| **Dados P2 / Spares** | `40` P2 + `4` spares (`max_pid=0x0F24`) | `28` P2 (`max_pid=0x0A1A`) | `24` P2 (`max_pid=0x0816`) |
| **Onde fica EEMUL?** | `SA134..SA141` (`0x847F0000..0x847FFFFF`) | Upper top-boot / conforme layout | Top-boot boundary (`0x3F0000`) |

## 2. Outras Armadilhas Catalogadas (`profiles/traps/geometry_and_binary_traps.json`)

- **`TRAP_ES35V_MAIN_XIP_VS_EH55_BGO`:** Em `DMR-ES35V`, `0x130000..0x40FFFF` é código `MAIN_XIP` — nunca BGO (`0x700000..0x7EFFFF`).
- **`TRAP_EZ485V_4MB_SPARC_VENDOR_NVRAM`:** `DMR-EZ485V` (`4 MiB` Macronix MX29LV320 bottom-boot) usa SPARC/VxWorks com `vendor_nvram` em `0x360000..0x3FFFFF` (`CLsiPMABS`), não EEMUL `0xF1`.
- **`TRAP_DIGADIAG_256K_RAW_VS_128K_EXTRHEAD`:** `ver0.61` exige `DIGADIAG.FRM` cru (`262.144 B`) + `PANA__SD.FRM sd48` (`0x1800`), ao passo que `ver0.63` usa `EXTRHEAD` (`131.072 B`) + `sd64` (`0x2000`).
- **`TRAP_EH58_P7501_5V8_HDMI`:** Na placa `VEP79183` (`EH58/EH59/EH68`), `P7501` transporta `+5.8V_AV` para HDMI — jamais conectar sonda LVTTL `3.3V` sem verificar `board.p7501_header_present`.
- **`TRAP_MN103S_SLEIGH_10_CLASSES` & `TRAP_PROG061_BGO_DESTRUCTIVE_CALLERS`:** Conferir `SUB fb 9c`, `NOT f2 3x`, `BTST mn_todo()`, blocos removidos e bloquear chamadas diretas a `0x50409DF2` / `0x50409E65`.
