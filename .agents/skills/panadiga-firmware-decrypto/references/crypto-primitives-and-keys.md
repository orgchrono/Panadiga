# Primitivas Criptográficas, Chaves & Codebook ECB (`PANA_DVD` / `AES` / `SDBOOT`)

## 1. Paradoxo Root-of-Trust & Codebook ECB 64-Bit
Como o SoC AM34 não possui descifrador inline no barramento externo, os arquivos de atualização óptica têm entropia máxima (~`7.9996 bits/byte`), mas a Flash NOR `IC5001` roda em **plaintext XIP** (`5.8–7.1 bits/byte`), revelando o algoritmo e as chaves no bootloader.

Assinaturas provadas via `panadiga frm codebook` (ligando `PANA_DVD.FRM` às partições do HDD):
- `03 50 65 C8 7B B8 E0 90` -> `FF FF FF FF FF FF FF FF` (`3.812` hits no `.FRM` — padding entre módulos).
- `E5 D1 F3 4A 51 8B 26 90` -> `00 00 00 00 00 00 00 00` (`196` hits no `.FRM` e match exato no HDD em `0x02000038`, `0x04000030`, `0x04000038`).
- `4D 3C 5D BB 0C 35 77 77` -> `51 87 13 8D 5B 0A FB 22` (`1.280` hits no `.FRM` — cabeçalho das partições HDD `32 MB` e `64 MB`).

## 2. Cifra Feistel Matsushita 64-Bit / 16 Rodadas (`crates/panadiga-core/src/crypto/feistel.rs`)
- **Rotinas no `/BOOT.BIN`:** `FUN_4400a666` (buffer), `FUN_44011d42` (key schedule 16 rodadas -> 32B round key com `PANA_FEISTEL_KEYSCHED_IV`), `FUN_44011ec0` (encrypt), `FUN_44012029` (decrypt), `FUN_44012191` (função F 32-bit + S-box 256 B em `0x44035D84`), `FUN_4401e5a3` (Adler-32 com stride).
- **Reconstrução em RAM (`crt0` `.data` `0x44039830 -> 0x50480000`):** XOR de `0x50480278` (`72 eb ca 91 30 45 68`) com `0x5048027F` (`ad 99 df 56 72 3a b8 10`) e byte 7 (`0x10`) aplicado sobre bytes 1 e 4.

### Catálogo das 12 Chaves `PANA_DVD_KEY_CATALOG`
| ID / Família | Chave 64-bit | Modelos / Uso Típico |
| :--- | :--- | :--- |
| `old_df72` | `0xDF7215C7427FD010` | `DMR-EH55`, `EX80SEG`, `EH49/59/69`, `EX72..93` |
| `le2011_08e0` | `0x08E03D859AF9F3EE` | `DMR-EH50`, `XW300`, `BS850/880/885` |
| `y2011_af4d` | `0xAF4D5B16C298C16D` | Série 2011 |
| `y2012_9407` | `0x9407311B7561F97E` | Série 2012 |
| `y2012_5d88` | `0x5D88719745A4EFFF` | Série 2012 (variação 2) |
| `y2013_38bb` | `0x38BB0C17D362701F` | Série 2013 |
| `y2013_ec23` | `0xEC238CD791456646` | Série 2013 (variação 2) |
| `hualu_0a1e` | `0x0A1EE6C5BDC4933F` | `2013 China` / OEM Hualu `BDR-9800` |
| `y2014_8f8c` | `0x8F8CA88482E43D9B` | Série 2014 |
| `y2014_21fc` | `0x21FCA17361E66B4E` | Série 2014 (variação 2) |
| `y2016_fdfe` | `0xFDFE0BEE4C90FA2C` | Híbrido 2016 (camada interna Feistel) |
| `y2018_06c9` | `0x06C943F3B997F7E0` | Híbrido 2018 (camada interna Feistel) |

## 3. AES-128-CBC, `keys.ukf`, `NetworkKeyIV`, `SDBOOT` & `SDDL.SEC`
- **Blu-ray/UHD `PANA_DVD_AES_CATALOG` (AES-128-CBC nopad externo + Feistel interno):**
  - `2016`: Key `0xA692605194CF7F4917E50521C1DF0BBA` · IV `0xF80A6BCAD57AA67C3C60E3BE755E05DF` · Feistel `0xFDFE0BEE4C90FA2C`
  - `2018`: Key `0x62A39E1C5594AE09244EB326EF7938FA` · IV `0x46D03C52B1A398FA03276FDE7CC4EA26` · Feistel `0x06C943F3B997F7E0`
- **Keyring Externo (`--key-file keys.ukf`):** Lê coleções `PANA_DVD_KEYONLY` e `PANA_DVD_AESPAIR` compatíveis com `unixtract`.
- **OTA `NetworkKeyIV`:** `derive_network_key_iv()` decifra blob de 32 B via rotina `libdownloadlib.so` `network_decipher_in_place`.
- **`SDBOOT` (`sddl_dec`):**
  - Par 0: Key `2E2A336233E55ABAF5FFEC54F8AB7125` · IV `2CA4B47AFFCB1AE8E1EA2D9EF512629A`
  - Par 1: Key `245E8DE8F499B0F96EC155B608E242F3` · IV `3E8F29D4BAE776A518A7B63C42CA1B43`
- **`SDDL.SEC` (`sddl_dec`):**
  - **Pré-2011 (`tbl2009`/`tbl2010`/`tbl_sdboot`):** Master 3DES Key `C2A421F6ADEB44BE B0FDA68C234BB3C5 E98EC68C326FD395 07C8D75EF1B1B142`.
  - **Pós-2011 (`SDDL_SEC_NEW_KEY`, de `/usr/local/customer_dl/crypto_key`):** AES-128 Key `26E096D3EF8A8FBBAA5E516F7726C22C` · IV `3E4AE23A69DB8154CD8838C4B90C7666`.
- **Proteção de Mídia Gravada:** DVD-RAM/HDD usa C-BSA/C2 ligado a `Drive ID` + `Media Key Block` (CPRM); DivX usa chave de 8 B; região óptica via RPC-2.
