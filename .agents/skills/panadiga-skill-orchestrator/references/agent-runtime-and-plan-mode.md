# Plan Mode & Recursos Nativos Antigravity (Guia do Orquestrador)

Este guia capacita o agente a usar todos os recursos nativos de plataforma do Antigravity com precisão e sem desperdício.

---

## 1. Plan Mode (Protocolo Dual-Model Plan / Execute)
O runtime alterna entre dois modelos na mesma conversa: **Planejador** (somente leitura + desenho do plano) e **Executor** (executa o plano aprovado passo a passo).

| Transição / Gatilho | Ação Obrigatória | Detalhes Críticos |
| :--- | :--- | :--- |
| Novo pedido complexo / multi-etapa | Planejar com inspeção read-only | Mapear arquivos reais, invariantes, riscos e plano em ondas antes de editar |
| Plano fechado com critérios objetivos | `exit_plan_mode(plan=...)` | Incluir ordem de execução, comandos exatos de verificação e seção `Re-plan if:` |
| Falha na 1ª tentativa de um passo | Executor corrige localmente | Desvios pequenos (import faltando, path movido, re-run) ficam no executor |
| Falha na **2ª tentativa** ou premissa falsa | `enter_plan_mode(reason=...)` | **Nunca** improvisar uma 3ª tentativa fora do plano; devolver com achados e opções |
| Decisão não prevista ou fora do escopo | `enter_plan_mode(reason=...)` | Não inventar escopo nem editar arquivos não previstos no plano |
| Mensagem `[plan review due]` | `enter_plan_mode(reason=...)` | Chamar **imediatamente** antes de qualquer outra tool, com status (feito / verificado / pendente) |

---

## 2. Progresso, Foco & Alinhamento com o Usuário
- **`update_steps`:** Checklist vivo de **3 a 12 etapas** com títulos curtos no imperativo (2–6 palavras, sem IDs/códigos internos). Chame em paralelo na primeira chamada da nova etapa (marcando anterior `done` e atual `in_progress`) e finalize marcando todas `done` antes da resposta final.
- **`update_focus`:** Frase curta no gerúndio (3–8 palavras) em paralelo ao iniciar cada fase distinta em tarefas de 3+ chamadas.
- **`ask_question`:** Use apenas para decisões com alternativas múltiplas (coloque a recomendada primeiro com `(Recommended)`). Para confirmações binárias sim/não, pergunte em texto normal.

---

## 3. Espectro Multi-Agente (Escolher a Primitiva Certa)

| Primitiva | Quando Usar | Características-Chave |
| :--- | :--- | :--- |
| **`invoke_subagent` / `define_subagent`** | 1–3 frentes paralelas ou delegação especializada (`research`, `self`, `panadiga-*`) | `send_message` para continuar mesmo subagente; `manage_subagents` (`list\|kill\|kill_all`); `Workspace`: `inherit`, `share` (worktree) ou `branch` (clone) |
| **`run_workflow`** | Orquestração programática em Python sobre muitos itens (fan-out em N arquivos/módulos, pipelines em estágios, agregação) | Primitivas `agent`, `parallel`, `pipeline`, `phase`. **Leia a skill builtin `run-workflow` antes de escrever o script** |
| **`swarm`** | Projeto grande dividido entre time autônomo em quadro compartilhado (`/teamwork`) | `add` (`workspace:"share"\|"branch"`), `status`, `models`, `model` (trocar modelo preservando contexto em cota/capacidade; nunca `kill`+`add`), `kill` só após integrar o trabalho |

---

## 4. Execução Assíncrona, Timers & Daemons
- **`run_command` + `manage_task`:** Ajuste `WaitMsBeforeAsync` (5000–10000 ms para comandos síncronos; 500 ms para daemons com `IsDaemon=true`). Gerencie tarefas em andamento via `manage_task` (`list|status|send_input|kill`).
- **`schedule`:** Timers one-shot (`DurationSeconds` + `TimerCondition`: `'never'|'any'|sender_id`) ou crons recorrentes (`CronExpression` 5 campos; `IsDaemon=true` apenas se deve continuar após a tarefa). **Nunca use `sleep` em shell como timer.**
- **Wakeup Reativo:** Nunca rode loops de polling. Pare de chamar tools quando aguardar subagentes, background tasks ou timers — o sistema acorda você automaticamente.

---

## 5. Artefatos, `scratch/`, Transcripts, MCP & Customizações
- **`artifacts` (`<appDataDir>\brain\<conversation-id>`):** Use `ArtifactMetadata` ao criar relatórios extensos, planos ou diagramas Mermaid (`flowchart`/`sequenceDiagram`/`stateDiagram-v2`/`classDiagram`/`erDiagram`/`xychart-beta` — evite HTML dentro de labels). Não re-resuma no chat o que já está no artefato.
- **`scratch/` (`<appDataDir>\brain\<conversation-id>/scratch/`):** Todo script temporário de análise, parser pontual ou dado descartável vai aqui — **nunca polua a raiz do repositório Git**.
- **Transcripts (`.system_generated/logs/`):** Inspecione `transcript.jsonl` primeiro e leia `transcript_full.jsonl` apenas nas linhas com `truncated_fields`. Linke conversas com `[rótulo](conversation://<id>)`.
- **MCP & Resources:** Além das chamadas nativas `call_mcp_tool`, use `list_resources` e `read_resource` para ler esquemas e snapshots (`panadiga://...`) sem abrir arquivos no editor.
- **Visualização & Descoberta:** Use `generative_ui` para diagramas/controles interativos inline e `search_marketplace` se o usuário pedir plugins/skills externos (nunca instale sozinho; sugira `<plugin-card>`).
- **Customizações (`agy-customizations`):** `.agents/rules/*.md` (sem frontmatter, limite 24 KB/arquivo, carregadas sempre), `.agents/skills/*/SKILL.md` (progressive disclosure), `hooks.json` e `plugin.json`.

---

## 6. Slash Commands Recomendáveis ao Usuário
Você não executa slash commands diretamente, mas deve sugeri-los quando agregarem valor real:
- `/plan` — alinhar plano passo a passo antes de execução complexa;
- `/grill-me` — entrevista interativa para fechar decisões de arquitetura/design antes de codar;
- `/boost` — raciocínio profundo, planejamento estratégico, múltiplas perspectivas e verificação rigorosa;
- `/teamwork` — coordenar múltiplos agentes autônomos em paralelo;
- `/goal` — tarefas longas (ex.: overnight / varredura completa de frota) até conclusão integral;
- `/schedule` — agendar rotinas recorrentes ou timers;
- `/learn` — persistir uma correção ou setup complexo para futuras sessões;
- `/compact` — resumir contexto quando a conversa ficar longa;
- `/browser` — interações visuais ou testes em páginas web (em vez de raspar via curl).
