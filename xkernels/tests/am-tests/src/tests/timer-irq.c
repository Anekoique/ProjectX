#include "test.h"

static volatile int fired = 0;

static void handler(TrapFrame *tf) {
    check(tf->mcause == ((1UL << 63) | 7)); /* M-mode timer */
    set_mtimecmp(~0ULL);                    /* disarm */
    fired = 1;
}

void test_timer_irq(void) {
    init_trap((void (*)(TrapFrame *))handler);
    csrs(mie, 1 << 7);     /* MTIE */
    csrs(mstatus, 1 << 3); /* MIE */
    /* Set mtimecmp AFTER enabling interrupts to avoid race */
    set_mtimecmp(mtime() + 100000);
    while (!fired)
        ;
    printf("timer-irq: OK\n");
}
