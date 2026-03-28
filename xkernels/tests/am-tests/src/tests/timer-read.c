#include "test.h"

void test_timer_read(void) {
    unsigned long long t1 = mtime();
    for (volatile int i = 0; i < 1000; i++)
        ;
    unsigned long long t2 = mtime();
    check(t2 > t1);
    printf("timer-read: OK (%llu -> %llu)\n", t1, t2);
}
