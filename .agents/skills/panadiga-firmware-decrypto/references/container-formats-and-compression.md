# Formatos de Contêiner `.FRM`, `EXTRHEAD`, Okumura LZSS & `MainList`

## 1. As 6 Variantes `UniversalFrmFormat` (`crates/panadiga-core/src/crypto/universal_frm/`)

| Variante | Cifra TOC / Dados | Stride & Início Payload | Checksum TOC / Notas |
| :--- | :--- | :--- | :--- |
| `LegacyDvd48B` | Feistel 64-bit | `0x30` (48 B) · payload `0x1800` | Byte-sum 16-bit (`+0x2E`); `EH55`/`EX80SEG` (`EH50 DRV J147` usa offset em `+0x08`) |
| `LegacyDvd64B` | Feistel 64-bit | `0x40` (64 B) · payload `0x2000` | Byte-sum 16-bit (`+0x3E`); `EH49/59/69`, `EX81S`, `EX72..EX93`, `EZ49V` |
| `SdPlain48B` | Plaintext SD | `0x30` (48 B) · payload `0x1800` | Byte-sum 16-bit (`+0x2E`); **`DMR-EH55` `Loader ver0.61`** (`PANA__SD.FRM` `sd48`) |
| `SdPlain64B` | Plaintext SD | `0x40` (64 B) · payload `0x2000` | Byte-sum 16-bit (`+0x3E`); **`Loader ver0.63+`** (`/BOOT.BIN`) |
| `BlurayFeistel` | Outer 48B `"PANASONIC\0\0\0"` + Feistel | `0x30` (48 B) + Adler-32 | `bs850`, `bs880`, `bs885`, `xw300gn`; `VGD ` plain-skip; `DRV1`/`DRV2` (`0x20` + `-48` clear) |
| `BlurayAes` | AES-128-CBC externo + `BlurayFeistel` | `AESHeader` + `1..4` subcontêineres | `PANAEUSB.FRM` pós-2015 (`--drop-aes` disponível no `repack`) |

- **Slots Auxiliares `STB` (`try_decode_stb_slot`):** Cabeçalho ASCII `CRC:` de 256 B (`Model:`, opcional `zlib:`) para tuners `ST20`/`OS20` e `MStar`.

## 2. Cabeçalho `EXTRHEAD` (64 B `0x40`) & `EXTRFOOT` (8 B)
| Offset | Tam. | Campo | Observações de Implementação |
| :---: | :---: | :--- | :--- |
| `0x00` | 8 B | `Magic` | `"EXTRHEAD"` |
| `0x08` | 4 B | `Subtype` | `"DRV "`, `"MAIN"`, `"PROG"`, `"MINI"` |
| `0x0C` | `u16` | `CompressionFlag` | Deve ser `0x0001` |
| `0x0E` | `u16` | `Algo` | Em `panadiga-core`: `0=Zlib`, `1=Gzip`, `2=Okumura LZSS`, `3=LZMA` |
| `0x10` | `u32` | `DstLen` | Tamanho exato pós-descompressão |
| `0x14` | `u32` | `DstAddr` | Endereço alvo em SDRAM (ex.: `0x90202000`, `0x57000000`, `0x50622000`) |
| `0x18` | `u32` | `SrcLen` | Bytes comprimidos no contêiner |
| `0x20` | `u32` | `FooterOffset` | Deslocamento relativo para `"EXTRFOOT"` (alinhamento 8 B Feistel) |
| `0x28` | `u32` | `HeaderChecksum` | Adler-32 com passo `adler_step` (`0x2C`) |
| `0x30` | 16 B | `BuildName` | Tag de build (ex.: `3410PQ`, `3060NP`, `3080GR`) |

- **Edge Crypt Parcial:** `Plain` (nenhum), `Drv` (primeiros `0x2800` = 10 KiB), `Main` (cabeça + cauda com guarda não-sobreposta quando `aligned_len <= 2 * span`), além de autodetecção de dupla cifra (`detect_extrhead_key_and_double`).

## 3. Algoritmo LZSS de Okumura (`algo == 2`, `extrhead/lzss.rs`)
- Anel circular de **`4.096` bytes (`0x1000`)** inicializado com zeros e índice inicial **`0x0FEE`**.
- Byte de flags (LSB-first): bit `1` = 1 byte literal; bit `0` = backref de 2 bytes `(b1, b2)` onde `offset = b1 | ((b2 & 0xF0) << 4)` e `len = (b2 & 0x0F) + 3` (`3..18` bytes).

## 4. `MainListHeader` & Partições Linux `fmaX` (`pana_fw_go/main_list.rs`)
- **`MainListHeader` (`20 + 8*N` bytes):** `list_sum` (Adler-32), `version` (`1`), `list_size`, `decompressed_segment_size`, `compression_type`, seguido de `N` pares `(seg_size, seg_sum)`.
- **Cifra Seletiva de Extremidades:** `0x2800` (10 KiB cabeça + 10 KiB cauda) para tamanho alvo legado vs `0x1400` (5 KiB + 5 KiB) em modelos modernos.
- **Partições `fmaX`:** Stream concatenado divide em partições `fma0..fmaN` (`RomFS`/`CramFS`/`SquashFS` + kernel Linux `2.6.11.12` UniPhier) extraíveis via `frm extract-fs --elf-only` e substituíveis com `frm repack --patch-partition fma5=...`.

## 5. Matriz de Compatibilidade OEM
- **Mesma arquitetura Matsushita:** `Loewe Centros / ViewVision DR+` (`MN2DS` + placas `VEP`), `Hualu BDR-9800` (usa chave `0x0A1EE6C5BDC4933F` — aceita mesma engine matemática `.FRM`, exigindo conferência de `DEV.BIN` antes de gravar).
- **Incompatíveis:** `Tevion / Medion / Funai / maioria Philips` (`FUNAI_UPG` / MediaTek) — jamais gravar binários Panadiga.
