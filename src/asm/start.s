    .section .init, "ax"
    .global _start
_start:
    li   sp, 0x80400000
    la   a0, __bss_start
    la   a1, __bss_end
1:  beq  a0, a1, 2f
    sd   zero, (a0)
    addi a0, a0, 8
    j    1b
2:
    call main
    ebreak
