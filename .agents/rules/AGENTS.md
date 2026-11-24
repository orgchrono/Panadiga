# Panadiga Workspace AI Rules

## Diretrizes Gerais de Engenharia
1. **Preservação de Dados e Integridade Forense**:
   - Nunca modifique arquivos de dump originais (`*.bin`, `*.FRM`) diretamente.
   - Qualquer operação de transformação deve gerar novo arquivo e validar hashes SHA-256.
2. **Segurança de Silício e Hardware de Bancada**:
   - No DMR-EH55 `P7501` (`UART 115200 8N1`), os sinais UART/controle (`SW_UART`/`XRESET`/`UARTM2P_T`/`UARTP2M_T`) operam em 3.3V LVTTL, e `GP3` (`Pino 5 NSW5V`, `~5.15V` upstream via `D7510`) deve permanecer **SEMPRE como `Hi-Z Input` sem pulls (NUNCA dirigir push-pull)**.
   - Na Flash NOR Spansion `S29JL064H` (`IC5001`, TSOP-48), a tensão máxima é rigorosamente **3.3V**; `XRESET` (`GP5`) deve ser acionado exclusivamente em **Open-Drain** (sem nunca forçar `+3.3V` push-pull).
3. **Padrão de Código e Qualidade**:
   - Zero clones no detector de duplicação `jscpd` (threshold 0).
   - Rust: código idiomático, memory safety estrita (`#![deny(unsafe_code)]` onde aplicável).
   - Python: tipagem completa verificada por `mypy --strict`.
   - TypeScript: modo estrito (`noImplicitAny`, etc.), testes Vitest e Playwright cobrindo novos fluxos.

## 4. Orquestração Obrigatória de Skills, Metas, Plan Mode, Testes e Subagentes (Always-On)
1. **Bootstrap Obrigatório do Orquestrador:**
   - Em qualquer tarefa não-trivial (2+ etapas, bancada, forense, decrypto, RE, código, docs, pesquisa ou testes), **LEIA OBRIGATORIAMENTE primeiro** [`.agents/skills/panadiga-skill-orchestrator/SKILL.md`](../skills/panadiga-skill-orchestrator/SKILL.md) para ativar just-in-time apenas as skills certas pelos caminhos explícitos ali definidos.
   - O mapa completo e diagrama do ecossistema das 29 skills reside em [`.agents/skills/README.md`](../skills/README.md).
2. **Pilares Transversais & Economia de Tokens:**
   - **Segurança & Barreiras:** [`.agents/skills/panadiga-security-and-violation-guards/SKILL.md`](../skills/panadiga-security-and-violation-guards/SKILL.md).
   - **Auditoria Anti-Alucinação & Red-Team:** [`.agents/skills/panadiga-adversarial-and-hallucination-auditor/SKILL.md`](../skills/panadiga-adversarial-and-hallucination-auditor/SKILL.md).
   - **Subagentes & Pesquisa:** [`.agents/skills/panadiga-subagent-and-research/SKILL.md`](../skills/panadiga-subagent-and-research/SKILL.md).
   - **Validação & Portões (9+1):** [`.agents/skills/panadiga-validate/SKILL.md`](../skills/panadiga-validate/SKILL.md).
   - Economize tokens apenas no uso de ferramentas (`panadiga_rag_query`/`panadiga_search_code` antes de leituras amplas, CLI `--json -`, `read_file` segmentado, `references/*.md` sob demanda), **nunca** abreviando raciocínio técnico, FMEA ou checagens de segurança.
3. **Disciplina de Plan Mode & Recursos Nativos:**
   - Consulte [`.agents/skills/panadiga-skill-orchestrator/references/agent-runtime-and-plan-mode.md`](../skills/panadiga-skill-orchestrator/references/agent-runtime-and-plan-mode.md) para usar corretamente `enter_plan_mode`/`exit_plan_mode` (devolver na 2ª falha ou `[plan review due]`, nunca improvisar na 3ª), `update_steps`, `update_focus`, `run_workflow`, `swarm`, `schedule` (jamais usar `sleep` como timer), `scratch/` (nunca sujar o repo Git) e slash commands (`/plan`, `/grill-me`, `/boost`, `/teamwork`, `/goal`).
