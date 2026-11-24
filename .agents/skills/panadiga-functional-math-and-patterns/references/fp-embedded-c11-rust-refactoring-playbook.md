# Playbook Prático: Programação Funcional & Refatoração em `C11` Bare-Metal e `Rust`

Exemplos concretos de simplificação de código procedural complexo para núcleos funcionais determinísticos, sem heap, voltados para **`firmware/panadiga-openos/` (`AM34`)**, **`firmware/pico-p7501/` (`RP2040/RP2350`)** e **`crates/panadiga-*` (`Rust Edition 2024`)**.

---

## 1. Refatoração Prática #1 (`C11` Bare-Metal): FSM Pura vs Loop Procedural com MMIO

### ❌ Antes: Procedural Acoplado (difícil de testar, risco de hang e race em `volatile`)
```c
static int g_state = 0;
static uint32_t g_start_ms = 0;

bool davis_poll_legacy(uint32_t now_ms) {
    volatile uint16_t *rst = (volatile uint16_t *)0x89000C20UL;
    volatile uint16_t *sts = (volatile uint16_t *)0x89000C00UL;
    if (g_state == 0) {
        *rst = 0x0000U;
        g_start_ms = now_ms;
        g_state = 1;
        return false;
    }
    if (g_state == 1) {
        if (now_ms >= g_start_ms + 1000U) { /* BUG 1: quebra no rollover u32! */
            g_state = 2;
        }
        return false;
    }
    if (g_state == 2) {
        if ((*sts & 0x0080U) == 0U && (*sts & 0x0040U) != 0U) { /* BUG 2: lê volatile 2x! */
            g_state = 3;
            return true;
        }
        if (now_ms >= g_start_ms + 5000U) {
            g_state = -1;
        }
    }
    return false;
}
```

### ✅ Depois: Núcleo Funcional C11 (`step(state, event) -> (next_state, effect)`) + Casca Fina
Toda regra vive em função pura (zero globais, zero `volatile`, zero heap); a casca imperativa tira **1 snapshot** do MMIO e aplica o efeito:

```c
#include <stdbool.h>
#include <stdint.h>

typedef enum {
    DAVIS_PHASE_IDLE = 0,
    DAVIS_PHASE_BRIDGE_WAIT,
    DAVIS_PHASE_POLL_DRDY,
    DAVIS_PHASE_READY,
    DAVIS_PHASE_TIMEOUT
} davis_phase_t;

typedef struct {
    davis_phase_t phase;
    uint32_t t0_ms;
} davis_fsm_t;

typedef struct {
    uint32_t now_ms;
    uint16_t status_snapshot; /* Snapshot único lido pela casca */
    bool start_req;
} davis_ev_t;

typedef enum {
    DAVIS_EFF_NONE = 0,
    DAVIS_EFF_ASSERT_CH1_RESET,
    DAVIS_EFF_COMPLETE_OK,
    DAVIS_EFF_REPORT_FAULT
} davis_effect_t;

typedef struct {
    davis_fsm_t next;
    davis_effect_t effect;
} davis_step_t;

static davis_step_t davis_fsm_step(davis_fsm_t st, davis_ev_t ev) {
    /* Subtração modular unsigned: imune a rollover de 32 bits */
    const uint32_t elapsed = (uint32_t)(ev.now_ms - st.t0_ms);

    switch (st.phase) {
    case DAVIS_PHASE_IDLE:
        if (!ev.start_req) {
            return (davis_step_t){ st, DAVIS_EFF_NONE };
        }
        return (davis_step_t){
            { DAVIS_PHASE_BRIDGE_WAIT, ev.now_ms },
            DAVIS_EFF_ASSERT_CH1_RESET
        };

    case DAVIS_PHASE_BRIDGE_WAIT:
        /* Espera obrigatória de 1000 ms para ponte StarTech IDE2SAT2 */
        if (elapsed < 1000U) {
            return (davis_step_t){ st, DAVIS_EFF_NONE };
        }
        return (davis_step_t){
            { DAVIS_PHASE_POLL_DRDY, st.t0_ms },
            DAVIS_EFF_NONE
        };

    case DAVIS_PHASE_POLL_DRDY: {
        const bool bsy = (ev.status_snapshot & 0x0080U) != 0U;
        const bool drdy = (ev.status_snapshot & 0x0040U) != 0U;
        if (!bsy && drdy) {
            return (davis_step_t){ { DAVIS_PHASE_READY, st.t0_ms }, DAVIS_EFF_COMPLETE_OK };
        }
        if (elapsed >= 5000U) {
            return (davis_step_t){ { DAVIS_PHASE_TIMEOUT, st.t0_ms }, DAVIS_EFF_REPORT_FAULT };
        }
        return (davis_step_t){ st, DAVIS_EFF_NONE };
    }

    case DAVIS_PHASE_READY:
    case DAVIS_PHASE_TIMEOUT:
    default:
        return (davis_step_t){ st, DAVIS_EFF_NONE };
    }
}
```
- **Ganho concreto:** Testável no host (`-DPANADIGA_OPENOS_HOST_TEST=1`) com rollover `now_ms = 0xFFFF_FC00 -> 0x0000_0200` sem hardware ligado; casca imperativa só executa `am34_mmio_write16` / `am34_mmio_read16` com barreiras de segurança.

---

## 2. Refatoração Prática #2 (`C11`): Table-Driven Dispatch & LUTs em `.rodata` vs Árvore `if/else`

Em firmwares restritos (`256 KiB P0_FW_RO` no Pico e `512 KiB SDRAM_STAGE15` no AM34), cadeias longas de `if/else if` ou `switch` de decodificação consomem instruções e ramificações difíceis de auditar.

### ❌ Antes: Cadeia Procedural Repetitiva
```c
int tv_mode_to_vmodesel(const char *name, uint16_t *out_reg) {
    if (strcmp(name, "ntsc") == 0) {
        *out_reg = 0x8020U;
        return 0;
    } else if (strcmp(name, "pal-m") == 0) {
        *out_reg = 0x8021U;
        return 0;
    } else if (strcmp(name, "pal") == 0) {
        *out_reg = 0x8027U;
        return 0;
    }
    return -1;
}
```

### ✅ Depois: Tabela Declarativa `static const` em `.rodata` + Functor/Lookup Puro
(Padrão já aplicado com sucesso em `firmware/pico-p7501/uart/p7501_diag_watch_codec.c` e `k_crc32_nibble_lut` em `p7501_crc_csum.c`.)

```c
typedef struct {
    const char *name;
    openos_tv_system_t sys;
    uint16_t vmodesel;
} tv_preset_row_t;

static const tv_preset_row_t k_tv_presets[] = {
    { "ntsc",  OPENOS_TV_NTSC,  0x8020U },
    { "pal-m", OPENOS_TV_PAL_M, 0x8021U },
    { "pal",   OPENOS_TV_PAL,   0x8027U },
};
_Static_assert((sizeof(k_tv_presets) / sizeof(k_tv_presets[0])) == 3u,
               "Tabela AIOF deve cobrir exatamente NTSC, PAL-M e PAL");

static const tv_preset_row_t *find_tv_preset(const char *name) {
    if (name == NULL) {
        return NULL;
    }
    for (size_t i = 0u; i < (sizeof(k_tv_presets) / sizeof(k_tv_presets[0])); ++i) {
        if (p7501_ascii_ieq(name, k_tv_presets[i].name)) {
            return &k_tv_presets[i];
        }
    }
    return NULL;
}
```
- **Regra:** Sempre que 3 ou mais ramos fazem a mesma estrutura (`compara chave -> devolve constante/ação`), mova os dados para `static const` em `.rodata` e deixe **1 único loop puro de busca**. Isso reduz tamanho de código, elimina divergência entre casos e zera clones `jscpd`.

---

## 3. Refatoração Prática #3 (`Rust` `#![deny(unsafe_code)]`): ADTs, Newtypes, Pipelines & Fatiamento de God-Modules

### A. Substituir Loops Mutáveis e `u32` Soltos por Newtypes + Combinadores
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddrSpan {
    lo: u64,
    hi: u64, // semiaberto [lo, hi)
}

impl AddrSpan {
    pub const fn from_start_len(start: u32, len: u32) -> Option<Self> {
        let lo = start as u64;
        let hi = lo + (len as u64); // sem wraparound u32!
        if len == 0 { None } else { Some(Self { lo, hi }) }
    }

    pub const fn overlaps(self, other: Self) -> bool {
        self.lo < other.hi && other.lo < self.hi
    }

    pub const fn contains(self, inner: Self) -> bool {
        self.lo <= inner.lo && inner.hi <= self.hi
    }
}

/// Decodifica entradas TOC de 48 B ou 64 B de forma pura, zero-copy e sem índices mutáveis.
pub fn valid_toc_entries(toc: &[u8], stride: usize) -> impl Iterator<Item = &[u8]> {
    toc.chunks_exact(stride)
        .take_while(|entry| entry[0] != 0x00 && entry[0] != 0xFF)
}
```

### B. Receita para Simplificar God-Modules Procedurais (`>450 LOC` em `crates/panadiga-core/src/re/`)
Quando um arquivo como `main_bgo_retarget.rs` ou `sata_bridge_stage2.rs` mistura dados, busca, patching e verificação:
1. **`spec.rs` (Dados Constantes):** `const` tabelas de assinaturas, offsets esperados e máscaras.
2. **`plan.rs` (Núcleo Puro):** `fn build_plan(image: &[u8], spec: &Spec) -> Result<Vec<PatchOp>>` — **somente leitura** sobre `&[u8]`, calculando todos os sítios e relocações sem mutar nenhum byte.
3. **`apply.rs` (Transformação CoW Determinística):** `fn apply_plan(image: &[u8], ops: &[PatchOp]) -> Result<Vec<u8>>` — aplica sobre cópia e valida pré-imagens antes de trocar bytes.
4. **`verify.rs` (Provas & Regressão):** testa invariantes algébricas, rejeições fail-closed e execução em `Mn103Vm`.

---

## 4. Os 8 Erros Comuns em FP Embarcado (`C11` & `Rust`) — Código Ruim vs Correto

| # | Erro Comum | ❌ Padrão Problemático | ✅ Correção Determinística |
| :-: | :--- | :--- | :--- |
| **1** | **Heap oculto em nome de imutabilidade** | `Vec::clone()` dentro de loop por byte/setor ou `malloc()` em `panadiga-openos` | Retorne *slices* `&[u8]`, `iter()`, `try_fold`; em C11 use buffers estáticos em `.bss` ou structs pequenas por valor |
| **2** | **Recursão em firmware sem TCO garantido** | Parser de árvores/LZSS recursivo no `AM34` ou `RP2040` (`-Os` pode não otimizar TCO) | Converta toda recursão em loop `for`/`while` com **teto máximo explícito** (`MAX_STEPS` / `SC0_SPIN_LIMIT`) |
| **3** | **Overflow `u32` em `addr + len`** | `if (addr + len <= 0x80040000U)` (`0x80050000 + 0xFFFF0000` dá wrap e burla guarda!) | Use `uint64_t end = (uint64_t)addr + (uint64_t)len;` ou `checked_add` / `AddrSpan` em `u64` |
| **4** | **Timer `now >= deadline` e UB *signed*** | `int32_t dt = t1 - t0;` ou `now >= start + wait` (overflow *signed* é UB em C11!) | Use exclusivamente subtração `uint32_t`: `(uint32_t)(now_ms - start_ms) >= wait_ms` |
| **5** | **Bit-fields C para MMIO ou headers** | `struct { uint32_t txbusy:1; } *reg;` (ordem de bits e largura variam por ABI/compilador) | Use máscaras e shifts explícitos sobre `uint16_t`/`uint32_t` + `load_le32`/`store_le32` portáveis |
| **6** | **Múltiplas leituras `volatile` na decisão** | `if ((*SC0STR & 0x80) && (*SC0STR & 0x20))` (estado muda entre leituras!) | Faça **1 única leitura** para `const uint16_t s = ...;` na casca e passe `s` por valor ao núcleo puro |
| **7** | **`float`/`double` ou `uint64_t /` em `-nostdlib`** | `double v = raw * 3.3 / 4096.0;` ou `u64_a / u64_b` puxando `libgcc` / `soft-float` | Use ponto fixo inteiro `((raw * 3300U) + 2048U) >> 12` e aritmética de 32 bits quando possível |
| **8** | **Refatoração *Big-Bang* sem prova diferencial** | Reescrever parser inteiro e apagar o antigo antes de comparar saídas | Mantenha teste diferencial `assert_eq!(pure_new(v), legacy(v))` cobrindo corpus + fuzz determinístico |
