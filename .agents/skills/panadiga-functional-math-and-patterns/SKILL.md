---
name: panadiga-functional-math-and-patterns
description: >-
  Programação funcional zero-heap para sistemas operacionais embarcados, firmware C11 bare-metal
  (Panadiga OpenOS AM34 e Pico P7501 RP2040/RP2350) e crates Rust (#![deny(unsafe_code)]),
  simplificação de código procedural para núcleos determinísticos (FSMs puras, ADTs, newtypes,
  tabelas const/.rodata), matemática sem float (ponto fixo Q8.8/Q16.16, aritmética modular,
  álgebra de intervalos [lo, hi)), reconhecimento estatístico/estrutural de padrões (entropia,
  qui-quadrado, ECB, S-box, espelhos, strides) e provas algébricas/diferenciais. Use ao
  projetar drivers/OS determinísticos, refatorar lógica procedural complexa ou criar algoritmos
  matemáticos e detectores de padrões em C11 ou Rust.
---

# Programação Funcional Embarcada, Matemática Determinística & Padrões (`C11` + `Rust`)

Esta skill converte lógica procedural acoplada, matemática frágil e heurísticas ad-hoc em **núcleos funcionais puros, determinísticos, sem heap e matematicamente verificáveis** para o `Panadiga OpenOS` (`AM34`), firmware `Pico P7501` (`RP2040/RP2350`) e os `11` crates Rust.

## 1. Quando Usar & Handoffs
- Ative ao projetar ou simplificar FSMs de SO/drivers (`firmware/panadiga-openos/`, `firmware/pico-p7501/`), refatorar módulos procedurais extensos (`crates/panadiga-core/src/re/`, `hdd/`, `nvram/`), implementar cálculos sem FPU/heap, analisar padrões em dumps/telemetria ou construir suítes de provas algébricas.
- Combine com `.agents/skills/panadiga-architecture-and-guardrails/SKILL.md` (fronteira FCIS e `jscpd 0%`), `.agents/skills/panadiga-opensource-firmware-dev/SKILL.md` (`AM34 C11`), `.agents/skills/panadiga-pico-flashing/SKILL.md` (`RP2040/RP2350`), `.agents/skills/panadiga-rust-backend-dev/SKILL.md` (`crates/`), `.agents/skills/panadiga-hardware-forensics/SKILL.md` (triagem de dumps) e `.agents/skills/panadiga-validate/SKILL.md`.
- **Referências Detalhadas (com exemplos completos Antes vs Depois e código ruim vs correto):**
  - Refatoração procedural → funcional em C11 & Rust, FSMs puras, `.rodata` LUTs, ADTs e **8 erros comuns** em [fp-embedded-c11-rust-refactoring-playbook.md](./references/fp-embedded-c11-rust-refactoring-playbook.md).
  - Matemática sem float (ponto fixo/modular/bitwise), álgebra de intervalos `[lo, hi)`, reconhecimento estatístico/estrutural de padrões, provas algébricas/diferenciais e **6 erros comuns** em [deterministic-math-pattern-recognition-and-proofs.md](./references/deterministic-math-pattern-recognition-and-proofs.md).

## 2. Os 6 Invariantes Não-Negociáveis (FP Embarcado & Matemática)
1. **Pure Core, Thin Imperative Shell (Zero Heap):** Separe 100% da decisão, parsing e transição de estado em funções puras sem acesso direto a `volatile`, `UART`, `USB` ou disco; a casca imperativa apenas captura snapshots de entrada e executa o `effect` retornado.
2. **Make Illegal States Unrepresentable:** Em Rust, substitua `u32` soltos e cadeias repetidas de `if` por *newtypes* (`AddrSpan`, `AlignedOffset`) e `enum` exaustivos; em C11, use `struct` imutável (`const *`), `_Static_assert` e predicados de fronteira na borda.
3. **Zero `float` e Zero `libgcc` Acidental em Bare-Metal (`AM34` / `RP2040`):** Use ponto fixo (`Q8.8` / `Q16.16`), escalonamento por potência de 2 com arredondamento simétrico, aritmética modular `uint32_t` para timers e evite divisão/módulo `uint64_t` em `-nostdlib`.
4. **Álgebra de Intervalos Semiabertos `[lo, hi)` em `u64` / `uint64_t`:** Nunca avalie `addr + len <= limit` em 32 bits — wraparound de `u32` pode burlar guardas de `SA0..SA10` IPL ou `0xC000_100C`.
5. **Iteração Estrutural Limitada (Zero Recursão em Firmware):** Toda função em `panadiga-openos` ou `pico-p7501` deve rodar com pilha `O(1)` e cota máxima explícita de passos (sem risco de stack overflow em `512 KiB SDRAM_STAGE15` ou `256 KiB P0_FW_RO`).
6. **Equivalência Algébrica & Diferencial Antes de Apagar Legado:** Toda simplificação de código crítico deve provar leis (*round-trip*, *idempotência*, *conservação*) e equivalência `novo_puro(x) == legado(x)` em vetores de borda e PRNG determinístico de semente fixa.

## 3. Caminho Prático em 5 Passos (Simplificação Procedural → Funcional)
1. **Isolar a Fronteira I/O:** Identifique toda leitura `volatile` / syscall / file I/O no bloco procedural e extraia uma `struct InputSnapshot` (dados puros) e um `enum/struct Effect` (ações a executar).
2. **Modelar Estado & Invariantes:** Converta flags booleanas soltas (`busy`, `done`, `err`) em um único estado de máquina válido (`enum` em Rust / `enum + tagged union` em C11).
3. **Substituir Controle Mutável por Transformação Pura:** Troque loops com mutação dispersa e árvores `if/else` por:
   - em **C11**: `step(state, ev) -> step_res_t` por valor + tabelas `static const` em `.rodata`;
   - em **Rust**: combinadores (`chunks_exact`, `windows`, `try_fold`, `filter_map`) e tabelas `const`.
4. **Aplicar Matemática Exata:** Converta delays bloqueantes em delta modular `(uint32_t)(now_ms - t0_ms)`, cálculos de tensão/filtro em `Q8.8`/`Q16.16` e validações de Flash/HDD em operações `AddrSpan` `[lo, hi)`.
5. **Provar Equivalência & Validar nos Gates:** Rode testes diferenciais (`novo == legado`) + casos de borda (`0`, `u32::MAX`, `0x8003_FFFF/0x8004_0000`, `0xC000_100B/0xC000_100C`), cheque `jscpd 0%` e passe nos gates da linguagem.

## 4. Erros Comuns Críticos (Detalhes e Correções nas Referências)
- Confundir FP embarcado com alocação no heap (`Vec`/`.clone()` em loop quente ou `malloc` em firmware) ou recursão sem TCO garantido.
- Comparar timers com `now >= deadline` (quebra no rollover de 49,7 dias / 71,5 min) ou somar `addr + len` em `u32` (overflow).
- Ler `volatile` múltiplas vezes dentro de regras de decisão (race / valor inconsistente) ou usar *bit-fields* C para MMIO/protocolo.
- Estimar entropia de Shannon em janelas `< 256 B` (subestimação sistemática), testar S-box só por bijeção (aceita `S[i] = i`), ou validar só *happy path* sem testes diferenciais.

## 5. Verificação
```bash
# Rust (núcleo puro, invariantes e testes diferenciais sem unsafe)
cargo test --workspace && cargo clippy --workspace --all-targets --no-deps -- -D warnings

# Duplicação zero após extrair tabelas/helpers
npx --prefix apps/desktop jscpd --config .jscpd.json

# C11 Host Tests (sem float/heap/UB; -fsanitize=address,undefined quando disponível)
python scripts/validate.py --sequential
```
