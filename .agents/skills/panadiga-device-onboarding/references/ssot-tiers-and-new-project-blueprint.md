# SSOT `profiles/`, Tiers `T0..T4` & Blueprint Portátil para Novos Projetos

## 1. Arquitetura Relacional SSOT (`profiles/` & `crates/panadiga-profiles/`)
- **12 JSON Schemas (Draft 2020-12 em `profiles/schema/`):** `board`, `capability_authorization`, `eeprom_chip`, `firmware_train`, `flash_chip`, `media_format`, `model_manifest`, `nor_layout`, `profiles_bundle`, `resolved_model_profile`, `soc`, `soc_core`.
- **9 Coleções (`89` modelos, `796` checagens relacionais):** `soc_cores` (`5`), `socs` (`12`), `flash_chips` (`18`), `eeprom_chips` (`6`), `firmware_trains` (`18`), `nor_layouts` (`3`), `media_formats`, `boards` (`31`), `models` (`89`) + `fixtures` + `blobs` (`loader_ver061_nfrom_drv.bin.gz`).
- **Tiers de Evidência (`EvidenceTier`):** `T0 community_unverified` -> `T1 service_manual_only` -> `T2 official_frm_verified` -> `T3 physical_dump_verified` -> `T4 bench_verified` (`DMR-EH55` exclusivo).
- **8 Capacidades (`CapabilityGate`):** `C0 InspectReadOnly` (sempre), `C1 NvramSanitizeOfflineCow`, `C2 NvramWriteLive` (`board>=T4`, `flash>=T3`, `layout>=T3`), `C3a UpperNorPatchOfflineCow`, `C3b UpperNorWriteLive` (`>=T4`), `C4a OfficialFrmPassThrough` (`>=T2` + SHA-256), `C4b CustomRecoveryMedia` (`>=T4` ou `LabOverride`), `C5 CriticalIplOrLethalMmio` (**SEMPRE NEGADA**).
- **`LabOverride` (CoW Offline):** Exige `--confirm-disclaimer I_ACCEPT_UNVERIFIED_HARDWARE_RISK` e `--override-reason` (>=5 chars); nunca libera hardware live.

---

## 2. Blueprint Portátil: Como Iniciar um Novo Projeto de Hardware/RE no Dia 0
Lições destiladas de toda a jornada Panadiga para qualquer novo dispositivo ou plataforma embarcada:

1. **Refute Hipóteses na Bancada & Esquemáticos Antes de Codar:** Meça rails reais, confirme o pino/protocolo em 2+ manuais oficiais e evite assumir compatibilidade entre revisões de PCB (`P7501` EH55 vs EH58).
2. **Isolamento Galvânico & Proteção de Silício Primeiro:** Use transformador isolador AC ou isolador USB `ADuM`, nunca puxe alimentação da porta de serviço e trave pinos bidirecionais/sensores desconhecidos em `Hi-Z`.
3. **Triagem Forense Contra Falsos Dumps:** Na chegada do primeiro dump, rode SHA-256, entropia de Shannon por setor, teste de byte-swap (`16-bit`) e espelho de pinos flutuantes antes de gastar horas em Ghidra.
4. **Identificação por Tupla Física & SSOT Declarativa:** Nunca espalhe nomes comerciais no código; modele `soc -> board -> flash -> layout -> train -> model` em JSON validado por Schema com geradores de código (`ssot export-all`).
5. **Política Fail-Closed em Camadas + `CapabilityGate`:** Separe explicitamente Leitura (`C0`) / CoW Offline (`C1`, `C3a`) / Escrita Live (`C2`, `C3b`) / Proibido Permanente (`C5`).
6. **Cofre Imutável em 3 Buckets (`Vault`):** Isole brutos imutáveis (`raw/`), pacotes (`frm/`) e artefatos modificados (`recompiled/`) barrando saída sem portão de emulação.
7. **Evidence Ledger Monotônico (`N1..N5`):** Nunca promova achado estático (`N1`) ou de emulador (`N2`) como verdade de silício real (`N4`) nem reprodução geral (`N5`).
8. **Quarentena Humana para Extração de PDFs/Docs:** Parsers automáticos de manuais geram propostas com página citada (`profiles/staging/`); revisão humana aprova antes do merge na SSOT.
9. **RE Focado em Código Alcançável + SLEIGH Versionado:** Em disassemblers, audite seções executáveis alcançáveis (não force código sobre dados) e mantenha correções de ISA em patches replicáveis sobre clones do projeto.
10. **Quality Gates + Acoplamento Automatizado Desde o Commit 1:** Exija formatação, lint arquitetural, detector de duplicidade `0%`, paridade documental e checagem `.gitignore` automatizada via runner único (`validate.py`).
