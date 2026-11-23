# Triagem de Integridade, Faixas de Entropia & Motor `ingest`

## 1. Testes Diagnósticos de Integridade (`nor_intake_analyzer.py` / `discover-nor`)

| Anomalia Detectada | Assinatura Empírica | Causa Física Típica | Ação Corretiva |
| :--- | :--- | :--- | :--- |
| **16-bit Byte-Swap** | `"XERTDAEH"`, strings pareadas invertidas (`"aLdo..."`) | Soquete/configuração endian x16 incorreta no gravador TSOP-48 | Re-capturar no modo correto ou gerar cópia CoW byte-swapped para inspeção |
| **Floating Address Pin Mirror** | Metade superior == metade inferior (ex.: `0x000000..0x3FFFFF == 0x400000..0x7FFFFF`) | Pino `A21`/`A22` sem contato no soquete TSOP-48 ou clip | Limpar/reassentar TSOP-48 e capturar novamente |
| **All-`0xFF` / Mock Sintético** | `>99%` de `0xFF` ou entropia `~0.0` fora de área spare | Chip em branco, falha de `CE#`/`OE#` ou fixture de teste | Rejeitar para `T3` e verificar alimentação/pinagem |
| **Fantasmas FTL `EXTRHEAD`** | Header `"EXTRHEAD"` dentro de região BGO/FTL com `src_len`/`footer_offset`/Adler inválidos | Resíduo de atualização antiga fragmentada em blocos FTL | Ignorar como módulo executável; filtrar via `nor_intake_analyzer.py` |
| **Corte por Timeout UART** | Stream truncado sem marcador `PANADONE` / checksum divergente | Overrun em `SC0RXB` (1 B) ou falta de espaçamento `300–350 µs` | Recapturar via `panadiga pico capture-nor` |

## 2. Assinaturas de Entropia de Shannon (Referência Empírica)
- **`0.00 – 0.50 bits/byte`:** Padding virgem `0xFF` / `0x00` ou setores `EEMUL` limpos.
- **`5.80 – 7.10 bits/byte`:** Código executável nativo XIP (`MN103S` / `Loader ver0.61` / `MAIN` descomprimido).
- **`~6.37 – 7.97 bits/byte`:** Partições de firmware HDD (`0x02000000`, `0x04000000`) com blocos mistos e cifra parcial.
- **`7.995 – 8.000 bits/byte`:** Contêiner cifrado/comprimido íntegro (ex.: `PANA_DVD.FRM` = **`7.99957973 bits/byte`** com 0 ASCII).

## 3. Anatomia Comparativa de NOR `8 MiB` (`DMR-EH55` vs `DMR-ES35V`)
- **`DMR-EH55` (`S29JL064H70TFI00`, `142` setores: `8×8K + 126×64K + 8×8K`, `EEPROM_W_C = NC`):**
  - `SA0..SA10` (`0x000000..0x03FFFF`, `256 KiB`): `Loader ver0.61 [06/03/06]` — Zona 1 RO (`E05`).
  - `SA11..SA25` (`0x040000..0x12FFFF`): Área `Nor-FROM` / Boot apps (`DRV` / `"fvu.exe"`).
  - `SA26..SA70` (`0x130000..0x3FFFFF`, **`45 × 64 KiB`**): FTL BGO `[nfrom]` (`1` `0xF101` + `40` `0xF2xx` + `4` spares).
  - `SA71..SA133` (`0x400000..0x7EFFFF`): Upper `EEMUL` (`"1284"`, `0x0FF00000`, `1.158` tags).
  - `SA134..SA141` (`0x7F0000..0x7FFFFF`, `64 KiB`): NVRAM/EEMUL `0xF1` (alvo `U99`).
- **`DMR-ES35V` (Triton dump + `24C64` externa `8 KiB`):**
  - `0x000000..0x03FFFF`: `Loader ver0.18 [06/03/13]`; `0x080000..0x12C0C0`: `EXTRHEAD "DRV "`; **`0x130000..0x40FFFF`: código/dados `MAIN` XIP** (colide com BGO `SA26..SA70` do EH55 — incompatibilidade fatal!); `0x430000..0x6D42F0`: `EXTRHEAD "MAIN"`; `0x700000..0x7EFFFF`: BGO `0xF2`; `0x7F0000..0x7FFFFF`: `EEMUL` `0xF1`.

## 4. Motor Autônomo `ingest` (`crates/panadiga-core/src/ingest/`)
- `fingerprint_binary` + `harvest_crypto_artifacts`:
  - Classifica `ContainerFormat` (`Sd48B`, `Sd64B`, `Dvd48B`, `Dvd64B`, `Digadiag`, `Extrhead`, `RawNor`, `MeihfsHdd`, `Unknown`) e `CipherFamily` (`MatsushitaFeistel64`, `Des`, `Blowfish`, `Plaintext`, `Unknown`).
  - Varre janela deslizante por **S-boxes bijetivas não-afins de 256 bytes** (`0x44035D84`), round keys Feistel, P-array Blowfish e S-box DES.
  - Coleta tokens de modelo/SoC (`DMR-`, `MN103`, `MN2WS`, `SDDIAG`, `UNIPHIER`) entrando nos primeiros 4 módulos do TOC e em payloads `EXTRHEAD`.

## 5. Checklist Portátil para Novos Projetos de Hardware
1. Fotografar PCB/etiquetas, anotar part numbers de SoC/Flash/EEPROM e tensão lógica real do barramento.
2. Capturar dump em modo somente leitura, gerar SHA-256 imediato e travar o original como imutável.
3. Rodar triagem de integridade (tamanho potência de 2, entropia por bloco, anti-byte-swap, anti-espelhamento).
4. Mapear regiões XIP vs blobs comprimidos vs NVRAM/EEPROM antes de iniciar disassembly no Ghidra.
