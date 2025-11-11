.section .text
.global _start
_start:
    li sp, 0x80400000
    call main
    ebreak
