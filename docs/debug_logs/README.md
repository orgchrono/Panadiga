> 🌐 **Language / Idioma:** **Bilingual (EN / PT-BR)**
> 🧭 **Portal:** [Docs Index](../INDEX.md) | [Complete Audit (EN)](../../COMPLETE_AUDIT.md) | [Auditoria Completa (PT-BR)](../../AUDITORIA_COMPLETA.md) | [Audits Archive](../audits/README.md) | [Ghidra Pseudocode](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md)

# Logs de Bancada (`COM10` / `P7501`) e Dossiê Unbrick / Live Bench UART Captures

**PT-BR:** Este diretório reúne o dossiê de bancada consolidado e as **28 capturas e transcrições verificadas** (`SHA-256` íntegros) coletadas diretamente no **Panasonic DMR-EH55** via sonda Pico **`P7501`** (`COM10`, `115200 8N1`) e no host Windows (`\\.\PhysicalDrive1`). Todas as capturas UART (`Pure UART Captures` e prefixos `[BOOTTRACE]`/`[CTRL]` nos transcritos híbridos) podem ser decodificadas offline pelo comando nativo em Rust:
**EN-US:** This directory stores the consolidated hardware unbrick dossier and **28 verified bench captures and transcripts** (`SHA-256` verified) recorded live from a **Panasonic DMR-EH55** via the **`P7501`** Pico probe (`COM10`, `115200 8N1`) and Windows host (`\\.\PhysicalDrive1`). All UART captures (`Pure UART Captures` and `[BOOTTRACE]`/`[CTRL]` frames inside hybrid transcripts) can be decoded offline using the native Rust decoder:

```powershell
cargo run -p panadiga-cli -- pico decode-log --file docs/debug_logs/<arquivo>.log
```

> [!NOTE]
> **`captura_post_probe_20261115_030851.log` (Deduplication / Deduplicação):** `captura_post_probe_20261115_030851.log` (`3,265 B`, `SHA-256: 987ef8c43925faca...`) is byte-identical to (é idêntico byte a byte a) [`captura_sd_flat_fix_20261115_025903.log`](./captura_sd_flat_fix_20261115_025903.log), so only the earlier chronological capture (`02:59:03`) is retained here.

## Documento Consolidado / Consolidated Text Dossier

- [**`DOSSIE_COMPLETO_EH55_UNBRICK_E_FIRMWARE.txt`**](./DOSSIE_COMPLETO_EH55_UNBRICK_E_FIRMWARE.txt) (`36.628 B`, `SHA-256: 79d8a00ebb92975b...`) — Síntese operacional de bancada e diagnóstico do EH55 / Consolidated EH55 bench recovery & firmware operational synthesis.

---

## 1. Capturas Puras de UART (`16` Pure UART Captures — `panadiga pico decode-log`)

Fluxo direto do Pico `P7501` (`UARTMON`, `UARTSTREAM`, `BOOTTRACE`) sem cabeçalhos de orquestração Python externos:

| Arquivo `.log` / File | Tamanho / Size | SHA-256 Verificado | Loader / Boot Mode | `startup reason` / `BGO` | Erros SD (`SDCD`) / Drive (`RTSD`) | Leitura `VDD` |
| :--- | ---: | :--- | :--- | :--- | :--- | ---: |
| [`captura_live_20261112_143843.log`](./captura_live_20261112_143843.log) | 126 B | `5F8A1712DD0C16416CB6D91606E15D8E6D612C1F25F30BE9F952CA233148690D` | `DWIRE32A` banner (`UARTMON 30000`) | — | — | — |
| [`captura_live_20261112_144101.log`](./captura_live_20261112_144101.log) | 324.209 B | `C7775A5C93F20327628D4C7FB37759CB7A3A213C8875455B64C8CCA9725BD815` | `Loader ver0.61 [06/03/06]` · `cd_diag` | `70` | SD `-91006/-90002/-90001`; `80020401`, `80040904` (1,361 SENSE) | — |
| [`captura_live_20261112_151352.log`](./captura_live_20261112_151352.log) | 126 B | `FC82AFBC6E9F23B9064DD39C03323FD1BCD32F7E098DE5C7122CE84C0727B62A` | `DWIRE32A` banner (`UARTMON 30000`) | — | — | — |
| [`captura_samsung_2gb_20261115_020351.log`](./captura_samsung_2gb_20261115_020351.log) | 3.238 B | `E1475AA221FF7032563BD0DEE0B35037AEF338DB7F1D0219C3746D5100DBFC21` | `Loader ver0.61 [06/03/06]` · `cd_diag` (UTF-16LE) | `5` | SD `-91006/-90002`; `80020401`, `80040904` (4 SENSE) | `3.24 V` |
| [`captura_live_20261115_021346.log`](./captura_live_20261115_021346.log) | 283 B | `BA5F1CE793E1D3C258D1824FDFE1336FDB767D89B442CB9BB8BCA35D4B41B8BB` | `PANADIGA>` `UARTMON 30000 ms` (0 bytes) | — | — | — |
| [`captura_flat_odd_20261115_021446.log`](./captura_flat_odd_20261115_021446.log) | 2.611 B | `B16843875FFD03792483C2650D81751A4BE7FF3A5E15480D27486082CF522955` | `Loader ver0.61 [06/03/06]` · `cd_diag` (`PINCHECK`) | `70` | SD `-90002` | `3.24 V` |
| [`captura_flat_odd_85s_20261115_021614.log`](./captura_flat_odd_20261115_021614.log) | 4.834 B | `62A9AB97E8123CB744ADC16FCCB789F21F3AD16F71E3187DF1964C626E4C1E08` | `Loader ver0.61 [06/03/06]` · `cd_diag` (`85s`) | `70` | SD `-90002`; `80020401`, `80040904` (14 SENSE `0x83A7`) | — |
| [`captura_sdhc32gb_cd_20261115_022258.log`](./captura_sdhc32gb_cd_20261115_022258.log) | 2.453 B | `852E031F85E091B86ED2A66D5BB0BFAF9A3C50A14D436E625EBBE71587B4C61D` | `Loader ver0.61 [06/03/06]` · `cd_diag` (SDHC 32 GB) | `70` | SD `-90002`; `80020401`, `80040904` (5 SENSE) | `3.24 V` |
| [`captura_sdhc32gb_cd_75s_20261115_022548.log`](./captura_sdhc32gb_cd_75s_20261115_022548.log) | 12.447 B | `F91215F9B6B1F1C4A2E9EFEBAF5066F8DAFB8E5F2EA062D347B1F936B5B03083` | `Loader ver0.61 [06/03/06]` · `normal` (post retry) | `70` / `BGO=1` | SD `-90002`; `80020401`, `80040904` (52 SENSE) | — |
| [`captura_retest_cabo_20261115_023052.log`](./captura_retest_cabo_20261115_023052.log) | 8.205 B | `5DE5C4C19DDDC0BCB1EA572FBA722B62797D4EFD41DBF53407782B1FA4E32020` | `Loader ver0.61 [06/03/06]` · `cd_diag` (cable retest) | `70` | SD `-90001` (no card); `80020401`, `80040904` (29 SENSE) | `3.25 V` |
| [`captura_sd_retest_20261115_023237.log`](./captura_sd_retest_20261115_023237.log) | 3.266 B | `2DA382B028FF92BE973315281D90AFD8B9A12DD891DC99EE566CFF8B36DA5703` | `Loader ver0.61 [06/03/06]` · `cd_diag` (UTF-16LE) | `70` | SD `-91006/-90002`; `80020401`, `80040904` (9 SENSE) | `3.25 V` |
| [`captura_sd_flat_fix_20261115_025903.log`](./captura_sd_flat_fix_20261115_025903.log) | 3.265 B | `987EF8C43925FACA27C4CC4D458FF3C6C738168D9078ECE6AA63A819AE8B2C54` | `Loader ver0.61 [06/03/06]` · `cd_diag` (`JIG:2500`) | `5` | SD `-91006/-90002`; `80020401`, `80040904` (9 SENSE) | `3.25 V` |
| [`sddiag_ver061_live_20261119.log`](./sddiag_ver061_live_20261119.log) | 1.147 B | `6C5EC83CC18ABFF4233F50DC1B1586AA049C0F60C39AB81F424E9F0CCBC63B25` | `Loader ver0.61 [06/03/06]` · `serial boot mode` (`UARTSTREAM`, `641 B`, `ferr=0`) | `70` | SD `0x3c4000[LBA]` `4-bit` OK -> `Fvu ver0.61 [05/11/01]` | `3.25 V` |
| [`verify_bgo_restored_boot_20261119.log`](./verify_bgo_restored_boot_20261119.log) | 5.434 B | `B102000ED7E38CE5DA9BA3954ED316E540413FFF3B1CF8BC204588BCE7EBCDFC` | `Loader ver0.61 [06/03/06]` · `normal mode` (`BOOTTRACE 15000 NOJIG`) | `70` / Loader BGO OK! | `CH0` OK -> `CH1` `80000404`/`E8000000` -> `[nfrom] file_no[17]` -> stock `Fvu 05/11/01` (`0x2182DC9`, `BGO driver init failed[1]`) | `3.30 V` (`SW_PU`) |
| [`sd_fvu_upd_ng_diag_20261122.log`](./sd_fvu_upd_ng_diag_20261122.log) | 8.881 B | `9FFC8842185F5E799E9131F4326C3BFB1028CFC4F3E0C881857CE65F5459D839` | `Loader ver0.61 [06/03/06]` · `normal mode` (`BOOTTRACE 60000 NOJIG`, retargeted `PANA__SD.FRM` `0x218A128`) | `70` / `BGO` OK (0 `U99`) | Retargeted FVU mounts BGO & verifies `MAIN3410PQ` (`0x2f825ed9`), then fails on `CH1` (`80000404`/`E8000000` `Reg[00..00]` `HDD MAX_LBA get failed`) | `3.28 V` |
| [`boottrace_cabo_novo_20261122.log`](./boottrace_cabo_novo_20261122.log) | 7.695 B | `681AA1AAD2783D658B78CC52F5310032091C9AB78DE909C1C54A4477B9771A75` | `Loader ver0.61 [06/03/06]` · `normal mode` (`BOOTTRACE 20000 NOJIG`, `PANA__SD.FRM` `MAIN3060NP` `0x644FD8`) | `70` / `BGO` OK (0 `U99`) | Pós-cabo flat novo: FVU `0x218A128` valida `MAIN3060NP` (`0x3287e591`) e falha na escrita `start to update mainsoft.` (`80000404`/`E8000000` `Reg[00..00]`) | `3.25..3.30 V` |

---

## 2. Transcritos de Automação Python & Sondas Híbridas (`11` Python & Probe Bench Transcripts)

Logs estruturados emitidos pelos harnesses de bancada em `COM10` combinando `STATUS`, `BOOTTRACE` e sessões interativas `PANAMON1>` (compatíveis com `panadiga pico decode-log`, preservando linhas `[PRECHECK]`/`[1A..2D]`):

| Arquivo `.log` / File | Tamanho / Size | SHA-256 Verificado | Classe / Escopo | Evidência Empírica Registrada |
| :--- | ---: | :--- | :--- | :--- |
| [`apply_bgo_repair_com10_20261119.log`](./apply_bgo_repair_com10_20261119.log) | 1.898 B | `928CA5AD858DC1FFD677FC7F38AB2A456CB6034A38199F407E85046B57BFA8BC` | Cirurgia FTL BGO Ciclo 1 ao vivo via `PANAMON1>` (`U CONFIRM_UPPER_NOR` / `U LOCK`) | Formata `SA70` (`256` zeros em `20.7s`) + `1.629` escritas puras `1->0` em 13 setores (`SA42..SA66`) + `ifodev2_blk2` (`0x0055`) + páginas FAT `0x0F0A`/`0x0F19` com read-back 100% verificado. |
| [`diagnose_please_wait_90s_20261119.log`](./diagnose_please_wait_90s_20261119.log) | 3.741 B | `2880A883B7233C9C234DDEE450B665E1FA8ACE32C802BA44BD9E23F4FCC64DE6` | Harness `STATUS` + `BOOTTRACE 90000 NORMAL` + `UARTSTREAM 40000 PASSIVE` | Prova ciclos `PLEASE WAIT` / reset WDT (`~43.3s` em `+44.9s` / `+55.0s` / `+74.5s`) antes da isolação do bloqueio em `CH1`. |
| [`repair_bgo_sa39_sa54_com10_20261119.log`](./repair_bgo_sa39_sa54_com10_20261119.log) | 5.782 B | `E17D9AC36CF31CEAA876673931052C8393CA817AE838DF3D9D1FDCBFF4B3206E` | Harness `STATUS` + `BOOTTRACE 15000 NOJIG` (`--boot-only` pós-reparo `SA54/SA41/SA29/SA39`) | Confirma `VDD: 3.25V (OK)`, `Loader ver0.61 [06/03/06]` -> `normal mode` -> `*** RTSC Initialize ***` **sem `U99` no Loader**; falha `CH1` `80000404`/`E8000000` -> `[nfrom] file_no[17]` -> fallback em FVU stock `05/11/01` (`0x2182DC9`). |
| [`probe_fe_release_com10_20261119.log`](./probe_fe_release_com10_20261119.log) | 2.839 B | `7973A4AB246AD5D17BCBCD1522979608E4F3D47FA58D836A3E652A189C9BC948` | Sonda `DAVIS` pós-liberação F/E (`0x90500000` + `TM2`/`TM11` + `0x9040C700`) | Prova `CH0` (`[2A]`/`[2B]` `[RTSD] ATAPI Software Reset` OK em `2.20s`, `STATUS=0x50` / `0xEB14` ATAPI) e isola travamento em `CH1_DEVCTL_89000D58` (`R 89000D58,4` timeout `2.5s`) enquanto `CH1_CTL_89000C00` responde `030000007A52...`. |
| [`probe_ch1_after_fe_com10_20261119.log`](./probe_ch1_after_fe_com10_20261119.log) | 809 B | `3AB8BE1378C9430A3A60E3C0F1016DAB047F6EBB4247A518D70368BEEE532E64` | Sonda `CH1 b444(1) + b55b(0,1,0)` após liberação F/E & `TM11` | Registra `[2D FAIL]` timeout em `J 9040C700` (`5.0s`) sobre `CH1` antes da rotação de orientação `IDE2SAT2` (`180°` vs `0°`). |
| [`probe_ch1_full_diag_20261119.log`](./probe_ch1_full_diag_20261119.log) | 12.244 B | `A6E71B147C17BAAE6F5F139C3F6D8F604E9C1461B7F22E557A00DBEA8FC0646C` | Híbrido `BOOTTRACE 15000 NORMAL` + `PANAMON1>` (`MN103S` RAM probe) | Entrada direta em `PANAMON1>` via `PANA__SD.FRM` `cdcc2cdf...` (`0x214E681`) sem JIG; `J 504156AD` (`0x504F81D0=2` vs `0x504F81D4=1`); `CH0=0x50` vs `IDE2SAT2 180° = 0x00002004` (`DD13\|DD2`). |
| [`probe_ch1_live_v2_0deg_20261119.log`](./probe_ch1_live_v2_0deg_20261119.log) | 7.455 B | `F6225540D20818667FF7C81BF48FC45D202D115BCDC71A3F5ECC656C7E935705` | Híbrido `BOOTTRACE 15000 NORMAL` + `PANAMON1>` (`MN103S` RAM probe) | Orientação **`0°`** retorna `0x00000000` em `CH1` (`2A..2C`, sem travamento em `0x89000D58`) e `FUN_5041CD4D(1)` `rc=0x00000000` (`struct_504F8234`: `is_init=1, is_atapi=0, is_ata=0`). |
| [`test_b_jigboot_panamon1_20261122_033201.log`](./test_b_jigboot_panamon1_20261122_033201.log) | 2.054 B | `068F58C819F5A7460BFDD8A893A484C94B19985F608059C4A42A06B0A1D139B9` | Híbrido `BOOTTRACE 10000 JIG` + `BRIDGE` `PANAMON1>` | Retargeted `DIGADIAG.FRM` (`305cc2ee...`) entra em `PANAMON1>` na primeira tentativa e lê `ifodev2` `"M6930LB"` (`4D363933304C42004D...`) em `0x84150604`. |
| [`davis_probe_ch1_20261122.log`](./davis_probe_ch1_20261122.log) | 320 B | `5120C52E929B8542F87D52B8120C152483CB443BEF8FE8FDCDC423C034262A5A` | Sonda `panadiga pico davis-probe --port COM10` (relatório JSON em [`davis_probe_ch1_20261122.json`](../evidence/davis_probe_ch1_20261122.json)) | Pós-troca de cabo flat novo + `IDE2SAT2` + HDD SATA 250 GB: `fe_released=true` (`C0001008=0x007F->0x807F`), `CH0` (`0xEB14`) ativo e `CH1` todos `0x00` (`ResetStuckOrUnclocked`). |
| [`davis_deep_probe_cabo_novo_20261122.log`](./davis_deep_probe_cabo_novo_20261122.log) | 2.678 B | `B4FDC37AFB7508B206F42DC7F70B154DE4358AFF690AC89936C4CFD109C7744A` | Híbrido `BOOTTRACE 8000 JIG` + `BRIDGE` `PANAMON1>` (cabo flat novo) | `PANAMON1>` lê bloco global `R 89000000,120 -> 4C466359...` no cabo flat novo antes do diagnóstico `davis-probe`. |
| [`davis_mmio_dump_20261122.log`](./davis_mmio_dump_20261122.log) | 2.854 B | `36661DFFAC675F5896E05F44384C1B442F83997DBA0DD8E2A4F9DF023B34D11B` | Dump MMIO DAVIS/SysCtrl via `PANAMON1>` (relatório JSON [`davis_probe_ch1_cabo_novo_20261122.json`](../evidence/davis_probe_ch1_cabo_novo_20261122.json)) | Comparativo `CH0_520` (`01 01 01 14 EB A0`) vs `CH1_D20` (`00..00` em 4 leituras `TEST_C20`), `BSS_81D0`, `C0001000=7F800000` e `DC0F0080=00202000`. |

---

## 3. Log de Host Windows & Verificação `PhysicalDrive1` (`1` Host PhysicalDrive1 Log)

Operação física de gravação Copy-on-Write (backup prévio de `64 KiB` + verificação read-back integral SHA-256) sobre alvo `250 GB`:

| Arquivo `.log` / File | Tamanho / Size | SHA-256 Verificado | Alvo / Dispositivo | Evidência Empírica Registrada |
| :--- | ---: | :--- | :--- | :--- |
| [`hdd_unbrick_flash_250gb_20261119.log`](./hdd_unbrick_flash_250gb_20261119.log) | 3.934 B | `021806054BAE733CB6E2514E604877EB75AB7B6C9E55C1E193339DD30F03B67C` | `\\.\PhysicalDrive1` (`250.059.350.016 B`, `BusType=7`, `488.397.168 LBAs`) | Backup prévio `65.536 B` (`354d38bd...`) substituindo MBR Windows `55aa` (`3db0e904...`); gravação e read-back SHA-256 verificados de `panasonic_dmr_eh55_hdd_250gb_patched.img` (`104.857.600 B`, `4fde753c...`, `LBA 0[16:512]` 100% zeros) + espelho P1 no `LBA 488266096` (`6.173.696 B`, `d932692c...`). |
