# Matriz Completa de Roteamento, Cadeias Multi-Skill & Economia de Tokens (`29 Skills`)

## 1. Cadeias Multi-Skill Canônicas (End-to-End)

### Cadeia A — Intake de Dump Novo até SSOT `T3` & Ghidra
1. `.agents/skills/panadiga-security-and-violation-guards/SKILL.md` (garantir read-only e isolamento)
2. `.agents/skills/panadiga-forensic-archival/SKILL.md` (`panadiga session init` + `session ingest --bucket raw`)
3. `.agents/skills/panadiga-hardware-forensics/SKILL.md` (`nor_intake_analyzer.py`, `profiles discover-nor`, `nvram audit-nor-sectors`)
4. `.agents/skills/panadiga-anti-trap-geometry-verification/SKILL.md` (`verify_geometry_traps.py --check`, checar `4 MiB` vs `8 MiB`, `35` vs `45` setores, `DIGADIAG` `256K` vs `128K`)
5. `.agents/skills/panadiga-firmware-decrypto/SKILL.md` (`frm unpack-all`, `fleet_decryptor.py`, `frm_train_harvester.py`)
6. `.agents/skills/panadiga-ghidra-workflow/SKILL.md` + `.agents/skills/panadiga-firmware-forensic-atlas/SKILL.md` (`ghidra_fleet_orchestrator.py` + `generate_firmware_atlas.py --write`)
7. `.agents/skills/panadiga-device-onboarding/SKILL.md` (`manual_spec_extractor.py`, `profiles promote`, `profiles validate`, `ssot export-all`)
8. `.agents/skills/panadiga-validate/SKILL.md` (`python scripts/validate.py`)

### Cadeia B — Diagnóstico em Bancada ao Vivo & Cura `U99`
1. `.agents/skills/panadiga-security-and-violation-guards/SKILL.md` + `.agents/skills/panadiga-anti-trap-geometry-verification/SKILL.md` (SMPS/ADuM, `NSW5V` Hi-Z, `XRESET` Open-Drain, geometria BGO `4 MiB`/`8 MiB` e pinout `P7501`)
2. `.agents/skills/panadiga-bench-automation-and-headless/SKILL.md` + `.agents/skills/panadiga-bench-operations/SKILL.md` (heartbeat `VID 2E8A`, Standby `IC7501`, `SERV`, `PANAMON1>`, `DAVIS`, `P58001`, `scripts.bench.runner`, `pico bench-wizard`, `pico capture-nor`)
3. `.agents/skills/panadiga-nvram-unbrick/SKILL.md` (`bgo-audit --nor`, `bgo-repair-plan`, `bgo-live-apply --backup-dir ... --confirm-upper-nor`, `prepare-sd ver061`)
4. `.agents/skills/panadiga-extract-partitions/SKILL.md` (`hdd inspect`, `hdd flash-eh55 --backup-dir ... --confirm-physical-drive ...`)
5. `.agents/skills/panadiga-forensic-archival/SKILL.md` (salvar log SHA-256 em `docs/debug_logs/`, atualizar `ledger.json` e timeline)

### Cadeia C — Novo Patch Binário & Validação Antes de Silício
1. `.agents/skills/panadiga-ghidra-workflow/SKILL.md` + `.agents/skills/panadiga-firmware-forensic-atlas/SKILL.md` (auditar site, `side_effects` BGO, callers/MMIO e armadilhas #1..#10)
2. `.agents/skills/panadiga-anti-trap-geometry-verification/SKILL.md` + `.agents/skills/panadiga-emulation-and-patching/SKILL.md` (conferir mapa `0x84200000` vs `0x84130000`, CoW patch + `panadiga re verify-patches` em `Mn103Vm`)
3. `.agents/skills/panadiga-forensic-archival/SKILL.md` (`session ingest --bucket recompiled` exige `PreWriteGateReport.passed == true`)
4. `.agents/skills/panadiga-adversarial-and-hallucination-auditor/SKILL.md` (auditoria red-team 5 lentes com template adversarial antes de gravar no aparelho)
5. `.agents/skills/panadiga-bench-operations/SKILL.md` (teste físico isolado para eventual promoção `N2_EMULATED` -> `N4_HARDWARE`)

### Cadeia D — Evolução de Código, Compliance, Contratos, Testes & CI/CD
1. `.agents/skills/panadiga-architecture-and-guardrails/SKILL.md` + `.agents/skills/panadiga-functional-math-and-patterns/SKILL.md` + `.agents/skills/panadiga-contracts-stream-and-journal/SKILL.md` (onde criar, FCIS/MVVM, FP embarcado `C11`/`Rust` zero heap, matemática determinística/padrões, Plano A/B, Zero-Trust IPC, `SSOT_CONTRACT_HASH`, DRY)
2. Skill de stack: `panadiga-device-onboarding`, `panadiga-rust-backend-dev`, `panadiga-typescript-frontend-dev`, `panadiga-pico-flashing` ou `panadiga-rag-and-mcp-engineering`
3. `.agents/skills/panadiga-security-and-violation-guards/SKILL.md` + `.agents/skills/panadiga-adversarial-and-hallucination-auditor/SKILL.md`
4. `.agents/skills/panadiga-repo-sanitization-and-commit-sync/SKILL.md` + `.agents/skills/panadiga-git-cicd-and-releases/SKILL.md` + `.agents/skills/panadiga-validate/SKILL.md` (`python scripts/validate.py` — 9 Quality Gates verdes sem flakiness)

### Cadeia E — Pesquisa Profunda, Documentação & Consolidação de Conhecimento
1. `.agents/skills/panadiga-rag-and-mcp-engineering/SKILL.md` + `.agents/skills/panadiga-firmware-forensic-atlas/SKILL.md` (RAG + manuais + catálogo visual + Atlas/Símbolos + código)
2. `.agents/skills/panadiga-subagent-and-research/SKILL.md` (subagentes paralelos nas 5 fontes)
3. `.agents/skills/panadiga-adversarial-and-hallucination-auditor/SKILL.md` (tentar refutar achados antes de consolidar)
4. `.agents/skills/panadiga-technical-documentation/SKILL.md` + `.agents/skills/panadiga-forensic-archival/SKILL.md` (Diátaxis PT/EN, `docs/INDEX.md`, `ledger.json`)
5. `.agents/skills/panadiga-skill-authoring/SKILL.md` + `.agents/skills/panadiga-repo-sanitization-and-commit-sync/SKILL.md` (propagar lições duráveis e sincronizar `ARSENAL_INDEX.md`)

### Cadeia F — Desenvolvimento do `Panadiga OpenOS` Bare-Metal (`AM34`, `M0 -> M4`)
1. `.agents/skills/panadiga-opensource-firmware-dev/SKILL.md` + `.agents/skills/panadiga-functional-math-and-patterns/SKILL.md` (Stage-1.5 `0x50400000` -> Stage-2 `0x57000000`, `hal/am34_mmio.h`, FSMs puras em C11 freestanding, ponto fixo/modular sem `float`, ISRs/DMA/WDT, `zlib`/LZSS sem heap, `-DPANADIGA_OPENOS_HOST_TEST=1`, `M0 -> M4`)
2. `.agents/skills/panadiga-ghidra-workflow/SKILL.md` (confirmar mapas MMIO reais do hardware alvo)
3. `.agents/skills/panadiga-emulation-and-patching/SKILL.md` + `.agents/skills/panadiga-nvram-unbrick/SKILL.md` (empacotar `DIGADIAG.FRM` / testar limites `Mn103Vm`)
4. `.agents/skills/panadiga-bench-operations/SKILL.md` + `.agents/skills/panadiga-bench-automation-and-headless/SKILL.md` (`ADuM3160` + `BOOTTRACE` + `DIGAOS>`)
5. `.agents/skills/panadiga-forensic-archival/SKILL.md` (arquivar capturas UART + atualizar `ledger.json`)

---

## 2. Gestão de Metas vs Escolhas de Ferramenta

| Tipo de Ação | Escolha Cara (Evitar) | Escolha Eficiente (Padrão) |
| :--- | :--- | :--- |
| Descobrir onde fica uma função/regra | Ler 6 arquivos inteiros em sequência | `panadiga_search_code` / `panadiga_rag_query` ou `rg -n`, depois `read_file` no intervalo exato |
| Consultar pinagem ou mapa de memória | Abrir PDFs ou dossiês de 40 KB | `panadiga://invariants`, `panadiga://pinout-p7501-uart`, `panadiga_memory_map`, `panadiga_pico_catalog` |
| Analisar captura UART ou dump | Colar 300 KB de log ou hexdump no prompt | `panadiga pico decode-log -f <LOG> --json`, `panadiga_read_hex`, `panadiga headers inspect` |
| Verificar formato de comando CLI | Adivinhar flags por memória | Conferir skill correspondente ou `cargo run -q -p panadiga-cli -- <subcmd> --help` |
| Varredura multi-domínio | Fazer 40 leituras sequenciais na janela principal | Delegar pesquisa fatiada a 1–2 subagentes `research` em paralelo com brief fechado |
| Validar alteração de código | Rodar 10 comandos manuais soltos e esquecer `jscpd` | `python scripts/validate.py --sequential` (ou testes unitários focados durante iteração + `validate.py` no fechamento) |

---

## 3. Personas Especialistas (`.agents/agents/`) × Skills
- `panadiga-architect`: `panadiga-skill-orchestrator`, `panadiga-architecture-and-guardrails`, `panadiga-functional-math-and-patterns`, `panadiga-contracts-stream-and-journal`, `panadiga-security-and-violation-guards`, `panadiga-anti-trap-geometry-verification`, `panadiga-device-onboarding`, `panadiga-rag-and-mcp-engineering`, `panadiga-firmware-forensic-atlas`, `panadiga-repo-sanitization-and-commit-sync`, `panadiga-git-cicd-and-releases`.
- `panadiga-storage-forensics`: `panadiga-extract-partitions`, `panadiga-hardware-forensics`, `panadiga-forensic-archival`, `panadiga-bench-operations`, `panadiga-anti-trap-geometry-verification`.
- `panadiga-firmware-re`: `panadiga-ghidra-workflow`, `panadiga-firmware-forensic-atlas`, `panadiga-firmware-decrypto`, `panadiga-emulation-and-patching`, `panadiga-opensource-firmware-dev`, `panadiga-functional-math-and-patterns`, `panadiga-hardware-forensics`, `panadiga-anti-trap-geometry-verification`.
- `panadiga-hardware-embedded`: `panadiga-pico-flashing`, `panadiga-bench-operations`, `panadiga-bench-automation-and-headless`, `panadiga-nvram-unbrick`, `panadiga-opensource-firmware-dev`, `panadiga-functional-math-and-patterns`, `panadiga-anti-trap-geometry-verification`.
- `panadiga-desktop-ui`: `panadiga-typescript-frontend-dev`, `panadiga-contracts-stream-and-journal`, `panadiga-architecture-and-guardrails`, `panadiga-validate`.
- `panadiga-qa-auditor`: `panadiga-adversarial-and-hallucination-auditor`, `panadiga-validate`, `panadiga-security-and-violation-guards`, `panadiga-anti-trap-geometry-verification`, `panadiga-device-onboarding`, `panadiga-contracts-stream-and-journal`, `panadiga-functional-math-and-patterns`, `panadiga-technical-documentation`, `panadiga-forensic-archival`, `panadiga-repo-sanitization-and-commit-sync`.
