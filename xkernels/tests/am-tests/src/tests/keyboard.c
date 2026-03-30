#include "test.h"

#define UART     0x10000000UL
#define UART_RBR REG8(UART + 0x00) /* Receive Buffer Register */
#define UART_LSR REG8(UART + 0x05) /* Line Status Register    */

#define LSR_DR 0x01 /* Data Ready */

static int uart_getc(void) {
    if (UART_LSR & LSR_DR)
        return UART_RBR;
    return -1;
}

void test_keyboard(void) {
    printf("=== Keyboard (UART RX) test ===\n");
    printf("Type characters to echo. Press 'q' to quit.\n");

    int count = 0;
    while (1) {
        int c = uart_getc();
        if (c < 0)
            continue;
        count++;
        if (c == '\r' || c == '\n') {
            printf("\n");
        } else if (c == 'q') {
            printf("\n\nReceived %d character(s). Done.\n", count);
            break;
        } else {
            printf("%c\n", c);
        }
    }
    check(count > 0);
}
