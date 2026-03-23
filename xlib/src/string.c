#include <string.h>

void *memset(void *s, int c, size_t n) {
    unsigned char *p = s;
    while (n--)
        *p++ = (unsigned char)c;
    return s;
}

void *memcpy(void *dst, const void *src, size_t n) {
    unsigned char *d = dst;
    const unsigned char *s = src;
    while (n--)
        *d++ = *s++;
    return dst;
}

void *memmove(void *dst, const void *src, size_t n) {
    char *d = dst;
    const char *s = src;
    if (d < s) {
        while (n--)
            *d++ = *s++;
    } else {
        while (n)
            n--, d[n] = s[n];
    }
    return dst;
}

int memcmp(const void *vl, const void *vr, size_t n) {
    const unsigned char *l = vl, *r = vr;
    for (; n && *l == *r; n--, l++, r++)
        ;
    return n ? *l - *r : 0;
}

size_t strlen(const char *s) {
    const char *a = s;
    for (; *s; s++)
        ;
    return s - a;
}

char *strcpy(char *dst, const char *src) {
    char *d = dst;
    while ((*d++ = *src++))
        ;
    return dst;
}

char *strncpy(char *dst, const char *src, size_t n) {
    size_t i;
    for (i = 0; i < n && src[i]; i++)
        dst[i] = src[i];
    for (; i < n; i++)
        dst[i] = '\0';
    return dst;
}

char *strcat(char *dst, const char *src) {
    strcpy(dst + strlen(dst), src);
    return dst;
}

int strcmp(const char *l, const char *r) {
    for (; *l == *r && *l; l++, r++)
        ;
    return *(unsigned char *)l - *(unsigned char *)r;
}

int strncmp(const char *l, const char *r, size_t n) {
    const unsigned char *a = (void *)l, *b = (void *)r;
    if (!n--)
        return 0;
    for (; *a && *b && n && *a == *b; a++, b++, n--)
        ;
    return *a - *b;
}

char *strchr(const char *s, int c) {
    for (; *s && *s != (char)c; s++)
        ;
    return *(unsigned char *)s == (unsigned char)c ? (char *)s : 0;
}

char *strrchr(const char *s, int c) {
    const char *found = 0;
    for (; *s; s++)
        if (*s == (char)c)
            found = s;
    return c == '\0' ? (char *)s : (char *)found;
}
