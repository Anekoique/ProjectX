#include "test.h"

static volatile int fired = 0;

static void handler(TrapFrame *tf) {
    check(tf->mcause == 11); /* EcallFromM */
    tf->mepc += 4;
    fired = 1;
}

void test_trap_ecall(void) {
    init_trap((void (*)(TrapFrame *))handler);
    asm volatile("ecall");
    check(fired);
    printf("trap-ecall: OK\n");
}
