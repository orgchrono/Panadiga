# Templates Prontos: Prompts de Direcionamento & Auditoria Adversarial

## Template 1 — Auditoria Adversarial de Firmware, Patch ou Operação em Hardware

```text
Você é um Auditor Adversarial (Red-Team). Parta da hipótese de que este plano/patch possui
pelo menos 1 defeito oculto que pode travar, brickar ou gerar falso-positivo em bancada.

Alvo da auditoria: <ARQUIVO_OU_PLANO>
Hardware alvo (tupla obrigatória): (<MODEL>, <REGION>, <BOARD_REV>, <FLASH_JEDEC_ID>)

Ataque sistematicamente pelas 5 frentes:
1. FMEA Elétrico: checar SMPS/ADuM, pinagem real do conector no modelo (ex.: P7501 em EH58/EH59 é +5.8V_AV!), GP3/NSW5V Hi-Z sem pulls, XRESET Open-Drain, proibição de alimentar Pico via P7501, e bloqueio 0xC000_100C.
2. Proteção Flash & Storage: confirmar IPL SA0..SA10 em E05, backup SHA-256 prévio, confirmação explícita para Upper NOR (SA11..SA133) ou disco físico (jamais PhysicalDrive0/NVMe/sda/disk0), CoW antes de live.
3. Premissas Ocultas de Firmware: checar se confunde Loader ver0.61 (45 setores BGO 0x440337A4, sd48, DIGADIAG 256 KiB raw) com ver0.63 (sd64, 128 KiB EXTRHEAD); checar SW_UART=LOW cortando Q7514; checar CLI + vetores SDRAM antes de CFI Spansion (~700 ms).
4. Contratos CLI/MCP: conferir se cada comando e flag citada existe literalmente em crates/panadiga-cli/src/cli/args/.
5. Epistemologia: separar estritamente N1_STATIC / N2_EMULATED (Mn103Vm) de N4_HARDWARE.

Entregue:
- Tabela de achados [BLOQUEADOR | ALTO | MÉDIO | NOTA] com arquivo:linha ou endereço hex exato;
- Lista do que tentou quebrar e passou (com evidência);
- Veredito binário: [APROVADO | REPROVADO].
```

## Template 2 — Auditoria Adversarial de Código (Anti-Regressão, Hardcode, Duplicidade & Flakiness)

```text
Execute uma revisão adversarial sobre o diff/módulo <ALVO>. Procure ativamente por:
1. Violações arquiteturais: lógica de domínio/física vazando para TypeScript, @tauri-apps/* fora de src/services/, god-modules, ciclos de import, ou unsafe/println! em crates Rust não-FFI.
2. Hardcode & supressões: strings literais fora de locales/, números mágicos fora de constants/, modelos hardcoded fora de profiles/, comentários eslint-disable/@ts-ignore/@ts-expect-error.
3. Teatro de testes & flakiness: testes usando sleeps fixos, mocks frouxos escondendo falha real de IPC/serial, casos críticos sem asserção de erro/falha fechada.
4. Quebra de acoplamento: mudança em Rust sem atualizar DTO/schema TS/Zod, mudança em CLI/MCP/docs/profiles sem atualizar testes e contagens paritárias.

Cada achado DEVE citar arquivo:linha e o comando de verificação rodado.
```

## Template 3 — Auditoria Epistêmica de Dossiês, Ledger & Timeline

```text
Audite <DOC_OU_LEDGER> contra os artefatos reais em docs/debug_logs/, docs/evidence/ledger.json e crates/:
1. Todo [PROVADO EM HARDWARE REAL] no doc possui entrada N4_HARDWARE/N5_REPRODUCED correspondente no ledger.json com SHA-256 batendo?
2. Existem hashes obsoletos ou contagens dessincronizadas entre README, INDEX, STATUS e translation_pairs.json?
3. Alguma descoberta recente (ex.: 2026-11-22) está citada em relatório sem estar ancorada em evidência rastreável ou contradiz achado posterior?
4. Projetos experimentais (firmware/panadiga-blackbox-fw) e limites de Mn103Vm estão declarados com honestidade técnica e jurídica?
```

## Template 4 — Direcionamento Preciso de Tarefa de Engenharia

```text
Objetivo: <RESULTADO_VERIFICÁVEL>
Skills obrigatórias: <CAMINHOS_REAIS_DA_SKILL> (ex.: .agents/skills/panadiga-bench-operations/SKILL.md)
Escopo permitido: <ARQUIVOS_PERMITIDOS>
Proibido alterar: <ARQUIVOS_PROIBIDOS>
Invariantes aplicáveis: <LISTA_CURTA>
Verificação obrigatória ao concluir:
1. <COMANDO_1>
2. <COMANDO_2>
Se qualquer verificação falhar após 1 tentativa de correção ou surgir divergência de premissa, PARE e reporte o bloqueio exato antes de improvisar.
```
