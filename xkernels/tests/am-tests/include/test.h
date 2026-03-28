#ifndef TEST_H
#define TEST_H

#include <stdio.h>

extern void halt(int code);

#define check(cond)                                      \
    do {                                                 \
        if (!(cond)) {                                   \
            printf("FAIL: %s:%d\n", __FILE__, __LINE__); \
            halt(1);                                     \
        }                                                \
    } while (0)

/* ── MMIO ── */
#define REG32(a) (*(volatile unsigned int *)(a))
#define REG8(a)  (*(volatile unsigned char *)(a))

/* ── ACLINT (0x0200_0000) ── */
#define ACLINT 0x02000000UL
#define MSIP   REG32(ACLINT + 0x0000)

/* ── PLIC (0x0C00_0000) ── */
#define PLIC        0x0C000000UL
#define PLIC_PRI(s) REG32(PLIC + (s) * 4)
#define PLIC_EN(c)  REG32(PLIC + 0x2000 + (c) * 0x80)
#define PLIC_THR(c) REG32(PLIC + 0x200000 + (c) * 0x1000)
#define PLIC_CLM(c) REG32(PLIC + 0x200004 + (c) * 0x1000)

/* ── CSR ── */
#define csrr(c)                                   \
    ({                                            \
        unsigned long __v;                        \
        asm volatile("csrr %0, " #c : "=r"(__v)); \
        __v;                                      \
    })
#define csrw(c, v) asm volatile("csrw " #c ", %0" ::"r"((unsigned long)(v)))
#define csrs(c, v) asm volatile("csrs " #c ", %0" ::"r"((unsigned long)(v)))
#define csrc(c, v) asm volatile("csrc " #c ", %0" ::"r"((unsigned long)(v)))

/* ── TrapFrame (matches xam trap.rs layout) ── */
typedef struct {
    unsigned long zero, ra, sp, gp, tp;
    unsigned long t0, t1, t2;
    unsigned long s0, s1;
    unsigned long a0, a1, a2, a3, a4, a5, a6, a7;
    unsigned long s2, s3, s4, s5, s6, s7, s8, s9, s10, s11;
    unsigned long t3, t4, t5, t6;
    unsigned long mepc;
    unsigned long mcause;
} TrapFrame;

/* ── xam HAL ── */
extern unsigned long long mtime(void);
extern void set_mtimecmp(unsigned long long val);
extern void init_trap(void (*handler)(TrapFrame *));

#endif
