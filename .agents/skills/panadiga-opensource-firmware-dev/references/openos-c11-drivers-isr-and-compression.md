# DigaOS — Engenharia C11 Freestanding, Drivers, ISRs, Descompressão Sem Heap & Testes Host

Este guia destila práticas essenciais de arquitetura embarcada, drivers bare-metal, disciplina de interrupções/DMA, descompressão `zlib`/LZSS sem alocador dinâmico e verificação host para evoluir **`firmware/digaos/`** (`M0 -> M4`) e **`firmware/picofw/`** sem importar suposições incompatíveis de Cortex-M ou RTOS.

---

## 1. Revisão Arquitetural em 2 Fases (`M1 -> M4`)

Antes de escrever subsistemas grandes (TBC/AIOF, FAT16/SDCD, ATA/DAVIS, Stage-2 loader ou `AVFE`/`MPE4`):
1. **Fase 1 (Contrato & Orçamento antes do Código):**
   - Defina contratos de interface primeiro (`.h` público, posse de buffers/periféricos, semântica de erro, orçamento de RAM/SRAM/SDRAM, deadline e jitter máximo de ISR).
   - Distinga sempre **Fatos Verificados** (MMIO medido/disassembly Ghidra), **Suposições** e **Medições Reais** em bancada.
   - Avalie opções (`Superloop / Event-Driven Bare-Metal` vs `Scheduler Cooperativo` vs `RTOS`) nos 5 eixos com **`Good` / `Trade-off` / `Problem` / `Unknown`** — **nunca esconda `Unknown` como `Good`**:
     - Eficiência de hardware (RAM/SDRAM, CPU, DMA) · Complexidade · Manutenibilidade/seams de teste · Tempo real (pior caso de bloqueio, budget de ISR) · Segurança/robustez (timeouts, WDT, recuperação).
   - **Regra Bare-Metal:** Prefira superloop/event-loop com tempo limitado (*bounded run-to-completion*) sempre que os deadlines couberem sem escalonador; só adote RTOS se o isolamento entre tarefas pagar o custo de stacks, latência e depuração.
2. **Fase 2 (Entrega Incremental por Dependência):**
   - Contrato público -> implementação modular -> testes host (`-DPANADIGA_DIGAOS_HOST_TEST=1`) -> validação `Mn103Vm` -> checkpoint em silício (`VEP79138`). Nunca misture aprovação de arquitetura com permissão para gravar Flash física.

---

## 2. Estilo ISO C11 Freestanding (`AM34 / MN103S`)

- **Toolchain Obrigatória (`mn10300-elf-gcc`):** O SoC Matsushita **`AM34 / MN103S` não possui backend LLVM/`rustc`**; o kernel e drivers do `DigaOS` são **100% ISO C11 + GNU Assembly (`crt0_am34.S`)**. Rust `no_std` aplica-se apenas a eventuais alvos futuros `ARMv7`/`AArch64` (UniPhier).
- **Flags Estritas:** `-std=c11 -Os -Wall -Wextra -Werror -ffreestanding -nostdlib`.
- **Tipos & Checagens em Compilação:** Use exclusivamente tipos de largura fixa (`uint8_t`, `uint16_t`, `uint32_t`, `uintptr_t`) de `<stdint.h>` e `_Static_assert` para validar endereços de barramento proibido (`0xC000100C`), tamanhos de região (`256 KiB` IPL) e layouts de struct/frame (`16 B` `TBUS`).
- **Zero Alocação Dinâmica:** `malloc`/`calloc`/`free` são proibidos no kernel e drivers bare-metal; use buffers estáticos em `.bss` ou arenas pré-dimensionadas com falha explícita e limitada.
- **Evitar `libgcc` Acidental & `const`-Correctness:** Em `-nostdlib -Os`, evite divisão/módulo 64-bit (`uint64_t /`) sem prover `udivdi3`; marque ponteiros de leitura/tabelas como `const` e registre explicitamente **Endianness** + distinção entre **`file offset`** e endereço virtual/físico em memória (**`VMA`/`LMA`**).

---

## 3. Design de Drivers Bare-Metal & HAL (`firmware/digaos/drivers/`)

- **Posse Única (*Single-Owner*) & Costura HAL Limpa:** Cada periférico e buffer tem dono explícito. Encapsule acessos de registrador através de `hal/am34_mmio.h` (`am34_mmio_read*` / `am34_mmio_write*`) para viabilizar `-DPANADIGA_DIGAOS_HOST_TEST=1` sem criar camadas redundantes para cada bit isolado.
- **Polling Finito com Clock Válido:** Toda espera em hardware deve usar limite determinístico baseado em clock ativo naquele estágio de boot ou contador finito (configurado em `digaos_ssot.h` ao checar `SC0STR & 0xA0`). Para contadores com *wraparound*, use subtração modular (`(uint32_t)(now - start) < timeout`).
- **Sequência de Hardware Verificada (`DAVIS CH1` / `IDE2SAT2`):** `DAVIS CH0` (`0x89000400`) usa reset ativo-BAIXO (`0x0002`); `DAVIS CH1` (`0x89000C00`) usa reset ativo-ALTO (`0x0000`) e exige **`1000 ms`** após reset antes de consultar `BSY`/`DRDY` quando conectado à ponte StarTech `IDE2SAT2` (`88SA8052`).
- **Estados Degradados Explícitos:** Drivers devem lidar com dispositivo ausente, barramento travado, frame malformado e timeout retornando `bool`/código de erro sem entrar em loop infinito.

---

## 4. Disciplina de ISRs, Concorrência, DMA & Watchdog (`AM34` & `RP2040/RP2350`)

*(Nota: Não assuma NVIC Cortex-M nem APIs FreeRTOS `FromISR` no AM34 — verifique sempre a semântica real do controlador de interrupção e periférico alvo.)*

1. **ISRs Curtas & Semântica de Ack:** Limite o trabalho na ISR ao mínimo determinístico; confira no datasheet/disassembly se o status limpa por leitura (*clear-on-read*), *write-1-to-clear* (W1C) ou sequência ordenada antes de sair da ISR para evitar tempestade de IRQs repetidas. Nunca faça heap alloc ou `digaos_uart0_puts` pesado dentro de ISR.
2. **Filas SPSC Estáticas & `volatile`:** `volatile` garante apenas visibilidade de acesso ao registrador MMIO — **jamais garante atomicidade, exclusão mútua ou ordenação de memória**. Proteja operações RMW compostas ou variáveis multi-word compartilhadas entre ISR e main loop usando seção crítica (`CLI`/desabilitar IRQ naquele nível) e filas ring-buffer SPSC estáticas com política clara de overflow (*drop newest/oldest* + contador).
3. **Invariante Crítico para Flash NOR (`S29JL064H`):** Antes de emitir sequências CFI que tiram a NOR de `Read Array` (`~700 ms` por erase + Pino 14 `WP#/ACC` HIGH), **desligue IRQs (`CLI`) e mova a tabela de vetores para SDRAM** — caso contrário qualquer IRQ durante o erase lê instruções inválidas da Flash e trava a CPU.
4. **Coerência de DMA & Cache (`DAVIS` / `SDCD` / `AVFE`):** Documente posse CPU ↔ periférico, alinhamento de buffer, regiões de memória acessíveis pelo DMA, alinhamento de cache line, direção `clean` (CPU→DMA) vs `invalidate` (DMA→CPU) e barreiras de conclusão antes de liberar buffers.
5. **Watchdog `IC7501 TBUS` Orientado a Progresso:** `IC7501` derruba os rails SMPS após `~44 s` sem keepalive válido (`0xDC010010..0xDC01001F`, frame `[0x8A, 0x02, 0x00, 0xFF, ..., 0x30]`, checksum `(sum(body15) - 0x5B) & 0xFF`). **Nunca** alimente o watchdog cegamente a partir de um timer ISR isolado que mascararia travamento do loop principal — só emita o keepalive após verificar progresso real de saúde/subsistema, respeitando janelas de espera longas (`1000 ms` `IDE2SAT2` / erase).

---

## 5. Descompressão Bare-Metal Sem Heap (`Stage-1.5 -> Stage-2`)

Para o Stage-1.5 em `SDRAM_STAGE15` (`0x50400000`, `512 KiB`) descomprimir o kernel Stage-2 para `SDRAM_STAGE2` (`0x57000000`, `16 MiB`):
- **Assinaturas & Rodapés de Stream:**
  - `zlib` (RFC 1950): header `0x78 0x01` / `0x78 0x9C` / `0x78 0xDA`, checksum **Adler-32** no footer (4 bytes big-endian).
  - `raw deflate` (RFC 1951): sem cabeçalho/rodapé externo (usado quando o contêiner pai já provê tamanho/hash).
  - `gzip` (RFC 1952): magic `0x1F 0x8B`, checksum **CRC-32** + tamanho `ISIZE` mod 2³² (little-endian) no footer.
  - **Okumura LZSS (`EXTRHEAD algo=2`):** ring buffer de `4 KiB` (`4096 B` pré-preenchido com `0x20` até `4078`), zero alocação dinâmica.
- **Execução Sem Heap:** Use decodificador *freestanding* em buffers estáticos (`.bss`): `tinfl` (`miniz`) com janela deslizante fixa de `32 KiB` em SDRAM (ou saída direta in-place em `0x57000000` se contíguo) ou o decoder Okumura LZSS de `4 KiB`. Sempre valide tamanho descomprimido + Adler-32/CRC-32/SHA antes de saltar para `0x57000000` e atualize ponteiros/tamanhos em headers ao empacotar (`DIGADIAG.FRM`).

---

## 6. Testes Host C11, Fault Injection & Separação Epistêmica

- **Testes Negativos & Injeção de Falhas (`tests/test_digaos_host.c`):** Valide não só o *happy path*, mas também tentativas de leitura/escrita em `0xC000100C`, `0xDC0F0000..0xDC0F0FFF`, escrita no IPL `SA0..SA10` (`0x80000000..0x8003FFFF` e `0x84000000..0x8403FFFF`), rollover de tick, timeouts expirados, saturação de buffer, frames `TBUS` corrompidos e ausência de resposta `DAVIS`.
- **Sanitizers no Host:** Quando `gcc`/`clang` no host suportar, rode também com `-fsanitize=address,undefined` para capturar UB/overflow em parsers C11 antes do cross-compile.
- **Separação Epistêmica Obrigatória:**
  - `test_digaos_host` (`-DPANADIGA_DIGAOS_HOST_TEST=1`) prova apenas lógica C11 com mock MMIO (`N1_STATIC`).
  - `Mn103Vm` (`panadiga re verify-patches`) prova fluxo ISA e limites estáticos (`N2_EMULATED`).
  - **Somente hardware real (`VEP79138` isolado via SMPS/`ADuM` + `COM10` `P7501`)** prova timing elétrico, `IC7501 TBUS` > 10 min, vídeo `AIOF`/`AVDD` e `DAVIS CH1` com `IDE2SAT2` (`N4_HARDWARE`).
