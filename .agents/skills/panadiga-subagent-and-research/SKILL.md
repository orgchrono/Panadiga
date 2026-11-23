---
name: panadiga-subagent-and-research
description: >-
  Gestão de subagentes (research, self e as 6 personas panadiga-*), protocolo de pesquisa
  técnica em 5 fontes com refutação empírica e consolidação estruturada de conhecimento para
  SSOT, ledger, dossiês e skills. Use para investigações amplas, paralelização de levantamentos
  ou síntese de descobertas de bancada e engenharia reversa.
---

# Gestão de Subagentes, Pesquisa Técnica & Consolidação de Conhecimento

Esta skill define quando e como delegar trabalho a subagentes, como conduzir investigações técnicas à prova de viés e como transformar achados brutos em conhecimento durável.

## 1. Quando Usar & Handoffs
- Use quando uma investigação tocar múltiplos subsistemas, exigir leitura de manuais/logs/binários extensos ou demandar especialistas de domínio.
- Para desenhar prompts de direcionamento ou auditoria red-team, combine com `panadiga-prompt-and-adversarial-audit`.
- Para registrar conclusões e provas SHA-256 em disco, acione `panadiga-forensic-archival` e `panadiga-device-onboarding`.
- Templates de brief e matriz completa de personas em [delegation-and-research-playbooks.md](./references/delegation-and-research-playbooks.md).

## 2. Invariantes de Delegação & Operação Reativa
1. **Delegar por Contexto, Não por Preguiça:** Lookups simples (1–3 arquivos ou `rg` pontual) devem ser feitos diretamente; delegue surveys multi-diretório, auditorias cruzadas ou geração de inventários densos.
2. **Brief Fechado & Verificável:** Todo `invoke_subagent` deve especificar escopo exato, arquivos/diretórios de interesse, restrição read-only vs escrita, formato de entrega e instrução para marcar incertezas como `UNCERTAIN` e citar caminhos reais.
3. **Zero Polling:** Após disparar subagentes em paralelo, execute trabalho local independente ou encerre o turno aguardando notificação reativa.
4. **Recuperação de Erro / Cota (`429 RESOURCE_EXHAUSTED`):** Liste com `manage_subagents` (`list`), mate apenas o subagente com erro (`kill`) e relance com escopo fatiado ou menor volume simultâneo.
5. **Preservar Continuidade:** Se um subagente já carregou um domínio específico e precisa de aprofundamento no mesmo assunto, envie `send_message` em vez de recriar do zero.
6. **Escolha da Primitiva Multi-Agente:** Use `invoke_subagent`/`define_subagent` para 1–3 frentes delegadas, `run_workflow` (Python DAG com `agent`/`parallel`/`pipeline`/`phase`) para fan-out em muitos arquivos/módulos, e `swarm` (`share`/`branch` + `action:"model"`) para colaboração autônoma em quadro compartilhado — detalhes completos em [Guia Plan Mode & Runtime](../panadiga-skill-orchestrator/references/agent-runtime-and-plan-mode.md).

## 3. Protocolo de Pesquisa em 5 Fontes & Princípio da Refutação Empírica
1. **Fonte 1 — Código-Fonte & SSOT (`crates/`, `profiles/`, `firmware/`, `apps/`):** Verdade operacional de implementação, flags e invariantes compiladas.
2. **Fonte 2 — Manuais & Datasheets PDF (`manuals_and_schematics/`):** `panadiga manuals` ou `panadiga_search_manuals` para pinagem, rails de tensão e sequências de fábrica (`DSD0603003CE`, `CHM0605028CE`, `S29JL064H`).
3. **Fonte 3 — Evidência Física (`docs/debug_logs/`, `docs/evidence/ledger.json`):** Medições DC, capturas `COM10`, probes `DAVIS` e dumps SHA-256.
4. **Fonte 4 — Disassembly & Pseudocódigo (`docs/pseudocode/`, Ghidra):** Fluxo real no silício após corrigir Armadilhas MN103S (`#1..#10`).
5. **Fonte 5 — Dossiês & Compêndio Comunitário (`docs/pt-BR/`, `docs/en-US/`):** Referência histórica e cruzamento OEM — sempre subordinada às Fontes 1–4 em caso de conflito.

> **Lição Canônica (Refutação `DWire32A`):** Nunca extrapole pinagem ou protocolo por analogia entre gerações (`CN55001` vs `P7501`). Uma hipótese só se sustenta quando sobrevive à busca exaustiva nos manuais oficiais e à medição elétrica direta em bancada.

## 4. Pipeline de Consolidação de Conhecimento
1. **Triagem Epistêmica:** Classifique toda descoberta exatamente entre `N1_STATIC`, `N2_EMULATED`, `N3_LIVE_PROBE`, `N4_HARDWARE` e `N5_REPRODUCED`.
2. **Atualização Estrutural Primeiro:** Converta achados estabilizados em dados SSOT (`profiles/`), invariantes testáveis em Rust/C11/Python e registros em `docs/evidence/ledger.json`.
3. **Atualização Documental Bilíngue:** Propague para `docs/pt-BR/` + `docs/en-US/` (mantendo 1:1), `docs/debug_logs/README.md`, `docs/STATUS.md` e skills afetadas — eliminando afirmações antigas refutadas.

## 5. Armadilhas & Anti-Padrões
- Citar memória ou documentação antiga sem conferir no código atual (ex.: supor 35 setores BGO no EH55 quando a tabela real `0x440337A4` tem 45 setores `SA26..SA70`).
- Misturar conclusões de firmware `/BOOT.BIN` (`Loader ver0.63`) com o bootloader real do EH55 (`Loader ver0.61`).
- Lançar múltiplos subagentes escrevendo no mesmo arquivo sem particionamento claro.

## 6. Verificação
- Todo relatório de subagente usado em decisão técnica deve ser cruzado contra pelo menos 1 leitura direta de código/evidência no agente principal antes de virar código ou documentação.
