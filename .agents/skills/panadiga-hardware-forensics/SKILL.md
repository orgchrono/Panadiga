---
name: panadiga-hardware-forensics
description: >-
  Triagem forense não-destrutiva de dumps brutos de hardware (NOR Flash 2..16 MiB, NAND,
  EEPROM I²C, HDD e cartão SD), detecção de corrupções de captura (16-bit byte-swap XERTDAEH,
  espelhamento por pino flutuante, mocks 0xFF e fantasmas FTL EXTRHEAD), auditoria dos 142
  setores Spansion e EEMUL ("1284"), entropia de Shannon e fingerprinting autônomo de S-box.
  Use ao receber um dump novo ou validar integridade antes de RE, unbrick ou onboarding.
---

# Triagem Forense de Dumps de Hardware (NOR, NAND, EEPROM & SD)

Esta skill garante que nenhum dump falso, byte-swapped, espelhado ou truncado contamine engenharia reversa, gravação em hardware ou a SSOT `profiles/`.

## 1. Quando Usar & Handoffs
- Ative ao receber qualquer imagem `.bin`/`.dump`/`.nvram` de programador TSOP-48, SPI/I²C, SD ou captura UART `PANADMP1..PANADONE`.
- Para custódia em Vault (`panadiga session`) e `ledger.json`, acione `.agents/skills/panadiga-forensic-archival/SKILL.md`.
- Para particionamento HDD `MEIHFS`, use `.agents/skills/panadiga-extract-partitions/SKILL.md`; para decifração `.FRM`/`EXTRHEAD`, use `.agents/skills/panadiga-firmware-decrypto/SKILL.md`.
- Tabelas diagnósticas, faixas de entropia, mapa `EH55` vs `ES35V` e pipeline `ingest` em [dump-integrity-triage.md](./references/dump-integrity-triage.md).

## 2. Invariantes Não-Negociáveis
1. **Imutabilidade Forense:** Jamais edite, trunque ou faça byte-swap *in-place* em dumps originais; trabalhe sempre sobre cópias CoW e verifique SHA-256.
2. **Triagem Antes de `T3`:** Um dump com byte-swap 16-bit (`XERTDAEH`), espelhamento por pino de endereço solto ou `all-0xFF` **não pode** sustentar promoção `T3` (`physical_dump_verified`).
3. **Identidade Física Completa:** Registre sempre **`(model, region, board_rev, flash_jedec_id)`**; ausência de EEPROM externa no EH55 (`IC7501 EEPROM_W_C = NC`) difere de ES15/ES35V (`24LC64`).

## 3. Workflow Canônico (CLI, Python & MCP)
```bash
# 1. Calcular e fixar SHA-256 + ingerir no bucket imutável raw/
cargo run -q -p panadiga-cli -- verify-artifact tmp/dump_8mb.bin
cargo run -q -p panadiga-cli -- session ingest --session-dir tmp/vault_s1 --file tmp/dump_8mb.bin --bucket raw --origin programmer_tsop48 -j -

# 2. Triagem automatizada NOR (rejeita byte-swap, espelhamento e all-0xFF)
cargo run -q -p panadiga-cli -- profiles discover-nor -i tmp/dump_8mb.bin --model-id DMR-NOVO --out-dir tmp/draft_profiles -j -
python -m scripts.re.nor_intake_analyzer --input tmp/dump_8mb.bin --model-id DMR-NOVO --out-dir tmp/draft_profiles --offline-only

# 3. Auditoria de 142 setores Spansion, FTL BGO (45 setores) e mídia SD bruta
cargo run -q -p panadiga-cli -- nvram audit-nor-sectors --image tmp/dump_8mb.bin -j -
cargo run -q -p panadiga-cli -- unbrick bgo-audit --nor tmp/dump_8mb.bin -j -
cargo run -q -p panadiga-cli -- unbrick inspect-sd <SD_IMG_OU_DEV> -j -

# 4. Inspeção de cabeçalhos, strings e extração de DRV embutido na NOR para o Vault
cargo run -q -p panadiga-cli -- headers dump tmp/dump_8mb.bin -o 0x80000 -l 128
cargo run -q -p panadiga-cli -- scan tmp/dump_8mb.bin -m 6 -k Loader -k EEMUL -k EXTRHEAD -l 50 -j -
cargo run -q -p panadiga-cli -- session extract-drv --session-dir tmp/vault_s1 --nor-dump tmp/dump_8mb.bin -j -
```
- **MCP:** Use `panadiga_nor_sector_audit`, `panadiga_read_hex` (16..4096 B) e `panadiga_eh55_bgo(action="audit")` antes de abrir dumps no editor.

## 4. Armadilhas & Descobertas de Bancada
- **Mock `0xFF` Disfarçado:** O primeiro `panasonic_dmr-eh55_flash_dump.bin` tinha `8.388.509 B` (`99,999%`) de `0xFF` e `99` bytes de fixture sintética — sempre audite entropia por setor!
- **Byte-Swap `XERTDAEH`:** Programadores TSOP-48 x16 podem gravar big-endian word-swapped (`"EXTRHEAD"` vira `"XERTDAEH"`).
- **Fantasmas FTL `EXTRHEAD`:** Blocos apagados/antigos no FTL ainda carregam a string `"EXTRHEAD"` mas falham tamanho/Adler-32; confirme com `nor_intake_analyzer.py`.
- **`EEMUL` sem EEPROM I²C:** No DMR-EH55, `SA71..SA133` abrigam `1.158` tags `"1284"` (`0x0FF00000`) e `SA134..SA141` abrigam headers `0xF1` (alvo `U99`).

## 5. Daqui pra Frente & Reuso em Novos Projetos
- **Fronteiras abertas no Panadiga:** Parser completo em Rust para Upper `EEMUL` (`SA71..SA133`: `sched.bin`, `resetdiag`, TV Guide/Gemstar) e extensão de triagem NAND/eMMC para gerações UniPhier (`XW300`, `BS/XS`, `BW/BWT`).
- **Em qualquer novo hardware:** Nunca assuma formato pelo tamanho em MB — rode **Hash -> Entropia por bloco -> Half-mirror test -> Scan de assinaturas/S-boxes (`ingest`) -> Mapa físico** antes de qualquer desmontagem.

## 6. Verificação
- Dump aprovado possui `SHA-256` registrado, zero alertas de byte-swap/mirror em `nor_intake_analyzer.py`, e classificação clara por faixa de setor em `nvram audit-nor-sectors`.
