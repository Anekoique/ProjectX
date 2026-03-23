#include <limits.h>
#include <stdarg.h>
#include <stddef.h>

/* Format buffer: tracks position and enforces size limit. */
typedef struct {
    char *buf;
    size_t size;
    size_t pos;
} Out;

static void out(Out *o, const char *s, size_t len) {
    for (size_t i = 0; i < len; i++) {
        if (o->pos < o->size)
            o->buf[o->pos] = s[i];
        o->pos++;
    }
}

static void pad(Out *o, char c, int n) {
    char p[32];
    int k = n < (int)sizeof p ? n : (int)sizeof p;
    for (int i = 0; i < k; i++)
        p[i] = c;
    for (; n > 0; n -= k, k = n < (int)sizeof p ? n : (int)sizeof p)
        out(o, p, k);
}

/* Number formatters: write backwards from end of buf, return pointer to start.
 * Caller provides buf[N] and passes buf+N as `s`. */

static char *fmt_u(unsigned long long x, char *s) {
    for (; x >= 10; x /= 10)
        *--s = '0' + x % 10;
    *--s = '0' + x;
    return s;
}

static char *fmt_x(unsigned long long x, char *s, int lower) {
    const char *digits = lower ? "0123456789abcdef" : "0123456789ABCDEF";
    for (; x; x >>= 4)
        *--s = digits[x & 15];
    return s;
}

static char *fmt_o(unsigned long long x, char *s) {
    for (; x; x >>= 3)
        *--s = '0' + (x & 7);
    return s;
}

static unsigned long long va_uint(va_list *ap, int ll) {
    if (ll >= 2)
        return va_arg(*ap, unsigned long long);
    if (ll)
        return va_arg(*ap, unsigned long);
    return va_arg(*ap, unsigned int);
}

static long long va_int(va_list *ap, int ll) {
    if (ll >= 2)
        return va_arg(*ap, long long);
    if (ll)
        return va_arg(*ap, long);
    return va_arg(*ap, int);
}

int vsnprintf(char *buf, size_t size, const char *fmt, va_list ap) {
    Out o = {buf, size > 0 ? size - 1 : 0, 0};
    char numbuf[22], *s, *end = numbuf + sizeof numbuf;

    while (*fmt) {
        if (*fmt != '%') {
            out(&o, fmt++, 1);
            continue;
        }
        fmt++;

        /* flags */
        int left = 0, zero = 0;
        for (;; fmt++) {
            if (*fmt == '-')
                left = 1;
            else if (*fmt == '0')
                zero = 1;
            else
                break;
        }

        /* width */
        int w = 0;
        for (; *fmt >= '0' && *fmt <= '9'; fmt++)
            w = w * 10 + (*fmt - '0');

        /* length */
        int ll = 0;
        while (*fmt == 'l') {
            ll++;
            fmt++;
        }

        char pc = (zero && !left) ? '0' : ' ';

        switch (*fmt++) {
        case 'd':
        case 'i': {
            long long v = va_int(&ap, ll);
            int neg = v < 0;
            unsigned long long uv = neg ? -(unsigned long long)v : v;
            s = fmt_u(uv, end);
            int len = end - s + neg;
            if (!left && pc == ' ')
                pad(&o, ' ', w - len);
            if (neg)
                out(&o, "-", 1);
            if (!left && pc == '0')
                pad(&o, '0', w - len);
            out(&o, s, end - s);
            if (left)
                pad(&o, ' ', w - len);
            break;
        }
        case 'u': {
            s = fmt_u(va_uint(&ap, ll), end);
            int len = end - s;
            if (!left)
                pad(&o, pc, w - len);
            out(&o, s, len);
            if (left)
                pad(&o, ' ', w - len);
            break;
        }
        case 'x':
        case 'X': {
            unsigned long long v = va_uint(&ap, ll);
            if (v == 0) {
                s = end;
                *--s = '0';
            } else
                s = fmt_x(v, end, fmt[-1] == 'x');
            int len = end - s;
            if (!left)
                pad(&o, pc, w - len);
            out(&o, s, len);
            if (left)
                pad(&o, ' ', w - len);
            break;
        }
        case 'o': {
            unsigned long long v = va_uint(&ap, ll);
            if (v == 0) {
                s = end;
                *--s = '0';
            } else
                s = fmt_o(v, end);
            int len = end - s;
            if (!left)
                pad(&o, pc, w - len);
            out(&o, s, len);
            if (left)
                pad(&o, ' ', w - len);
            break;
        }
        case 'p': {
            unsigned long long v = (unsigned long long)(unsigned long)va_arg(ap, void *);
            if (v == 0) {
                s = end;
                *--s = '0';
            } else
                s = fmt_x(v, end, 1);
            int len = end - s + 2;
            if (!left)
                pad(&o, ' ', w - len);
            out(&o, "0x", 2);
            out(&o, s, end - s);
            if (left)
                pad(&o, ' ', w - len);
            break;
        }
        case 's': {
            const char *str = va_arg(ap, const char *);
            if (!str)
                str = "(null)";
            int len = 0;
            while (str[len])
                len++;
            if (!left)
                pad(&o, ' ', w - len);
            out(&o, str, len);
            if (left)
                pad(&o, ' ', w - len);
            break;
        }
        case 'c': {
            char c = (char)va_arg(ap, int);
            if (!left)
                pad(&o, ' ', w - 1);
            out(&o, &c, 1);
            if (left)
                pad(&o, ' ', w - 1);
            break;
        }
        case '%':
            out(&o, "%", 1);
            break;
        default:
            out(&o, "%", 1);
            out(&o, &fmt[-1], 1);
            break;
        }
    }

    if (size > 0)
        buf[o.pos < size - 1 ? o.pos : size - 1] = '\0';
    return (int)o.pos;
}
