# Playbooks de Delegação a Subagentes, Pesquisa & Consolidação

## 1. Template de Brief para Subagente (`invoke_subagent`)

```text
Workspace: c:\Users\danie\Documents\Panadiga
Escopo & Modo: [Read-only / Escrita restrita a <caminho>]
Objetivo exato: [O que responder ou entregar]

Fontes prioritárias:
- [Caminhos ou módulos exatos já conhecidos]

Restrições obrigatórias:
- Cite APENAS caminhos, símbolos, flags e endereços diretamente observados em arquivos/tool output.
- Marque qualquer ponto não verificado como UNCERTAIN (nunca preencha lacunas por memória).
- Não altere arquivos fora do escopo e não faça polling.

Formato de entrega:
- Resumo estruturado (~máx N linhas) com caminho + linha para cada achado crítico.
```

## 2. Seleção do Subagente Certo
| Subagente / Tipo | Quando Escolher |
| :--- | :--- |
| `research` | Surveys amplos read-only em código, docs, pseudocódigo e logs |
| `self` | Subtarefa autônoma de implementação que precisa das mesmas tools de escrita/execução |
| `panadiga-architect` | Decisões cross-stack, SSOT (`profiles/`), acoplamento Rust/TS/C11/Python |
| `panadiga-storage-forensics` | MEIHFS, `P58001`, capacidade HDD, extração de partições |
| `panadiga-firmware-re` | Ghidra MN103S/multi-ISA, criptoanálise `.FRM`/Feistel/AES, patches em máquina virtual |
| `panadiga-hardware-embedded` | Firmware C11 Pico, PIOs, `P7501`, `IC7501`, `DAVIS`, bancada elétrica |
| `panadiga-desktop-ui` | Tauri 2, React 18.3, TS 7, Oxlint MVVM, Vitest, Playwright, Android APK |
| `panadiga-qa-auditor` | Auditoria adversarial, Quality Gates 1–9, invariantes de segurança, `ledger.json` |

## 3. Recuperação Determinística de Quedas (`429` / Timeout)
1. Execute `manage_subagents(Action="list")` para identificar o `conversationId` em estado `errored`.
2. Execute `manage_subagents(Action="kill", ConversationIds=[...])` apenas no subagente morto.
3. Divida o prompt original se estava muito amplo e relance com escopo fatiado em `invoke_subagent`.

## 4. Escala Epistêmica para Consolidação de Pesquisa
| Nível | Significado Obrigatório | Exemplos Válidos |
| :--- | :--- | :--- |
| `N1_STATIC` | Disassembly Ghidra, strings, esquemático ou código estático | Mapa `AIOF`, matriz `IC3001`, incompatibilidade NOR `ES35V` |
| `N2_EMULATED` | Validado em `Mn103Vm` ou `Matshita R6-MV4` simulado | `re verify-patches`, `capacity_patch`, keepalive `M0` em VM |
| `N3_LIVE_PROBE` | Sonda elétrica/UART passiva sem ciclo completo de boot/gravação | `PINCHECK`, `BUSDIAG`, `DIAGWATCH` |
| `N4_HARDWARE` | Captura ao vivo em silício real verificada por SHA-256 e log | Dump NOR `IC5001` 8 MiB, cura BGO Ciclo 2, `CH1 0xEC IDENTIFY` |
| `N5_REPRODUCED` | Reproduzido ponta a ponta em 2ª unidade física independente | Ainda `0` entradas em `ledger.json` (fronteira aberta!) |

## 5. Checklist de Consolidação Pós-Descoberta
- [ ] Evidência bruta salva com hash SHA-256 (nunca sobrescrevendo dump original).
- [ ] Classificação epistêmica honesta (`N1..N5` e `T0..T4`).
- [ ] Dossiê antigo contradito foi corrigido ou marcado com errata (nunca deixar duas verdades concorrentes sem aviso).
- [ ] Se descoberta muda pinagem, setorização, flag CLI ou armadilha: atualizar também a skill/referência afetada.
