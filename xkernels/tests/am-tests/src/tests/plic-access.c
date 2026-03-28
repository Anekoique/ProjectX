#include "test.h"

void test_plic_access(void) {
    PLIC_PRI(10) = 5;
    check(PLIC_PRI(10) == 5);

    PLIC_EN(0) = 1 << 10;
    check(PLIC_EN(0) == (1u << 10));

    PLIC_THR(0) = 3;
    check(PLIC_THR(0) == 3);

    check(PLIC_CLM(0) == 0);

    printf("plic-access: OK\n");
}
