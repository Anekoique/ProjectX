#include "test.h"
#include <stdint.h>

/*
 * SMC torture test for the P4 decoded-instruction cache
 * (docs/perf/hotPath/04_PLAN.md §Architecture §P4).
 *
 * RISC-V Unprivileged ISA Manual §5.1 (Zifencei): a store to
 * instruction memory becomes visible to subsequent instruction
 * fetches on this hart only after FENCE.I. This test writes a tiny
 * function into RAM, executes it (expecting return value 0),
 * overwrites the immediate field, issues FENCE.I, and re-executes
 * (expecting return value 42).
 *
 * P4 contract (I-12): the icache is keyed on (pc, raw). The
 * overwrite changes the raw word at `pc`, so the cache comparison
 * misses and re-decodes with the new bits. FENCE.I remains a NOP
 * in xemu per the decoded-raw simplification; the
 * cache-miss-on-raw-change path is the actual correctness lever.
 *
 * Encodings (RISC-V Unprivileged ISA v2.2, Chapter 2 + Table 24.2):
 *   addi a0, zero, imm   = 0x00000513 | (imm << 20)
 *   jalr zero, 0(ra)     = 0x00008067        (= ret pseudo-instr)
 *
 * Observable channel: the function's return value flows through
 * a0 per the RV G-ABI (psABI), so the C-level assertion is
 * check(ret == expected).
 */

#define ENCODE_ADDI_A0_ZERO(imm)  (0x00000513u | ((uint32_t)(imm) << 20))
#define ENCODE_RET                 0x00008067u

static uint32_t smc_buf[2] __attribute__((aligned(4)));

typedef int (*smc_fn_t)(void);

void test_smc(void) {
    /* Phase 1 -- write `addi a0, zero, 0; ret`, execute, expect 0. */
    smc_buf[0] = ENCODE_ADDI_A0_ZERO(0);
    smc_buf[1] = ENCODE_RET;
    asm volatile ("fence.i" ::: "memory");
    smc_fn_t fn = (smc_fn_t)(uintptr_t)smc_buf;
    int r0 = fn();
    check(r0 == 0);

    /* Phase 2 -- overwrite immediate, fence.i, re-execute, expect 42. */
    smc_buf[0] = ENCODE_ADDI_A0_ZERO(42);
    asm volatile ("fence.i" ::: "memory");
    int r1 = fn();
    check(r1 == 42);

    printf("smc: OK\n");
}
