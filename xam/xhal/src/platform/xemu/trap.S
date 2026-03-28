.equ XLENB, 8
.equ FRAME_SIZE, 34 * XLENB

.align 4
.globl __am_trap_entry
__am_trap_entry:
    addi sp, sp, -FRAME_SIZE
    sd   x5,  5*XLENB(sp)          # save t0 FIRST (before scratch use)
    sd   x0,  0*XLENB(sp)          # zero = 0
    sd   x1,  1*XLENB(sp)          # ra
    addi x5, sp, FRAME_SIZE
    sd   x5,  2*XLENB(sp)          # sp = original (via t0 scratch)
    sd   x3,  3*XLENB(sp)          # gp
    sd   x4,  4*XLENB(sp)          # tp
    # x5/t0 already saved at slot 5
    sd   x6,  6*XLENB(sp)          # t1
    sd   x7,  7*XLENB(sp)          # t2
    sd   x8,  8*XLENB(sp)          # s0
    sd   x9,  9*XLENB(sp)          # s1
    sd   x10, 10*XLENB(sp)         # a0
    sd   x11, 11*XLENB(sp)         # a1
    sd   x12, 12*XLENB(sp)         # a2
    sd   x13, 13*XLENB(sp)         # a3
    sd   x14, 14*XLENB(sp)         # a4
    sd   x15, 15*XLENB(sp)         # a5
    sd   x16, 16*XLENB(sp)         # a6
    sd   x17, 17*XLENB(sp)         # a7
    sd   x18, 18*XLENB(sp)         # s2
    sd   x19, 19*XLENB(sp)         # s3
    sd   x20, 20*XLENB(sp)         # s4
    sd   x21, 21*XLENB(sp)         # s5
    sd   x22, 22*XLENB(sp)         # s6
    sd   x23, 23*XLENB(sp)         # s7
    sd   x24, 24*XLENB(sp)         # s8
    sd   x25, 25*XLENB(sp)         # s9
    sd   x26, 26*XLENB(sp)         # s10
    sd   x27, 27*XLENB(sp)         # s11
    sd   x28, 28*XLENB(sp)         # t3
    sd   x29, 29*XLENB(sp)         # t4
    sd   x30, 30*XLENB(sp)         # t5
    sd   x31, 31*XLENB(sp)         # t6
    csrr x5, mepc
    sd   x5, 32*XLENB(sp)          # mepc
    csrr x5, mcause
    sd   x5, 33*XLENB(sp)          # mcause
    mv   a0, sp
    call __trap_dispatch
    ld   x5, 32*XLENB(sp)
    csrw mepc, x5
    ld   x1,  1*XLENB(sp)          # ra
    ld   x3,  3*XLENB(sp)          # gp
    ld   x4,  4*XLENB(sp)          # tp
    ld   x5,  5*XLENB(sp)          # t0 (original)
    ld   x6,  6*XLENB(sp)          # t1
    ld   x7,  7*XLENB(sp)          # t2
    ld   x8,  8*XLENB(sp)          # s0
    ld   x9,  9*XLENB(sp)          # s1
    ld   x10, 10*XLENB(sp)         # a0
    ld   x11, 11*XLENB(sp)         # a1
    ld   x12, 12*XLENB(sp)         # a2
    ld   x13, 13*XLENB(sp)         # a3
    ld   x14, 14*XLENB(sp)         # a4
    ld   x15, 15*XLENB(sp)         # a5
    ld   x16, 16*XLENB(sp)         # a6
    ld   x17, 17*XLENB(sp)         # a7
    ld   x18, 18*XLENB(sp)         # s2
    ld   x19, 19*XLENB(sp)         # s3
    ld   x20, 20*XLENB(sp)         # s4
    ld   x21, 21*XLENB(sp)         # s5
    ld   x22, 22*XLENB(sp)         # s6
    ld   x23, 23*XLENB(sp)         # s7
    ld   x24, 24*XLENB(sp)         # s8
    ld   x25, 25*XLENB(sp)         # s9
    ld   x26, 26*XLENB(sp)         # s10
    ld   x27, 27*XLENB(sp)         # s11
    ld   x28, 28*XLENB(sp)         # t3
    ld   x29, 29*XLENB(sp)         # t4
    ld   x30, 30*XLENB(sp)         # t5
    ld   x31, 31*XLENB(sp)         # t6
    addi sp, sp, FRAME_SIZE
    mret
