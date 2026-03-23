#include <limits.h>
#include <stdio.h>

/* Platform output hook — weak default is a no-op.
 * xhal provides the real implementation when a console device exists. */
__attribute__((weak)) void _putch(char c) {
    (void)c;
}

int snprintf(char *buf, size_t size, const char *fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    int ret = vsnprintf(buf, size, fmt, ap);
    va_end(ap);
    return ret;
}

int vsprintf(char *buf, const char *fmt, va_list ap) {
    return vsnprintf(buf, INT_MAX, fmt, ap);
}

int sprintf(char *buf, const char *fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    int ret = vsprintf(buf, fmt, ap);
    va_end(ap);
    return ret;
}

int printf(const char *fmt, ...) {
    char buf[256];
    va_list ap;
    va_start(ap, fmt);
    int ret = vsnprintf(buf, sizeof buf, fmt, ap);
    va_end(ap);
    for (int i = 0; i < ret && buf[i]; i++)
        _putch(buf[i]);
    return ret;
}
