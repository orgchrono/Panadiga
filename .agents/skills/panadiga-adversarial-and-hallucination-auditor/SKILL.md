---
name: panadiga-adversarial-and-hallucination-auditor
description: >-
  Auditoria contínua anti-alucinação (evidência literal arquivo:linha/endereço/output),
  auditoria adversarial Red-Team em 5 lentes/vetores (silício, firmware, WebView/IPC, SSOT
  e teatro de testes) e engenharia de prompts de direcionamento e revisão em contexto limpo.
  Use antes de operações em bancada, em revisões críticas de PR ou para auditar premissas.
---

# Auditoria Adversarial (Red-Team), Anti-Alucinação & Prompts de Revisão

Esta skill atua como o **freio crítico interno** do ecossistema: audita código, planos e afirmações do agente contra evidência literal e gera prompts de direcionamento e revisão red-team para expor premissas falsas antes que virem dano em silício ou dívida técnica.

## 1. Quando Usar & Handoffs
- Ative como auto-auditoria antes de responder após investigações/edições, antes de aprovar qualquer escrita live em Flash NOR/HDD, ao revisar claims para `N4_HARDWARE`/`T4`, ou ao concluir refatorações estruturais.
- Para aplicar bloqueios concretos contra violações elétricas/Flash/AppSec, combine com `.agents/skills/panadiga-security-and-violation-guards/SKILL.md`.
- Para instanciar subagentes auditores em contexto limpo, combine com `.agents/skills/panadiga-subagent-and-research/SKILL.md`.
- **Referências:**
  - Protocolo anti-alucinação detalhado e checklist red-team em [anti-hallucination-and-redteam-protocol.md](./references/anti-hallucination-and-redteam-protocol.md).
  - 5 templates prontos de prompts adversariais em [adversarial-prompt-templates.md](./references/adversarial-prompt-templates.md).

## 2. Regras Invioláveis Anti-Alucinação (Zero Chute)
1. **Evidência Direta ou Silêncio (`arquivo:linha` / Output Real):** Nunca afirme um caminho, nome de símbolo, flag CLI, número de testes, versão de dependência ou endereço hexadecimal sem tê-lo lido nesta sessão em arquivo ou saída real de ferramenta.
2. **Proibição de Extrapolar Snippets:** Um match de `rg`/`Select-String` mostra onde um nome aparece, **não** como a função se comporta — leia o bloco antes de descrever a lógica.
3. **Veracidade sobre Testes & Portões:** Só reporte um teste/gate como passando se você o executou **após a última edição** e observou todas as suítes passarem sem skips ocultos. Escopo parcial (ex.: apenas `pytest` sem `validate.py`) deve ser declarado exatamente como parcial.
4. **Incerteza Explícita:** Quando algo não foi checado ou falhou, marque explicitamente como não verificado/bloqueado — jamais preencha lacunas com detalhes plausíveis.

## 3. Auditoria Adversarial de 5 Lentes (Red-Team Contínuo)
Um auditor adversarial parte da premissa de que **há um defeito oculto ou suposição não provada**, atacando por 5 vetores:
1. **Silício & Eletricidade (FMEA Cross-Model):** Assume pinagem errada (`P7501` em `EH58/EH59` = `+5.8V_AV`), tensão >3.3V, `NSW5V` não-Hi-Z, falta de isolamento SMPS/`ADuM` ou acesso a `0xC000_100C`?
2. **Firmware & Dados:** Confunde `Loader ver0.61` (`45` setores BGO `SA26..SA70`) com `ver0.63` (`35` setores), ignora Armadilha #8 (`SUB fb 9c`), `SW_UART=LOW` cortando `Q7514`, `CLI`/vetores SDRAM antes de CFI, ou aceita dump byte-swapped (`XERTDAEH`)?
3. **WebView / IPC / Android Security:** O WebView consegue forjar `CONFIRM-*`, ler/escrever paths arbitrários no Rust, injetar shell em bridge Kotlin `@JavascriptInterface` (`su`), ou carregar JS inline (`'unsafe-inline'`)?
4. **Coerência SSOT, Contratos & Hashes:** Flags CLI existem em `crates/panadiga-cli/src/cli/args/`? O dado vem de `profiles/` / `ssot export-all` ou foi duplicado/hardcodado e os SHA-256 foram recalculados?
5. **Teatro de Testes, Dívida & Flakiness:** O teste usa mock frouxo, assert trivial, `Mn103Vm` (`N2_EMULATED`) apresentado como prova de timing/flash/IRQ em silício (`N4_HARDWARE`), `temp_dir` fixo, `sleep` em E2E ou duplicação `jscpd`?

## 4. Prompts de Direcionamento & Fluxo Red-Team em Subagente Limpo
1. **Anatomia de Prompt Preciso (6 Blocos):** (1) Contexto e objetivo mensurável; (2) Escopo e fronteira de arquivos; (3) Invariantes não-negociáveis; (4) Fontes e ferramentas reais verificadas; (5) Critério objetivo de verificação; (6) Condição explícita de parada/escalação.
2. **Auditoria Independente:** Instancie um subagente limpo (`panadiga-qa-auditor`) com um template de [adversarial-prompt-templates.md](./references/adversarial-prompt-templates.md), exigindo **evidência literal de arquivo/linha/endereço**, classificação (`BLOQUEADOR`, `ALTO`, `MÉDIO`, `NOTA`) e **contra-provas negativas** (o que tentou quebrar e não conseguiu).
3. **Bloqueio & Anti-Sycophancy:** Evite prompts vagos ("veja se está bom") e reutilizar contexto enviesado em decisões críticas; bloqueie escrita física ou promoção enquanto houver `BLOQUEADOR` ou `ALTO` aberto.

## 5. Gate de Auto-Auditoria & Verificação
- [ ] Cada comando CLI/flag citado conferido em `crates/panadiga-cli/src/cli/args/` e cada afirmação lastreada em leitura/output desta sessão.
- [ ] Separação epistêmica (`N1_STATIC` vs `N2_EMULATED` `Mn103Vm` vs `N4_HARDWARE`) respeitada e todos os testes citados rodados **após** a última edição.
- Relatórios auditados listam caminhos exatos, linhas/endereços, saídas de comandos reais executados, contra-provas negativas e o que não foi rodado.
