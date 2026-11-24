/*
 * Panadiga OpenOS Linker Script — Matsushita AM34 / MN103S SDRAM Targets
 * EXPERIMENTAL HARDWARE-INTEROPERABILITY OS (NOT LEGALLY CLEAN-ROOM).
 *
 * Strictly excludes Flash IPL SA0..SA10 (0x80000000..0x8003FFFF / 0x84000000..0x8403FFFF).
 */

OUTPUT_ARCH(mn10300)
ENTRY(_start)

MEMORY
{
    SDRAM_PROG    (rwx) : ORIGIN = 0x50400000, LENGTH = 512K
    SDRAM_MAIN    (rwx) : ORIGIN = 0x57000000, LENGTH = 16M
}

SECTIONS
{
    . = 0x50400000;

    .text : ALIGN(4)
    {
        KEEP(*(.text.startup))
        *(.text*)
        *(.rodata*)
    } > SDRAM_PROG

    .data : ALIGN(4)
    {
        *(.data*)
    } > SDRAM_PROG

    .bss (NOLOAD) : ALIGN(4)
    {
        __bss_start = .;
        *(.bss*)
        *(COMMON)
        . = ALIGN(4);
        __bss_end = .;
    } > SDRAM_PROG

    . = ORIGIN(SDRAM_PROG) + LENGTH(SDRAM_PROG);
    __stack_top = 0x5047F000;

    /DISCARD/ :
    {
        *(.comment)
        *(.note*)
    }
}
