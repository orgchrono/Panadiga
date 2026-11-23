---
name: panadiga-adversarial-and-hallucination-auditor
description: >-
  Skill auditora geral de pensamento e ação do agente: auditoria anti-alucinação (exigência
  de evidência arquivo:linha/endereço/output real), auditoria adversarial Red-Team (5 vetores
  de ataque em hardware, firmware, WebView/IPC, SSOT e testes) e auditoria contínua de
  qualidade antes de concluir qualquer entrega. Use ao revisar código, planos, relatórios ou
  auditar as próprias premissas e respostas do agente.
---

# Auditoria Adversarial (Red-Team), Anti-Alucinação & Qualidade Contínua

Esta skill funciona como o **freio crítico interno** do ecossistema: ela audita não apenas o código do repositório, mas **tudo o que o próprio agente pensa, pressupõe, afirma, planeja e entrega**.

## 1. Quando Usar & Handoffs
- Ative como auto-auditoria antes de responder após investigações/edições, antes de aprovar operações live em hardware, ou quando o usuário pedir varredura red-team / busca de violações.
- Para templates copiáveis de prompts red-team destinados a subagentes limpos, use `.agents/skills/panadiga-prompt-and-adversarial-audit/SKILL.md`.
- Para aplicar bloqueios concretos contra violações elétricas/Flash/AppSec, combine com `.agents/skills/panadiga-security-and-violation-guards/SKILL.md`.
- Protocolo anti-alucinação detalhado e checklist red-team em [anti-hallucination-and-redteam-protocol.md](./references/anti-hallucination-and-redteam-protocol.md).

## 2. Regras Invioláveis Anti-Alucinação (Zero Chute)
1. **Evidência Direta ou Silêncio (`arquivo:linha` / Output Real):** Nunca afirme um caminho, nome de símbolo, flag CLI, número de testes, versão de dependência ou endereço hexadecimal sem tê-lo lido nesta sessão em arquivo ou saída real de ferramenta.
2. **Proibição de Extrapolar Snippets:** Um match de `rg`/`Select-String` mostra onde um nome aparece, **não** como a função se comporta — leia o bloco antes de descrever a lógica.
3. **Veracidade sobre Testes & Portões:** Só reporte um teste/gate como passando se você o executou **após a última edição** e observou todas as suítes passarem sem skips ocultos. Escopo parcial (ex.: apenas `pytest` sem `validate.py`) deve ser declarado exatamente como parcial.
4. **Incerteza Explícita:** Quando algo não foi checado ou falhou, marque explicitamente como não verificado/bloqueado — jamais preencha lacunas com detalhes plausíveis.

## 3. Auditoria Adversarial de 5 Vetores (Red-Team Contínuo)
Ao revisar qualquer plano, diff ou relatório, tente ativamente **quebrá-lo** em 5 frentes:
1. **Silício & Eletricidade:** Há suposição cross-model perigosa (`P7501` em `EH58/EH59` = `+5.8V_AV`), tensão >3.3V, `NSW5V` não-Hi-Z, falta de isolamento SMPS/`ADuM`, ou acesso a `0xC000_100C`?
2. **Firmware & Dados:** Confunde `Loader ver0.61` (`45` setores BGO `SA26..SA70`) com `ver0.63`, ignora `SW_UART=LOW` cortando `Q7514`, falha de `CLI`/vetores SDRAM antes de CFI, ou aceita dump byte-swapped (`XERTDAEH`)?
3. **WebView / IPC / Android Security:** O WebView consegue forjar `CONFIRM-*`, ler/escrever paths arbitrários no Rust, injetar shell em bridge Kotlin `@JavascriptInterface` (`su`), ou carregar JS inline (`'unsafe-inline'`)?
4. **Coerência SSOT & Deriva Cross-Layer:** O dado vem realmente de `profiles/` / `ssot export-all` ou foi duplicado/hardcodado em TS, C11, Python ou Tauri `constants.rs`?
5. **Teatro de Testes & Flakiness:** Os testes testam apenas *happy path* com mocks frouxos, têm diretórios temporários fixos sem ID único ou ignoram casos fail-closed?

## 4. Gate de Auto-Auditoria Antes de Responder
Antes de emitir a resposta final, o agente verifica internamente:
- [ ] Cada comando CLI e flag citada foi conferida na fonte real (`crates/panadiga-cli/src/cli/args/`)?
- [ ] Cada afirmação factual tem lastro observado nesta sessão?
- [ ] A separação epistêmica (`N1_STATIC` vs `N2_EMULATED` `Mn103Vm` vs `N4_HARDWARE`) foi respeitada?
- [ ] Todos os testes citados rodaram **depois** da última mudança de arquivo?

## 5. Verificação
- Relatórios auditados listam evidência verificável (`arquivo:linha`), contra-provas testadas e status explícito do que foi executado vs o que não foi rodado.
