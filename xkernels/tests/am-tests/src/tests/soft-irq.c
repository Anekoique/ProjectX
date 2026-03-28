#include "test.h"

static volatile int fired = 0;

static void handler(TrapFrame *tf) {
    check(tf->mcause == ((1UL << 63) | 3)); /* M-mode software */
    MSIP = 0;
    fired = 1;
}

void test_soft_irq(void) {
    init_trap((void (*)(TrapFrame *))handler);
    csrs(mie, 1 << 3);     /* MSIE */
    csrs(mstatus, 1 << 3); /* MIE */
    MSIP = 1;
    while (!fired)
        ;
    printf("soft-irq: OK\n");
}
