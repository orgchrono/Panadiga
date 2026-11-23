> 🌐 **Idioma / Language:** **Português (BR) & English (US)** · [Portal `docs/INDEX.md`](./INDEX.md) · [`translation_pairs.json`](./translation_pairs.json) · [Evidence Ledger (`ledger.json`)](./evidence/ledger.json) · [Logs `COM10` (`22`)](./debug_logs/README.md)

# Panadiga — Estado Verificado do Ecossistema & Guia Consolidado (Diátaxis)
# Panadiga — Verified Ecosystem Status & Consolidated Guide (Diátaxis)

Este painel centraliza a matriz verificável de evidências ([`docs/evidence/ledger.json`](./evidence/ledger.json)) e consolida os **39 pares de dossiês bilíngues** + auditorias raiz em **5 Trilhas Temáticas Diátaxis**, eliminando sobreposições sem quebrar os vínculos históricos.

This dashboard centralizes the verifiable evidence ledger ([`docs/evidence/ledger.json`](./evidence/ledger.json)) and organizes all **39 bilingual dossier pairs** plus root audits into **5 Consolidated Diátaxis Tracks**, eliminating ambiguity while preserving every historical link.

---

## 1. Taxonomia de Evidências / Evidence Maturity Levels (`ledger.json`)

| Nível / Level | Definição / Definition | Exemplos Verificados / Verified Examples |
| :--- | :--- | :--- |
| **`N5_REPRODUCED`** | Reproduzido independentemente fim-a-fim em unidade distinta | *(Aguardando 2ª unidade EH55 + desbloqueio `CH1`)* |
| **`N4_HARDWARE`** | Provado diretamente em hardware real `VEP79138` (`COM10` / multímetro / read-back SHA-256) | `EV-20261119-001` .. `EV-20261119-009` |
| **`N3_LIVE_PROBE`** | Telemetria passiva / barramento capturada sem alteração persistente | *(Incluído nas capturas `BOOTTRACE`/`BAUDSCAN` `22` logs)* |
| **`N2_EMULATED`** | Código MN103S / Spansion NOR executado no emulador Rust `Mn103Vm` | `EV-20261119-010` .. `EV-20261119-013`, `EV-20261120-017` |
| **`N1_STATIC`** | Triangulação estática Ghidra / disassembly bruto / esquemáticos PDF | `EV-20261120-014` .. `EV-20261120-016`, `EV-20261120-018` |

---

## 2. Matriz Completa do Ledger Verificável (`EV-20261119-001` .. `EV-20261120-018`)

| ID | Level | Escopo Técnico / Technical Claim | Evidência Primária / Primary Artifact(s) |
| :--- | :---: | :--- | :--- |
| `EV-20261119-001` | `N4_HARDWARE` | `Loader ver0.61`: `DIGADIAG.FRM` bruto `262.144 B` `@ 0x50400000` + `PANA__SD.FRM` `sd48` em SDSC FAT16 4-bit (`0x89001850`) | [`sddiag_ver061_live_20261119.log`](./debug_logs/sddiag_ver061_live_20261119.log) · [`DIGADIAG.FRM`](../firmware_and_images/README.md) |
| `EV-20261119-002` | `N4_HARDWARE` | `bgo_unbrick` + `PANAMON1>` (`0x5040A2BC` $\rightarrow$ `0x50439494`): dump `8 MiB` NOR & cirurgia Ciclo 1 (`bgo_repair_plan`) | [`apply_bgo_repair_com10_20261119.log`](./debug_logs/apply_bgo_repair_com10_20261119.log) · [`bgo_repair_plan_com10_20261119.json`](./evidence/bgo_repair_plan_com10_20261119.json) |
| `EV-20261119-003` | `N4_HARDWARE` | FTL BGO real (`45` setores `SA26..SA70` `@ 0x440337A4`) curada em 2 ciclos eliminando `U99` | [`verify_bgo_restored_boot_20261119.log`](./debug_logs/verify_bgo_restored_boot_20261119.log) · [`repair_bgo_sa39_sa54_com10_20261119.log`](./debug_logs/repair_bgo_sa39_sa54_com10_20261119.log) |
| `EV-20261119-004` | `N4_HARDWARE` | **Porta #1 (Door #1):** entrada headless em `PANAMON1>` sem `SW_UART` (JIG) via slot `PROG` `0x0B1800` (`PANA__SD.FRM` `cdcc2cdf...`, checksum `0x214E681`) | [`probe_ch1_full_diag_20261119.log`](./debug_logs/probe_ch1_full_diag_20261119.log) · [`PANA__SD.FRM`](../firmware_and_images/README.md) |
| `EV-20261119-005` | `N4_HARDWARE` | Polaridade assimétrica `DAVIS`: `CH0` ativo-BAIXO (`0x89000420=2`, `STATUS=0x50`) vs `CH1` ativo-ALTO (`0x89000C20=0`) | [`probe_fe_release_com10_20261119.log`](./debug_logs/probe_fe_release_com10_20261119.log) · [`probe_ch1_full_diag_20261119.log`](./debug_logs/probe_ch1_full_diag_20261119.log) |
| `EV-20261119-006` | `N4_HARDWARE` | Pinagem reversa `P58001` (`Pin k = ATA 41 - k`), prova `180°` (`0x00002004`) vs `0°` (`0x00000000`), e tensões verificadas na `IDE2SAT2` (`IORDY=3.34V`, `RESET-/DIOR-/DIOW-/CS0-/CS1-=2.45..2.47V`) | [`probe_ch1_live_v2_0deg_20261119.log`](./debug_logs/probe_ch1_live_v2_0deg_20261119.log) · [`probe_ch1_after_fe_com10_20261119.log`](./debug_logs/probe_ch1_after_fe_com10_20261119.log) |
| `EV-20261119-007` | `N4_HARDWARE` | Reconstrução brasileira `M6930LB` `ifodev2` (`2.048 B`, blocos `[0x0001, 0x0002, 0x0055, 0x0004]`, `f4f22dd0...`) da NOR reparada | [`eh55_ifodev2_m6930lb_2048b.bin` & `eh55_nor_8mb_ic5001_repaired.bin`](../firmware_and_images/README.md) |
| `EV-20261119-008` | `N4_HARDWARE` | Gravação e read-back SHA-256 do layout MEIHFS `250 GB` + espelho `LBA 488266096` em `PhysicalDrive1` | [`hdd_unbrick_flash_250gb_20261119.log`](./debug_logs/hdd_unbrick_flash_250gb_20261119.log) |
| `EV-20261119-009` | `N4_HARDWARE` | Timeout `~44s` sem SD faz `IC7501` pulsar `NSW5V=0` e `XRESET=0` (`PLEASE WAIT` / `SELF CHECK`) | [`diagnose_please_wait_90s_20261119.log`](./debug_logs/diagnose_please_wait_90s_20261119.log) |
| `EV-20261119-010` | `N2_EMULATED` | Proteção Spansion 2 camadas (`SA0..SA3` `E05` RO permanente, `U CONFIRM_UPPER_NOR`/`LOCK`, rejeição `'F'` `@ 0x50401F3C`) | [`cmd_handlers.rs`](../crates/panadiga-core/src/re/prog061_uart_monitor/cmd_handlers.rs) · [`checks_real_nor.rs`](../crates/panadiga-core/src/re/emu_patch_suite/checks_real_nor.rs) |
| `EV-20261119-011` | `N2_EMULATED` | `hdd::capacity_patch` (`3410PQ @ 0x574DD1FC`, `3060NP @ 0x574E499C`, `0xC6 -> 0xCA`, `5 GB..2 TB`) | [`specs.rs`](../crates/panadiga-core/src/hdd/capacity_patch/specs.rs) · [`checks_hdd_lba.rs`](../crates/panadiga-core/src/re/emu_patch_suite/checks_hdd_lba.rs) |
| `EV-20261119-012` | `N2_EMULATED` | `region_compat` (`DEV.BIN` `512 B`, `ifodev2` `2.048 B`, `.FRM` retarget, janela 21B `<deviceinfo>` `3410PQ`/`3060NP`) | [`main_fmu.rs`](../crates/panadiga-core/src/re/region_compat/main_fmu.rs) · [`checks_passthrough_region.rs`](../crates/panadiga-core/src/re/emu_patch_suite/checks_passthrough_region.rs) |
| `EV-20261119-013` | `N2_EMULATED` | `main_passthrough` (`3410PQ`: `0x5771ABFB`, `0x574F7B8C`, `0x574F7C5C`) após carga do `MAIN` em SDRAM | [`patcher.rs`](../crates/panadiga-core/src/re/main_passthrough/patcher.rs) · [`checks_passthrough_region.rs`](../crates/panadiga-core/src/re/emu_patch_suite/checks_passthrough_region.rs) |
| `EV-20261120-014` | `N1_STATIC` | Arquitetura Estágio 0 TBC: `AIOF` (`0x8800_0000`, `cps_ntsc=0x8020`, `cps_pal_m=0x8021`, `cps_pal=0x8027`, `FS0`, `CONT`) e `AVDD` (`0x8980_0000`) | [`tbc_mmio.rs`](../crates/panadiga-constants/src/constants/tbc_mmio.rs) · [`tbc_appliance/mod.rs`](../crates/panadiga-core/src/re/tbc_appliance/mod.rs) |
| `EV-20261120-015` | `N1_STATIC` | Dependência Main PCB (`IC3001`/`IC4001`), `IIC0`/`IIC1`, `IC7501 TBUS` (`sum - 0x5B`, `WDT 0x8A`) e bloqueio `0xC000_100C` | [`tbc_mmio.rs`](../crates/panadiga-constants/src/constants/tbc_mmio.rs) · [`doc_boot_core.txt`](./pseudocode/raw/doc_boot_core.txt) |
| `EV-20261120-016` | `N1_STATIC` | Incompatibilidade física `ES10`/`ES15`/`ES35V` vs `EH55` (`MAIN XIP` `0x130000..0x40FFFF` destrói BGO `SA26..SA70`) | [`dmr-es35v/README.md`](../firmware_and_images/dmr-es35v/README.md) · [`tbc_appliance/mod.rs`](../crates/panadiga-core/src/re/tbc_appliance/mod.rs) |
| `EV-20261120-017` | `N2_EMULATED` | Keepalive `M0` `IC7501 TBUS` (`build_ic7501_wdt_255s_frame` + `emit_m0_tbus_wdt_keepalive_stub`) executado em `Mn103Vm` (`D0=0x30`, mailbox `0xDC01_0010`) | [`keepalive.rs`](../crates/panadiga-core/src/re/tbc_appliance/keepalive.rs) · [`tbc_appliance/tests.rs`](../crates/panadiga-core/src/re/tbc_appliance/tests.rs) |
| `EV-20261120-018` | `N1_STATIC` | Triangulação `MAIN3410PQ`: `82` registros `AIOF` (`0x578935FC..0x57893888`), `98` presets (`0x578957D8..0x5789595C`), `AIOF_V` `0x8001` (`+0x66`), `IC3001` I²C (`0x4A`/`0x94`) | [`tbc_mmio.rs`](../crates/panadiga-constants/src/constants/tbc_mmio.rs) · [`doc_main.txt`](./pseudocode/raw/doc_main.txt) |

---

## 3. Consolidação Diátaxis em 5 Trilhas de Leitura / Consolidated Diátaxis Tracks

Para leitura direta por objetivo (sem duplicar esforço), os **39 pares de dossiês** agrupam-se em 5 trilhas canônicas:

1. **Trilha A — Bancada, Pinagem `P7501`, Pico C11 & Standby `IC7501` (`Tutorials & How-To`):**
   - [Blueprint Elétrico](./pt-BR/BLUEPRINT_ESQUEMA_ELETRICO_BANCADA.md) / [EN](./en-US/BLUEPRINT_BENCH_WIRING_AND_SCHEMATIC.md) · [Guia `P7501`](./pt-BR/GUIA_BANCADA_P7501_SEM_DESSOLDA.md) / [EN](./en-US/BENCH_GUIDE_P7501_NO_DESOLDERING.md) · [Montagem Probe](./pt-BR/GUIA_MONTAGEM_E_FOTOS_PROBE_BANCADA.md) / [EN](./en-US/BENCH_PROBE_ASSEMBLY_AND_PHOTOS_GUIDE.md) · [Standby `IC7501`/`TBUS`](./pt-BR/ARQUITETURA_STANDBY_IC7501_TBUS_TW7501_E_MODOS_BOOT.md) / [EN](./en-US/STANDBY_IC7501_TBUS_TW7501_AND_BOOT_STATES_ARCHITECTURE.md) · [Guia Pico C11](./pt-BR/FIRMWARE_PICO_P7501_GUIA.md) / [EN](./en-US/PICO_P7501_FIRMWARE_GUIDE.md)
2. **Trilha B — Cura BGO `U99`, NVRAM Spansion & Criptografia `PANA_DVD`:**
   - [Teoria & Prática `U99`](./pt-BR/TEORIA_E_PRATICA_UNBRICK_U99_E_CRIPTOGRAFIA_MATSUSHITA.md) / [EN](./en-US/THEORY_AND_PRACTICE_U99_UNBRICK_MATSUSHITA_CRYPTO.md) · [Dossiê Forense `ver0.61`](./pt-BR/DOSSIE_FORENSE_BOOTLOADER_SDDIAG_EH55.md) / [EN](./en-US/FORENSIC_DOSSIER_BOOTLOADER_SDDIAG_EH55.md) · [Criptografia `PANA_DVD`](./pt-BR/DOSSIE_CRIPTOGRAFIA_E_FIRMWARE_PANA_DVD.md) / [EN](./en-US/DOSSIER_CRYPTOGRAPHY_AND_FIRMWARE_PANA_DVD.md) · [Histórico BGO/`CH1`](./pt-BR/HISTORICO_UNBRICK_EH55_BGO_CH1_DAVIS_E_REGIAO.md) / [EN](./en-US/EH55_UNBRICK_BREAKTHROUGH_BGO_CH1_DAVIS_AND_REGION.md)
3. **Trilha C — Bootloader `MN103S`, Ghidra, GDB & Ferramental (`Reference`):**
   - [Índice Pseudocódigo Ghidra](./pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) · [Armadilhas MN103S](./pt-BR/ARMADILHAS_MN103S_E_RESOLUCAO_GHIDRA.md) / [EN](./en-US/MN103S_SEVEN_TRAPS_AND_GHIDRA_RESOLUTION.md) · [Estratégia Ghidra/GDB](./pt-BR/ESTRATEGIA_INTEGRACAO_GHIDRA_GDB.md) / [EN](./en-US/GHIDRA_GDB_INTEGRATION_STRATEGY.md) · [Inventário & Operação](./pt-BR/INVENTARIO_TECNICO_E_GUIA_DE_OPERACAO.md) / [EN](./en-US/TECHNICAL_INVENTORY_AND_OPERATIONS_MANUAL.md)
4. **Trilha D — Storage `MEIHFS`, `CH1 IDE`/`DAVIS` & Drive Óptico `R6-MV4`:**
   - [Storage `MEIHFS`](./pt-BR/ARQUITETURA_STORAGE_MEIHDFS_PARTICOES.md) / [EN](./en-US/STORAGE_ARCHITECTURE_MEIHDFS_PARTITIONS.md) · [Emulador `R6-MV4`](./pt-BR/EMULADOR_MATSHITA_R6MV4_E_INJECAO_ATAPI.md) / [EN](./en-US/MATSHITA_R6MV4_EMULATOR_AND_ATAPI_INJECTION.md)
5. **Trilha E — TBC Autônomo `Clean-Room`, `PAL-M` (`LBS`), Perfis SSOT & Arquitetura de Sistema:**
   - [TBC Autônomo & `DIGADIAG`](./pt-BR/DIGADIAG_PANA_SD_E_TBC_AUTONOMO.md) / [EN](./en-US/DIGADIAG_PANA_SD_AND_STANDALONE_TBC.md) · [Blueprint Arquitetura](./pt-BR/ARQUITETURA_E_BLUEPRINT_DE_ENGENHARIA.md) / [EN](./en-US/SYSTEM_ARCHITECTURE_AND_ENGINEERING_BLUEPRINT.md) · [Catálogo Perfis DIGA (`89` Modelos)](./pt-BR/CATALOGO_E_AUDITORIA_PERFIS_HARDWARE_LINHA_DIGA.md) / [EN](./en-US/DIGA_LINEUP_HARDWARE_PROFILES_CATALOG_AND_AUDIT.md) · [Qualidade & 9 Gates](./pt-BR/ARQUITETURA_DE_TESTES_E_CONFORMIDADE.md) / [EN](./en-US/TESTING_ARCHITECTURE_AND_COMPLIANCE.md) · [Zero-Error RAG](./pt-BR/ZERO_ERROR_RAG_ENGINE_E_SEGURANCA_FISICA.md) / [EN](./en-US/ZERO_ERROR_RAG_ENGINE_AND_PHYSICAL_SAFETY.md)
