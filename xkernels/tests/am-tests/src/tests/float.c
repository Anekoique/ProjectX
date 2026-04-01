#include "test.h"

/* Inline asm helpers for F/D extension */
#define fadd_s(rd, rs1, rs2) \
    asm volatile("fadd.s " rd ", " rs1 ", " rs2 ::: "memory")
#define fmul_s(rd, rs1, rs2) \
    asm volatile("fmul.s " rd ", " rs1 ", " rs2 ::: "memory")
#define fadd_d(rd, rs1, rs2) \
    asm volatile("fadd.d " rd ", " rs1 ", " rs2 ::: "memory")

static void test_f_arith(void) {
    /* fadd.s: 1.0 + 2.0 = 3.0 */
    float a = 1.0f, b = 2.0f, c;
    asm volatile(
        "flw ft0, %1\n"
        "flw ft1, %2\n"
        "fadd.s ft2, ft0, ft1\n"
        "fsw ft2, %0\n"
        : "=m"(c) : "m"(a), "m"(b)
        : "ft0", "ft1", "ft2"
    );
    check(c == 3.0f);

    /* fmul.s: 3.0 * 4.0 = 12.0 */
    float d = 3.0f, e = 4.0f, f;
    asm volatile(
        "flw ft0, %1\n"
        "flw ft1, %2\n"
        "fmul.s ft2, ft0, ft1\n"
        "fsw ft2, %0\n"
        : "=m"(f) : "m"(d), "m"(e)
        : "ft0", "ft1", "ft2"
    );
    check(f == 12.0f);
    printf("  f-arith: OK\n");
}

static void test_d_arith(void) {
    /* fadd.d: 1.5 + 2.5 = 4.0 */
    double a = 1.5, b = 2.5, c;
    asm volatile(
        "fld ft0, %1\n"
        "fld ft1, %2\n"
        "fadd.d ft2, ft0, ft1\n"
        "fsd ft2, %0\n"
        : "=m"(c) : "m"(a), "m"(b)
        : "ft0", "ft1", "ft2"
    );
    check(c == 4.0);

    /* fmul.d: 2.5 * 3.0 = 7.5 */
    double d = 2.5, e = 3.0, f;
    asm volatile(
        "fld ft0, %1\n"
        "fld ft1, %2\n"
        "fmul.d ft2, ft0, ft1\n"
        "fsd ft2, %0\n"
        : "=m"(f) : "m"(d), "m"(e)
        : "ft0", "ft1", "ft2"
    );
    check(f == 7.5);
    printf("  d-arith: OK\n");
}

static void test_fcvt(void) {
    /* fcvt.s.w: int 42 -> float 42.0 */
    float r;
    asm volatile(
        "li t0, 42\n"
        "fcvt.s.w ft0, t0\n"
        "fsw ft0, %0\n"
        : "=m"(r) :: "t0", "ft0"
    );
    check(r == 42.0f);

    /* fcvt.w.s: float 42.0 -> int 42 */
    float src = 42.0f;
    long result;
    asm volatile(
        "flw ft0, %1\n"
        "fcvt.w.s %0, ft0\n"
        : "=r"(result) : "m"(src)
        : "ft0"
    );
    check(result == 42);
    printf("  fcvt:    OK\n");
}

static void test_fclass(void) {
    /* fclass.s: +0.0 -> bit 4 (0x10) */
    float zero = 0.0f;
    long cls;
    asm volatile(
        "flw ft0, %1\n"
        "fclass.s %0, ft0\n"
        : "=r"(cls) : "m"(zero)
        : "ft0"
    );
    check(cls == 0x10);  /* positive zero */

    /* fclass.s: -inf -> bit 0 (0x01) */
    unsigned int neg_inf_bits = 0xFF800000;
    float neg_inf;
    __builtin_memcpy(&neg_inf, &neg_inf_bits, 4);
    asm volatile(
        "flw ft0, %1\n"
        "fclass.s %0, ft0\n"
        : "=r"(cls) : "m"(neg_inf)
        : "ft0"
    );
    check(cls == 0x01);  /* negative infinity */
    printf("  fclass:  OK\n");
}

static void test_fs_dirty(void) {
    /* After FP op, mstatus.FS should be Dirty (0b11 << 13) */
    float a = 1.0f, b = 2.0f;
    asm volatile(
        "flw ft0, %0\n"
        "flw ft1, %1\n"
        "fadd.s ft2, ft0, ft1\n"
        :: "m"(a), "m"(b)
        : "ft0", "ft1", "ft2"
    );
    unsigned long mstatus = csrr(mstatus);
    unsigned long fs = (mstatus >> 13) & 0x3;
    check(fs == 3);  /* Dirty */
    printf("  fs-dirty: OK\n");
}

void test_float(void) {
    printf("=== Float (F/D) test ===\n");
    test_f_arith();
    test_d_arith();
    test_fcvt();
    test_fclass();
    test_fs_dirty();
    printf("float: ALL PASSED\n");
}
