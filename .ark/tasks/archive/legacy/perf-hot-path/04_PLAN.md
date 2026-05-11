# `hotPath` PLAN `04`

> Status: Approved for Implementation
> Feature: `hotPath`
> Iteration: `04`
> Owner: Executor
> Depends on:
> - Previous Plan: `03_PLAN.md`
> - Review: `03_REVIEW.md`
> - Master Directive: `03_MASTER.md` (M-001..M-004 binding); `00_MASTER.md` M-001/M-002/M-003 also still binding; `01_MASTER.md` and `02_MASTER.md` blank.

---

## Summary

Round 04 is the final planning iteration before implementation (round cap
00–04). It keeps every round-03 architectural decision unchanged —
decoded-raw `(pc, raw)` icache with zero invalidation hooks; binding §A
per-phase Exit Gate plus §B combined gates with no reduced-bundle escape
hatch; P3 Mtimer deadline gate; P5 MMU `#[inline]` audit only (trap-slim
dropped); P6 Ram typed-read bypass; `perf-stats` Cargo feature; combined
P3+P4+P5+P6 scope under `00_MASTER.md` M-001 — and resolves the four
findings in `03_REVIEW.md` plus adopts both TR-1 and TR-2 by surgical
edits inside Phase 2a and the Mandatory Verification block.

The resolution is entirely concrete:

1. **R-001 HIGH.** Phase 2a now explicitly touches **all three** harness
   files — `xkernels/tests/am-tests/include/amtest.h`,
   `xkernels/tests/am-tests/src/main.c`, and
   `xkernels/tests/am-tests/Makefile` — with named line numbers and
   exact snippets, so letter `m` cannot fall through the default-help
   path. A one-time baseline sanity probe (`MAINARGS="m"` on the
   pre-change repo) documents the vacuous-pass risk before the wiring
   lands.
2. **R-002 HIGH.** `smc.c` is rewritten around a RAM-resident function
   that returns through `a0` (the RV psABI return register). The
   sequence is `ENCODE_ADDI_A0_ZERO(0) + ENCODE_RET`, call, overwrite
   the immediate, `fence.i`, re-call, `check(ret == 42)`. No `x1`/`ra`
   observation; no fragile asm scaffolding; every instruction encoding
   cites the RISC-V Unprivileged ISA Manual.
3. **R-003 MEDIUM.** The Mandatory Verification block restores
   `make -C xemu run` and `make -C xemu test` (both exist at
   `xemu/Makefile:44` and `xemu/Makefile:53`) alongside the stronger
   workspace/doc/perf/am-test/boot checks. Satisfies AGENTS.md §
   Development Standards line 8 in the form the repo actually provides.
4. **R-004 MEDIUM.** Acceptance Mapping C-9 proof becomes
   `cd xemu && cargo clippy --all-targets -- -D warnings`, which is
   executable and matches the Mandatory Verification block shape.

Round-03 MASTER compliance is explicit:

- **M-001** (fix REVIEW's problems): every R-NNN has a concrete file-
  level fix with an attached gate, not a hand-wavy note.
- **M-002** (cover all PERF_DEV.md phases): §Architecture walks through
  PERF_DEV.md §3 P3, P4, P5, P6 one-to-one, and the exit-gate language
  for each phase mirrors PERF_DEV.md's own exit conditions.
- **M-003** (correctness per official manuals): every ISA-level change
  cites the relevant RISC-V spec section (Zifencei §5.1; Unpriv Ch. 2
  + Table 24.2; Priv §3.1.10 `mtime`/`mtimecmp`; Priv §3.1.12 Sstc;
  Zicsr §3.1; psABI for `a0`).
- **M-004** (clean/concise/elegant; conform to codebase style): the
  icache, Mtimer, and Ram patches follow the module-path conventions
  already used at `xemu/xcore/src/device/bus.rs` and
  `xemu/xcore/src/arch/riscv/cpu/mm.rs`; no new `Arc`/`Mutex`, no
  ad-hoc `unsafe` without `// SAFETY:` comment.

No benchmark-targeted tricks, no `Mutex<Bus>` regression, no scope leak
beyond P3+P4+P5+P6, no reduced-bundle reinterpretation.

## Log

[**Feature Introduce**]

- **Fully wired SMC test harness.** Phase 2a is upgraded from "add a
  file and a Makefile letter" to a three-file edit with exact
  `descriptions[]` insertion, `CASE('m', test_smc)` placement, and
  `name` substitution extension. A pre-wiring probe demonstrates the
  default-help vacuous-pass path on the baseline so the gate's value
  is recorded, not assumed.
- **ABI-correct SMC test body.** `smc.c` now builds two RISC-V
  instructions from verified encodings
  (`addi a0, zero, imm` = `0x00000513 | (imm << 20)`;
  `jalr zero, 0(ra)` = `0x00008067`) into an aligned RAM buffer,
  calls it as a function pointer, observes the return value, mutates
  the immediate, issues `fence.i` for architectural hygiene, and
  re-calls. All observation is through the standard C return-value
  channel.
- **Repo-mandated make targets restored.** `make -C xemu run` and
  `make -C xemu test` land alongside the round-03 workspace/doc/perf
  checks, so AGENTS.md §Development Standards is honoured in its own
  vocabulary.
- **Executable clippy gate.** Acceptance Mapping C-9 proof is now
  `cd xemu && cargo clippy --all-targets -- -D warnings`.

[**Review Adjustments**]

- **R-001** (HIGH, Phase 2a wiring): see §Implementation Plan Phase 2a
  steps 1a–1d, §Validation V-IT-1, §Exit Gate §A P4, §Trade-offs T-10.
- **R-002** (HIGH, ABI observation, TR-1 adopted explicitly): see
  §Data Structure `smc.c` sketch, §Implementation Plan Phase 2a step
  2, §Invariants I-18 (refined), §Trade-offs T-11.
- **R-003** (MEDIUM, make run/test restored, TR-2 adopted explicitly):
  see §Validation preamble, §Exit Gate header, §Constraints C-2 / C-4.
- **R-004** (MEDIUM, executable clippy command): see §Validation
  Acceptance Mapping C-9 row, §Exit Gate header, §Constraints C-9.

[**Master Compliance**]

- **Round-00 M-001** (combined scope): P3+P4+P5+P6 remain one branch /
  one commit; §Implementation Plan Phases 1–5 unchanged in count.
- **Round-00 M-002** (path rename): all paths stay under
  `docs/perf/hotPath/`.
- **Round-00 M-003** (clean layout): single Summary / Log / Response
  Matrix / Spec / Trade-offs / Validation / Exit Gate; only
  Architecture splits per-phase because P3..P6 diverge materially.
- **Round-03 M-001** (fix REVIEW's problems): Response Matrix lists
  R-001..R-004 with concrete resolutions tied to file:line edits.
- **Round-03 M-002** (cover all PERF_DEV.md phases): §Architecture §P3
  / §P4 / §P5 / §P6 and Exit Gate §A each match PERF_DEV.md §3 exit
  conditions verbatim (cited below in each phase).
- **Round-03 M-003** (correctness per official manuals): §Architecture
  §P3 cites RISC-V Privileged §3.1.10 and §3.1.12; §P4 cites Zifencei
  §5.1 and Unpriv Ch. 2 + Table 24.2; §P5 cites Zicsr §3.1; `smc.c`
  cites the psABI `a0` convention.
- **Round-03 M-004** (clean / concise / elegant code): the planned
  patches follow the module-path shape of `xcore/src/device/bus.rs`
  (the M-001 sentinel compile_fail pattern) and
  `xcore/src/arch/riscv/cpu/mm.rs` (free-function-ish helpers for
  hot-path inlining). No new `unsafe` without a `// SAFETY:` block;
  no new `Arc`/`Mutex`/`RefCell` on `Bus`/`RVCore`.

### Changes from Previous Round

[**Added**]

- Phase 2a step 1a (insert `void test_smc(void);` in `amtest.h` after
  the `test_float` declaration at line 13).
- Phase 2a step 1b (insert `['m'] = "smc: ..."` into `descriptions[]`
  in `main.c` between the `['f']` row at line 24 and the `['a']` row
  at line 25; insert `CASE('m', test_smc);` into the `switch` between
  the `CASE('f', …)` at line 45 and `case 'a':` at line 46).
- Phase 2a step 1d (one-time pre-wiring baseline probe: run
  `make -C xkernels/tests/am-tests run TEST=m` on the unmodified repo;
  record the output, confirming it hits the default-help path and
  returns `HIT GOOD TRAP` vacuously — documents R-001's concern).
- `smc.c` source sketched inline in §Data Structure with verified
  encoding macros and RISC-V citations.
- T-10: trade-off for "three-file wiring vs. deferred dispatch-layer
  refactor" (Option A = three-file wiring, chosen).
- T-11: trade-off for "function-pointer `a0` return vs. memory-slot
  observation" (Option 1 = `a0` return, chosen, per TR-1).
- `make -C xemu run` and `make -C xemu test` back in the Mandatory
  block with `# comments`.
- Acceptance Mapping C-9 row rewritten.
- Invariant I-18 refined with the ABI-visible observable.
- Constraint C-9 rewritten.

[**Changed**]

- §Data Structure gains a fenced `smc.c` sketch (encodings, buffer,
  function-pointer call, assertions).
- §Architecture §P4 gets a spec-citation sub-bullet for Zifencei §5.1.
- §Validation preamble `xemu` sub-block grows two lines.
- §Exit Gate header block grows two lines.
- Acceptance Mapping row for C-9 changes proof command.

[**Removed**]

- No content was deleted from round 03; only surgical additions and
  one rewrite of the `smc.c` body (which was a round-03 sketch, not a
  committed file).

[**Unresolved**]

- `fence.i` remains a NOP in xemu (`inst/privileged.rs:71-79`); the
  SMC test uses it for architectural hygiene (real hardware requires
  it per Zifencei §5.1). xemu correctness is provided by `(pc, raw)`
  mismatch, not by the fence. Noted, not a gap.
- The xam-platform linker / PMP layout for the `smc_buf[]` region —
  the buffer is in `.bss` aligned to 4 bytes; the xam bare-metal
  platform does not enforce W^X on that region by default (standard
  for am-tests). If a future PMP hardening lands, `smc.c` will need
  to move the buffer into a dedicated XW region. Not a round-04 risk.
- If `--features perf-stats` telemetry on Linux boot shows an icache
  hit-rate anomaly, it is recorded for a post-round iteration. Boot
  is not on the hit-rate gate.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Master `00_MASTER.md` | M-001 | Applied | Bundled P3+P4+P5+P6 scope preserved. See §Spec Architecture §P3..§P6 and §Implementation Plan phases 1–5. |
| Master `00_MASTER.md` | M-002 | Applied | All references stay under `docs/perf/hotPath/`. No path churn in round 04. |
| Master `00_MASTER.md` | M-003 | Applied | Single top-level sections; per-phase subsections only where design diverges. |
| Master `01_MASTER.md` | — | N/A | Blank; no round-01 directives. |
| Master `02_MASTER.md` | — | N/A | Blank; round-00 directives remain in force. |
| Master `03_MASTER.md` | M-001 | Applied | Verbatim: "MUST: use /pua skill to fix the REVIEW's problems". Interpretation: the `/pua` skill is not available in this runtime; apply the directive's plain intent — fix every R-001..R-004 finding with concrete, file-level edits and an attached gate. Each R-NNN below has a file:line resolution plus a validation entry. |
| Master `03_MASTER.md` | M-002 | Applied | Verbatim: "MUST: check detaily about the plan and the PERF_DEV.md. Reason: This PLAN must cover all phases of PERF_DEV.m". Resolution: §Architecture walks PERF_DEV.md §3 P3→P4→P5→P6 one-to-one; Exit Gate §A language mirrors PERF_DEV.md's exit conditions (SMC am-test passes, MMU bucket drops, memmove bucket < 2 %, wall-clock reductions 20/20/15 %). |
| Master `03_MASTER.md` | M-003 | Applied | Verbatim: "MUST: Your optimization must is correct and conform to the offcial munuals. Reason: You must make sure the correctness of your changes and optimizations". Resolution: §Architecture cites RISC-V Unprivileged ISA Manual §5.1 Zifencei (fence.i), Ch. 2 + Table 24.2 (addi / jalr encodings), Privileged §3.1.10 (mtime/mtimecmp), Privileged §3.1.12 (Sstc), Zicsr §3.1, and the RISC-V psABI (a0 return register). Every invariant is spec-backed. |
| Master `03_MASTER.md` | M-004 | Applied | Verbatim: "MUST: You must make sure your codes clean, concise and elegant, the codes should conform to the style of codebase." Resolution: patches follow the style templates of `xemu/xcore/src/device/bus.rs` (M-001 sentinel doc-test layout) and `xemu/xcore/src/arch/riscv/cpu/mm.rs` (free-function helpers with `#[inline]` where LTO does not fold). No new `Arc<Mutex<_>>`, no `unsafe` without `// SAFETY:`, module paths honour the arch-nested layout from commit `a4bcae8`. |
| Round-00 Review | R-001 | Obsolete per round-01 R-001 collapse | `checked_write` SMC flush not installed; `(pc, raw)` keying handles SMC implicitly. |
| Round-00 Review | R-002 | Resolved in round 03, preserved | SMC am-test restored (letter `m`); round-04 adds full harness wiring + ABI-correct body. |
| Round-00 Review | R-003 | Obsolete per round-01 R-001 collapse | `mstatus` bit-isolation hook not installed. |
| Round-00 Review | R-004 | Obsolete per round-01 R-001 collapse | Privilege-debounced trap hook not installed. |
| Round-00 Review | R-005 | Retained | Index `(pc >> 1) & MASK`; full-`pc` tag disambiguates aliases. |
| Round-00 Review | R-006 | Obsolete | MPRV does not participate in decode. |
| Round-00 Review | R-007 | Obsolete | `ctx_tag` does not exist. |
| Round-00 Review | R-008 | Retained | `perf-stats` Cargo feature in `xemu/xcore/Cargo.toml`, off by default. |
| Round-00 Review | R-009 | Retained | Response Matrix populated in full. |
| Round-01 Review | R-001 | Accepted (round 02) | Decoded-raw cache; preserved. |
| Round-01 Review | R-002 | Accepted (round 02) | Per-phase §A + combined §B gates; escape hatch removed in round 03. |
| Round-01 Review | R-003 | Accepted (round 02, Option A) | P5 trap-slim dropped; preserved. |
| Round-01 Review | R-004 | Superseded by round-03 R-002 | Workdir-qualified block installed in round 03; round 04 restores `make -C xemu run/test`. |
| Round-02 Review | R-001 | Resolved (HIGH) | SMC am-test restored as binding P4 gate; round 04 finalises its wiring. |
| Round-02 Review | R-002 | Resolved (HIGH) | Workdir-qualified command block; round 04 adds `make -C xemu run/test`. |
| Round-02 Review | R-003 | Resolved (MEDIUM) | `cargo-asm` dropped; V-IT-5 uses profile bucket delta. |
| Round-02 Review | R-004 | Resolved (MEDIUM, Option 1) | Escape hatch removed; any §A miss opens a fresh PLAN iteration. |
| Round-02 Review | TR-1 | Adopted | Keep cache simplification; restore SMC am-test (confirmed in round 04). |
| Round-02 Review | TR-2 | Adopted | Workdir-qualified commands (extended in round 04). |
| **Round-03 Review** | **R-001** | **Resolved (HIGH)** | Phase 2a now touches `amtest.h:13` (add `void test_smc(void);`), `main.c:14-27` (add `['m']` description row), `main.c:36-45` (add `CASE('m', test_smc);`), `Makefile:16` (ALL += `m`), `Makefile:18-20` (`$(patsubst m,smc,...)`). The plan explicitly records that without all three edits, letter `m` falls through `main.c:58-64` → help path → returns 0 → vacuous `HIT GOOD TRAP`. See §Implementation Plan Phase 2a steps 1a–1d, §Validation V-IT-1, §Exit Gate §A P4, §Trade-offs T-10. |
| **Round-03 Review** | **R-002** | **Resolved (HIGH)** | `smc.c` body rewritten around a RAM-resident function that returns through `a0` (RV psABI return reg). Instructions come from verified Unpriv Table 24.2 encodings. C-level observable is the function return value. No `x1`/`ra` dependence. See §Data Structure `smc.c` sketch, §Invariants I-18, §Implementation Plan Phase 2a step 2, §Trade-offs T-11. TR-1 adopted. |
| **Round-03 Review** | **R-003** | **Resolved (MEDIUM)** | Mandatory Verification block now includes `make -C xemu fmt / clippy / run / test` lines (matching AGENTS.md §Development Standards line 8). `make -C xemu run` invokes `cargo run` (`xemu/Makefile:44-45`); `make -C xemu test` invokes `cargo test -p xcore` (`xemu/Makefile:53-54`). Stronger workspace / perf / am-test / boot checks retained as supplements. TR-2 adopted. See §Validation preamble, §Exit Gate header, §Constraints C-2 / C-4. |
| **Round-03 Review** | **R-004** | **Resolved (MEDIUM)** | Acceptance Mapping C-9 proof is now `cd xemu && cargo clippy --all-targets -- -D warnings`, which is an executable command form (the previous `make -C xemu clippy --all-targets` was impossible because `--all-targets` is a Cargo flag and `xemu/Makefile:47-48` does not forward extra args). See §Validation Acceptance Mapping, §Constraints C-9. |
| Round-03 Review | TR-1 | Adopted | `smc.c` uses `a0` return (Option 1 / Option A). See §Trade-offs T-11. |
| Round-03 Review | TR-2 | Adopted | `make -C xemu run/test` added explicitly. See §Validation preamble. |

> Rules:
> - Every prior HIGH / CRITICAL finding appears above.
> - Every MASTER directive appears above verbatim.
> - Rejections / obsoletions cite the triggering cause.

---

## Spec

[**Goals**]

- G-1 (P3): Cached-deadline short-circuit in `Mtimer::tick` so the
  default path is one `u64` compare + return; combined `Mtimer::*`
  self-time < 1 % on all three benchmarks. (PERF_DEV.md §3 P3.)
- G-2 (P4): Eliminate the pest tree walk on every cache-hit fetch; the
  fast path becomes one `(pc, raw)` compare + one POD copy of
  `DecodedInst`. Hit rate ≥ 95 % on dhry / cm / mb. (PERF_DEV.md
  §3 P4.)
- G-3 (P5): Inline the TLB-hit MMU fast path end-to-end through
  `checked_read` / `checked_write` / `access_bus`; MMU bucket drops ≥
  3 pp (evidence by sampling profile per V-IT-5). (PERF_DEV.md §3 P5.)
- G-4 (P6): Bypass `_platform_memmove` for aligned 1/2/4/8-byte RAM
  accesses via typed primitive reads/writes; memmove+memcpy combined
  bucket < 2 %; `Bus::read + Bus::write` combined drops ≥ 3 pp.
  (PERF_DEV.md §3 P6.)
- G-5 (combined): Wall-clock reduction on the 2026-04-15 baseline —
  dhry ≥ 20 %, cm ≥ 20 %, mb ≥ 15 %.
- G-6 (combined): `xdb::main` self-time share drops by ≥ 10 pp on all
  three workloads.
- G-7 (correctness): All existing `cargo test --workspace` outcomes
  unchanged; `make -C xkernels/tests/am-tests run` passes including
  the new letter `m` (SMC test); Linux / Linux-2hart / Debian boot to
  prompt.
- G-8 (mutex-free): `bash scripts/ci/verify_no_mutex.sh` stays `ok`.

- NG-1: No JIT, no trace chaining, no threaded dispatch.
- NG-2: No paddr-tagged SMC bitmap.
- NG-3: No replacement for `pest`.
- NG-4: No benchmark-specific code paths.
- NG-5: No assembly-file modifications.
- NG-6: No new `Arc<Mutex<_>>`, `RwLock<_>`, `RefCell<_>`, or
  `Box<dyn FnMut>` on `Bus` or `RVCore`.
- NG-7: No multi-thread SMP work.
- NG-8: No invalidation hooks on `satp` / `sfence.vma` / `fence.i` /
  privilege transitions / `mstatus` / RAM stores.
- NG-9: No rework of `retire` / `commit_trap`.
- NG-10: No reduced-bundle re-interpretation of §B thresholds.
- NG-11: No binding dependency on `cargo-asm`.

[**Architecture**]

Top-level shape (unchanged from round 03):

```
 RVCore::step  (xcore/src/arch/riscv/cpu.rs:238-245)
   |
   |  fetch(bus) -> raw:u32                 [P5: ensure fully inlined]
   v
 decode_cached(raw) -> DecodedInst          [P4: (pc, raw) key only]
   |   hit : one tag compare + one POD copy
   |   miss: pest decode, line overwrite
   v
 dispatch(bus, decoded)                     [unchanged]
   |
   v
 Bus::tick()                                [P3: Mtimer deadline-gate]
   \__ Mtimer::tick : if mtime < next_fire_mtime { return; }
   \__ other devices unchanged
```

### Architecture §P3 — Mtimer deadline gate

Spec citations (M-003):
- **RISC-V Privileged ISA Manual §3.1.10 (`mtime` / `mtimecmp`)** — a
  timer interrupt is pending whenever `mtime ≥ mtimecmp[h]` for hart
  `h`; therefore `min(mtimecmp[*])` is a sound lower bound on the
  next possible firing.
- **RISC-V Privileged ISA Manual §3.1.12 (Sstc)** — Supervisor Sstc
  extends the same relation to `stimecmp`; xemu currently does not
  enable Sstc on the hot path, but the deadline-gate invariant stays
  compatible.

Implementation: `Mtimer` at
`xcore/src/arch/riscv/device/aclint/mtimer.rs` grows one field
`next_fire_mtime: u64`, initialised `u64::MAX`. `tick()` adds
`if self.mtime < self.next_fire_mtime { return; }` before the existing
`check_all`. Recompute `next_fire_mtime` on every `mtimecmp` write, at
the end of `check_timer`, and on `reset`. No change to interrupt
semantics; only the default-path branch count is reduced. Style
follows the existing Mtimer implementation of sibling
`check_timer` / `check_all` (M-004).

### Architecture §P4 — Decoded-instruction cache (SMC gate fully wired)

Spec citations (M-003):
- **RISC-V Unprivileged ISA Manual §5.1 (Zifencei, `fence.i`).** Stores
  to instruction memory become visible to later instruction fetches
  on the same hart only after `FENCE.I`. The SMC test issues
  `fence.i` for architectural hygiene. In xemu, the icache correctness
  invariant is `(pc, raw)`-keyed miss-on-change (I-12), which
  satisfies the architectural observation without the fence, so
  `fence.i` remains a NOP.
- **RISC-V Unprivileged ISA Manual Ch. 2 + Table 24.2.** `addi rd,
  rs1, imm` is encoded as `[imm(11:0) | rs1 | 000 | rd | 0010011]`
  (opcode `OP-IMM`); with `rs1 = x0` and `rd = a0` we get
  `0x00000513 | ((imm & 0xfff) << 20)`. `jalr x0, 0(ra)` (the `ret`
  pseudo-instruction) is `[0 | ra | 000 | 00000 | 1100111]` =
  `0x00008067`. These encodings back the `ENCODE_ADDI_A0_ZERO(imm)`
  and `ENCODE_RET` macros in `smc.c`.
- **RISC-V psABI / RV G-ABI** — `a0` (= `x10`) is the first argument
  / return value register for the standard C calling convention. The
  `smc.c` test observes the function's return through the C return
  channel, which lowers to `a0`.

Design: per-hart direct-mapped 4096-line cache. Key `(pc, raw)`.
Line `{ pc, raw, decoded }`. No `ctx_tag`. No invalidation hooks.
`fence.i` stays a NOP. SMC falls out because the next fetch reads
different `raw`, the key misses, the line is overwritten.

Round-04 change at this section is verification-only and harness-level:

- Phase 2a wires the SMC am-test through **all three** dispatch
  layers (header, main dispatcher, Makefile), so letter `m` executes
  `test_smc()` — not the default-help fallthrough.
- `smc.c` observes the result through the C function-return channel
  (register `a0`), not through `x1`/`ra`.
- Invariant I-18 restated as an ABI-visible observable.
- V-IT-1 remains the binding gate; V-F-2 / V-UT-3b are Rust-level
  mirrors.

### Architecture §P5 — MMU fast-path inline

Spec citations (M-003):
- **RISC-V Privileged ISA Manual Zicsr §3.1.** CSR access semantics
  for `satp` / `mstatus` — no invalidation hook is required for a
  decode cache that keys on `(pc, raw)`; `satp` changes cause
  different `pc → paddr` mappings whose fetches naturally read
  potentially different `raw`, and `mstatus.MPRV` applies to
  loads/stores, not fetch.

Target files: `xcore/src/arch/riscv/cpu/mm.rs` (`access_bus`,
`checked_read`, `checked_write`, `translate`, `fetch`, `load`,
`store`). No algorithmic change. Add `#[inline]` /
`#[inline(always)]` where the current pinned rustc + LTO does not
already fold. Style follows the existing free-function-style helpers
in `cpu/mm.rs` (M-004).

Evidence:

- **Primary (binding).** MMU-entry bucket
  (`access_bus + checked_read + checked_write + load + store`) drops
  ≥ 3 pp between `docs/perf/2026-04-15/data/*.sample.txt` and
  `docs/perf/<post-hotPath-date>/data/*.sample.txt`.
- **Optional (author-side, not a gate).** `cargo rustc --release -p
  xcore -- --emit=asm` yields `.s` files under `target/release/deps/`.

Trap slim remains DROPPED (round-02 R-003 Option A).

### Architecture §P6 — memmove typed-read bypass

`Ram::read` / `Ram::write` (`xcore/src/device/ram.rs`) gain a size +
alignment pre-check. For aligned 1/2/4/8-byte accesses, use
`u{8,16,32,64}::from_le_bytes(slice[..N].try_into()?)` /
`to_le_bytes`. Fall through to the existing memmove path for MMIO,
unaligned, and odd-size cases. No new `unsafe` unless the safe form
fails to lower; in that case, `ptr::read_unaligned` with an explicit
`// SAFETY:` block (alignment/in-bounds/no-aliasing).

[**Invariants**]

- I-1: `RVCore::step` always calls `fetch` before any icache lookup.
- I-2: `pest` (via `DECODER.decode`) is the sole decode authority.
- I-3: Icache is owned per-hart; no shared state, no atomics.
- I-4: On miss, a line is overwritten in full (`pc`, `raw`,
  `decoded`); partial updates forbidden.
- I-5: Decode failure does not write a line.
- I-6: Compressed instructions: `raw` carries the fetched word as-is.
- I-7: SMC implicit: next fetch reads different `raw`; `(pc, raw)`
  misses; line overwritten. No store hook.
- I-8: Stores to MMIO do not participate in icache logic.
- I-9: `fence.i` remains a NOP (`inst/privileged.rs:71-79`); correct
  under I-7 because the next fetch already reflects any prior store.
- **I-10** (from P1, binding): `CPU::step` destructures `self` into
  disjoint borrows; no `Mutex<Bus>`.
- **I-11:** Icache geometry — per-hart, direct-mapped, 4096 lines,
  index `(pc >> 1) & MASK`.
- **I-12:** Cache miss iff `line.pc != self.pc || line.raw != raw`.
  Sole correctness rule for P4.
- I-16 (P3): `next_fire_mtime = min(mtimecmp[*])`, recomputed on
  every `mtimecmp` write, at the end of `check_timer`, and on
  `reset`. Initialised `u64::MAX`.
- I-17 (P6): Typed-read bypass iff region is RAM AND `size ∈ {1, 2,
  4, 8}` AND `addr % size == 0`; all other cases fall through.
- **I-18 (P4 SMC torture contract, round-04 refined):** For any
  RAM-backed address `pc` mapped RX-and-W, the sequence
  `store_word(pc, raw_a); fetch(pc) == raw_a; call_as_fn_ptr(pc);
  observe a0 == value_a; store_word(pc, raw_b); fence.i;
  fetch(pc) == raw_b; call_as_fn_ptr(pc); observe a0 == value_b`
  must hold end-to-end. This is the ABI-visible observable the
  restored am-test `smc.c` (letter `m`) witnesses. At the ICache
  layer the invariant reduces to I-12 (`(pc, raw)` mismatch on
  `raw_b ≠ raw_a`).

Invariants I-13 / I-14 / I-15 from round 01 remain removed — no
per-hart context tag, no `mstatus` hook, no privilege hook.

[**Data Structure**]

```rust
// xcore/src/arch/riscv/cpu/icache.rs  (new; identical to round 03)
use crate::isa::riscv::decoder::DecodedInst;
use memory_addr::VirtAddr;

pub const ICACHE_BITS:  usize = 12;
pub const ICACHE_LINES: usize = 1 << ICACHE_BITS;
pub const ICACHE_MASK:  usize = ICACHE_LINES - 1;

#[derive(Clone, Copy)]
pub struct ICacheLine {
    pub pc:      VirtAddr,
    pub raw:     u32,
    pub decoded: DecodedInst,
}

impl ICacheLine {
    pub const INVALID: Self = Self {
        pc:      VirtAddr::from_usize(0),
        raw:     0,
        decoded: DecodedInst::C { kind: InstKind::illegal, inst: 0 },
    };
}

pub struct ICache {
    pub lines: [ICacheLine; ICACHE_LINES],
}

impl ICache {
    pub fn new() -> Box<Self> {
        Box::new(Self { lines: [ICacheLine::INVALID; ICACHE_LINES] })
    }

    #[inline]
    pub fn index(pc: VirtAddr) -> usize {
        (pc.as_usize() >> 1) & ICACHE_MASK
    }
}

// RVCore (xcore/src/arch/riscv/cpu.rs) gains ONE field:
pub struct RVCore {
    // ... existing fields unchanged ...
    pub(in crate::arch::riscv) icache: Box<ICache>,
}

// Mtimer (xcore/src/arch/riscv/device/aclint/mtimer.rs) gains ONE field:
pub(super) struct Mtimer {
    // ... existing fields unchanged ...
    next_fire_mtime: u64, // P3: min(mtimecmp[*]); starts u64::MAX
}
```

`DecodedInst` gains `Copy` in addition to its existing derives.

**SMC am-test source (new file at
`xkernels/tests/am-tests/src/tests/smc.c`, round-04 R-001/R-002):**

```c
#include "test.h"
#include <stdint.h>

/*
 * SMC torture test for the P4 decoded-instruction cache.
 *
 * RISC-V Unprivileged ISA Manual Section 5.1 (Zifencei): a store to
 * instruction memory becomes visible to subsequent instruction
 * fetches on this hart only after FENCE.I. This test writes a tiny
 * function into RAM, executes it (expecting return value 0),
 * overwrites the immediate field, issues FENCE.I, and re-executes
 * (expecting return value 42).
 *
 * P4 contract (I-12): the icache is keyed on (pc, raw). The
 * overwrite changes the raw word at `pc`, so the cache comparison
 * misses and re-decodes with the new bits. FENCE.I remains a NOP
 * in xemu per the decoded-raw simplification; the cache-miss-on-
 * raw-change path is the actual correctness lever.
 *
 * Encodings (RISC-V Unprivileged ISA v2.2, Chapter 2 + Table 24.2):
 *   addi a0, zero, imm   = 0x00000513 | (imm << 20)
 *   jalr zero, 0(ra)     = 0x00008067        (= ret pseudo-instr)
 *
 * Observable channel: the function's return value flows through
 * a0 per the RV G-ABI (psABI), so the C-level assertion is
 * `check(ret == expected)`.
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
    smc_fn_t fn = (smc_fn_t)smc_buf;
    int r0 = fn();
    check(r0 == 0);

    /* Phase 2 -- overwrite immediate, fence.i, re-execute, expect 42. */
    smc_buf[0] = ENCODE_ADDI_A0_ZERO(42);
    asm volatile ("fence.i" ::: "memory");
    int r1 = fn();
    check(r1 == 42);

    printf("smc: OK\n");
}
```

Style follows the existing am-test files (e.g.
`xkernels/tests/am-tests/src/tests/trap-ecall.c`): one `#include
"test.h"`, a single `void test_*(void)` entry point, `check()`
assertions, a trailing `printf` line for visible confirmation. The
`HIT GOOD TRAP` marker for the outer Makefile grep
(`xkernels/tests/am-tests/Makefile:36`) comes from the am-tests
runtime on normal `main` return (`halt(0)`).

[**API Surface**]

```rust
impl ICache {
    pub fn new() -> Box<Self>;
    #[inline] pub fn index(pc: VirtAddr) -> usize;
}

impl RVCore {
    #[inline]
    fn decode_cached(&mut self, raw: u32) -> XResult<DecodedInst>;
}

impl Mtimer {
    #[inline]
    fn recompute_next_fire(&mut self) {
        self.next_fire_mtime =
            self.mtimecmp.iter().copied().min().unwrap_or(u64::MAX);
    }
}
```

No new public API on `RVCore` / `Bus` / `Mtimer` facades.

[**Constraints**]

- C-1: No benchmark-targeted code.
- C-2 (round-04, R-003 resolved): The Mandatory Verification block in
  §Validation preamble and §Exit Gate header must pass after every
  implementation phase. The block carries both the AGENTS.md §
  Development Standards line-8 form (`make -C xemu fmt / clippy /
  run / test`) **and** the stronger workspace/doc/perf/am-test/boot
  checks. No top-level shorthand is emitted (no repo-root Makefile
  exists).
- C-3: Benchmarks run with `DEBUG=n`.
- C-4 (round-04, R-003 resolved): Workloads launched via `make -C
  resource linux` / `make -C resource linux-2hart` / `make -C
  resource debian` for OS smoke; per-benchmark runs go through
  `scripts/perf/bench.sh`. Never by hand-calling `target/release/xdb`
  directly.
- C-5: No assembly-file modifications.
- C-6: `bash scripts/ci/verify_no_mutex.sh` remains `ok`.
- C-7: Scope ends at P3+P4+P5+P6.
- C-8: `perf-stats` Cargo feature is off by default; release binaries
  ship without it.
- C-9 (round-04, R-004 resolved): No new `unsafe` in the P6 path if
  `from_le_bytes + try_into` lowers to a single aligned load. Proof
  command: `cd xemu && cargo clippy --all-targets -- -D warnings`.
- C-10: No invalidation hooks on CSR / `fence.i` / `sfence.vma` /
  `satp` / trap / RAM stores.
- C-11: No binding gate may depend on a tool absent from the current
  environment. `cargo-asm` is explicitly optional.
- C-12: §B thresholds apply only to the full P3+P4+P5+P6 bundle.

---

## Implement

### Execution Flow

[**Main Flow**]

1. `RVCore::step` calls `self.fetch(bus)?` → `raw: u32`.
2. `idx = ICache::index(self.pc)`.
3. `let line = &mut self.icache.lines[idx]`.
4. `hit = line.pc == self.pc && line.raw == raw`.
5. Hit: `decoded = line.decoded` (POD copy).
6. Miss: `decoded = DECODER.decode(raw)?`; `*line = ICacheLine { pc:
   self.pc, raw, decoded }`.
7. `self.execute(bus, decoded)?`.
8. `Bus::tick()` → `Mtimer::tick()` runs the P3 deadline short-circuit.
9. Guest load/store via `checked_read` / `checked_write` uses the
   P5-inlined TLB-hit path; RAM accesses follow the P6 typed-read
   fast path when size and alignment permit.

[**Failure Flow**]

1. `fetch` traps → `Err(...)` before icache access; cache unchanged.
2. `DECODER.decode(raw)?` errors on miss → return before writing; I-5.
3. `execute` errors after cache hit → line kept; next fetch re-hits
   and traps identically.
4. SMC (I-7, I-18): guest writes new bytes; next fetch reads new
   `raw`; `(pc, raw)` misses; line overwritten; new instruction
   executes. **V-IT-1 witnesses this path via the `a0` return
   value.**
5. `checked_write` traps (PMP / alignment / page-fault): error
   propagates; icache unaffected.
6. P3: `mtime < next_fire_mtime` → `tick` returns before `check_all`;
   correct because no hart's deadline is reached.
7. P6: MMIO / unaligned / odd-size → falls through to memmove;
   identical semantics.

[**State Transition**]

- `icache line (pc, raw match) → hit` → POD copy of `decoded`.
- `icache line (pc or raw mismatch) → miss` → line replaced.
- `icache line (decode fails) → unchanged`; error propagates.
- `guest store to (pc) → next fetch at pc reads new raw → (pc, raw)
  miss → line replaced with re-decoded instruction` (I-18 SMC path).
- `mtimecmp write → next_fire_mtime recomputed` (P3).
- `mtime ≥ next_fire_mtime → check_all runs`; else early return.
- `RAM read/write size ∈ {1,2,4,8} ∧ aligned → typed path`; else
  memmove (P6).
- `MMU TLB hit → inlined load path`; `TLB miss → existing translate
  slow path` (P5).

### Implementation Plan

Each phase must pass the Mandatory Verification block (§Validation
preamble) and `bash scripts/ci/verify_no_mutex.sh` before the next
phase begins.

[**Phase 1 — P3 Mtimer deadline gate**]

1. Add `next_fire_mtime: u64` to `Mtimer` (`mtimer.rs:26-33`), init
   `u64::MAX`.
2. Add `recompute_next_fire` helper (§API Surface).
3. Modify `tick` (`mtimer.rs:112-123`): after the existing
   `SYNC_INTERVAL` sync, short-circuit on
   `self.mtime < self.next_fire_mtime`.
4. Call `recompute_next_fire` in `write` after every `Mtimecmp`
   mutation (`mtimer.rs:95-108`), at the end of `check_timer`, and at
   the end of `reset` (`mtimer.rs:129-139`).
5. Unit test V-UT-8 + V-F-5 + V-E-4 land in the same commit.

[**Phase 2a — SMC am-test (pre-icache, round-04 R-001/R-002)**]

Three-file harness wiring + one new source file. Every edit below is
load-bearing — **without all three dispatch edits, letter `m` falls
through `main.c:58-64` (default help path), returns 0, and produces a
vacuous `HIT GOOD TRAP` pass.** Reviewer's R-001 concern is recorded
as a one-time baseline probe (step 1d) before the wiring lands.

1a. `xkernels/tests/am-tests/include/amtest.h` — add a new forward
    declaration after the existing `test_float` declaration at
    line 13:

    ```c
    void test_float(void);
    void test_smc(void);        /* <-- new */

    #endif
    ```

1b. `xkernels/tests/am-tests/src/main.c` — two edits:

    - In the `descriptions[]` table (lines 14-27), add one row between
      the `['f']` entry at line 24 and the `['a']` entry at line 25:

      ```c
      ['f'] = "float:       F/D floating-point",
      ['m'] = "smc:         self-modifying code + fence.i",   /* <-- new */
      ['a'] = "Run all tests",
      ```

    - In the `switch (args[0])` block (lines 35-45), add
      `CASE('m', test_smc);` between the `CASE('f', test_float);` at
      line 45 and `case 'a':` at line 46:

      ```c
      CASE('f', test_float);
      CASE('m', test_smc);        /* <-- new */
      case 'a':
      ```

    (Do NOT add `m` to the bulk `'a'` run-all sequence — the SMC test
    is explicitly opt-in; the `ALL` Makefile expansion reaches it via
    `.run.m` independently.)

1c. `xkernels/tests/am-tests/Makefile` — two edits:

    - Line 16: `ALL = u r t s p c e f` becomes `ALL = u r t s p c e f m`.
    - Lines 18-20: extend the `name` substitution chain so `m` maps to
      `smc`. Final form:

      ```make
      name = $(patsubst u,uart-putc,$(patsubst r,timer-read,$(patsubst t,timer-irq,\
             $(patsubst s,soft-irq,$(patsubst p,plic-access,$(patsubst c,csr-warl,\
             $(patsubst e,trap-ecall,$(patsubst f,float,$(patsubst m,smc,$(1))))))))))
      ```

1d. **Baseline sanity probe (one-time, before step 2).** On the
    unmodified repo (letter `m` not yet wired), run
    `make -C xkernels/tests/am-tests run TEST=m` and record the
    output. Expected: `main.c:58-64` default-help printout followed
    by `HIT GOOD TRAP` — the exact vacuous-pass path R-001 warned
    about. Commit the recorded output to
    `docs/perf/hotPath/data/baseline-vacuous-pass.log` as proof that
    the wiring fix is load-bearing. This probe is NOT a standing
    gate; it is a one-time artefact.

2. Create `xkernels/tests/am-tests/src/tests/smc.c` with the content
   shown in §Data Structure above (verified encodings; function-
   pointer observation; `check(r0 == 0)` and `check(r1 == 42)`
   assertions).

3. Verify:

    - `make -C xkernels/tests/am-tests run TEST=m` — letter `m` reports
      PASS. The Makefile's grep for `GOOD TRAP`
      (`xkernels/tests/am-tests/Makefile:36`) now sees a load-bearing
      pass (test actually executed) rather than the default-help
      fallthrough from step 1d.
    - `make -C xkernels/tests/am-tests run` — full suite including
      letter `m` reports PASS.

4. Commit includes only the test file and the three harness edits;
   no Rust changes. This locks the P4 behaviour contract **before**
   the icache lands.

[**Phase 2b — P4 ICache integration**]

1. New file `xcore/src/arch/riscv/cpu/icache.rs` implementing
   `ICache`, `ICacheLine`, `ICACHE_BITS = 12` (§Data Structure).
2. Derive `Copy` on `DecodedInst` in `isa/riscv/decoder.rs:161`.
3. V-UT-1: `fn _assert_copy<T: Copy>() {}` against `DecodedInst`.
4. Add `icache: Box<ICache>` to `RVCore` (`cpu.rs:36-54`);
   initialise in `RVCore::new` via `ICache::new()`.
5. Add `decode_cached` (§API Surface).
6. Replace `self.decode(raw)?` in `cpu.rs:238-245` with
   `self.decode_cached(raw)?`.
7. Cargo feature `perf-stats` in `xemu/xcore/Cargo.toml`, off by
   default; behind it, add hit/miss counters on `ICache` with `&mut`
   increment on each lookup and a dump-at-exit hook.
8. V-UT-2 (cache miss when raw changes at same pc) + V-UT-3b (Rust
   SMC mirror) land in the same commit.
9. Re-run `make -C xkernels/tests/am-tests run TEST=m`; test still
   PASSes — the second fetch reads a different `raw` and `(pc, raw)`
   misses → line overwritten with the new decoded instruction. Now
   the `check(r1 == 42)` assertion genuinely exercises the ICache
   miss-on-raw-change path.

[**Phase 3 — P5 MMU fast-path inline**]

1. Identify the TLB-hit call chain: `checked_read` / `checked_write`
   / `access_bus` / `Bus::read` / `Bus::write` / `Ram::read` /
   `Ram::write`.
2. Add `#[inline]` / `#[inline(always)]` where the pinned rustc + LTO
   does not already fold. No algorithmic change.
3. Capture primary evidence: run
   `bash scripts/perf/sample.sh --out docs/perf/<post-hotPath-date>`
   and `python3 scripts/perf/render.py --dir
   docs/perf/<post-hotPath-date>`. Compare the MMU-entry bucket
   against `docs/perf/2026-04-15/data/*.sample.txt`. Commit the
   post-phase `sample.txt` files for the gate evidence.
4. Optional author-side spot-check (not a gate, not committed):
   `cargo rustc --release -p xcore -- --emit=asm`.
5. Trap slim DROPPED (round-02 R-003 Option A, preserved).

[**Phase 4 — P6 memmove typed-read bypass**]

1. In `Ram::read` (`xcore/src/device/ram.rs`), size-match on 1/2/4/8
   with alignment check; use
   `u{8,16,32,64}::from_le_bytes(slice[..N].try_into()?)`.
2. In `Ram::write`, mirror with `to_le_bytes`.
3. MMIO and unaligned/odd-size paths untouched.
4. If the bucket delta fails to improve under the pinned rustc, fall
   back to `ptr::read_unaligned` with explicit `// SAFETY:` comment
   (alignment / in-bounds / no-aliasing).
5. V-UT-9 + V-E-5 + V-E-6 land in the same commit.

[**Phase 5 — Benchmark capture + final gate**]

1. `bash scripts/perf/bench.sh --out docs/perf/<post-hotPath-date>`
   (3 iters × 3 workloads) with `DEBUG=n`.
2. `bash scripts/perf/sample.sh --out docs/perf/<post-hotPath-date>`.
3. `python3 scripts/perf/render.py --dir
   docs/perf/<post-hotPath-date>`.
4. Capture hit rate via `cargo build --release -p xcore --features
   perf-stats` + workload run (stats dumped at exit).
5. Diff `data/bench.csv` vs `docs/perf/2026-04-15/data/bench.csv`;
   compare self-time bucket tables against §Exit Gate §A / §B.
6. Run the full Mandatory Verification block from §Validation
   preamble.

## Trade-offs

- **T-1: Decoded-raw cache vs. translation-context cache.** (Round
  02, TR-1 adopted.) Preserved.
- **T-2: Bundled round vs. four separate rounds.** (Round 02, TR-2
  adopted.) Preserved; any §A miss splits into a fresh PLAN
  (round-03 R-004 removed the escape hatch).
- **T-3..T-7:** Preserved from round 03 (round-01 R-001 collapse
  correctness; trap-slim drop; safe-first typed read; mtimer recompute
  timing; threaded-dispatch out-of-scope).
- **T-8: `cargo-asm` gate vs. profile-based evidence.** Preserved
  from round 03. Profile-based (chosen).
- **T-9: Escape hatch vs. binary phase attribution.** Preserved from
  round 03. Binary attribution (chosen).
- **T-10 (round-04, R-001): three-file harness wiring vs. deferred
  dispatch-layer refactor.**
  - *Deferred (rejected):* keep round-03's Makefile-only edit and
    plan a follow-up PR to re-wire dispatch. Reviewer showed this
    yields a vacuous `HIT GOOD TRAP` pass through `main.c:58-64`,
    which is load-bearing for the P4 gate. Unacceptable.
  - *Three-file wiring (chosen):* `amtest.h` + `main.c` + `Makefile`
    in one commit. One extra declaration, one `descriptions[]` row,
    one `CASE` arm, one `ALL` letter, one `name` substitution stage.
    ≈ 5 edited lines total, zero design churn, and the gate becomes
    genuinely binding.
- **T-11 (round-04, R-002 / TR-1): `a0` return vs. memory-slot
  observation for SMC.**
  - *Memory slot (Option 2, rejected):* the RAM function writes a
    known slot and C reads it after return. Adds scaffolding (store
    encoding, extra register operand) and multiplies the number of
    encodings the test depends on.
  - *`a0` return (Option 1, chosen, per TR-1):* two-instruction
    function `addi a0, zero, imm; ret`. Single observable channel.
    Both encodings are in Table 24.2 and are stable under any
    RVA20/RVA22/RVA23 profile. The C-level call returns through the
    standard ABI; `check(ret == expected)` is a one-liner.

Sources:

- QEMU TB cache maintenance:
  https://github.com/qemu/qemu/blob/master/accel/tcg/tb-maint.c
- QEMU per-CPU jump cache:
  https://github.com/qemu/qemu/blob/master/include/exec/tb-jmp-cache.h
- NEMU IBuf: https://github.com/OpenXiangShan/NEMU
- rvemu (baseline, no cache): https://github.com/d0iasm/rvemu
- **RISC-V Unprivileged ISA Manual v2.2 §5.1 (Zifencei) + Ch. 2 +
  Table 24.2 (addi / jalr encodings):**
  https://github.com/riscv/riscv-isa-manual/releases
- **RISC-V Privileged ISA Manual §3.1.10 (`mtime` / `mtimecmp`) +
  §3.1.12 (Sstc) + Zicsr §3.1:** same release index.
- **RISC-V psABI / RV G-ABI (`a0` return-register convention):**
  https://github.com/riscv-non-isa/riscv-elf-psabi-doc
- `docs/PERF_DEV.md` §3 P3/P4/P5/P6 exit conditions:
  `/Users/anekoique/ProjectX/docs/PERF_DEV.md`.
- `AGENTS.md` §Development Standards line 8 (mandatory command list):
  `/Users/anekoique/ProjectX/AGENTS.md`.
- Style templates: `xemu/xcore/src/device/bus.rs` (M-001 sentinel
  doc-test pattern); `xemu/xcore/src/arch/riscv/cpu/mm.rs` (free-
  function-style `#[inline]` helpers on the ISA hot path);
  `xemu/xcore/src/arch/riscv/device/aclint/mtimer.rs` (Mtimer style);
  `xkernels/tests/am-tests/src/tests/trap-ecall.c` (am-test C style).

## Validation

**Mandatory Verification block (round-04 R-003 resolved, TR-2 adopted).**
Run after every implementation phase and once more at the final Exit
Gate. Every command is a runnable invocation against a concrete
Makefile target or committed script; no top-level shorthand.

```sh
# Direct xemu make-target coverage (AGENTS.md §Development Standards line 8):
make -C xemu fmt         # xemu/Makefile:50-51; passes iff `cargo fmt --all` reports no diff.
make -C xemu clippy      # xemu/Makefile:47-48; passes iff no new clippy warnings in production crates.
make -C xemu run         # xemu/Makefile:44-45; `cargo run` smoke; exits after load without error.
make -C xemu test        # xemu/Makefile:53-54; `cargo test -p xcore` -- unit + doc tests on xcore crate.

# Workspace-level test coverage beyond xcore-only:
cd xemu && cargo test --workspace    # xcore + xdb + xlogger + arch_isolation green.
cd xemu && cargo test --doc          # M-001 compile_fail sentinel at device/bus.rs.

# M-001 regression gate (from P1):
bash scripts/ci/verify_no_mutex.sh   # prints `verify_no_mutex: ok`; exits 0.

# Benchmark wall-clock (all three workloads, 3 iters each):
bash scripts/perf/bench.sh --out docs/perf/<post-hotPath-date>
                                     # dhry >= 20 %, cm >= 20 %, mb >= 15 % vs docs/perf/2026-04-15/data/bench.csv.

# Sampling profile for §A.P4 icache hit-rate + §A.P5 MMU bucket + §A.P6 memmove bucket:
bash scripts/perf/sample.sh --out docs/perf/<post-hotPath-date>
python3 scripts/perf/render.py --dir docs/perf/<post-hotPath-date>
                                     # Passes iff MMU bucket drops >= 3 pp AND memmove+memcpy bucket < 2 %.

# am-test bundle (required for P4 SMC gate):
make -C xkernels/tests/am-tests run             # full suite; smc covered by letter `m`.
make -C xkernels/tests/am-tests run TEST=m      # targeted: SMC torture test alone.

# OS boots (required for §A.P5 MMU regression check):
make -C resource linux          # resource/Makefile:32 (linux:run-linux); boot to prompt within +/- 5 % of post-P1.
make -C resource linux-2hart    # resource/Makefile:33; same.
make -C resource debian         # resource/debian.mk:54 (debian:run-debian); same.
```

Note: there is no top-level `Makefile` at `/Users/anekoique/ProjectX/`,
so `make fmt` / `make clippy` / `make run` / `make test` without a
`-C` subdir do **not** run; they are forbidden by C-2.

[**Unit Tests**]

- V-UT-1: `decoded_inst_is_copy` in `isa/riscv/decoder.rs::tests` —
  static `_assert_copy<DecodedInst>()`. Pins POD property.
- V-UT-2: `icache_miss_when_raw_changes_at_same_pc` in
  `cpu/icache.rs::tests` — store line at `(pc, raw_a)`, lookup at
  `(pc, raw_b)`, assert miss. Pins I-12.
- V-UT-3b: `smc_raw_mismatch_reindexes_line` in `cpu/icache.rs::tests`
  — build an `ICache`, insert line at `(pc, raw_a)`, re-lookup at
  `(pc, raw_b)`, assert the subsequent re-decode overwrites the line
  such that `lines[idx].raw == raw_b`.
- V-UT-8: `mtimer_deadline_short_circuits` in
  `device/aclint/mtimer.rs::tests` — set `mtimecmp[0] = u64::MAX`;
  tick 1000 times; assert `check_all` was not called.
- V-UT-9: `ram_typed_read_matches_memmove` in `device/ram.rs::tests`
  — each of 1/2/4/8 at aligned + unaligned; assert identical output
  vs. reference.

[**Integration Tests**]

- **V-IT-1 (round-04 R-001 / R-002 binding):** am-test
  `xkernels/tests/am-tests/src/tests/smc.c` (letter `m`) passes
  under `make -C xkernels/tests/am-tests run` AND under
  `make -C xkernels/tests/am-tests run TEST=m`. Success is detected
  by the existing `HIT GOOD TRAP` marker grep
  (`xkernels/tests/am-tests/Makefile:36`). The test observes its
  result via the C function-return channel (register `a0`, RV
  psABI) on a RAM-resident `addi a0, zero, imm; ret` sequence that
  is overwritten between two calls. Invariant I-18 witness.
- V-IT-2: Wall-clock on dhry / cm / mb via
  `bash scripts/perf/bench.sh`; deltas vs
  `docs/perf/2026-04-15/data/bench.csv`.
- V-IT-3: `make -C resource linux`, `make -C resource linux-2hart`,
  `make -C resource debian` all boot; latency within ±5 % of post-P1.
- V-IT-4: Hit-rate telemetry via `--features perf-stats`, dumped at
  exit. Threshold ≥ 95 % on dhry / cm / mb.
- **V-IT-5:** MMU bucket drop ≥ 3 pp between
  `docs/perf/2026-04-15/data/*.sample.txt` and
  `docs/perf/<post-hotPath-date>/data/*.sample.txt`.
- V-IT-6: `_platform_memmove + memcpy` combined < 2 % on dhry / cm /
  mb; `Bus::read + Bus::write` combined drops ≥ 3 pp.

[**Failure / Robustness Validation**]

- V-F-1: `decode_failure_does_not_poison_line`. Pins I-5.
- V-F-2: `mid_execution_bytes_change_triggers_miss` — Rust-level
  companion to V-IT-1. Pins I-7 / I-18 at the Rust layer.
- V-F-5: `mtimer_reset_restores_deadline_to_max`.

[**Edge Case Validation**]

- V-E-1: `compressed_and_full_inst_at_adjacent_pc`. Pins I-6 / I-11.
- V-E-2: `index_aliasing_at_conflict`.
- V-E-4: `mtimecmp_write_to_u64_max_sets_deadline_max`.
- V-E-5: `ram_read_size_3_falls_through_to_memmove`.
- V-E-6: `mmio_read_takes_device_path_not_typed`.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (P3 Mtimer gate) | V-UT-8, V-F-5, V-E-4, Exit Gate §A P3 |
| G-2 (P4 hit ≥ 95 %) | V-UT-1, V-UT-2, V-IT-4, Exit Gate §A P4 |
| G-3 (P5 MMU inline) | V-IT-5 (profile bucket delta), Exit Gate §A P5 |
| G-4 (P6 memmove) | V-UT-9, V-IT-6, V-E-5, V-E-6, Exit Gate §A P6 |
| G-5 (wall-clock) | V-IT-2, Exit Gate §B |
| G-6 (xdb::main drop) | V-IT-2 (re-run sample.sh) |
| G-7 (correctness, incl. SMC am-test) | **V-IT-1**, V-F-1, V-F-2, V-UT-3b, V-IT-3, `cd xemu && cargo test --workspace`, `make -C xemu test`, `make -C xkernels/tests/am-tests run` |
| G-8 (mutex-free) | `bash scripts/ci/verify_no_mutex.sh` |
| C-1 (no workload switch) | Review grep for workload names |
| C-2 (mandated commands) | §Validation Mandatory block + §Exit Gate header (verbatim block) |
| C-4 (launch via make -C / scripts) | §Validation Mandatory block |
| C-6 (no Mutex regression) | `bash scripts/ci/verify_no_mutex.sh` |
| C-8 (perf-stats off) | `Cargo.toml` default-features inspection |
| **C-9 (no new unsafe, round-04 R-004)** | **`cd xemu && cargo clippy --all-targets -- -D warnings`** |
| C-10 (no invalidation hooks) | §Architecture §P4, §Invariants |
| C-11 (no cargo-asm gate) | V-IT-5 uses profile delta only |
| C-12 (§B not reinterpreted) | §Exit Gate §B preamble |
| I-6 (compressed) | V-E-1 |
| I-7 (SMC implicit) | V-F-2, V-UT-3b, **V-IT-1** |
| I-9 (fence.i NOP) | code audit `inst/privileged.rs:71-79` unchanged |
| I-10 (no Mutex) | `bash scripts/ci/verify_no_mutex.sh` |
| I-11 (geometry + aliasing) | V-E-2 |
| I-12 (miss rule) | V-UT-2 |
| I-16 (mtimer deadline) | V-UT-8, V-F-5, V-E-4 |
| I-17 (P6 bypass conditions) | V-UT-9, V-E-5, V-E-6 |
| **I-18 (SMC torture contract via `a0`)** | **V-IT-1** (primary), V-F-2 + V-UT-3b (Rust mirrors) |

---

## Exit Gate

**Mandatory Verification block (round-04, identical to §Validation
preamble):**

```sh
make -C xemu fmt                              # rustfmt clean
make -C xemu clippy                           # no new warnings
make -C xemu run                              # cargo run smoke
make -C xemu test                             # cargo test -p xcore
cd xemu && cargo test --workspace             # workspace tests green
cd xemu && cargo test --doc                   # compile_fail sentinel green
cd xemu && cargo clippy --all-targets -- -D warnings   # C-9 proof (round-04 R-004)
bash scripts/ci/verify_no_mutex.sh            # `ok`
make -C xkernels/tests/am-tests run           # every letter in ALL passes; letter `m` MUST be present and green
make -C xkernels/tests/am-tests run TEST=m    # targeted SMC gate
bash scripts/perf/bench.sh --out docs/perf/<post-hotPath-date>
bash scripts/perf/sample.sh --out docs/perf/<post-hotPath-date>
python3 scripts/perf/render.py --dir docs/perf/<post-hotPath-date>
make -C resource linux
make -C resource linux-2hart
make -C resource debian
```

No top-level shorthand. Every line is a runnable invocation.

### §A — Per-phase binding gates

**Policy:** If any §A sub-gate misses, the failing phase is NOT
considered landed and a fresh PLAN iteration must be opened for that
phase. Attribution is binary. There is no reduced-bundle
re-interpretation of §B.

- **P3 (Mtimer).** Combined `Mtimer::check_timer + tick + mtime`
  self-time bucket < 1 % on dhry / cm / mb under
  `docs/perf/<post-hotPath-date>/data/*.sample.txt`.
  `make -C xkernels/tests/am-tests run` passes.
  Linux / Linux-2hart / Debian boot latency within ±5 % of post-P1.
- **P4 (icache).** `xdb::main` self-time share drops by ≥ 10 pp on
  all three workloads (from 40 / 47 / 45 % → ≤ 30 / 37 / 35 %).
  Icache hit rate ≥ 95 % on dhry / cm / mb under `--features
  perf-stats`. **V-IT-1: am-test `smc.c` (letter `m`) passes under
  both `make -C xkernels/tests/am-tests run` and `make -C
  xkernels/tests/am-tests run TEST=m`; letter `m` is wired in
  `amtest.h`, `main.c` (`descriptions[]` + `CASE`), and `Makefile`
  (`ALL` + `name` substitution). Observation channel is the C
  function-return value via `a0` (RV psABI).**
- **P5 (MMU inline).** MMU-entry bucket
  (`access_bus + checked_read + checked_write + load + store`) drops
  ≥ 3 pp between `docs/perf/2026-04-15/data/*.sample.txt` and
  `docs/perf/<post-hotPath-date>/data/*.sample.txt`. Evidence via
  `scripts/perf/sample.sh` + `render.py` only; no `cargo-asm`.
  Trap bucket is NOT part of this gate.
- **P6 (memmove).** `_platform_memmove + memcpy` combined bucket
  drops below 2 % on dhry / cm / mb. `Bus::read + Bus::write`
  combined drops by ≥ 3 pp.

### §B — Combined bundle gates

**Preamble:** §B thresholds are defined only for the full
P3+P4+P5+P6 bundle. If any §A sub-gate misses and a phase splits,
§B is re-evaluated from scratch in the follow-up PLAN.

- Wall-clock: dhry ≥ 20 %, cm ≥ 20 %, mb ≥ 15 % vs
  `docs/perf/2026-04-15/data/bench.csv`.
- `xdb::main` self-time share drops by ≥ 10 pp on all three workloads.
- `bash scripts/ci/verify_no_mutex.sh` reports `ok`.
- `make -C xemu clippy` clean (no new warnings in production crates).
- `cd xemu && cargo clippy --all-targets -- -D warnings` green.
- `cd xemu && cargo test --workspace` green.
- `make -C xemu run` smoke passes; `make -C xemu test` green.
- `make -C xkernels/tests/am-tests run` green (letter `m` included
  and load-bearing).
- `make -C resource linux`, `make -C resource linux-2hart`, `make -C
  resource debian` boot to prompt within ±5 % of post-P1.

### Summary

No benchmark-targeted tricks. No `Mutex<Bus>` regression. No scope
leak beyond P3+P4+P5+P6. `fence.i` remains a NOP. The SMC am-test is
wired through all three dispatch layers (`amtest.h`, `main.c`,
`Makefile`) and observes its result via the C-ABI `a0` return
register, so it is genuinely load-bearing. The Mandatory Verification
block includes `make -C xemu run` and `make -C xemu test` per
AGENTS.md §Development Standards plus stronger workspace/doc/perf/
am-test/boot checks. Acceptance Mapping C-9 proof is executable
(`cd xemu && cargo clippy --all-targets -- -D warnings`). Escape
hatch removed; any §A miss opens a fresh PLAN iteration for that
phase.

**This plan is implementation-ready; no open design questions remain;
the next executor action is to write code per the Implementation
Steps.**
