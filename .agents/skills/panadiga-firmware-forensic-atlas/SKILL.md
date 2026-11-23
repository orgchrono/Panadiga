---
name: panadiga-firmware-forensic-atlas
description: >-
  Atlas forense auto-atualizável para binários 100% disassemblados pelo Ghidra MN103S
  (53 programas zero-undef, dicionários de funções e callers, constantes hexadecimais
  Flash/SDRAM/MMIO, cabeçalhos e footers EXTRHEAD/TOC/MEIHFS/EEMUL/BGO, máscaras bitwise
  e side-effects BGO). Use em engenharia reversa e validação de endereços/estruturas.
---

# Atlas Forense Auto-Atualizável de Firmware (`100%` Disassembly Ghidra)

Esta skill mantém um atlas determinístico combinando os resultados `0 ??` da frota Ghidra (`docs/audits/zero_undef/fleet_verify.json`), capturas curadas (`docs/pseudocode/raw/doc_*.txt`), índices de símbolos (`profiles/symbols/`) e layouts/formatos SSOT via `scripts/generate_firmware_atlas.py`.

## 1. Quando Usar & Handoffs
- Ative ao consultar funções, callers/callees, constantes hexadecimais/MMIO, assinaturas de cabeçalho/footer (`EXTRHEAD`, `EXTRFOOT`, `MainListHeader`, `sd48`/`sd64`, `EEMUL`, `BGO`) ou máscaras bitwise já extraídas sem precisar carregar megabytes de texto bruto no contexto.
- Índice resumido em [FORENSIC_ATLAS_INDEX.md](./references/FORENSIC_ATLAS_INDEX.md) e JSON completo em `profiles/forensic_atlas/firmware_forensic_atlas.json`.
- Para checagem de armadilhas BGO `4 MiB` vs `8 MiB`, combine com `.agents/skills/panadiga-anti-trap-geometry-verification/SKILL.md`.
- Para consultas diretas em símbolos indexed por trem via MCP/RAG, use `panadiga_symbol_lookup` e `panadiga://symbols/<train_id>/<module>` (`.agents/skills/panadiga-rag-and-mcp-engineering/SKILL.md`).

## 2. Comandos (`scripts/generate_firmware_atlas.py`)
```bash
# Regenerar JSON em profiles/forensic_atlas/ + índice Markdown em ./references/
python scripts/generate_firmware_atlas.py --write

# Verificar deriva sem modificar arquivos (exit 0 = sincronizado, exit 1 = deriva)
python scripts/generate_firmware_atlas.py --check

# Exportar atlas completo em JSON para filtragem programática
python scripts/generate_firmware_atlas.py --json -
```
- Wrapper local na skill: `.agents/skills/panadiga-firmware-forensic-atlas/scripts/sync_atlas.py`.

## 3. Verificação
```bash
python -m mypy --strict scripts
python -m pytest scripts/tests/test_firmware_atlas.py
```
