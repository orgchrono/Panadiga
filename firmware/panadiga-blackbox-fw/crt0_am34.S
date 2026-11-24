/* Open-Source Hardware-Interop MN103S / AM34 DIAG_PROG_RAM SDRAM CRT0 (0x50400000) */
    .section .vectors, "ax", @progbits
    .global _start
    .extern _main
    .extern _bss_start
    .extern _bss_end
    .extern _stack_top

_start:
    /* Establish clean 4-byte aligned stack pointer */
    mov     _stack_top, a0
    mov     a0, sp

    /* Zero .bss cleanly without touching Flash or MMIO */
    mov     _bss_start, a0
    mov     _bss_end, a1
    clr     d0
.Lbss_loop:
    cmp     a1, a0
    bcc     .Lbss_done
    movbu   d0, (a0)
    inc     a0
    bra     .Lbss_loop

.Lbss_done:
    /* Enter C11 freestanding monitor (_main -> main) */
    call    _main, [], 0

.Lhalt:
    bra     .Lhalt
