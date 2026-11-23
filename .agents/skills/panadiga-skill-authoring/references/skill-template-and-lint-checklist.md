# Template Canônico de `SKILL.md` & Checklist de Auditoria

## 1. Template Pronto de `SKILL.md`

```markdown
---
name: panadiga-nome-em-kebab-case
description: >-
  Resumo claro em terceira pessoa do que a skill resolve em PT-BR com palavras-chave técnicas
  em inglês. Use quando [condições objetivas de ativação].
---

# Título Objetivo da Skill

Introdução de 1–2 frases definindo o escopo exato e o resultado entregue.

## 1. Quando Usar & Handoffs
- Ativar quando...
- Encaminhar para `panadiga-outra-skill` quando...
- Dados detalhados em `<./references/arquivo.md>` (substitua pelo caminho relativo real da referência).

## 2. Invariantes Não-Negociáveis
1. ...

## 3. Workflow Canônico (Comandos Verificados)
```bash
cargo run -q -p panadiga-cli -- <subcomando-verificado>
```

## 4. Armadilhas & Anti-Padrões
- ...

## 5. Daqui pra Frente & Reuso em Novos Projetos
- ...

## 6. Verificação de Sucesso
- Critério objetivo + comando real que confirma sucesso.
```

## 2. Checklist de Auditoria Antes de Aceitar Nova Skill
- [ ] Pasta está em `.agents/skills/<name>/` e frontmatter `name:` bate byte-a-byte com `<name>`.
- [ ] `description:` usa terceira pessoa, sem aspas quebradas, com gatilhos claros de ativação.
- [ ] `SKILL.md` permanece enxuto (material denso movido para `references/*.md` com link relativo).
- [ ] Zero comandos inventados e zero flags herdadas sem checar `crates/panadiga-cli/src/cli/args/` ou `scripts/`.
- [ ] Zero ocorrências de faixas de setores obsoletas, hashes revogados ou versões de dependências desatualizadas.
- [ ] Skill foi registrada no roteador `.agents/skills/panadiga-skill-orchestrator/SKILL.md`, em [../../README.md](../../README.md) e nas personas `.agents/agents/*.md` pertinentes.

## 3. Script One-Liner de Verificação Rápida (Python Read-Only)
Execute na raiz do repositório após criar ou editar skills:

```bash
python -c "
import pathlib, re
root = pathlib.Path('.agents/skills')
for sk in sorted(root.iterdir()):
    if not sk.is_dir(): continue
    md = sk / 'SKILL.md'
    assert md.is_file(), f'Falta SKILL.md em {sk}'
    txt = md.read_text(encoding='utf-8')
    m = re.match(r'^---\r?\n(.*?)\r?\n---', txt, re.S)
    assert m and f'name: {sk.name}' in m.group(1), f'Frontmatter inválido em {md}'
    for target in re.findall(r'\[[^\]]+\]\(([^)#\s]+)', txt):
        if '://' not in target:
            assert (md.parent / target).exists(), f'Link quebrado {target} em {md}'
print('OK: skills válidas:', len([p for p in root.iterdir() if p.is_dir()]))
"
```
