# Antigravity & Gemini Pair Programming Directives — Panadiga Ecosystem

Este arquivo define as diretrizes de projeto para Antigravity e assistentes baseados em Gemini.

## Contexto do Projeto
- **Nome do Projeto**: Panadiga (Panasonic DIGA DMR-EH55 Engineering, Forensics, and Hardware Lab)
- **Idiomas Principais**: Rust Edition 2024 (6 crates: `panadiga-profiles`, `panadiga-core`, `panadiga-rag`, `panadiga-mcp`, `panadiga-cli`, `panadiga-desktop`), TypeScript (React 19/Tauri 2 Desktop + Android APK), C11 (RP2040/RP2350 sem gadgets externos na sonda), Python 3.10+ (Typer, Pytest, Mypy Strict)
- **Objetivo**: Engenharia reversa, recuperação forense do sistema de arquivos de disco MEIHFS, unbrick cirúrgico de NVRAM (erro U99) e diagnóstico via interface UART assíncrona `P7501` (`115200 8N1`).

## Diretrizes de Execução do Agente
1. **Verificação de Invariantes Físicas**:
   - Nunca sugira aplicar > 3.3V nos pinos LVTTL do SoC AM34 e mantenha `GP3` (`P7501 Pino 5 NSW5V`, ~5.15V upstream) sempre em `Hi-Z Input` sem pulls (nunca push-pull).
   - **Camada 1 — IPL Crítico de Boot (`SA0..SA10`, `256 KiB`: `0x80000000..0x8003FFFF` e `0x84000000..0x8403FFFF`)**: mantenha leitura estrita permanente (`E05`), preservando o boot `SDDIAG`/`P7501`.
   - **Camada 2 — Upper NOR / BGO (`SA11..SA133`: `0x80040000..0x807EFFFF` e `0x84040000..0x847EFFFF`)**: bloqueada por padrão (`E05`); desbloqueável somente mediante alerta explícito dos riscos, backup prévio completo de 8 MiB verificado por SHA-256 e `CONFIRM_UPPER_NOR` (ou CoW `panadiga nvram patch-upper-nor`) para recuperar aparelhos já bricados em `U99` (`BGO driver init failed[1]`).
   - **Camada 3 — NVRAM / EEMUL (`SA134..SA141`: `0x807F0000..0x807FFFFF`, `64 KiB` finais)**: escrita autorizada por padrão sempre validando hashes SHA-256 antes e após a sanitização.
   - **Registrador MMIO Letal (`0xC000100C`)**: leitura e escrita estritamente proibidas (trava barramento/WDT).

2. **Delegação a Subagentes**:
   - Ao executar tarefas de alta complexidade em áreas específicas, consulte e instancie os subagentes localizados em `.agents/agents/`.

3. **Validação Contínua**:
   - Ao concluir modificações em código fonte (Rust, C, Python, TypeScript), rode `npm run validate` (ou `python scripts/validate.py`).
   - Mantenha o limiar de duplicação `jscpd` em 0% (0 clones).
   - Mantenha `mypy --strict` passando sem erros nas ferramentas Python.

4. **Ferramentas MCP**:
   - Utilize as ferramentas expostas pelo servidor MCP nativo em Rust (`panadiga mcp`, configurado em `.mcp.json`).
