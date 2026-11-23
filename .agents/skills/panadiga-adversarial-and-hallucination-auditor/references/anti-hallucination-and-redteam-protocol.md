# Protocolo Anti-Alucinação & Red-Team Contínuo

## 1. Taxonomia de Alucinações Comuns de Agentes (e Como Bloquear)

| Tipo de Alucinação | Exemplo Real Evitado | Contramedida Obrigatória |
| :--- | :--- | :--- |
| **Versão/Contagem por Memória** | Citar `React 19` ou `22` logs ou `337` testes quando os arquivos mudaram | Ler `package.json`, `Cargo.toml`, `docs/INDEX.md` e rodar o comando real antes de citar números |
| **Comando/Flag Fantasma** | Citar comandos ou flags que existiam em rascunhos ou outros projetos | Conferir `crates/panadiga-cli/src/cli/args/*.rs` ou rodar `--help` |
| **Falso "Clean-Room" / Falso `N4_HARDWARE`** | Tratar `Mn103Vm` como prova de silício ou protótipo com disassembly ao lado como *clean-room* | Aplicar escala `N1..N5` e proveniência jurídica explícita |
| **Alegação de Teste Sem Execução** | Dizer "Quality Gates verdes" após editar um arquivo sem re-rodar o comando | Executar o comando **após o último edit** e reportar escopo exato (ex.: `9 passed` no `pytest`, quais lanes rodaram) |
| **Certeza em Premissa Cross-Model** | Assumir que `P7501` é UART 3.3V em toda placa DIGA (`EH58/EH59` é `+5.8V_AV`!) | Checar tupla `(model, region, board_rev, flash_jedec_id)` e esquemático do modelo |

## 2. Checklist Red-Team para Revisão de Código e Segurança
1. **Fronteira WebView -> Rust/Kotlin:** Todo comando IPC que lê/escreve arquivo valida `canonicalize()` + diretório permitido? Comandos destrutivos validam confirmação e política no backend (não só na UI)? Bridges Android `@JavascriptInterface` sanitizam parâmetros antes de tocar sysfs/shell?
2. **Segurança de Hardware & Concorrência:** Timeouts Tauri permitem a duração real da operação (`185_000 ms` para dump NOR de 180s, nunca truncado em `125_000 ms`)? GPIOs de IR excluem `GP26` (`SW_UART`) e `GP10..15` (`SDSNIFF`) além de `GP0..5`?
3. **Escrita Atômica & Isolamento:** Manifests e settings usam temp file único (PID + sequência atômica) + rename sem colidir entre threads concorrentes?
4. **Cobertura de Falha:** Existe teste garantindo que entradas maliciosas/corrompidas (payload shell, `schema_version: 999`, dump byte-swapped, `GP3` / `GP26`) **falham fechado**?
