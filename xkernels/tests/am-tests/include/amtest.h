#ifndef AMTEST_H
#define AMTEST_H

void test_uart_putc(void);
void test_timer_read(void);
void test_timer_irq(void);
void test_soft_irq(void);
void test_plic_access(void);
void test_csr_warl(void);
void test_trap_ecall(void);
void test_rtc(void);

#endif
