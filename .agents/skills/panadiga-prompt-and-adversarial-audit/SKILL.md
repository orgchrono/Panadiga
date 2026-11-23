---
name: panadiga-prompt-and-adversarial-audit
description: >-
  Engenharia de prompts de direcionamento técnico preciso e criação de prompts de auditoria
  adversarial (red-teaming) para hardware, firmware, patches, arquitetura, testes e evidências.
  Use antes de operações arriscadas em bancada, em revisões críticas de PR, após refatorações
  estruturais ou para expor premissas falsas e teatro de testes.
---

# Engenharia de Prompts & Auditoria Adversarial (Red-Team)

Esta skill padroniza tanto o direcionamento claro de agentes quanto a geração de prompts adversariais que buscam ativamente refutar suposições antes que virem dano em silício ou dívida técnica.

## 1. Quando Usar & Handoffs
- Use antes de aprovar qualquer escrita live em Flash NOR/HDD, ao concluir grandes módulos novos, ao revisar claims para `N4_HARDWARE`/`T4`, ou quando o usuário pedir explicitamente uma auditoria rigorosa/adversarial.
- Combine com `panadiga-security-and-violation-guards` para bloquear violações e `panadiga-subagent-and-research` para executar auditorias independentes em contexto limpo.
- Copie e adapte os 5 prompts prontos em [adversarial-prompt-templates.md](./references/adversarial-prompt-templates.md).

## 2. Anatomia de um Prompt de Direcionamento de Alta Precisão
Todo prompt de tarefa ou delegação deve conter 6 blocos explícitos:
1. **Contexto & Objetivo Mensurável:** O problema exato e o estado final verificável.
2. **Escopo & Fronteira de Arquivos:** O que pode ser alterado e o que é estritamente proibido tocar.
3. **Invariantes Não-Negociáveis:** Regras físicas (`P7501`, `NSW5V`, `SA0..SA10`, `0xC000_100C`), arquiteturais (FCIS/MVVM) e de paridade aplicáveis ao recorte.
4. **Fontes & Ferramentas Verificadas:** Comandos CLI/MCP/scripts reais a consultar (zero nomes inventados).
5. **Critério de Verificação Objetiva:** Comandos exatos que devem passar ao final (não "testar se funciona").
6. **Condição de Parada / Escalação:** Quando parar e reportar bloqueio em vez de improvisar.

## 3. As 5 Lentes da Auditoria Adversarial
Um auditor adversarial parte da premissa de que **o trabalho submetido contém um defeito oculto ou uma suposição não provada**, atacando por 5 vetores:
1. **Lente 1 — FMEA Elétrico & Hardware Cross-Model:** O procedimento assume pinagem (`P7501` em `EH58/EH59` é `+5.8V_AV`!), tensão `3.3V`, setorização ou driver de outro modelo? Há risco de ground loop SMPS ou backpower?
2. **Lente 2 — Falsificação de Premissas de Firmware/Dados:** Confunde `Loader ver0.61` com `ver0.63`? Ignora Armadilha #8 (`SUB fb 9c`), byte-swap `XERTDAEH` ou `SW_UART=LOW` cortando `Q7514`?
3. **Lente 3 — Integridade de Contratos & Hashes:** Flags CLI existem mesmo em `crates/panadiga-cli/src/cli/args/`? Hashes SHA-256 foram recalculados ou copiados de cache antigo?
4. **Lente 4 — Teatro de Testes vs Prova Real:** O teste usou mock frouxo, assert trivial ou extrapolou `Mn103Vm` (`N2_EMULATED`) como se provasse timing de flash/IRQ/WDT em silício real (`N4_HARDWARE`)?
5. **Lente 5 — Dívida Técnica, Hardcode & Flakiness:** Há duplicação (`jscpd`), god-module, string prose fora de `locales/`, `sleep` arbitrário em E2E ou acoplamento quebrado entre Rust e TS?

## 4. Fluxo de Execução de Auditoria Adversarial
1. Defina o alvo da auditoria (PR, patch binário, plano de bancada, dossiê ou módulo).
2. Instancie um subagente limpo (ou `panadiga-qa-auditor`) usando um dos templates de [adversarial-prompt-templates.md](./references/adversarial-prompt-templates.md), exigindo **evidência literal de arquivo/linha/endereço** para cada achado e classificação por severidade (`BLOQUEADOR`, `ALTO`, `MÉDIO`, `NOTA`).
3. Exija que o auditor liste também **contra-provas negativas** (o que tentou quebrar e não conseguiu, com comandos executados).
4. Bloqueie qualquer promoção de tier/ledger ou gravação física enquanto houver item `BLOQUEADOR` ou `ALTO` aberto.

## 5. Armadilhas & Anti-Padrões
- Prompts vagos do tipo "revise tudo e diga se está bom" produzem aprovação complacente (*sycophancy*). Sempre exija busca ativa por falhas específicas e evidência literal.
- Usar o mesmo contexto carregado de vieses para auditar a própria suposição em casos críticos; prefira contexto fresco de subagente com acesso somente aos artefatos e provas.

## 6. Verificação
- Um relatório adversarial válido cita caminhos exatos, linhas de código, endereços hexadecimais e saídas de comandos reais executados — nunca opiniões genéricas.
