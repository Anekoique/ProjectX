#include "test.h"

extern unsigned long long uptime(void);

void test_rtc(void) {
    int sec = 1;
    while (1) {
        while (uptime() / 1000000 < (unsigned long long)sec)
            ;
        unsigned long long us = uptime();
        unsigned long long total_s = us / 1000000;
        printf("%d second(s) elapsed [uptime = %llu us]\n", sec, us);
        check(total_s >= (unsigned long long)sec);
        sec++;
    }
}
