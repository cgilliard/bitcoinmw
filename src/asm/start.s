    .section .init, "ax"          # executable, allocated
    .global _start
_start:
    li   sp, 0x80400000           # 0x8000_0000 + 2 MiB
    la   a0, _bss_start
    la   a1, _bss_end
1:  beq  a0, a1, 2f
    sd   zero, (a0)
    addi a0, a0, 8
    j    1b
2:
    call main 
    ebreak   
