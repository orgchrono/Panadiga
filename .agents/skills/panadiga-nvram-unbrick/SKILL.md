---
name: panadiga-nvram-unbrick
description: >-
  Procedimento de cirurgia de NVRAM/NOR e recuperação do erro U99 na Flash Spansion S29JL064H (IC5001), reparo BGO FTL e geração do kit SD DIGADIAG.FRM.
  Use esta skill ao sanitizar NVRAM (64 KB), auditar/reparar FTL BGO, gerar kit SD ver061 ou adaptar região/PAL-M (M6930LB).
---

# Recuperação Cirúrgica U99, Política NOR em 3 Zonas & Kit SD (`ver061`)

O erro `U99` no DMR-EH55 decorre de corrupção na emulação de NVRAM/EEMUL (`SA134..SA141`) ou de corrupção de metadados no FTL **BGO** (`SA26..SA70` em `Loader ver0.61`) da Flash física **Spansion `S29JL064H70TFI00`** (`8 MiB / 8.388.608 B`, setores `SA0..SA141`, base AM34 `0x80000000` / uncached `0x84000000`).

## Política Estrita de Escrita em 3 Zonas (`IC5001` 8 MiB)
1. **Zona 1 — Bootloader IPL Crítico (`SA0..SA3`, `0x80000000..0x8003FFFF` / `0x84000000..0x8403FFFF`)**:
   - **LEITURA ESTRITA PERMANENTE (`E05`)** na sonda Pico. Preserva o loader `P7501` e evita brick irreversível sem dessoldar TSOP-48.
2. **Zona 2 — Upper NOR App & BGO FTL (`SA4..SA133`, `0x80040000..0x807EFFFF`)**:
   - Protegida por padrão (`E05`). Inclui os **45 setores BGO (`SA26..SA70`, `0x84130000..0x843FFFFF`)** do `Loader ver0.61` (`1 P1 + 40 P2 + 4 spares`).
   - Reparo offline Copy-on-Write (`nvram patch-upper-nor` / `unbrick bgo-repair-plan`) ou ao vivo via `PANAMON1>` mediante opt-in explícito `CONFIRM_UPPER_NOR` (`unbrick bgo-live-apply --confirm-upper-nor`).
3. **Zona 3 — NVRAM / EEMUL (`SA134..SA141`, últimos 64 KiB, `0x807F0000..0x807FFFFF`)**:
   - Escrita autorizada para reset cirúrgico de NVRAM preservando 100% os primeiros `8.323.072` bytes (`0x000000..0x7EFFFF`).

## Comandos Canônicos de Operação (CLI Nativo)

### 1. Limpeza / Patch CoW de NVRAM e Auditoria de 142 Setores
```bash
# Limpar 64 KB finais (gera backup corrupt + valida SHA-256 pré-IPL):
cargo run -p panadiga-cli -- nvram clean <DUMP_ORIG.bin> <DUMP_CLEAN.bin>

# Auditar todos os 142 setores (SA0..SA141) de um dump Spansion 8 MiB:
cargo run -p panadiga-cli -- nvram audit-nor-sectors --image firmware_and_images/eh55_nor_8mb_ic5001_dump.bin

# Patch CoW em Upper NOR / BGO (exige saída nova, bloqueia SA0..SA3 por padrão):
cargo run -p panadiga-cli -- nvram patch-upper-nor --input <IN.bin> --output <OUT.bin> --offset <HEX> --hex "<BYTES>"
```

### 2. Auditoria & Reparo Determinístico do FTL BGO (`SA26..SA70`)
```bash
cargo run -p panadiga-cli -- unbrick bgo-audit --input firmware_and_images/eh55_nor_8mb_ic5001_dump.bin
cargo run -p panadiga-cli -- unbrick bgo-repair-plan --input <DUMP.bin> --out-image <REPAIRED.bin> --out-plan <PLAN.json>
# Aplicação ao vivo via sonda Pico (exige opt-in explícito CONFIRM_UPPER_NOR):
cargo run -p panadiga-cli -- unbrick bgo-live-apply --plan <PLAN.json> --port <PORT> --confirm-upper-nor
```
*(No hardware brasileiro `DMR-EH55LB-S` / `M6930LB` / `PAL-M`, o dump reparado auditado encontra-se em `firmware_and_images/eh55_nor_8mb_ic5001_repaired.bin` junto ao `ifodev2` em `firmware_and_images/eh55_ifodev2_m6930lb_2048b.bin`.)*

### 3. Kit de Recuperação em Cartão SD (`--loader-profile ver061`) & Região
```bash
# Gerar kit canônico (262.144 B DIGADIAG.FRM raw + sd48 PANA__SD.FRM + SHA256SUMS.txt):
cargo run -p panadiga-cli -- unbrick prepare-sd --loader-profile ver061 --outdir firmware_and_images/sd_unbrick_kit
cargo run -p panadiga-cli -- unbrick verify-sd --dir firmware_and_images/sd_unbrick_kit

# Compatibilidade regional / PAL-M (`M6930LB`) e plano TBC autônomo:
cargo run -p panadiga-cli -- unbrick region-compat --help
cargo run -p panadiga-cli -- unbrick tbc-plan --help
```
