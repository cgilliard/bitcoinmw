ENTRY(_start)

SECTIONS
{
    . = 0x80000000;        
    .init   : { *(.init) } 
    .text   : { *(.text.*) }
    .rodata : { *(.rodata.*) }
    .data   : { *(.data.*) }

    . = 0x80200000;
    __bss_start = .;
    .bss    : { *(.bss.*) *(COMMON) }
    __bss_end = .;
}
