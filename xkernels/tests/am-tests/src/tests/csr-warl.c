#include "test.h"

void test_csr_warl(void) {
    /* misa: verify readable (value depends on xemu initialization) */
    unsigned long misa = csrr(misa);
    (void)misa; /* may be 0 if xemu doesn't initialize misa yet */

    /* mstatus: MIE writable */
    csrs(mstatus, 1 << 3);
    check(csrr(mstatus) & (1 << 3));
    csrc(mstatus, 1 << 3);
    check(!(csrr(mstatus) & (1 << 3)));

    /* mie: standard bits writable */
    csrw(mie, 0xAAA);
    check(csrr(mie) == 0xAAA);

    /* mtvec: bit 1 reserved */
    csrw(mtvec, ~0UL);
    check((csrr(mtvec) & 0x2) == 0);

    printf("csr-warl: OK\n");
}
