# Matemática Determinística, Reconhecimento de Padrões & Provas Algébricas

Guia matemático e algorítmico para diagnósticos físicos (`PicoFW`), kernel/drivers (`DigaOS`), forense de dumps/storage (`MEIHFS`/`NOR`/`FRM`) e verificação de equivalência sem *flakiness* em `C11` e `Rust`.

---

## 1. Matemática Determinística Sem Float (Ponto Fixo, Modular & Bitwise)

### A. Conversão ADC, Ponto Fixo (`Q8.8` / `Q16.16`) e Filtro EMA
No `RP2040/RP2350` (ADC 12 bits, `VREF = 3300 mV`) e no `AM34` (sem FPU), todo cálculo analógico/temporal deve ser **inteiro e reproduzível bit a bit**:

```c
#include <stdbool.h>
#include <stdint.h>

/* Conversão ADC 12-bit -> milivolts com arredondamento simétrico (+2048 antes de >> 12) */
static inline uint32_t adc12_to_mv(uint16_t raw12) {
    return (((uint32_t)(raw12 & 0x0FFFu) * 3300u) + 2048u) >> 12u;
}

/* Filtro Média Móvel Exponencial (EMA) em Q8.8 com ganho alpha = 1 / 2^k (sem divisão!) */
static inline int32_t ema_q8_step(int32_t acc_q8, int32_t sample_mv, uint8_t shift_k) {
    const int32_t target_q8 = (int32_t)((uint32_t)sample_mv << 8u);
    const int32_t diff_q8 = target_q8 - acc_q8;
    /* Em C11 evite shift direito negativo: divida por potência de 2 mantendo sinal explícito */
    const int32_t delta = (diff_q8 >= 0) ? (diff_q8 >> shift_k) : -((-diff_q8) >> shift_k);
    return acc_q8 + delta;
}

/* Schmitt Trigger (histerese formal para VDD / GP26 SW_UART / detecção de Standby) */
typedef enum { RAIL_OFF = 0, RAIL_ON = 1 } rail_state_t;

static inline rail_state_t schmitt_rail_step(rail_state_t cur, uint32_t mv,
                                             uint32_t lo_th_mv, uint32_t hi_th_mv) {
    if ((cur == RAIL_OFF) && (mv >= hi_th_mv)) {
        return RAIL_ON;
    }
    if ((cur == RAIL_ON) && (mv <= lo_th_mv)) {
        return RAIL_OFF;
    }
    return cur; /* Banda morta [lo_th_mv, hi_th_mv] evita chattering no limiar */
}
```

### B. Checksums Algébricos & Operações Bitwise Seguras
- **Watchdog `IC7501 TBUS` (`tbus_ic7501.c`):** `csum = (uint8_t)((sum(body15[0..15]) - 0x5Bu) & 0xFFu)`. Exemplo `WDT 255s`: `0x8A + 0x02 + 0xFF - 0x5B = 0x130 -> 0x30`.
- **Adler-32 (`EXTRHEAD` / `MainList`):** `MOD = 65521`, acumule em `u32`/`u64` e só aplique `% 65521` a cada `NMAX = 5552` bytes (ou por passo em entradas curtas).
- **CRC-32 IEEE Nibble-LUT (`.rodata` compacta):** `16` entradas de `uint32_t` (`64 B` em Flash vs `1 KiB` da tabela de 256 entradas), processando 4 bits por passo.
- **Alinhamento & Potência de 2 (sem UB):**
```rust
pub const fn is_pow2_u64(x: u64) -> bool {
    x != 0 && (x & (x - 1)) == 0
}

pub const fn align_up_pow2(val: u64, align: u64) -> Option<u64> {
    if !is_pow2_u64(align) {
        return None;
    }
    let mask = align - 1;
    match val.checked_add(mask) {
        Some(v) => Some(v & !mask),
        None => None,
    }
}
```

---

## 2. Álgebra de Intervalos `[lo, hi)` para Memória Flash / SDRAM / LBA

Representar regiões Flash (`IPL RO`, `BGO`, `EEMUL`, `SDRAM`) como intervalos semiabertos `[lo, hi)` em `u64` elimina erros `off-by-one` (`4 MiB` vs `8 MiB`, `35` vs `45` setores) e impede bypass de segurança por overflow de `u32`:

| Lei / Predicado | Fórmula Exata (`lo <= hi` em `u64`) | Uso no Panadiga |
| :--- | :--- | :--- |
| **Interseção (`A ∩ B ≠ ∅`)** | `a.lo < b.hi && b.lo < a.hi` | Bloquear escrita/patch se tocar `SA0..SA10` ou `0xC000_100C..1010` |
| **Contenção (`B ⊆ A`)** | `a.lo <= b.lo && b.hi <= a.hi` | Exigir `patch_span ⊆ nor_layout.regions[bgo]` (`SA26..SA70`) |
| **Disjunção Estrita** | `a.hi <= b.lo \|\| b.hi <= a.lo` | Provar que `IPL`, `BGO` e `EEMUL` nunca se sobrepõem |
| **Partição Contígua Exata** | `∀i: r[i].hi == r[i+1].lo`, `r[0].lo == 0`, `r.last().hi == total` | Validar tabela completa de `142` setores `S29JL064H` (`8 MiB`) |

---

## 3. Reconhecimento de Padrões Estatístico & Estrutural (Dumps & Sinais)

### A. Assinaturas Estatísticas (Entropia de Shannon + Qui-Quadrado $\chi^2$)
Para uma janela de tamanho $W \ge 256$ bytes (ideal `4096 B` = 1 bloco NOR/cluster):
- **Entropia de Shannon:** $H = -\sum_{b=0}^{255} p_b \log_2 p_b \in [0, 8]$, onde $p_b = \text{count}[b] / W$.
- **Teste de Uniformidade $\chi^2$:** $\chi^2 = \sum_{b=0}^{255} \frac{(\text{count}[b] - E)^2}{E}$, com $E = W / 256$ e $df = 255$.
  - Em payload cifrado/comprimido autêntico de `4096 B` ($E = 16$), $\chi^2$ típico fica próximo de `200..320`; em bloco com viés estrutural ou tabela, $\chi^2 \gg 500$.

| Banda $H$ (bits/byte) | Diagnóstico Empírico Panadiga | Ação Recomendada |
| :---: | :--- | :--- |
| `0.00 .. 0.15` | Setor apagado (`0xFF`), zerado (`0x00`) ou mock sintético | Sinalizar setor vazio; auditar contra mock `0xFF` |
| `1.00 .. 4.50` | Strings ASCII, tabelas TOC/CLI `<8sIII>`, tabelas BGO/EEMUL `"1284"` | Extrair strings/tabelas estruturadas |
| `5.50 .. 7.10` | Código executável XIP (`MN103S` `Loader ver0.61`, `MAIN`, `DRV`) | Elegível para análise Ghidra e busca de S-box |
| `7.50 .. 7.95` | Stream comprimido (`zlib`, `gzip`, `Okumura LZSS`, `LZMA`) | Inspecionar headers `0x78`, `0x1F 0x8B`, `EXTRHEAD` |
| `>= 7.98` | Payload cifrado (`Feistel 64-bit` / `AES-128-CBC`) ou PRNG | Rodar ataque **ECB Codebook** (`panadiga frm codebook`) |

### B. 5 Detectores Estruturais Determinísticos (Implementação Direta)
1. **Detecção ECB 64-bit vs CBC:** Quebre o buffer em blocos `chunks_exact(8)`; se qualquer bloco de 8 bytes repetir múltiplas vezes em área de entropia `>= 7.98` (ex.: `03 50 65 C8 7B B8 E0 90` = `0xFF*8` em `PANA_DVD.FRM`), é cifra ECB com bloco de 8 B!
2. **Detecção de Espelhamento por Pino de Endereço Solto:** Para cada metade `half = len >> k` (`k = 1..5`), se `buf[..half] == buf[half..2*half]` (ou `>= 99.9%` igual), houve pino de endereço flutuante na leitura TSOP-48 (`N3/T3` bloqueado).
3. **Autocorrelação de Stride de Tabelas:** Teste $S \in \{16, 20, 48, 64\}$ medindo coincidência de tags/delimitadores em `buf[i]` e `buf[i + S]` (revela automaticamente `TBUS 16B`, CLI `<8sIII> 20B`, `sd48`, `sd64`).
4. **Fingerprinting de S-box Criptográfica (256 B):** Uma S-box real satisfaz **3 predicados simultâneos**:
   ```rust
   pub fn looks_like_crypto_sbox(win: &[u8]) -> bool {
       if win.len() != 256 {
           return false;
       }
       let mut seen = [false; 256];
       let mut identity_hits = 0usize;
       let mut xor_const_hits = 0usize;
       let c0 = win[0];
       for (i, &v) in win.iter().enumerate() {
           if seen[v as usize] {
               return false; // 1. Não é bijeção!
           }
           seen[v as usize] = true;
           if v as usize == i || v as usize == (255 - i) {
               identity_hits += 1;
           }
           if (v ^ (i as u8)) == c0 {
               xor_const_hits += 1;
           }
       }
       // 2. Rejeitar rampas/tabelas afins triviais (`i`, `255 - i`, `i ^ c`)
       identity_hits < 16 && xor_const_hits < 256
   }
   ```
5. **Detecção de Byte-Swap 16-bit (`XERTDAEH`):** Teste palavras de 2 bytes invertidas (`b[i] <-> b[i+1]`) contra assinatura conhecida (`"EXTRHEAD"` -> `"XERTDAEH"`, `"Loader"` -> `"oLdaer"`).

---

## 4. Provas por Leis Algébricas & Testes Diferenciais Determinísticos

Nunca dependa de aleatoriedade não-reprodutível nem teste apenas 1 vetor fixo. Use um PRNG determinístico de semente fixa (`splitmix64`) + **5 leis algébricas**:

1. **Round-Trip:** `decode(encode(x)) == x` (`EXTRHEAD`/`LZSS`/`TBUS`/`hex`).
2. **Idempotência:** `normalize(normalize(x)) == normalize(x)`.
3. **Conservação / Checksum:** alterar 1 bit qualquer no payload **deve** mudar CRC-32/Adler-32/SHA-256 (`hamming >= 1`).
4. **Monotonicidade & Disjunção de Intervalos:** após `merge_spans`, todo par resultante é estritamente disjunto e ordenado.
5. **Equivalência Diferencial (`novo_puro(x) == legado(x)`):**

```rust
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
fn verify_differential_and_roundtrip() {
    // 1. Matriz obrigatória de fronteiras físicas Panadiga
    let edges = [0x0000_0000u32, 0x8003_FFFF, 0x8004_0000, 0xC000_100B, 0xC000_100C, u32::MAX];
    for &x in &edges {
        assert_eq!(pure_new(x), legacy_proc(x), "Divergência na borda 0x{x:08X}");
    }
    // 2. 10.000 vetores determinísticos (mesma semente sempre)
    let mut rng = 0x5041_4E41_4449_4741u64;
    for _ in 0..10_000 {
        let v = splitmix64(&mut rng) as u32;
        assert_eq!(pure_new(v), legacy_proc(v));
    }
}
```

---

## 5. Os 6 Erros Comuns em Matemática & Reconhecimento de Padrões

| # | Erro Comum | Consequência Real | Como Evitar |
| :-: | :--- | :--- | :--- |
| **1** | **Entropia de Shannon em janela `< 256 B`** | Subestimação estrutural (com `16 B`, $H_{max} \le 4.0$, parecendo texto!) | Exija janela mínima $W \ge 256$ B (ideal `4096 B`) para $H \in [0, 8]$ |
| **2** | **Comparar `float` com `==` ou usar FPU sem necessidade** | Deriva entre host x86 e target ARM/MN103S; puxa `soft-float` | Calcule razões em ponto fixo inteiro e compare dentro de banda ou inteiros exatos |
| **3** | **Esquecer viés de arredondamento no shift direito** | `(raw * 3300) >> 12` trunca sempre para baixo (`-0.99 mV` de viés) | Some metade do divisor antes do shift: `+ (1u << (k - 1))` |
| **4** | **Detectar S-box só testando bijeção** | Tabelas lineares (`0x00..0xFF`), contadores e ASCII passam como S-box | Exija bijeção + rejeição afim (`i`, `255 - i`, `i ^ c`) + análise de não-linearidade |
| **5** | **Shift `>= width` ou shift negativo em C11** | Comportamento Indefinido (UB) — compilador `-Os` pode eliminar guardas! | Valide `k < 32u` e nunca faça `neg >> k` nem `val << 32` |
| **6** | **Testar apenas *happy path* sem bordas/rollover** | Bugs ocultos em `len = 0`, `u32::MAX`, `0x8003_FFFF/0x8004_0000` e `0xC000_100C` | Inclua sempre matriz de fronteiras + `splitmix64` com semente fixa |
