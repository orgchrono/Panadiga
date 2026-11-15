> 🌐 **Language / Idioma:** **Bilingual (EN / PT-BR)**
> 🧭 **Portal:** [Docs Index](../INDEX.md) | [Complete Audit (EN)](../../COMPLETE_AUDIT.md) | [Auditoria Completa (PT-BR)](../../AUDITORIA_COMPLETA.md) | [Audits Archive](../audits/README.md) | [Ghidra Pseudocode](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md)

# Logs de Bancada (`COM10` / `P7501`) e Dossiê Unbrick / Live Bench UART Captures

**PT-BR:** Este diretório reúne o dossiê de bancada consolidado e as **12 capturas únicas** (`SHA-256` verificados) coletadas diretamente no **Panasonic DMR-EH55** via sonda Pico **`P7501`** (`COM10`, `115200 8N1`). Todos os arquivos `.log` podem ser inspecionados offline pelo comando nativo em Rust:
**EN-US:** This directory stores the consolidated hardware unbrick dossier and **12 unique UART captures** (`SHA-256` verified) recorded live from a **Panasonic DMR-EH55** via the **`P7501`** Pico probe (`COM10`, `115200 8N1`). Every `.log` capture can be decoded offline using the native Rust decoder:

```powershell
cargo run -p panadiga-cli -- pico decode-log --file docs/debug_logs/<arquivo>.log
```

> [!NOTE]
> **`captura_post_probe_20261115_030851.log` (Deduplication / Deduplicação):** `captura_post_probe_20261115_030851.log` (`3,265 B`, `SHA-256: 987ef8c43925faca...`) is byte-identical to (é idêntico byte a byte a) [`captura_sd_flat_fix_20261115_025903.log`](./captura_sd_flat_fix_20261115_025903.log), so only the earlier chronological capture (`02:59:03`) is retained here.

## Documento Consolidado / Consolidated Text Dossier

- [**`DOSSIE_COMPLETO_EH55_UNBRICK_E_FIRMWARE.txt`**](./DOSSIE_COMPLETO_EH55_UNBRICK_E_FIRMWARE.txt) (`36.628 B`, `SHA-256: 79d8a00ebb92975b...`) — Síntese operacional de bancada e diagnóstico do EH55 / Consolidated EH55 bench recovery & firmware operational synthesis.

## Catálogo das 12 Capturas Únicas Decodificadas / 12 Decoded Unique Captures (`panadiga pico decode-log`)

| Arquivo `.log` / File | Tamanho / Size | Loader / Boot Mode | `startup reason` / `BGO` | Erros SD (`SDCD`) / SD Errors | Erros Drive (`RTSD`) / SENSE | Leitura `VDD` |
| :--- | ---: | :--- | :--- | :--- | :--- | ---: |
| [`captura_live_20261112_143843.log`](./captura_live_20261112_143843.log) | 126 B | `DWIRE32A` banner (`UARTMON 30000`) | — | — | — | — |
| [`captura_live_20261112_144101.log`](./captura_live_20261112_144101.log) | 324.209 B | `Loader ver0.61 [06/03/06]` · `cd_diag` | `70` | `-91006`, `-90002`, `-90001` | `80020401`, `80040904` (1,361 SENSE frames) | — |
| [`captura_live_20261112_151352.log`](./captura_live_20261112_151352.log) | 126 B | `DWIRE32A` banner (`UARTMON 30000`) | — | — | — | — |
| [`captura_samsung_2gb_20261115_020351.log`](./captura_samsung_2gb_20261115_020351.log) | 3.238 B | `Loader ver0.61 [06/03/06]` · `cd_diag` (UTF-16LE BOM-less) | `5` | `-91006`, `-90002` | `80020401`, `80040904` (4 SENSE) | `3.24 V` |
| [`captura_live_20261115_021346.log`](./captura_live_20261115_021346.log) | 283 B | `PANADIGA>` `UARTMON 30000 ms` session (0 bytes) | — | — | — | — |
| [`captura_flat_odd_20261115_021446.log`](./captura_flat_odd_20261115_021446.log) | 2.611 B | `Loader ver0.61 [06/03/06]` · `cd_diag` (`PINCHECK` + boot) | `70` | `-90002` | — | `3.24 V` |
| [`captura_flat_odd_85s_20261115_021614.log`](./captura_flat_odd_85s_20261115_021614.log) | 4.834 B | `Loader ver0.61 [06/03/06]` · `cd_diag` (`85s` UARTSTREAM) | `70` | `-90002` | `80020401`, `80040904` (14 SENSE `0x83A7`) | — |
| [`captura_sdhc32gb_cd_20261115_022258.log`](./captura_sdhc32gb_cd_20261115_022258.log) | 2.453 B | `Loader ver0.61 [06/03/06]` · `cd_diag` (SDHC 32 GB) | `70` | `-90002` | `80020401`, `80040904` (5 SENSE) | `3.24 V` |
| [`captura_sdhc32gb_cd_75s_20261115_022548.log`](./captura_sdhc32gb_cd_75s_20261115_022548.log) | 12.447 B | `Loader ver0.61 [06/03/06]` · `normal` (post serial+CD retry) | `70` / `BGO=1` | `-90002` | `80020401`, `80040904` (52 SENSE) | — |
| [`captura_retest_cabo_20261115_023052.log`](./captura_retest_cabo_20261115_023052.log) | 8.205 B | `Loader ver0.61 [06/03/06]` · `cd_diag` (cable retest + PINCHECK) | `70` | `-90001` (no card) | `80020401`, `80040904` (29 SENSE) | `3.25 V` |
| [`captura_sd_retest_20261115_023237.log`](./captura_sd_retest_20261115_023237.log) | 3.266 B | `Loader ver0.61 [06/03/06]` · `cd_diag` (UTF-16LE BOM-less) | `70` | `-91006`, `-90002` | `80020401`, `80040904` (9 SENSE) | `3.25 V` |
| [`captura_sd_flat_fix_20261115_025903.log`](./captura_sd_flat_fix_20261115_025903.log) | 3.265 B | `Loader ver0.61 [06/03/06]` · `cd_diag` (`JIG:2500` prior to flat fix) | `5` | `-91006`, `-90002` | `80020401`, `80040904` (9 SENSE) | `3.25 V` |
