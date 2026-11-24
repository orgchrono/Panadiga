/*
 * Panadiga OpenOS CRT0 — Matsushita AM34 / MN103S Bare-Metal Startup
 * EXPERIMENTAL HARDWARE-INTEROPERABILITY OS (NOT LEGALLY CLEAN-ROOM).
 *
 * Sets stack pointer in SDRAM, zeroes .bss, and invokes openos_kernel_entry().
 */

    .section .text.startup, "ax", @progbits
    .global _start
    .type   _start, @function

_start:
    mov     __stack_top, a0
    mov     a0, sp

    mov     __bss_start, a0
    mov     __bss_end, a1
    clr     d0
.Lzero_bss:
    cmp     a1, a0
    bge     .Lbss_done
    movbu   d0, (a0)
    inc     a0
    bra     .Lzero_bss

.Lbss_done:
    clr     d0
    call    openos_kernel_entry, [], 0x00

.Lhalt:
    bra     .Lhalt
    .size   _start, . - _start
