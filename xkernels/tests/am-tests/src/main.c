#include "amtest.h"
#include <stdio.h>

/* Test selector: -DMAINARGS=x picks test by key.
 * make run          — run all tests
 * make run TEST=t   — run one test by key
 */

#ifndef MAINARGS
#define MAINARGS ""
#endif
const char mainargs[] = MAINARGS;

static const char *descriptions[256] = {
    ['u'] = "uart-putc:   UART MMIO output",
    ['r'] = "timer-read:  ACLINT mtime read",
    ['t'] = "timer-irq:   M-mode timer interrupt",
    ['s'] = "soft-irq:    M-mode software interrupt",
    ['p'] = "plic-access: PLIC register r/w",
    ['c'] = "csr-warl:    CSR WARL masks",
    ['e'] = "trap-ecall:  ecall roundtrip",
    ['R'] = "rtc:         clock accuracy (interactive)",
    ['k'] = "keyboard:    UART RX echo (interactive)",
    ['f'] = "float:       F/D floating-point",
    ['a'] = "Run all tests",
    ['h'] = "Show this help",
};

#define CASE(ch, func) \
    case ch:           \
        func();        \
        break

int main(const char *args) {
    switch (args[0]) {
        CASE('u', test_uart_putc);
        CASE('r', test_timer_read);
        CASE('t', test_timer_irq);
        CASE('s', test_soft_irq);
        CASE('p', test_plic_access);
        CASE('c', test_csr_warl);
        CASE('e', test_trap_ecall);
        CASE('R', test_rtc);
        CASE('k', test_keyboard);
        CASE('f', test_float);
    case 'a':
        printf("=== am-tests ===\n");
        test_uart_putc();
        test_timer_read();
        test_csr_warl();
        test_trap_ecall();
        test_timer_irq();
        test_soft_irq();
        test_plic_access();
        test_float();
        printf("=== ALL PASSED ===\n");
        break;
    case 'h':
    default:
        printf("Usage: make run TEST=<key>\n");
        for (int ch = 0; ch < 256; ch++)
            if (descriptions[ch])
                printf("  %c: %s\n", ch, descriptions[ch]);
        break;
    }
    return 0;
}
