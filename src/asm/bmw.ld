/* link.ld */
ENTRY(_start)

SECTIONS
{
    . = 0x80000000;               /* QEMU -kernel loads here */

    .init   : { *(.init) }        /* start.S */
    .text   : { *(.text.*) }
    .rodata : { *(.rodata.*) }
    .data   : { *(.data.*) }

    _bss_start = .;
    .bss    : { *(.bss.*) *(COMMON) }
    _bss_end = .;
}
