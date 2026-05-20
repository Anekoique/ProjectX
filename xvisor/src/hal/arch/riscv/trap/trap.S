// HS-mode trap entry. Installed in `stvec` by `boot.rs`.
//
// On entry the hardware delivers the trap from HS-mode; we land here in
// HS-mode with all GPRs holding the trap-time values. We allocate a 288-byte
// TrapFrame on the current kernel stack, save x1..x31 and the CSR shadows,
// dispatch into Rust via `trap_handler(a0 = &mut TrapFrame)`, restore the
// sepc and sstatus CSRs and x1..x31, then `sret`.
//
// Field layout (must match `TrapFrame` in `mod.rs`; the `offset_of!` static
// asserts there pin the layout at build time):
//
//   regs[0]   ←  0(sp)   x0 slot (stored zero so frame.regs[0] is sound)
//   regs[1]   ←  8(sp)   x1 (ra)
//   regs[2]   ← 16(sp)   x2 (sp at trap entry)
//   ...
//   regs[31]  ←248(sp)   x31
//   sepc      ←256(sp)
//   scause    ←264(sp)
//   stval     ←272(sp)
//   sstatus   ←280(sp)

.section .text
.globl trap_entry
.balign 4
trap_entry:
    addi    sp, sp, -288

    // x0 slot stored zero so frame.regs[0] is always sound.
    sd      zero,   0(sp)

    // x1..x31 → regs[1..32] at 8(sp) .. 248(sp).
    sd      x1,     8(sp)
    sd      x2,    16(sp)
    sd      x3,    24(sp)
    sd      x4,    32(sp)
    sd      x5,    40(sp)
    sd      x6,    48(sp)
    sd      x7,    56(sp)
    sd      x8,    64(sp)
    sd      x9,    72(sp)
    sd      x10,   80(sp)
    sd      x11,   88(sp)
    sd      x12,   96(sp)
    sd      x13,  104(sp)
    sd      x14,  112(sp)
    sd      x15,  120(sp)
    sd      x16,  128(sp)
    sd      x17,  136(sp)
    sd      x18,  144(sp)
    sd      x19,  152(sp)
    sd      x20,  160(sp)
    sd      x21,  168(sp)
    sd      x22,  176(sp)
    sd      x23,  184(sp)
    sd      x24,  192(sp)
    sd      x25,  200(sp)
    sd      x26,  208(sp)
    sd      x27,  216(sp)
    sd      x28,  224(sp)
    sd      x29,  232(sp)
    sd      x30,  240(sp)
    sd      x31,  248(sp)

    // CSR shadows.
    csrr    t0, sepc
    sd      t0,   256(sp)
    csrr    t0, scause
    sd      t0,   264(sp)
    csrr    t0, stval
    sd      t0,   272(sp)
    csrr    t0, sstatus
    sd      t0,   280(sp)

    // a0 = &mut TrapFrame.
    mv      a0, sp
    call    trap_handler

    // Restore sepc and sstatus (scause / stval are HW inputs only).
    ld      t0,   256(sp)
    csrw    sepc, t0
    ld      t0,   280(sp)
    csrw    sstatus, t0

    // Restore x1..x31 (skip x0; it stays hardwired zero in HW).
    ld      x1,     8(sp)
    ld      x2,    16(sp)
    ld      x3,    24(sp)
    ld      x4,    32(sp)
    ld      x5,    40(sp)
    ld      x6,    48(sp)
    ld      x7,    56(sp)
    ld      x8,    64(sp)
    ld      x9,    72(sp)
    ld      x10,   80(sp)
    ld      x11,   88(sp)
    ld      x12,   96(sp)
    ld      x13,  104(sp)
    ld      x14,  112(sp)
    ld      x15,  120(sp)
    ld      x16,  128(sp)
    ld      x17,  136(sp)
    ld      x18,  144(sp)
    ld      x19,  152(sp)
    ld      x20,  160(sp)
    ld      x21,  168(sp)
    ld      x22,  176(sp)
    ld      x23,  184(sp)
    ld      x24,  192(sp)
    ld      x25,  200(sp)
    ld      x26,  208(sp)
    ld      x27,  216(sp)
    ld      x28,  224(sp)
    ld      x29,  232(sp)
    ld      x30,  240(sp)
    ld      x31,  248(sp)

    addi    sp, sp, 288
    sret
