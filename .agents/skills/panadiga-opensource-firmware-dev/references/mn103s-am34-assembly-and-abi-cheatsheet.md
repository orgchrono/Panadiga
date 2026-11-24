# Cheatsheet de Assembly & ABI `MN103S / AM34` (`mn10300-elf-gcc` & Stubs)

Referência prática verificada diretamente no emulador Rust (`crates/panadiga-core/src/re/mn10300_emu/`), no emissor (`mn10300_emit.rs`), no decoder (`mn10300/mod.rs`), no patch SLEIGH (`tools/ghidra/sleigh_patches/am33_panadiga_zero_undef.patch`) e no `firmware/panadiga-openos/boot/crt0_am34.S`.

---

## 1. Arquitetura de Registradores (`MN103S` Base + Extensão `AM33/AM34`)

- **Base `MN103`:** `D0..D3` (dados 32-bit), `A0..A3` (endereços 32-bit), `SP` (stack pointer), `PC`, `MDR` (Machine Data / return register para `CALLS`/`RETF`), `LIR` / `LAR` (hardware zero-overhead loop `SETLB`..`Lcc`), `PSW` (16-bit).
- **Extensão `AM33 / AM34`:** `R0..R7` (também nomeados `E0..E7` nos manuais/disassemblers), `MDRQ`, `MCRH`, `MCRL`, `MCVF`, `EPSW` (32-bit).
- **Indexação de 4 bits nos opcodes `AM33` (`read_reg16` em `mn10300_emu/mod.rs`):**
  - `0..=7` -> `R0..R7` (`E0..E7`)
  - `8..=11` -> `A0..A3`
  - `12..=15` -> `D0..D3`

---

## 2. `PSW` / `EPSW`, Flags ALU & Controle de Interrupções (`CLI` / `STI`)

- **Bits verificados no `PSW` (`mn10300_emu/mod.rs`):**
  - `PSW_V = 0x0001` (bit 0 — Overflow)
  - `PSW_C = 0x0002` (bit 1 — Carry / Borrow)
  - `PSW_N = 0x0004` (bit 2 — Negative)
  - `PSW_Z = 0x0008` (bit 3 — Zero)
  - `PSW_RESERVED_MASK = 0xC0F0` (bits `4..7` e `14..15` reservados; escrita falha em `Mn103Vm` com `ReservedPswBits`).
- **Instruções de escrita em `PSW` / `EPSW` (`ops_ext.rs` + `am33_panadiga_zero_undef.patch`):**
  - `AND imm16, PSW`: `0xFA 0xFC <u16_le>` · `OR imm16, PSW`: `0xFA 0xFD <u16_le>`
  - `AND imm32, EPSW`: `0xFC 0xFC <u32_le>` · `OR imm32, EPSW`: `0xFC 0xFD <u32_le>`
  *(Nota: no SLEIGH stock do Ghidra antes do nosso patch, `0xFC 0xFC` fazia `|` em vez de `&`!)*
- **Aviso Epistêmico Obrigatório (Antes de IRQs / CFI NOR em Silício):** O `Mn103Vm` não modela interrupções nem `RTI` (`Mn103Fault::InvalidOpcode`). Antes de implementar `CLI`/`STI`, `RTI` ou remapeamento da tabela de vetores em SDRAM no `Panadiga OpenOS` (obrigatório antes de tirar a NOR `S29JL064H` de `Read Array`), confirme no disassembly real de `MAIN` / `Loader ver0.61` os bits exatos de máscara/nível de IRQ em `PSW`/`EPSW` (`0x0F00`) e o registrador/endereço base real dos vetores no `AM34`.

---

## 3. Convenção de Chamada (`ABI` Real Panasonic + `mn10300-elf-gcc`) & Máscara `MOVM`

- **Regra PC-Relativa (`mn10300/mod.rs`):** Todo desvio ou chamada relativa calcula `target_va = instruction_va + signed_disp` a partir do endereço da **própria instrução** (NÃO da instrução seguinte).
- **Convenção Verificada (`1.386/1.386` em `PROG061`, `1.065/1.065` em `BOOT.BIN`):**
  - **Callers** emitem `CALL target, [], 0x00` (`imm8 = 0`, escreve `PC + insn_len` em `(SP)` e `MDR`) ou `CALLS` (`0xFA 0xFF` / `0xFC 0xFF`, escreve em `(SP)` e `MDR`).
  - **Leaf / Callees** alocam pilha explicitamente com `ADD -N, SP` (`0xF8 0xFE <s8>`) / `ADD +N, SP` e retornam com `RET [], 0x00` (`0xDF 0x00 0x00`) — preservando o endereço de retorno do caller em `(N, SP)`.
  - **Frame com callee-saved (`PROG061 @ 0x50415CAD`):** `MOVM [D2, D3, A2, A3], (SP)` (`0xCF 0xF0`), `ADD -16, SP` ... `RET [D2, D3, A2, A3], 0x20` (`0xDF 0xF0 0x20`, somando `16 + 16 = 32 = 0x20` para restaurar `MOVM` e retornar).
  - **Retornos:** `RET <regs>, <imm8>` (`0xDF`) lê o PC de retorno de `(new_sp)`; `RETF <regs>, <imm8>` (`0xDE`) salta via `MDR`; `RETS` (`0xF0 0xFC`) retorna de `CALLS` lendo `(SP)`.
- **Papel dos Registradores:** `D0`/`D1` argumentos e retorno inteiro (caller-saved), `A0`/`A1` ponteiros caller-saved, `D2`/`D3`/`A2`/`A3` callee-saved (além de `exreg0`/`exreg1` no AM33).
- **Bits exatos da máscara `regs_mask` (`MOVM` `0xCF`/`0xCE`, `CALL`, `RET`, `RETF` — `mn10300_emu/memory.rs`):**

| Bit | Grupo | Registradores Salvos (ordem decrescente de endereço em pilha) | Bytes |
| :-: | :--- | :--- | :-: |
| `0x80` | `d2` | `D2` | `4 B` |
| `0x40` | `d3` | `D3` | `4 B` |
| `0x20` | `a2` | `A2` | `4 B` |
| `0x10` | `a3` | `A3` | `4 B` |
| `0x08` | `other` | `D0, D1, A0, A1, MDR, LIR, LAR, pad` | `32 B` |
| `0x04` | `exreg0` | `R2, R3` | `8 B` |
| `0x02` | `exreg1` | `R4, R5, R6, R7` | `16 B` |
| `0x01` | `exother` | `R0, R1, MDRQ, MCRH, MCRL, MCVF` | `24 B` |

---

## 4. Tabela de Bolso de Opcodes & Relocações (`Little-Endian`)

### Branches Condicionais (`0xC0..=0xC9 <s8>`) & Fluxo de Controle
| Opcode | Mnemônico | Condição (`PSW`) | Opcode | Mnemônico / Encoding | Semântica |
| :---: | :--- | :--- | :---: | :--- | :--- |
| `0xC0` | `blt <s8>` | `N ^ V` (signed `<`) | `0xCA` | `bra <s8>` | Salto relativo 8-bit |
| `0xC1` | `bgt <s8>` | `!(Z \| (N ^ V))` (`>`) | `0xCB` | `nop` | 1 byte no-op |
| `0xC2` | `bge <s8>` | `!(N ^ V)` (`>=`) | `0xCC` | `jmp <s16>` | Salto relativo 16-bit (`3 B`) |
| `0xC3` | `ble <s8>` | `Z \| (N ^ V)` (`<=`) | `0xDC` | `jmp <s32>` | Salto relativo 32-bit (`5 B`) |
| `0xC4` | `bcs <s8>` | `C` (unsigned `<`) | `0xCD` | `call <s16>, <regs>, <u8>` | Chamada 16-bit (`5 B`) |
| `0xC5` | `bhi <s8>` | `!(C \| Z)` (unsigned `>`) | `0xDD` | `call <s32>, <regs>, <u8>` | Chamada 32-bit (`7 B`) |
| `0xC6` | `bcc <s8>` | `!C` (unsigned `>=`) | `0xF0 0xF0..F3` | `calls (An)` | Chamada indireta (`MDR`) |
| `0xC7` | `bls <s8>` | `C \| Z` (unsigned `<=`) | `0xF0 0xF4..F7` | `jmp (An)` | Salto indireto |
| `0xC8` | `beq <s8>` | `Z` (`==`) | `0xDB <u32>` | `setlb` + `0xD0..=0xDA` `lcc`/`lra` | Loop zero-overhead (`PC = LAR - 4`) |
| `0xC9` | `bne <s8>` | `!Z` (`!=`) | `0xDF`/`0xDE`/`0xF0 0xFC` | `ret` / `retf` / `rets` | Retorno (`(SP)` / `MDR` / `CALLS`) |

### Movimentação, Imediatos, ALU, Bitops & Sistema (`mn10300_emit.rs` + `step_core.rs` + `ops_ext.rs`)
- **Zerar / Extensão / Shift rápido:** `0x00/04/08/0C` `clr Dm`; `0x10..13` `extb Dn`; `0x14..17` `extbu Dn`; `0x18..1B` `exth Dn`; `0x1C..1F` `exthu Dn`; `0x54..57` `asl2 Dn` (shift left 2).
- **Aritmética de ponteiro e pilha (Atenção às flags!):**
  - `0x20..23 <s8>` `add s8, An` e `0x28..2B <s8>` `add s8, Dn` (atualizam `PSW`).
  - `0x40/44/48/4C` `inc Dn` (**atualiza `PSW`**); já `0x41/45/49/4D` `inc An` e `0x50..53` `inc4 An` (**NÃO alteram `PSW`** — preservam flags de `CMP` anterior!).
  - `0xF8 0xFE <s8>` `add s8, SP` · `0xFC 0xFE <s32>` `add s32, SP`.
- **Transferências Comuns & Imediatos 32-bit (`0xFC`):**
  - `0x60..6F` `mov Dm, (An)` · `0x70..7F` `mov (An), Dm` · `0x80..8F` `mov Dm, Dn` (`m != n`) / `mov s8, Dn` (`m == n`)
  - `0xF0 0x40..4F` `movbu (An), Dm` · `0xF0 0x50..5F` `movbu Dm, (An)` · `0xF0 0x60..6F` `movhu (An), Dm` · `0xF0 0x70..7F` `movhu Dm, (An)`
  - `0xF1 0xD0..DF` `mov Am, Dn` · `0xF1 0xE0..EF` `mov Dm, An`
  - `0xF9 0xEA <reg>` `movhu (Rm0+), Rn2` · `0xF9 0xFA <reg>` `movhu Rm2, (Rn0+)` (pós-incremento `+2`)
  - `0xFC 0xCC..CF <u32>` `mov imm32, Dn` · `0xFC 0xDC..DF <u32>` `mov imm32, An`
  - `0xFC 0x81/82/83 <abs32>` `mov D0, (abs32)` (`u32` / `u8` / `u16`) · `0xFC 0xA4/A8/AC <abs32>` `mov (abs32), D0` (`u32` / `u8` / `u16`)
  - `0xFC 0xE0..E3 <u32>` `and imm32, Dn` · `0xF0 0x80/90` `bset`/`bclr Dm, (An)` · `0xFE 0x00/01/02 <abs32> <u8>` `bset`/`bclr`/`btst imm8, (abs32)` · `0xF0 0xC0`/`0xEx` `syscall` · `0xFF` `break`.

---

## 5. Padrões GNU `as` (`mn10300-elf`) & Armadilhas de Escrita Manual

1. **Startup Bare-Metal (`firmware/panadiga-openos/boot/crt0_am34.S`):**
   ```asm
       mov     __stack_top, a0
       mov     a0, sp
       /* Zerar .bss byte a byte com movbu d0, (a0); inc a0 */
       clr     d0
       call    openos_kernel_entry, [], 0x00
   ```
2. **Alinhamento Estrito em Memória:** Acessos 16-bit (`MOVHU`) exigem `addr & 1 == 0`; acessos 32-bit (`MOV` word) exigem `addr & 3 == 0` (`Mn103Fault::AlignError`).
3. **Largura Exata em `SC0` UART0 (`0xD400_2008`/`2009`):** Use `movbu` (8-bit) ao escrever `SC0TXB` (`0xD400_2008`) — escrita 16/32-bit invade `SC0RXB` (`0xD400_2009`).
4. **Símbolo C (`main` vs `_main`):** Confira com `mn10300-elf-nm` se a toolchain prefixa símbolos com `_` antes de chamar funções C a partir de `.S`.
5. **Emissão em Rust & Validação Obrigatória:** Ao gerar machine code a partir do Rust, reutilize os helpers SSOT de `crates/panadiga-core/src/re/mn10300_emit.rs` (nunca duplique emitters) e valide todo stub/patch no emulador (`cargo run -q -p panadiga-cli -- re verify-patches --json`).
