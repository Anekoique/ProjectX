# `benchmark-adaptation` IMPL `02`

> Feature: `benchmark-adaptation`
> Iteration: `02`
> Owner: Executor
> Approved Plan: `02_PLAN.md`

---

## Completed Scope

All goals from 02_PLAN achieved:

1. **xam HAL: heap linker symbols** — `_heap_start` and `_heap_end` added to `linker.lds.S`, backed by `mem-size` in platform config.
2. **xam HAL: `uptime()`** — `mtime() / 10` → microseconds, added to `timer.rs`.
3. **xlib: `assert.h`** — C/C++-safe with `extern "C"` guards.
4. **xlib: C++ safety** — Added `extern "C"` guards to `stdio.h`, `string.h`.
5. **alu-tests** — Auto-generates `tests/test.c` from `gen_alu_test.c` on first build. 22k+ arithmetic checks, GOOD TRAP.
6. **coremark** — `MAIN_HAS_NOARGC=1`, `uptime()` timing, NJU headers removed. 1000 iterations, GOOD TRAP.
7. **dhrystone** — `uptime()` timing, NJU headers removed. 500k runs, GOOD TRAP.
8. **microbench** — All 10 sub-benchmarks (including 3 C++), heap via linker symbols, `mainargs` via `-DMAINARGS`. GOOD TRAP.
9. **am-tests: rtc** — New interactive clock accuracy test (`R` key).
10. **CI** — `test-alu` and `bench` jobs added.

## Deviations from Approved PLAN

| Deviation | Reason |
|-----------|--------|
| `mem-size` added to xconfig platform configs + build.rs | Linker script needs RAM end address; hardcoding 0x88000000 would break if config changes |
| `build_c.mk`: added `.cc`/`.cpp` → `.o` in OBJS pattern | OBJS only handled `.c`/`.S`; C++ sources weren't compiled |
| `build_c.mk`: added `-fno-exceptions -fno-rtti` to CXXFLAGS | Bare-metal C++ without runtime needs these flags |
| `xlib/include/stdio.h`: added `extern "C"` guards | C++ microbench sources link against C symbols |
| `xlib/include/string.h`: added `extern "C"` guards | Same reason |
| `quicklz.h`: added `<stdint.h>` | `quicklz.c` uses `uint32_t`/`uint8_t` |
| `dhrystone/dry.c`: added `<stdbool.h>` | Uses `true`/`false` from NJU's `klib.h` |
| `microbench/Makefile`: added `-Wno-maybe-uninitialized` | False positive in 15pz.cc from GCC inlining |
| `alu-tests/tests/test.c` not committed | User directive: generated locally, `.gitignore`'d |
| `xemu/Cargo.toml`: added release profile (LTO, codegen-units=1) | Emulator performance was unusable without optimization |
| `xemu/Makefile`: refactored MODE/BATCH separation | Separated build profile (MODE) from batch/interactive (BATCH) |
| Added rtc test to am-tests | User request for clock accuracy validation |

## Verification Results

| Test | Result |
|------|--------|
| alu-tests | GOOD TRAP (22365 lines of arithmetic checks) |
| coremark | GOOD TRAP (1000 iterations, 36 Marks) |
| dhrystone | GOOD TRAP (500k runs, 19 Marks) |
| microbench (ref) | GOOD TRAP (10/10 pass, 46 Marks) |
| am-tests | 7/7 PASS |
| cpu-tests | 30/30 PASS |
| am-tests rtc (interactive) | Clock accuracy verified (1s ≈ 1000000 us) |
| xemu fmt | Clean |
| xemu clippy | Clean |

## Files Changed

**xam HAL:**
- `xam/xhal/linker.lds.S` — `_heap_start`, `_heap_end` symbols
- `xam/xhal/build.rs` — `%MEM_SIZE%` replacement
- `xam/xhal/src/platform/xemu/timer.rs` — `uptime()`
- `xam/xconfig/configs/platforms/xemu.toml` — `mem-size`
- `xam/xconfig/configs/platforms/riscv64-qemu-virt.toml` — `mem-size`
- `xam/scripts/build_c.mk` — `.cc`/`.cpp` OBJS, `-fno-exceptions -fno-rtti`

**xlib:**
- `xlib/include/assert.h` — NEW
- `xlib/include/stdio.h` — `extern "C"` guards
- `xlib/include/string.h` — `extern "C"` guards

**Benchmarks:**
- `xkernels/benchmarks/coremark/Makefile` — `SRCS`, removed NJU deps
- `xkernels/benchmarks/coremark/include/core_portme.h` — xam headers, `MAIN_HAS_NOARGC=1`
- `xkernels/benchmarks/coremark/src/core_portme.c` — `uptime()` timing
- `xkernels/benchmarks/coremark/src/core_main.c` — removed `ioe_init()`
- `xkernels/benchmarks/dhrystone/Makefile` — `SRCS`
- `xkernels/benchmarks/dhrystone/dry.c` — xam headers, `uptime()`, removed `ioe_init()`
- `xkernels/benchmarks/microbench/Makefile` — `SRCS`, `-DMAINARGS`
- `xkernels/benchmarks/microbench/include/benchmark.h` — xam headers, compat macros
- `xkernels/benchmarks/microbench/src/bench.c` — `uptime()`, heap symbols, `mainargs`
- `xkernels/benchmarks/microbench/src/lzip/quicklz.h` — xam headers

**Tests:**
- `xkernels/tests/alu-tests/Makefile` — NEW (auto-generates test.c)
- `xkernels/tests/alu-tests/.gitignore` — ignore generated tests/
- `xkernels/tests/am-tests/src/tests/rtc.c` — NEW (clock accuracy test)
- `xkernels/tests/am-tests/include/amtest.h` — `test_rtc` declaration
- `xkernels/tests/am-tests/src/main.c` — `R` key dispatch

**xemu:**
- `xemu/Cargo.toml` — release profile (LTO)
- `xemu/Makefile` — MODE/BATCH separation

**CI:**
- `.github/workflows/ci.yml` — `test-alu` and `bench` jobs
