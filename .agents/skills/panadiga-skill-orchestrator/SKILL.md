---
name: panadiga-skill-orchestrator
description: >-
  Orquestrador central (bootstrap obrigatório) do ecossistema de 35 skills Panadiga,
  roteamento com caminhos explícitos de ativação, gestão de metas em 5 fases, cadeias
  multi-skill, testes/subagentes/auditoria anti-alucinação e disciplina de economia de tokens
  no uso de ferramentas sem reduzir a profundidade do raciocínio técnico. Leia sempre no
  início de tarefas não-triviais.
---

# Orquestrador Central de Skills, Metas & Disciplina de Ferramentas (`35 Skills`)

Este orquestrador é o ponto de entrada canônico para subir as skills certas no momento exato com caminhos explícitos, gerir metas ponta a ponta, auditar o próprio raciocínio e manter o uso cirúrgico de tools.

## 1. Protocolo de Gestão de Meta & Carregamento Just-in-Time (5 Fases)
1. **Fase 0 — Meta, Escopo & Barreiras:**
   - Defina objetivo mensurável, arquivos-alvo e critérios de aceite; registre o plano em `update_steps`.
   - Antes de qualquer operação em hardware, Flash/HDD, WebView ou código sensível, suba `.agents/skills/panadiga-security-and-violation-guards/SKILL.md`.
2. **Fase 1 — Pesquisa, RAG/MCP & Subagentes:**
   - Use `.agents/skills/panadiga-rag-and-mcp-engineering/SKILL.md` para consultar primeiro `panadiga_rag_query`, `panadiga_search_code`, `panadiga_search_manuals` e `panadiga://invariants` (ou `.agents/skills/panadiga-tool-arsenal-catalog/SKILL.md` para descobrir ferramentas existentes).
   - Para levantamentos amplos ou paralelos, suba `.agents/skills/panadiga-subagent-and-research/SKILL.md` (brief fechado, zero polling).
3. **Fase 2 — Execução Especializada (Carregamento Sob Demanda):**
   - Abra apenas os `SKILL.md` da tabela abaixo correspondentes à etapa atual; abra os `references/*.md` listados somente quando precisar de tabelas de bytes, chaves, endereços ou templates.
4. **Fase 3 — Testes, Compliance & Auditoria Adversarial/Anti-Alucinação:**
   - Para novas features ou erradicação de flakiness, suba `.agents/skills/panadiga-testing-and-feature-delivery/SKILL.md`; para bancada/headless, `.agents/skills/panadiga-bench-automation-and-headless/SKILL.md`.
   - Para revisar tudo o que o agente pensou/afirmou/fez (anti-alucinação + red-team contínuo), suba `.agents/skills/panadiga-adversarial-and-hallucination-auditor/SKILL.md` (e `.agents/skills/panadiga-prompt-and-adversarial-audit/SKILL.md` para subagentes auditores limpos).
   - Valide código/SSOT/CI via `.agents/skills/panadiga-code-compliance-and-layer-coherence/SKILL.md`, `.agents/skills/panadiga-repo-sanitization-and-commit-sync/SKILL.md` e `.agents/skills/panadiga-validate/SKILL.md`.
5. **Fase 4 — Consolidação, Documentação & Fechamento:**
   - Registre dumps, hashes SHA-256, `ledger.json`, timeline e documentação técnica bilíngue via `.agents/skills/panadiga-forensic-archival/SKILL.md` + `.agents/skills/panadiga-technical-documentation/SKILL.md` antes de marcar a meta como `done`.

## 2. Disciplina de Tokens em Tools (NUNCA no Raciocínio)
- **RAG/MCP antes de leitura bruta:** Consulte `panadiga_rag_query` / `panadiga_search_code` antes de abrir múltiplos arquivos.
- **Saída JSON pura:** Use `cargo run -q -p panadiga-cli -- <cmd> --json -` e parsers específicos (`pico decode-log`, `pico parse-diag`, `panadiga_read_hex`).
- **Leitura cirúrgica:** Localize com `rg -n` / `Select-String` e leia apenas o bloco (`StartLine`/`EndLine`). Jamais despeje dumps ou `docs/pseudocode/raw/` no contexto.
- **Zero polling:** Dispare chamadas/subagentes independentes em paralelo e aguarde notificação reativa.
- **Raciocínio irrestrito:** Economia de tokens vale apenas para I/O e seleção de contexto; nunca pule checagem de invariantes físicas, FMEA, auditoria anti-alucinação ou verificação pós-edição.

## 3. Tabela de Acionamento com Caminhos Explícitos (`35 Skills`)

| Pilar | Assunto / Quando Subir | Caminho Explícito da Skill & Referências |
| :--- | :--- | :--- |
| **P0 Meta** | Orquestrar skills, metas, Plan Mode, runtime, economia de tokens | `.agents/skills/panadiga-skill-orchestrator/SKILL.md` · [Matriz](./references/routing-matrix-and-token-discipline.md) · [Plan Mode & Runtime](./references/agent-runtime-and-plan-mode.md) |
| **P0 Meta** | Subagentes, pesquisa 5 fontes, consolidação de conhecimento | `.agents/skills/panadiga-subagent-and-research/SKILL.md` · `.agents/skills/panadiga-subagent-and-research/references/delegation-and-research-playbooks.md` |
| **P0 Meta** | Auditoria contínua anti-alucinação, qualidade e red-team do agente | `.agents/skills/panadiga-adversarial-and-hallucination-auditor/SKILL.md` · `.agents/skills/panadiga-adversarial-and-hallucination-auditor/references/anti-hallucination-and-redteam-protocol.md` |
| **P0 Meta** | Templates de prompts de direcionamento e auditoria adversarial | `.agents/skills/panadiga-prompt-and-adversarial-audit/SKILL.md` · `.agents/skills/panadiga-prompt-and-adversarial-audit/references/adversarial-prompt-templates.md` |
| **P0 Meta** | Usar ou estender RAG 6 camadas e 26 tools MCP Rust | `.agents/skills/panadiga-rag-and-mcp-engineering/SKILL.md` · `.agents/skills/panadiga-rag-and-mcp-engineering/references/rag-mcp-catalog-and-extension.md` |
| **P0 Meta** | Criar, refatorar ou auditar skills em `.agents/skills/` | `.agents/skills/panadiga-skill-authoring/SKILL.md` · `.agents/skills/panadiga-skill-authoring/references/skill-template-and-lint-checklist.md` |
| **P0 Meta** | Catálogo auto-atualizável do arsenal (crates, CLI, MCP, IPC, scripts, skills) | `.agents/skills/panadiga-tool-arsenal-catalog/SKILL.md` · `.agents/skills/panadiga-tool-arsenal-catalog/references/ARSENAL_INDEX.md` |
| **P1 Forense** | Triagem NOR/NAND/EEPROM/SD, byte-swap, espelhamento, `EEMUL`, `ingest` | `.agents/skills/panadiga-hardware-forensics/SKILL.md` · `.agents/skills/panadiga-hardware-forensics/references/dump-integrity-triage.md` |
| **P1 Storage** | HDD `MEIHFS`, partições, `patch-capacity`, `flash-eh55`, `P58001` | `.agents/skills/panadiga-extract-partitions/SKILL.md` |
| **P1 Decrypto** | `.FRM`, Feistel, AES, `keys.ukf`, `EXTRHEAD`, `MainList`, LZSS, `fmaX` | `.agents/skills/panadiga-firmware-decrypto/SKILL.md` · `.agents/skills/panadiga-firmware-decrypto/references/crypto-primitives-and-keys.md` · `.agents/skills/panadiga-firmware-decrypto/references/container-formats-and-compression.md` |
| **P2 RE** | Ghidra multi-ISA, 10 Armadilhas MN103S, SLEIGH patch, pseudocódigo | `.agents/skills/panadiga-ghidra-workflow/SKILL.md` · `.agents/skills/panadiga-ghidra-workflow/references/mn103s-traps-and-sleigh.md` |
| **P2 Atlas** | Atlas forense 100% Ghidra (funções, callers, hex/MMIO, headers, máscaras bitwise, `side_effects`) | `.agents/skills/panadiga-firmware-forensic-atlas/SKILL.md` · `.agents/skills/panadiga-firmware-forensic-atlas/references/FORENSIC_ATLAS_INDEX.md` |
| **P2 RE/Emu** | Patches CoW, `Mn103Vm`, `Matshita R6-MV4`, GDB RSP, `PANAMON2` | `.agents/skills/panadiga-emulation-and-patching/SKILL.md` · `.agents/skills/panadiga-emulation-and-patching/references/emulation-limits-and-silicon-abi.md` |
| **P2 FW Livre** | Firmware open-source bare-metal `AM34/MN103S` (`M0->M3`, HAL, MMIO) | `.agents/skills/panadiga-opensource-firmware-dev/SKILL.md` · `.agents/skills/panadiga-opensource-firmware-dev/references/am34-baremetal-bringup-and-roadmap.md` |
| **P3 Bancada** | SMPS/ADuM, `VEP79138`, Standby `IC7501`, `SERV`/IR, `PANAMON1>`, `DAVIS` | `.agents/skills/panadiga-bench-operations/SKILL.md` · `.agents/skills/panadiga-bench-operations/references/bench-electrical-and-standby.md` · `.agents/skills/panadiga-bench-operations/references/panamon1-davis-and-frontiers.md` |
| **P3 Auto/HL** | Testes automatizados em bancada, execução headless, 6 caminhos E2E | `.agents/skills/panadiga-bench-automation-and-headless/SKILL.md` · `.agents/skills/panadiga-bench-automation-and-headless/references/headless-pipelines-and-decision-trees.md` |
| **P3 Unbrick** | Cura `U99`, NVRAM `SA134..SA141`, BGO 45 setores, kit SD `ver061` | `.agents/skills/panadiga-nvram-unbrick/SKILL.md` |
| **P3 Pico C11** | Firmware RP2040/RP2350, PIOs, UF2, `pico flash` e portas seriais | `.agents/skills/panadiga-pico-flashing/SKILL.md` · `.agents/skills/panadiga-pico-flashing/references/pico-c11-module-architecture.md` |
| **P3 Heartbeat** | Heartbeat `P7501` (autodescoberta USB VID `2E8A`) e disparador guiado read-only | `.agents/skills/panadiga-pico-heartbeat-and-guided-ops/SKILL.md` · `.agents/skills/panadiga-pico-heartbeat-and-guided-ops/references/heartbeat-states-and-guided-macros.md` |
| **P4 Guardas** | Arquitetura FCIS/MVVM, SSOT/DRY/SRP/SoC, anti-god-module, criação de módulos | `.agents/skills/panadiga-architecture-and-guardrails/SKILL.md` · `.agents/skills/panadiga-architecture-and-guardrails/references/architecture-fcis-mvvm-and-anti-patterns.md` |
| **P4 Segur.** | Guarda fail-closed contra violações elétricas, Flash/HDD, epistêmicas, AppSec | `.agents/skills/panadiga-security-and-violation-guards/SKILL.md` · `.agents/skills/panadiga-security-and-violation-guards/references/violation-taxonomy-and-controls.md` |
| **P4 Anti-Trap** | Verificação anti-armadilhas de geometria Flash/BGO (`4 MiB` vs `8 MiB`, `35` vs `45` setores, `DIGADIAG` `256K`/`128K`) | `.agents/skills/panadiga-anti-trap-geometry-verification/SKILL.md` · `.agents/skills/panadiga-anti-trap-geometry-verification/references/bgo-4mb-vs-8mb-and-geometry-traps.md` |
| **P4 Coerência** | Compliance cross-layer, eliminação de silos SSOT e codegen unificado | `.agents/skills/panadiga-code-compliance-and-layer-coherence/SKILL.md` · `.agents/skills/panadiga-code-compliance-and-layer-coherence/references/cross-layer-contracts-and-ssot-coherence.md` |
| **P4 Contratos** | Plano A (`SSOT_CONTRACT_HASH`), Plano B (`panadiga-bench`/`panadiga-vault`), Zero-Trust IPC, `profiles/symbols/` | `.agents/skills/panadiga-contracts-stream-and-journal/SKILL.md` · `.agents/skills/panadiga-contracts-stream-and-journal/references/plano-a-plano-b-and-zero-trust-ipc.md` |
| **P4 Rust** | Desenvolvimento nos 11 crates Rust Edition 2024, `tracing`, `#![deny(unsafe_code)]` | `.agents/skills/panadiga-rust-backend-dev/SKILL.md` · `.agents/skills/panadiga-rust-backend-dev/references/rust-workspace-patterns.md` |
| **P4 TS/UI** | Tauri 2 + React 18.3 + TS 7, JSDoc, Oxlint, i18n, Vitest, Playwright | `.agents/skills/panadiga-typescript-frontend-dev/SKILL.md` · `.agents/skills/panadiga-typescript-frontend-dev/references/ts-mvvm-jsdoc-and-testing.md` |
| **P4 UX/WebView** | UX/UI, layout responsivo `h-dvh`, a11y, `ux-guard` & segurança WebView Tauri/Android | `.agents/skills/panadiga-ux-ui-and-layout/SKILL.md` · `.agents/skills/panadiga-ux-ui-and-layout/references/ux-layout-a11y-and-webview-security.md` |
| **P5 Arquivo** | Vault `panadiga session`, `ledger.json`, timeline, logs, cadeia de custódia | `.agents/skills/panadiga-forensic-archival/SKILL.md` · `.agents/skills/panadiga-forensic-archival/references/archival-docs-timeline-and-ledger.md` |
| **P5 Docs** | Documentação técnica Diátaxis, paridade bilíngue, Mermaid, anti-drift | `.agents/skills/panadiga-technical-documentation/SKILL.md` · `.agents/skills/panadiga-technical-documentation/references/diataxis-bilingual-and-doc-standards.md` |
| **P5 SSOT** | Construir e evoluir mapas `profiles/` (`nor_layouts`, `traps`), `validate_bgo.rs` e `ssot export-all` | `.agents/skills/panadiga-ssot-builder-and-validation/SKILL.md` · `.agents/skills/panadiga-ssot-builder-and-validation/references/ssot-schema-validation-and-evolution.md` |
| **P5 Onboard** | Novo modelo `profiles/` (`T0..T4`), quarentena PDF, blueprint novos projetos | `.agents/skills/panadiga-device-onboarding/SKILL.md` · `.agents/skills/panadiga-device-onboarding/references/ssot-tiers-and-new-project-blueprint.md` |
| **P5 Testes** | Gestão de novas implementações, criação de testes e anti-flakiness | `.agents/skills/panadiga-testing-and-feature-delivery/SKILL.md` · `.agents/skills/panadiga-testing-and-feature-delivery/references/testing-patterns-and-anti-flakiness.md` |
| **P5 CI/CD** | Git, `.gitignore`, GitHub Actions (`ci.yml`/`release.yml`) e Releases seguras | `.agents/skills/panadiga-git-cicd-and-releases/SKILL.md` · `.agents/skills/panadiga-git-cicd-and-releases/references/workflows-and-release-hardening.md` |
| **P5 Sync/Commit** | Sanitização programática, sync SSOT/Ghidra/Arsenal/Atlas e plano read-only de commits em 6 ondas | `.agents/skills/panadiga-repo-sanitization-and-commit-sync/SKILL.md` · `.agents/skills/panadiga-repo-sanitization-and-commit-sync/references/commit-wave-order-and-hygiene-rules.md` |
| **P5 Quality** | 9+1 Quality Gates, pirâmide universal de testes e acoplamento | `.agents/skills/panadiga-validate/SKILL.md` · `.agents/skills/panadiga-validate/references/universal-testing-and-gates-matrix.md` |

## 4. Plan Mode & Recursos Nativos Antigravity
- **Plan Mode (`enter_plan_mode` / `exit_plan_mode`):** Planeje em read-only antes de mudanças multi-arquivo/arriscadas; na execução, **devolva via `enter_plan_mode`** na 2ª falha seguida, premissa incorreta, escolha não prevista ou `[plan review due]` (nunca improvise uma 3ª tentativa).
- **Progresso & Espectro Multi-Agente:** Use `update_steps` + `update_focus` em paralelo; escolha `invoke_subagent` (1–3 frentes), `run_workflow` (fan-out programático) ou `swarm` (time autônomo com `workspace:"share"|"branch"`).
- **Assíncrono, `scratch/` & Slash Commands:** Use `schedule` (jamais `sleep` em shell), salve scripts descartáveis em `scratch/` (nunca suje o Git) e sugira `/plan`, `/grill-me`, `/boost`, `/teamwork`, `/goal`, `/schedule`, `/learn`, `/compact` ou `/browser` quando útil — guia completo em [agent-runtime-and-plan-mode.md](./references/agent-runtime-and-plan-mode.md).

## 5. Verificação & Portal
- Portal visual e fluxograma de decisão: [../README.md](../README.md).
- Antes de encerrar qualquer tarefa com mudanças de código, docs ou SSOT, passe por `.agents/skills/panadiga-adversarial-and-hallucination-auditor/SKILL.md` e valide via `.agents/skills/panadiga-validate/SKILL.md`.
