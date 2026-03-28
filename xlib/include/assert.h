#ifndef _XLIB_ASSERT_H
#define _XLIB_ASSERT_H

#include <stdio.h>

#ifdef __cplusplus
extern "C" {
#endif

extern void halt(int code);

#ifdef __cplusplus
}
#endif

// clang-format off
#define assert(cond)                                              \
    do {                                                          \
        if (!(cond)) {                                            \
            printf("assert fail: %s:%d\n", __FILE__, __LINE__);   \
            halt(1);                                              \
        }                                                         \
    } while (0)
// clang-format on

#endif
