> 🌐 **Language / Idioma:** **Bilingual Portal (English US / Português BR)**
> 🧭 **Root Docs:** [README (EN)](../README.md) | [README (PT-BR)](../README.pt-BR.md) | [Complete Audit (EN)](../COMPLETE_AUDIT.md) | [Auditoria Completa (PT-BR)](../AUDITORIA_COMPLETA.md) | [Agent Rules](../AGENTS.md)

# Panadiga Technical Documentation Index / Índice da Documentação Técnica

All **35** primary technical engineering dossiers in this repository maintain **1:1 bidirectional parity** between [**English (`docs/en-US/`)**](#english-documentation-docsen-us--35-dossiers) and [**Português (BR) (`docs/pt-BR/`)**](#documentação-em-português-docspt-br--35-dossiês), paired automatically with [`translation_pairs.json`](./translation_pairs.json), plus root audits, preserved `2026-11-11` audit snapshots ([`audits/`](./audits/README.md)), Ghidra C/disassembly evidence ([`pseudocode/`](./pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md)), and live `COM10` bench UART logs ([`debug_logs/`](./debug_logs/README.md)).

---

## English Documentation (`docs/en-US/`) — 35 Dossiers

| # | Document (`en-US`) | PT-BR Counterpart | Engineering Domain & Scope |
| :---: | :--- | :--- | :--- |
| 1 | [ADVERSARIAL_AUDIT_PICO_FIRMWARE_AND_PROG061_MAIN_DISASSEMBLY.md](en-US/ADVERSARIAL_AUDIT_PICO_FIRMWARE_AND_PROG061_MAIN_DISASSEMBLY.md) | [PT-BR](pt-BR/AUDITORIA_ADVERSARIAL_FIRMWARE_PICO_E_DISASSEMBLY_PROG061_MAIN.md) | Adversarial Pico C11/LED audit & live `COM10` vs `PROG061`/`MAIN` disassembly |
| 2 | [ANDROID_APK_AND_BLUETOOTH_ARCHITECTURE.md](en-US/ANDROID_APK_AND_BLUETOOTH_ARCHITECTURE.md) | [PT-BR](pt-BR/ARQUITETURA_MOBILE_ANDROID_E_BLUETOOTH.md) | Android 12–16 APK native HAL, Bluetooth RFCOMM (`SPP`), and Tauri 2 FFI |
| 3 | [BENCH_GUIDE_P7501_NO_DESOLDERING.md](en-US/BENCH_GUIDE_P7501_NO_DESOLDERING.md) | [PT-BR](pt-BR/GUIA_BANCADA_P7501_SEM_DESSOLDA.md) | Non-destructive `P7501` Async UART probe connection (`JST ZHR-6`) |
| 4 | [BENCH_PROBE_ASSEMBLY_AND_PHOTOS_GUIDE.md](en-US/BENCH_PROBE_ASSEMBLY_AND_PHOTOS_GUIDE.md) | [PT-BR](pt-BR/GUIA_MONTAGEM_E_FOTOS_PROBE_BANCADA.md) | Illustrated assembly guide, BOM, JST ZR specs & multimeter checklist |
| 5 | [BLUEPRINT_BENCH_WIRING_AND_SCHEMATIC.md](en-US/BLUEPRINT_BENCH_WIRING_AND_SCHEMATIC.md) | [PT-BR](pt-BR/BLUEPRINT_ESQUEMA_ELETRICO_BANCADA.md) | Bench electrical schematics (`P7501` UART, TSOP-48 `IC5001` & Pico) |
| 6 | [DIGADIAG_JUMP_50400000_AND_GUARDED_RW.md](en-US/DIGADIAG_JUMP_50400000_AND_GUARDED_RW.md) | [PT-BR](pt-BR/DIGADIAG_SALTO_50400000_E_ESCRITA_PROTEGIDA.md) | `JMP 0x50400000` (`EXTRHEAD`) & two-step guarded NOR/SDRAM R/W |
| 7 | [DIGADIAG_PANA_SD_AND_STANDALONE_TBC.md](en-US/DIGADIAG_PANA_SD_AND_STANDALONE_TBC.md) | [PT-BR](pt-BR/DIGADIAG_PANA_SD_E_TBC_AUTONOMO.md) | `DIGADIAG.FRM` vs `PANA__SD.FRM` / `PANA_DVD.FRM` & standalone TBC `E-E` |
| 8 | [DOSSIER_CRYPTOGRAPHY_AND_FIRMWARE_PANA_DVD.md](en-US/DOSSIER_CRYPTOGRAPHY_AND_FIRMWARE_PANA_DVD.md) | [PT-BR](pt-BR/DOSSIE_CRIPTOGRAFIA_E_FIRMWARE_PANA_DVD.md) | Feistel-32/CPRM cipher, periodic TOC (`0x7108`), and ECB flash leak proofs |
| 9 | [EH55_REVERSE_ENGINEERING_SESSION_FAITHFUL_REPORT.md](en-US/EH55_REVERSE_ENGINEERING_SESSION_FAITHFUL_REPORT.md) | [PT-BR](pt-BR/RELATORIO_SESSAO_ENGENHARIA_REVERSA_EH55_100_FIEL.md) | 100% verbatim transcript of EH55 live bench & Ghidra RE milestones |
| 10 | [EXPLODED_VIEW_PICO_INJECTION_ARCHITECTURE_AND_BYPASS_PASSTHROUGH.md](en-US/EXPLODED_VIEW_PICO_INJECTION_ARCHITECTURE_AND_BYPASS_PASSTHROUGH.md) | [PT-BR](pt-BR/VISTA_EXPLODIDA_ARQUITETURA_INJECAO_PICO_E_BYPASS_PASSTHROUGH.md) | End-to-end Pico `PANARSP` injection, `18` MCP tools & `MAIN` TBC bypass |
| 11 | [FINAL_AUDIT_AND_MODULARIZATION_REPORT.md](en-US/FINAL_AUDIT_AND_MODULARIZATION_REPORT.md) | [PT-BR](pt-BR/RELATORIO_AUDITORIA_E_MODULARIZACAO_FINAL.md) | Pre-refactor audit, modular decomposition & 9 Quality Gates architecture |
| 12 | [FORENSIC_AUDIT_REPORT_CRYPTANALYSIS_AND_MEIHDFS.md](en-US/FORENSIC_AUDIT_REPORT_CRYPTANALYSIS_AND_MEIHDFS.md) | [PT-BR](pt-BR/PARECER_AUDITORIA_FORENSE_CRIPTOANALISE_E_MEIHDFS.md) | Independent cryptographic & MEIHDFS forensic opinion + bench verdict |
| 13 | [FORENSIC_VALIDATION_INDEPENDENT_AUDIT_AND_COMPRESSION_DOSSIER.md](en-US/FORENSIC_VALIDATION_INDEPENDENT_AUDIT_AND_COMPRESSION_DOSSIER.md) | [PT-BR](pt-BR/DOSSIE_VALIDACAO_FORENSE_AUDITORIA_INDEPENDENTE_E_COMPACTACAO.md) | Binary compatibility proofs, audit framework & multi-algo compression |
| 14 | [GDB_GHIDRA_IDA_DEBUGGING_GUIDE.md](en-US/GDB_GHIDRA_IDA_DEBUGGING_GUIDE.md) | [PT-BR](pt-BR/GUIA_DEPURACAO_GDB_GHIDRA_IDA.md) | Live GDB / Ghidra / IDA Pro hardware debugging guide via `127.0.0.1:3333` |
| 15 | [GHIDRA_GDB_INTEGRATION_STRATEGY.md](en-US/GHIDRA_GDB_INTEGRATION_STRATEGY.md) | [PT-BR](pt-BR/ESTRATEGIA_INTEGRACAO_GHIDRA_GDB.md) | AM34 SSOT memory map, Jython scripts, GDB RSP bridge & HxD launcher |
| 16 | [GLOBAL_COMMUNITY_COMPENDIUM_DIGA_FIRMWARE_AND_HARDWARE.md](en-US/GLOBAL_COMMUNITY_COMPENDIUM_DIGA_FIRMWARE_AND_HARDWARE.md) | [PT-BR](pt-BR/COMPENDIO_GLOBAL_COMUNIDADE_FIRMWARE_E_HARDWARE_DIGA.md) | Global compendium: 30-URL matrix, remont-aud/elektroda, SERV mode & MEIHDFS |
| 17 | [LOGGING_AND_TELEMETRY_SUBSYSTEM.md](en-US/LOGGING_AND_TELEMETRY_SUBSYSTEM.md) | [PT-BR](pt-BR/SUBSISTEMA_LOGGING_E_TELEMETRIA.md) | `panadiga_core::logger` (`LogLevel`, `set_sink`, Tauri forwarding, macros) |
| 18 | [MATSHITA_R6MV4_EMULATOR_AND_ATAPI_INJECTION.md](en-US/MATSHITA_R6MV4_EMULATOR_AND_ATAPI_INJECTION.md) | [PT-BR](pt-BR/EMULADOR_MATSHITA_R6MV4_E_INJECAO_ATAPI.md) | `DRV12` / `R6-MV4` ATAPI `0x3B` write injection & optical unlock synthesis |
| 19 | [MN103S_SEVEN_TRAPS_AND_GHIDRA_RESOLUTION.md](en-US/MN103S_SEVEN_TRAPS_AND_GHIDRA_RESOLUTION.md) | [PT-BR](pt-BR/ARMADILHAS_MN103S_E_RESOLUCAO_GHIDRA.md) | Catalog of the 7 MN103S decompiler traps & Rust/Jython resolution |
| 20 | [PANASONIC_DVD_TV_IR_CAPTURE_REFERENCE.md](en-US/PANASONIC_DVD_TV_IR_CAPTURE_REFERENCE.md) | [PT-BR](pt-BR/REFERENCIA_CAPTURA_IR_PANASONIC_DVD_TV.md) | Flipper-IRDB `Panasonic_DVD_TV.ir` Kaseikyo decoding (`50` signals: TV `0x80` + DVD/HDD `0xB0`) |
| 21 | [PANASONIC_P7501_UART_RECONSTRUCTION.md](en-US/PANASONIC_P7501_UART_RECONSTRUCTION.md) | [PT-BR](pt-BR/PANASONIC_P7501_UART_RECONSTRUCTION.md) | `P7501` Async UART (`115200 8N1`) electrical signal reconstruction |
| 22 | [PICO_C11_UPGRADES_GHIDRA_UNDEF_AND_PYTHON_RUST_PARITY.md](en-US/PICO_C11_UPGRADES_GHIDRA_UNDEF_AND_PYTHON_RUST_PARITY.md) | [PT-BR](pt-BR/UPGRADES_PICO_C11_UNDEF_GHIDRA_E_PARIDADE_PYTHON_RUST.md) | C11 firmware upgrades, Ghidra `??` root causes & Python-to-Rust parity matrix |
| 23 | [PICO_P7501_FIRMWARE_GUIDE.md](en-US/PICO_P7501_FIRMWARE_GUIDE.md) | [PT-BR](pt-BR/FIRMWARE_PICO_P7501_GUIA.md) | Pico RP2040/RP2350 C11 firmware guide (`UARTMON`, `JIGBOOT`, `PANARSP`) |
| 24 | [RE_REPORT_FEISTEL_TOC_EXTRHEAD_AND_UART0_GATES.md](en-US/RE_REPORT_FEISTEL_TOC_EXTRHEAD_AND_UART0_GATES.md) | [PT-BR](pt-BR/RELATORIO_RE_FEISTEL_TOC_EXTRHEAD_E_GATES_UART0.md) | Static/dynamic RE of `FUN_4400a666`, `.FRM` TOCs, `EXTRHEAD` & UART0 gates |
| 25 | [RE_SCRIPTS_COMPENDIUM_STEPS_01_50.md](en-US/RE_SCRIPTS_COMPENDIUM_STEPS_01_50.md) | [PT-BR](pt-BR/COMPILADO_SCRIPTS_RE_STEPS_01_50.md) | Consolidated reference of Python RE & bench automation (`Steps 1–50`) |
| 26 | [REVERSE_ENGINEERING_BOOTLOADER_AND_OPTICAL_DRIVE_MN103S.md](en-US/REVERSE_ENGINEERING_BOOTLOADER_AND_OPTICAL_DRIVE_MN103S.md) | [PT-BR](pt-BR/DOSSIE_ENGENHARIA_REVERSA_BOOTLOADER_E_DRIVE_MN103S.md) | Deep RE of `BOOT.BIN`, `DRV_decomp`, `83 A7`, 431 errors, 62 CLI & `P7501` |
| 27 | [RP2350_AND_GDB_INTEGRATION_PLAN.md](en-US/RP2350_AND_GDB_INTEGRATION_PLAN.md) | [PT-BR](pt-BR/PLANO_INTEGRACAO_RP2350_E_GDB.md) | Dual-target RP2040 / RP2350 Pico SDK architecture & GDB RSP integration |
| 28 | [STORAGE_ARCHITECTURE_MEIHDFS_PARTITIONS.md](en-US/STORAGE_ARCHITECTURE_MEIHDFS_PARTITIONS.md) | [PT-BR](pt-BR/ARQUITETURA_STORAGE_MEIHDFS_PARTICOES.md) | MEIHDFS HDD layout (`'SH'`), `XOR 0xFF` records, Partitions 1–3 & `IDE2SAT2` |
| 29 | [SYSTEM_ARCHITECTURE_AND_ENGINEERING_BLUEPRINT.md](en-US/SYSTEM_ARCHITECTURE_AND_ENGINEERING_BLUEPRINT.md) | [PT-BR](pt-BR/ARQUITETURA_E_BLUEPRINT_DE_ENGENHARIA.md) | End-to-end Rust, MCP, Tauri 2 React, C11, Android & Python blueprint |
| 30 | [TECHNICAL_INVENTORY_AND_OPERATIONS_MANUAL.md](en-US/TECHNICAL_INVENTORY_AND_OPERATIONS_MANUAL.md) | [PT-BR](pt-BR/INVENTARIO_TECNICO_E_GUIA_DE_OPERACAO.md) | Cryptographic asset SHA-256 inventory, `30` PDFs, bench ops & FMEA |
| 31 | [TECHNICAL_REPORT_FOR_CLAUDE_OPUS_UNBRICK_DMR_EH55.md](en-US/TECHNICAL_REPORT_FOR_CLAUDE_OPUS_UNBRICK_DMR_EH55.md) | [PT-BR](pt-BR/RELATORIO_TECNICO_CLAUDE_OPUS_UNBRICK_DMR_EH55.md) | Cross-validated DMR-EH55 unbrick dossier, power/laser triage & `P7501` |
| 32 | [TESTING_ARCHITECTURE_AND_COMPLIANCE.md](en-US/TESTING_ARCHITECTURE_AND_COMPLIANCE.md) | [PT-BR](pt-BR/ARQUITETURA_DE_TESTES_E_CONFORMIDADE.md) | 4-layer Testing Pyramid, 9 Quality Gates & physical safety compliance |
| 33 | [THEORY_AND_PRACTICE_U99_UNBRICK_MATSUSHITA_CRYPTO.md](en-US/THEORY_AND_PRACTICE_U99_UNBRICK_MATSUSHITA_CRYPTO.md) | [PT-BR](pt-BR/TEORIA_E_PRATICA_UNBRICK_U99_E_CRIPTOGRAFIA_MATSUSHITA.md) | NOR `S29JL064H` flash physics, `EEMUL` `U99` cure, handshake & hex maps |
| 34 | [VISUAL_CATALOG_EH55_SCHEMATICS_AND_PCB.md](en-US/VISUAL_CATALOG_EH55_SCHEMATICS_AND_PCB.md) | [PT-BR](pt-BR/CATALOGO_VISUAL_ESQUEMAS_E_PCB_EH55.md) | Visual catalog (`238` entries) of DMR-EH55 schematics, PCBs & RE renders |
| 35 | [ZERO_ERROR_RAG_ENGINE_AND_PHYSICAL_SAFETY.md](en-US/ZERO_ERROR_RAG_ENGINE_AND_PHYSICAL_SAFETY.md) | [PT-BR](pt-BR/ZERO_ERROR_RAG_ENGINE_E_SEGURANCA_FISICA.md) | 6-layer Zero-Error RAG engine, `FactCritic` & physical voltage guardrails |

---

## Documentação em Português (`docs/pt-BR/`) — 35 Dossiês

| # | Documento (`pt-BR`) | Versão `en-US` | Área / Escopo Técnico |
| :---: | :--- | :--- | :--- |
| 1 | [ARMADILHAS_MN103S_E_RESOLUCAO_GHIDRA.md](pt-BR/ARMADILHAS_MN103S_E_RESOLUCAO_GHIDRA.md) | [EN-US](en-US/MN103S_SEVEN_TRAPS_AND_GHIDRA_RESOLUTION.md) | As 7 Armadilhas do decompilador MN103S/AM34 e resolução em Rust + Jython |
| 2 | [ARQUITETURA_DE_TESTES_E_CONFORMIDADE.md](pt-BR/ARQUITETURA_DE_TESTES_E_CONFORMIDADE.md) | [EN-US](en-US/TESTING_ARCHITECTURE_AND_COMPLIANCE.md) | Pirâmide de 4 camadas, 9 Quality Gates e guardas de segurança física |
| 3 | [ARQUITETURA_E_BLUEPRINT_DE_ENGENHARIA.md](pt-BR/ARQUITETURA_E_BLUEPRINT_DE_ENGENHARIA.md) | [EN-US](en-US/SYSTEM_ARCHITECTURE_AND_ENGINEERING_BLUEPRINT.md) | Blueprint mestre do ecossistema Rust, Tauri, Android, Pico C11 e Python |
| 4 | [ARQUITETURA_MOBILE_ANDROID_E_BLUETOOTH.md](pt-BR/ARQUITETURA_MOBILE_ANDROID_E_BLUETOOTH.md) | [EN-US](en-US/ANDROID_APK_AND_BLUETOOTH_ARCHITECTURE.md) | APK Android 12–16, Bluetooth `SPP` sem fio e integração FFI no Tauri 2 |
| 5 | [ARQUITETURA_STORAGE_MEIHDFS_PARTICOES.md](pt-BR/ARQUITETURA_STORAGE_MEIHDFS_PARTICOES.md) | [EN-US](en-US/STORAGE_ARCHITECTURE_MEIHDFS_PARTITIONS.md) | SSOT MEIHDFS: `'SH'`, `XOR 0xFF`, journal, partições 1–3 e ponte `IDE2SAT2` |
| 6 | [AUDITORIA_ADVERSARIAL_FIRMWARE_PICO_E_DISASSEMBLY_PROG061_MAIN.md](pt-BR/AUDITORIA_ADVERSARIAL_FIRMWARE_PICO_E_DISASSEMBLY_PROG061_MAIN.md) | [EN-US](en-US/ADVERSARIAL_AUDIT_PICO_FIRMWARE_AND_PROG061_MAIN_DISASSEMBLY.md) | Auditoria adversarial Pico (`P7501`/LED) e disassembly `PROG061` / `MAIN` |
| 7 | [BLUEPRINT_ESQUEMA_ELETRICO_BANCADA.md](pt-BR/BLUEPRINT_ESQUEMA_ELETRICO_BANCADA.md) | [EN-US](en-US/BLUEPRINT_BENCH_WIRING_AND_SCHEMATIC.md) | Esquemáticos elétricos de bancada (`P7501` UART, TSOP-48 `IC5001` e Pico) |
| 8 | [CATALOGO_VISUAL_ESQUEMAS_E_PCB_EH55.md](pt-BR/CATALOGO_VISUAL_ESQUEMAS_E_PCB_EH55.md) | [EN-US](en-US/VISUAL_CATALOG_EH55_SCHEMATICS_AND_PCB.md) | Catálogo visual completo (`238` itens) de esquemas, PCBs e diagramas RE |
| 9 | [COMPENDIO_GLOBAL_COMUNIDADE_FIRMWARE_E_HARDWARE_DIGA.md](pt-BR/COMPENDIO_GLOBAL_COMUNIDADE_FIRMWARE_E_HARDWARE_DIGA.md) | [EN-US](en-US/GLOBAL_COMMUNITY_COMPENDIUM_DIGA_FIRMWARE_AND_HARDWARE.md) | Compêndio global: matriz de 30 URLs, remont-aud/elektroda, modo SERV e MEIHDFS |
| 10 | [COMPILADO_SCRIPTS_RE_STEPS_01_50.md](pt-BR/COMPILADO_SCRIPTS_RE_STEPS_01_50.md) | [EN-US](en-US/RE_SCRIPTS_COMPENDIUM_STEPS_01_50.md) | Compilado de scripts Python de RE e bancada (`Steps 1–50`) |
| 11 | [DIGADIAG_PANA_SD_E_TBC_AUTONOMO.md](pt-BR/DIGADIAG_PANA_SD_E_TBC_AUTONOMO.md) | [EN-US](en-US/DIGADIAG_PANA_SD_AND_STANDALONE_TBC.md) | Consolidação `DIGADIAG.FRM` vs `PANA__SD.FRM` / `PANA_DVD.FRM` e TBC `E-E` |
| 12 | [DIGADIAG_SALTO_50400000_E_ESCRITA_PROTEGIDA.md](pt-BR/DIGADIAG_SALTO_50400000_E_ESCRITA_PROTEGIDA.md) | [EN-US](en-US/DIGADIAG_JUMP_50400000_AND_GUARDED_RW.md) | Salto `0x50400000` (`EXTRHEAD`) e leitura/gravação protegida (Guarded R/W) |
| 13 | [DOSSIE_CRIPTOGRAFIA_E_FIRMWARE_PANA_DVD.md](pt-BR/DOSSIE_CRIPTOGRAFIA_E_FIRMWARE_PANA_DVD.md) | [EN-US](en-US/DOSSIER_CRYPTOGRAPHY_AND_FIRMWARE_PANA_DVD.md) | Cifra Feistel-32/CPRM, TOC periódica (`0x7108`) e provas de vazamento ECB |
| 14 | [DOSSIE_ENGENHARIA_REVERSA_BOOTLOADER_E_DRIVE_MN103S.md](pt-BR/DOSSIE_ENGENHARIA_REVERSA_BOOTLOADER_E_DRIVE_MN103S.md) | [EN-US](en-US/REVERSE_ENGINEERING_BOOTLOADER_AND_OPTICAL_DRIVE_MN103S.md) | RE profunda `BOOT.BIN`, `DRV_decomp`, `83 A7`, 431 erros, 62 CLI e `P7501` |
| 15 | [DOSSIE_VALIDACAO_FORENSE_AUDITORIA_INDEPENDENTE_E_COMPACTACAO.md](pt-BR/DOSSIE_VALIDACAO_FORENSE_AUDITORIA_INDEPENDENTE_E_COMPACTACAO.md) | [EN-US](en-US/FORENSIC_VALIDATION_INDEPENDENT_AUDIT_AND_COMPRESSION_DOSSIER.md) | Compatibilidade forense de binários, auditoria independente e benchmark `.gz` |
| 16 | [EMULADOR_MATSHITA_R6MV4_E_INJECAO_ATAPI.md](pt-BR/EMULADOR_MATSHITA_R6MV4_E_INJECAO_ATAPI.md) | [EN-US](en-US/MATSHITA_R6MV4_EMULATOR_AND_ATAPI_INJECTION.md) | Emulador `R6-MV4`, injeção ATAPI `0x3B` e geração de unlock via SD/CD |
| 17 | [ESTRATEGIA_INTEGRACAO_GHIDRA_GDB.md](pt-BR/ESTRATEGIA_INTEGRACAO_GHIDRA_GDB.md) | [EN-US](en-US/GHIDRA_GDB_INTEGRATION_STRATEGY.md) | Mapa AM34 SSOT, scripts Ghidra, ponte GDB RSP `:3333` e lançador HxD |
| 18 | [FIRMWARE_PICO_P7501_GUIA.md](pt-BR/FIRMWARE_PICO_P7501_GUIA.md) | [EN-US](en-US/PICO_P7501_FIRMWARE_GUIDE.md) | Guia do firmware C11 no Pico (`UARTMON`, `JIGBOOT`, `PANARSP`, `DIAGWATCH`) |
| 19 | [GUIA_BANCADA_P7501_SEM_DESSOLDA.md](pt-BR/GUIA_BANCADA_P7501_SEM_DESSOLDA.md) | [EN-US](en-US/BENCH_GUIDE_P7501_NO_DESOLDERING.md) | Guia não-destrutivo de conexão ao `P7501` (`JST ZHR-6`) sem dessolda |
| 20 | [GUIA_DEPURACAO_GDB_GHIDRA_IDA.md](pt-BR/GUIA_DEPURACAO_GDB_GHIDRA_IDA.md) | [EN-US](en-US/GDB_GHIDRA_IDA_DEBUGGING_GUIDE.md) | Depuração interativa ao vivo com GDB, Ghidra e IDA Pro em `127.0.0.1:3333` |
| 21 | [GUIA_MONTAGEM_E_FOTOS_PROBE_BANCADA.md](pt-BR/GUIA_MONTAGEM_E_FOTOS_PROBE_BANCADA.md) | [EN-US](en-US/BENCH_PROBE_ASSEMBLY_AND_PHOTOS_GUIDE.md) | Roteiro ilustrado da sonda `P7501`, BOM, JST ZR e checklist no multímetro |
| 22 | [INVENTARIO_TECNICO_E_GUIA_DE_OPERACAO.md](pt-BR/INVENTARIO_TECNICO_E_GUIA_DE_OPERACAO.md) | [EN-US](en-US/TECHNICAL_INVENTORY_AND_OPERATIONS_MANUAL.md) | Inventário criptográfico SHA-256, `30` PDFs, rotas de bancada e FMEA |
| 23 | [PANASONIC_P7501_UART_RECONSTRUCTION.md](pt-BR/PANASONIC_P7501_UART_RECONSTRUCTION.md) | [EN-US](en-US/PANASONIC_P7501_UART_RECONSTRUCTION.md) | Especificação técnica de reconstrução elétrica `P7501` Async UART `115200` |
| 24 | [PARECER_AUDITORIA_FORENSE_CRIPTOANALISE_E_MEIHDFS.md](pt-BR/PARECER_AUDITORIA_FORENSE_CRIPTOANALISE_E_MEIHDFS.md) | [EN-US](en-US/FORENSIC_AUDIT_REPORT_CRYPTANALYSIS_AND_MEIHDFS.md) | Parecer conclusivo de auditoria forense (`PANA_DVD.FRM`, `MEIHDFS` e bancada) |
| 25 | [PLANO_INTEGRACAO_RP2350_E_GDB.md](pt-BR/PLANO_INTEGRACAO_RP2350_E_GDB.md) | [EN-US](en-US/RP2350_AND_GDB_INTEGRATION_PLAN.md) | Planejamento dual-target RP2040/RP2350 no Pico SDK e integração GDB |
| 26 | [REFERENCIA_CAPTURA_IR_PANASONIC_DVD_TV.md](pt-BR/REFERENCIA_CAPTURA_IR_PANASONIC_DVD_TV.md) | [EN-US](en-US/PANASONIC_DVD_TV_IR_CAPTURE_REFERENCE.md) | Decodificação Kaseikyo `Panasonic_DVD_TV.ir` (`50` sinais: TV `0x80` + DVD/HDD `0xB0`) |
| 27 | [RELATORIO_AUDITORIA_E_MODULARIZACAO_FINAL.md](pt-BR/RELATORIO_AUDITORIA_E_MODULARIZACAO_FINAL.md) | [EN-US](en-US/FINAL_AUDIT_AND_MODULARIZATION_REPORT.md) | Auditoria pré-refatoração, decomposição modular e pipeline dos 9 Gates |
| 28 | [RELATORIO_RE_FEISTEL_TOC_EXTRHEAD_E_GATES_UART0.md](pt-BR/RELATORIO_RE_FEISTEL_TOC_EXTRHEAD_E_GATES_UART0.md) | [EN-US](en-US/RE_REPORT_FEISTEL_TOC_EXTRHEAD_AND_UART0_GATES.md) | Engenharia reversa da cifra Feistel (`FUN_4400a666`), TOC, `EXTRHEAD` e UART0 |
| 29 | [RELATORIO_SESSAO_ENGENHARIA_REVERSA_EH55_100_FIEL.md](pt-BR/RELATORIO_SESSAO_ENGENHARIA_REVERSA_EH55_100_FIEL.md) | [EN-US](en-US/EH55_REVERSE_ENGINEERING_SESSION_FAITHFUL_REPORT.md) | Registro 100% fiel da sessão de engenharia reversa ao vivo no EH55 |
| 30 | [RELATORIO_TECNICO_CLAUDE_OPUS_UNBRICK_DMR_EH55.md](pt-BR/RELATORIO_TECNICO_CLAUDE_OPUS_UNBRICK_DMR_EH55.md) | [EN-US](en-US/TECHNICAL_REPORT_FOR_CLAUDE_OPUS_UNBRICK_DMR_EH55.md) | Dossiê técnico de validação cruzada, triagem elétrica/laser e unbrick |
| 31 | [SUBSISTEMA_LOGGING_E_TELEMETRIA.md](pt-BR/SUBSISTEMA_LOGGING_E_TELEMETRIA.md) | [EN-US](en-US/LOGGING_AND_TELEMETRY_SUBSYSTEM.md) | Subsistema `panadiga_core::logger` (`LogLevel`, `set_sink`, Tauri e macros) |
| 32 | [TEORIA_E_PRATICA_UNBRICK_U99_E_CRIPTOGRAFIA_MATSUSHITA.md](pt-BR/TEORIA_E_PRATICA_UNBRICK_U99_E_CRIPTOGRAFIA_MATSUSHITA.md) | [EN-US](en-US/THEORY_AND_PRACTICE_U99_UNBRICK_MATSUSHITA_CRYPTO.md) | Física NOR `S29JL064H`, cura circular `EEMUL` (`U99`), handshakes e hex maps |
| 33 | [UPGRADES_PICO_C11_UNDEF_GHIDRA_E_PARIDADE_PYTHON_RUST.md](pt-BR/UPGRADES_PICO_C11_UNDEF_GHIDRA_E_PARIDADE_PYTHON_RUST.md) | [EN-US](en-US/PICO_C11_UPGRADES_GHIDRA_UNDEF_AND_PYTHON_RUST_PARITY.md) | Upgrades C11 no Pico, diagnóstico `??` no Ghidra e matriz de paridade Rust |
| 34 | [VISTA_EXPLODIDA_ARQUITETURA_INJECAO_PICO_E_BYPASS_PASSTHROUGH.md](pt-BR/VISTA_EXPLODIDA_ARQUITETURA_INJECAO_PICO_E_BYPASS_PASSTHROUGH.md) | [EN-US](en-US/EXPLODED_VIEW_PICO_INJECTION_ARCHITECTURE_AND_BYPASS_PASSTHROUGH.md) | Vista explodida (`DMR-EH55`), `MCP(18)` tools, `PANARSP` e bypass AV `E-E` |
| 35 | [ZERO_ERROR_RAG_ENGINE_E_SEGURANCA_FISICA.md](pt-BR/ZERO_ERROR_RAG_ENGINE_E_SEGURANCA_FISICA.md) | [EN-US](en-US/ZERO_ERROR_RAG_ENGINE_AND_PHYSICAL_SAFETY.md) | Motor RAG Zero-Error (6 camadas), `FactCritic` e guardrails de tensão |

---

## Root Audits, Ghidra Pseudocode & Live Debug Logs / Auditorias, Pseudocódigo & Logs

### Root Audit Reports & Project Guides
* [**`COMPLETE_AUDIT.md`**](../COMPLETE_AUDIT.md) — Current Consolidated Technical Audit (**English**)
* [**`AUDITORIA_COMPLETA.md`**](../AUDITORIA_COMPLETA.md) — Auditoria Técnica Consolidada Atual (**Português BR**)
* [**`README.md`**](../README.md) / [**`README.pt-BR.md`**](../README.pt-BR.md) — Main Workspace Architecture, Build & Quality Documentation
* [**`AGENTS.md`**](../AGENTS.md) — Engineering Constraints, 9 Quality Gates & Agent Constitution

### Preserved Historical Audits (`docs/audits/`)
* [**`audits/README.md`**](./audits/README.md) — Bilingual Audit History Index
* [`audits/AUDITORIA_COMPLETA_2026-11-11.md`](./audits/AUDITORIA_COMPLETA_2026-11-11.md) — Pre-refactor workspace snapshot (`2026-11-11`)
* [`audits/AUDITORIA_FIRMWARE_PICO_E_PIPELINE_2026-11-11.md`](./audits/AUDITORIA_FIRMWARE_PICO_E_PIPELINE_2026-11-11.md) — Pico C11/UF2 & build pipeline snapshot
* [`audits/AUDITORIA_RUNTIME_PANADIGA_2026-11-11.md`](./audits/AUDITORIA_RUNTIME_PANADIGA_2026-11-11.md) — Serial runtime, Tauri parser & electrical safety snapshot

### Ghidra MN103S Pseudocode, Undefined `??` Audit & Raw Captures (`docs/pseudocode/`)
* [**`pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md`**](./pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) — Bilingual line-indexed guide (`166` blocks/functions)
* [**`pseudocode/EVIDENCIA_PSEUDOCODIGO_GHIDRA_MN103S.md`**](./pseudocode/EVIDENCIA_PSEUDOCODIGO_GHIDRA_MN103S.md) — Verbatim 13,677-line decompilation & disassembly dossier (`488,538 B`)
* [**`pseudocode/GHIDRA_UNDEF_AUDIT_REPORT.md`**](./pseudocode/GHIDRA_UNDEF_AUDIT_REPORT.md) — Exhaustive `??` undefined-byte quantitative root-cause report
* Raw text captures (`docs/pseudocode/raw/`): [`doc_boot_core.txt`](./pseudocode/raw/doc_boot_core.txt) · [`doc_boot_media_crypto.txt`](./pseudocode/raw/doc_boot_media_crypto.txt) · [`doc_prog061.txt`](./pseudocode/raw/doc_prog061.txt) · [`doc_main.txt`](./pseudocode/raw/doc_main.txt) · [`doc_drv_and_atapi.txt`](./pseudocode/raw/doc_drv_and_atapi.txt)

### Live `COM10` Bench UART Captures & Dossier (`docs/debug_logs/`)
* [**`debug_logs/README.md`**](./debug_logs/README.md) — Bilingual catalog of `12` unique `SHA-256`-verified captures (`panadiga pico decode-log`)
* [`debug_logs/DOSSIE_COMPLETO_EH55_UNBRICK_E_FIRMWARE.txt`](./debug_logs/DOSSIE_COMPLETO_EH55_UNBRICK_E_FIRMWARE.txt) — Consolidated operational text dossier (`36,628 B`)
* Unique `.log` captures (`docs/debug_logs/`):
  [`captura_live_20261112_143843.log`](./debug_logs/captura_live_20261112_143843.log) · [`captura_live_20261112_144101.log`](./debug_logs/captura_live_20261112_144101.log) · [`captura_live_20261112_151352.log`](./debug_logs/captura_live_20261112_151352.log) · [`captura_samsung_2gb_20261115_020351.log`](./debug_logs/captura_samsung_2gb_20261115_020351.log) · [`captura_live_20261115_021346.log`](./debug_logs/captura_live_20261115_021346.log) · [`captura_flat_odd_20261115_021446.log`](./debug_logs/captura_flat_odd_20261115_021446.log) · [`captura_flat_odd_85s_20261115_021614.log`](./debug_logs/captura_flat_odd_85s_20261115_021614.log) · [`captura_sdhc32gb_cd_20261115_022258.log`](./debug_logs/captura_sdhc32gb_cd_20261115_022258.log) · [`captura_sdhc32gb_cd_75s_20261115_022548.log`](./debug_logs/captura_sdhc32gb_cd_75s_20261115_022548.log) · [`captura_retest_cabo_20261115_023052.log`](./debug_logs/captura_retest_cabo_20261115_023052.log) · [`captura_sd_retest_20261115_023237.log`](./debug_logs/captura_sd_retest_20261115_023237.log) · [`captura_sd_flat_fix_20261115_025903.log`](./debug_logs/captura_sd_flat_fix_20261115_025903.log)
