---
name: panadiga-skill-authoring
description: >-
  Guia oficial para projetar, criar, refatorar e auditar skills do ecossistema Antigravity em
  .agents/skills/ (SKILL.md + references/ + scripts/), garantindo progressive disclosure,
  ortogonalidade por assunto, precisão factual de CLI/MCP e zero tokens obsoletos. Use quando
  precisar criar uma nova skill ou reorganizar skills existentes.
---

# Criação, Refatoração & Auditoria de Skills (Skill Creator)

Esta skill orienta a criação e evolução do próprio ecossistema `.agents/skills/`, evitando tanto *god-skills* inchadas quanto micro-skills fragmentadas.

## 1. Quando Usar & Handoffs
- Use ao identificar um fluxo recorrente não coberto, ao extrair aprendizados portáteis para novos projetos ou ao atualizar skills após grandes descobertas técnicas.
- Consulte os padrões da plataforma em `agy-customizations` (`docs/skills.md`) e mantenha o catálogo sincronizado em [../README.md](../README.md) e `panadiga-skill-orchestrator`.
- Template pronto e script de lint em [skill-template-and-lint-checklist.md](./references/skill-template-and-lint-checklist.md).

## 2. Critérios de Design & Organização por Assunto
1. **Quando Criar Skill vs `references/` vs Atualizar Existente:**
   - **Atualize skill existente** se o novo procedimento pertence ao mesmo domínio operacional e ciclo de vida.
   - **Adicione arquivo em `references/`** para tabelas extensas, mapas de memória, listas de chaves, endereços hex ou checklists longos.
   - **Crie nova skill** apenas quando houver um domínio coeso com gatilhos, ferramentas e invariantes próprios (máximo foco, sem duplicar conteúdo de outras skills).
2. **Progressive Disclosure Obrigatório:**
   - `SKILL.md` ideal tem **~90–170 linhas**: frontmatter claro + quando usar/handoffs + invariantes + workflow executável + armadilhas + verificação + links relativos para `./references/*.md`.
3. **Escopo de Instalação:**
   - `.agents/skills/<nome>/` no repositório: skills versionadas do projeto/ecossistema.
   - `~/.gemini/config/skills/<nome>/`: skills pessoais cross-workspace (somente se pedido pelo usuário).
   - Plugin (`plugin.json`): quando empacotar skills + rules + MCP em unidade distribuível.

## 3. Invariantes Obrigatórias de Qualidade em Skills Panadiga
1. **Frontmatter YAML Válido:** `name` exatamente igual ao nome da pasta (kebab-case) e `description` em terceira pessoa com palavras-chave bilíngues (PT-BR + termos técnicos EN) dizendo **o quê** faz e **quando** ativar.
2. **Veracidade Literal de Ferramentas:** Cada comando `panadiga ...`, script `python -m ...`, script `npm run ...` e tool MCP citado deve ser verificado antes na fonte (`crates/panadiga-cli/src/cli/args/`, `crates/panadiga-mcp/src/tools/`, `package.json`, `scripts/`).
3. **Higiene de Tokens & Segurança (`test_profiles_ssot.py` / `test_docs_parity.py`):**
   - Use sempre `SA0..SA10` (256 KiB IPL RO), `SA11..SA133` (Upper NOR/BGO) e `SA134..SA141` (64 KiB NVRAM).
   - Nunca propague os ranges incorretos antigos, hashes revogados de `PANA__SD.FRM`, ou versões erradas de stack (`11 crates` Rust; `React 18.3` + `TypeScript 7` + `Tauri 2`).
4. **Handoffs Sem Duplicação:** Referencie outras skills pelo nome em vez de copiar blocos extensos entre skills.

## 4. Workflow Canônico para Nova Skill
1. Verifique o gap real consultando [../README.md](../README.md) e `.agents/skills/*/SKILL.md`.
2. Colete e confira diretamente na fonte todos os comandos CLI, flags, paths e invariantes envolvidos.
3. Crie `.agents/skills/<nome>/SKILL.md` e, se houver material denso, `.agents/skills/<nome>/references/<tema>.md`.
4. Atualize o mapa/tabela de roteamento em [../README.md](../README.md), em `panadiga-skill-orchestrator`, no `## Skills Associadas` das personas aplicáveis em `.agents/agents/` e regenere o arsenal com `python scripts/generate_tool_arsenal.py --write`.
5. Rode a verificação automatizada de frontmatter, links relativos e higiene descrita em [skill-template-and-lint-checklist.md](./references/skill-template-and-lint-checklist.md), mais `pytest scripts/tests/test_docs_parity.py scripts/tests/test_profiles_ssot.py`.

## 5. Armadilhas & Anti-Padrões
- Gerar arquivos com redirecionamento PowerShell sem UTF-8 criando mojibake (sequências Latin-1 corrompidas ou `U+FFFD`); escreva sempre em UTF-8 limpo.
- Copiar tabelas de endereços inteiras em várias skills criando deriva (*drift*) na primeira mudança de código.
- Criar skill genérica de programação ("como escrever bom código") ignorando os guards concretos do repositório (`oxlint`, `tracing`, `jscpd 0%`, `CapabilityGate`).

## 6. Verificação
- Confirme frontmatter parseável, `100%` dos links relativos existentes em disco, zero ocorrências de flags/setores obsoletos e aprovação dos testes Python de paridade/SSOT.
