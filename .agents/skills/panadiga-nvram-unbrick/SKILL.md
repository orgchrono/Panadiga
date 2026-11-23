---
name: panadiga-nvram-unbrick
description: >-
  Cirurgia de NVRAM/NOR e recuperação do erro U99 na Flash Spansion S29JL064H (IC5001),
  política estrita de 3 zonas (SA0..SA10 IPL RO E05, SA11..SA133 Upper NOR/BGO 45 setores
  SA26..SA70, SA134..SA141 NVRAM 64 KiB), reparo BGO FTL em 2 ciclos, kit SD DIGADIAG.FRM /
  PANA__SD.FRM (ver061), rebuild-from-dump, sd/dvd create e região PAL-M (M6930LB). Use ao
  sanitizar NVRAM, auditar/reparar FTL BGO ou gerar mídia de recuperação.
---

# Recuperação Cirúrgica `U99`, Política NOR em 3 Zonas & Kit SD (`ver061`)

O erro `U99` no `DMR-EH55` (`Loader ver0.61`) decorre de corrupção de NVRAM/EEMUL (`SA134..SA141`) ou de corrupção no FTL **BGO** (`SA26..SA70`, 45 setores na tabela `0x440337A4`) da Flash **Spansion `S29JL064H70TFI00`** (`8 MiB`, `142` setores `SA0..SA141`, base `0x80000000` / uncached `0x84000000`).

## 1. Quando Usar & Handoffs
- Ative ao limpar NVRAM (`64 KiB`), auditar/reparar setores BGO, montar kit SD/DVD de recuperação ou ajustar `ifodev2`/`M6930LB`.
- Para operar a bancada e `PANAMON1>` ao vivo, use `.agents/skills/panadiga-bench-operations/SKILL.md`; para validar integridade do dump antes, use `.agents/skills/panadiga-hardware-forensics/SKILL.md`.
- Para fronteiras `CapabilityGate` (`T0..T4`) e `LabOverride`, consulte `.agents/skills/panadiga-security-and-violation-guards/SKILL.md`.

## 2. Política Estrita de 3 Zonas (`IC5001` 8 MiB)
1. **Zona 1 — IPL Crítico (`SA0..SA10`, `256 KiB`: `8×8K + 3×64K`, `0x80000000..0x8003FFFF`):** **LEITURA ESTRITA PERMANENTE (`E05`)** na sonda Pico. Preserva `Loader ver0.61` e evita brick irreversível sem gravador TSOP-48.
2. **Zona 2 — Upper NOR & BGO FTL (`SA11..SA133`, `0x80040000..0x807EFFFF`):** Protegida por padrão (`E05`). Abrange os **45 setores BGO (`SA26..SA70`, `0x84130000..0x843FFFFF`)** (`1` `0xF101` + `40` `0xF2xx` + `4` spares). Reparo CoW offline (`bgo-repair-plan`) ou live via `bgo-live-apply` com `--backup-dir` + `--confirm-upper-nor` (`U CONFIRM_UPPER_NOR` / RAII `U LOCK`).
3. **Zona 3 — NVRAM / EEMUL (`SA134..SA141`, últimos `64 KiB`, `0x807F0000..0x807FFFFF`):** Escrita autorizada para reset cirúrgico de NVRAM preservando intactos os primeiros `8.323.072` bytes (`0x000000..0x7EFFFF`).

## 3. Workflow Canônico (Comandos Verificados)

### A. Auditoria & Reparo CoW Offline (`NVRAM` & `BGO 45 Setores`)
```bash
# Limpar 64 KiB NVRAM em arquivo novo + auditar 142 setores (bloqueia modelos < T3 sem LabOverride)
cargo run -q -p panadiga-cli -- nvram clean <DUMP_ORIG.bin> <DUMP_CLEAN.bin>
cargo run -q -p panadiga-cli -- nvram audit-nor-sectors --image firmware_and_images/eh55_nor_8mb_ic5001_dump.bin -j -
cargo run -q -p panadiga-cli -- nvram patch-upper-nor -i <IN.bin> -o <OUT.bin> --offset <HEX> --hex "<BYTES>"

# Auditoria & plano CoW BGO (ATENÇÃO: flags reais são --nor e --out-plan / --cow-out)
cargo run -q -p panadiga-cli -- unbrick bgo-audit --nor firmware_and_images/eh55_nor_8mb_ic5001_dump.bin -j -
cargo run -q -p panadiga-cli -- unbrick bgo-repair-plan --nor <DUMP.bin> --out-plan tmp/plan.json --cow-out tmp/repaired.bin -j -

# Aplicação ao vivo via PANAMON1> (exige --backup-dir + --confirm-upper-nor + model T4)
cargo run -q -p panadiga-cli -- unbrick bgo-live-apply -p COM10 --plan tmp/plan.json --backup-dir tmp/bgo_backup --confirm-upper-nor -j -
```

### B. Kit SD `ver061`, `rebuild-from-dump` & Mídia Física (`sd create` / `dvd create`)
```bash
# Kit canônico EH55 ver061: DIGADIAG.FRM (262.144 B, 305cc2ee...) + PANA__SD.FRM (7.563.224 B sd48, 2d3d62a7...)
cargo run -q -p panadiga-cli -- unbrick prepare-sd --loader-profile ver061 -o firmware_and_images/sd_unbrick_kit -j -
cargo run -q -p panadiga-cli -- unbrick verify-sd firmware_and_images/sd_unbrick_kit -j -

# Reconstruir kit diretamente de dump NOR íntegro + Partição 1 MAIN (gate < T3 sem --allow-unverified-profile)
cargo run -q -p panadiga-cli -- unbrick rebuild-from-dump --model DMR-EH55 --nor-dump <NOR.bin> --hdd-main <P1.bin> --target-gb 250 -o tmp/sd_rebuild -j -

# Gravação física segura em cartão SD FAT16B (<= 2 GiB SDSC) ou ISO/DVD
cargo run -q -p panadiga-cli -- sd targets -j -
cargo run -q -p panadiga-cli -- sd create --out tmp/recovery_sd.img --model DMR-EH55 --loader-profile ver061 --cleanroom -j -
```
- **MCP:** `panadiga_clean_nvram`, `panadiga_nor_sector_audit`, `panadiga_eh55_bgo`, `panadiga_sd_unbrick_kit`.

## 4. Lições Críticas da Cura BGO em 2 Ciclos & Semicondutores
- **Postmortem da Recaída (`SA41 + 0x14` & `Fvu` 35 setores):** Um slot aberto de rollback em `SA41 + 0x14` (`FUN_4401F536`) e o `Fvu` stock de `35` setores causaram perda da `0xF101` ao tratar `SA70` (64 KiB reais) como `8×8 KiB`. O **Ciclo 2** consolidou exatamente **`1` `0xF101` (`SA29`) com `+0x14` e `+0x5C = 0xFFFF`**, `40` `0xF2xx`, **`4` spares limpos (`SA41, 67, 68, 70`)** e `19/19` cadeias FAT válidas (`eh55_nor_8mb_ic5001_repaired.bin`, `bd089ed1...`), curando o `U99` (`ferr=0`).
- **Incompatibilidade Fatal `ES10`/`ES15`/`ES35V`:** Nunca grave imagens NOR dessas linhas no `EH55` — o `MAIN XIP` (`0x130000..0x40FFFF`) destrói `SA26..SA70`.
- **TSOP-48 Físico:** Mantenha Pino 14 **`WP#/ACC` em HIGH (`3.3V`)** (senão `SA140`/`SA141` ficam protegidos por hardware) e respeite erase real de **`~700 ms`** (polling `0xFF × 32`).

## 5. Verificação
- Pós-reparo ao vivo: `BGO_INSPECT` com `healthy=YES`, `0` blocos corrompidos, zero diferença em `SA0..SA25` e `SA71..SA141`, e `BOOTTRACE` concluindo sem `U99`.
