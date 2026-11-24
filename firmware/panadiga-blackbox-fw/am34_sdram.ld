/* Open-Source Hardware-Interop GNU LD Script for Panasonic DIGA AM34 / MN103S DIAG_PROG_RAM SDRAM (0x50400000) */
ENTRY(_start)

MEMORY
{
    /* DIAG_PROG_RAM execution window inside cached SDRAM (256 KiB) */
    PROG_SDRAM (rwx) : ORIGIN = 0x50400000, LENGTH = 0x00040000
    /* Dedicated stack tail region inside upper PROG_SDRAM slot */
    STACK_RAM  (rw)  : ORIGIN = 0x5043C000, LENGTH = 0x00004000
}

SECTIONS
{
    .text 0x50400000 : ALIGN(4)
    {
        KEEP(*(.vectors))
        KEEP(*(.init))
        *(.text .text.*)
        *(.rodata .rodata.*)
        . = ALIGN(4);
    } > PROG_SDRAM

    .data : ALIGN(4)
    {
        _data_start = .;
        *(.data .data.*)
        . = ALIGN(4);
        _data_end = .;
    } > PROG_SDRAM

    .bss (NOLOAD) : ALIGN(4)
    {
        _bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(4);
        _bss_end = .;
    } > PROG_SDRAM

    _stack_top = ORIGIN(STACK_RAM) + LENGTH(STACK_RAM) - 16;

    /DISCARD/ :
    {
        *(.comment)
        *(.note*)
        *(.eh_frame*)
    }
}
