ENTRY(_start)

SECTIONS
{
    . = 0x80000000;        
    .init   : { *(.init) } 
    .text   : { *(.text.*) }
    .rodata : { *(.rodata.*) }
    .data   : { *(.data.*) }
    _bss_start = .;
    .bss    : { *(.bss.*) *(COMMON) }
    _bss_end = .;
}
