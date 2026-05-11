# `perfBusFastPath` REVIEW `01`

> Status: Open
> Feature: `perfBusFastPath`
> Iteration: `01`
> Owner: Reviewer
> Target Plan: `01_PLAN.md`
> Review Scope:
>
> - Plan Correctness
> - Spec Alignment
> - Design Soundness
> - Validation Adequacy
> - Trade-off Advice

---

## Verdict

- Decision: Rejected (Revise and rerun)
- Blocking Issues: 4 (1 CRITICAL, 3 HIGH)
- Non-Blocking Issues: 4 (3 MEDIUM, 1 LOW)

The round-01 plan does an honest job of addressing 00_REVIEW R-001..R-009 in
form: every prior R-ID has a Response Matrix row, source URLs are added per
R-008, the criterion bench is correctly downgraded per R-009, the API split
per R-007 is clean, and the gain band is softened per R-003. The migration
table grew, the reentry invariant is added, and the ownership factory was
reshaped.

However, the plan does not survive a substantive audit:

1. The premise it inherits from 00_PLAN — that the `Shared` arm must remain
   `Arc<Mutex<Bus>>` to preserve byte-identical multi-hart semantics — is
   not justified by the actual concurrency model in this codebase. Multi-hart
   in xemu is a single-threaded round-robin scheduler (`CPU::step` →
   `advance_current()` at `cpu/mod.rs:213-249`); the only OS-thread crossing
   is the UART stdin reader which already owns its own
   `Arc<Mutex<VecDeque<u8>>>`. There is therefore no concurrent `Bus`
   access to protect. `Mutex<Bus>` is dead weight in both arms, not just the
   single-hart arm. 01_MASTER M-001 raises exactly this point. The plan does
   not address it.
2. The R-001 migration table claims completeness ("every site from
   `rg 'bus\.lock\(\)' xemu/xcore/src`") but five chained-call test-helper
   sites are missing because the executor's grep was line-anchored and did
   not catch the multi-line `core.bus\n.lock()\n.unwrap()...` formatting.
3. The R-002 V-UT-5 test design names a frozen baseline constant but does
   not specify how the constant is captured (which SHA / branch / fixture /
   instruction stream); this is not implementable as written.
4. The architecture text describing how `Core` and `CPU` share the `Owned`
   bus introduces a "re-borrow handle that is reset each call" concept that
   does not type-check in Rust as written; the actual per-step ownership
   story for `Core::bus` in single-hart mode is left unspecified.

The R-005 reshape is a partial fix (the panic is gone, but `into_handles`
still takes `num_harts` at runtime — not a true type-state). The R-003
bucket math contradicts itself: the reasoning offered in G-2 derives a
10-30 % wall-clock ceiling, then names 35 % as the theoretical ceiling
without reconciling the two figures.

The plan is structurally implementable and the executor has done careful
work, but the premise gap (C-1 above) deserves explicit response before
implementation begins.



## Summary

Round-01 carries forward `BusHandle { Owned(Box<Bus>), Shared(Arc<Mutex<Bus>>) }`
unchanged in shape from 00_PLAN. Adds I-9 reentry invariant with a
`Cell<bool>` debug guard, replaces `clone_for_hart` with
`BusOwner::into_handles(num_harts)`, splits `CPU::bus` / `CPU::bus_mut` per
R-007, expands the migration table, adds V-UT-5 (lock-width counter) and
V-IT-7 (2-hart torture), V-IT-8 (`cargo asm` no-CAS check), softens the
exit-gate band to 15 % floor / 20-30 % expected / 35 % ceiling, and adds
source URLs.

The plan is well-organised and the Response Matrix is honest. Rejections
are absent because no 00_REVIEW finding was rejected. Trade-offs T-1..T-4
are committed (T-2 adopting reviewer Option C, T-4 declining to fuse
`store_op`'s `clear_reservation` per the user-brief constraint).

What blocks approval is not the form of the plan but its substance:
the `Shared` arm exists to protect against concurrent access that does
not occur in this codebase; the migration table that proves G-1 is not
literally complete; the V-UT-5 baseline is under-specified; and the
single-hart ownership story for `Core::bus` is hand-waved.

---

## Findings

### C-1 `Shared` arm preserves a Mutex that protects nothing in this codebase

- Severity: CRITICAL
- Section: `Spec / Goals (G-3), Architecture, Trade-offs (T-1)`
- Type: Spec Alignment / Design Soundness
- Problem:
  The plan's two-arm design (`Owned` for single-hart, `Shared` for
  multi-hart) presumes that multi-hart configurations have concurrent
  `Bus` access from multiple OS threads, and therefore need
  `Arc<Mutex<Bus>>` for safety. The actual code does not work that way.
  `CPU::step` (`xemu/xcore/src/cpu/mod.rs:213-243`) runs one hart per
  call and round-robins via `advance_current()`; `CPU::run` is a plain
  `for` loop over `step()`. There is no `thread::spawn` anywhere in
  the per-hart execution path — only `device/uart.rs:94-100` spawns a
  background reader, and it owns its own
  `Arc<Mutex<VecDeque<u8>>>` for the rx buffer. No second OS thread
  ever holds `&mut Bus`.
  01_MASTER M-001 asks exactly this question:
  "Does the Bus really need the structure of Mutex? Our implementation
  of multi-hart is not multi-thread which can be accessed safely
  without lock?" The plan does not respond to this premise.
- Why it matters:
  The `Shared` arm preserves overhead the codebase doesn't need. Today
  every single bus access on a multi-hart guest pays
  `pthread_mutex_lock`/`unlock` for a lock that has zero contention by
  construction. The same structural fix that this plan applies to
  single-hart (replace `Arc<Mutex<Bus>>` with direct ownership) would
  apply uniformly: make `bus: Box<Bus>` on `CPU`, share with `Core`s
  via `&mut Bus` passed per step (or `Rc<RefCell<Bus>>` if a stored
  field is required). The discriminant check in `BusHandle::with` /
  `with_guard` then disappears, the `Shared` arm disappears, the
  `BusOwner::into_handles` factory becomes trivial, and G-2's gain
  applies to `linux-2hart` too instead of being capped at single-hart.
  Concretely, the plan's exit gate G-2 (≥ 15 % single-hart wall-clock
  reduction) is achievable on `linux-2hart` as well under the simpler
  design, and the C-4 constraint (`linux-2hart ±5 %`) becomes
  trivially satisfied by being faster.
  This is also a maintainability win: a single ownership shape across
  configurations, no `match` in the hot path, no `BusHandle` enum
  surface to teach.
- Recommendation:
  In `02_PLAN`, either:
  - (a) Adopt the simpler design — `bus: Box<Bus>` on `CPU` and pass
    `&mut Bus` through each hart's step. Drop `BusHandle`, `BusGuard`,
    `BusOwner` entirely. Re-derive the call-site migration table:
    every `bus.lock().unwrap()` becomes a direct `bus.` call (or a
    direct `&mut Bus` parameter). Re-derive the gain band: G-2 should
    apply to `linux-2hart` as well; the C-4 ±5 % becomes "must be
    faster, not within ±5 %".
  - (b) Justify the `Shared` arm explicitly: identify the OS-thread
    boundary that requires `Arc<Mutex<Bus>>` (no candidate exists in
    today's tree), or commit to a future MTTCG-style design that
    introduces such a thread. If (b), the plan must show what that
    design looks like and why P1 must pre-commit to its lock shape.
  Reviewer recommends (a) on parsimony grounds; the plan's own
  evidence (Trade-offs T-1, surveying rvemu / Rare / riscv-rust /
  rrs) shows that owned-bus shapes are the norm in single-threaded
  Rust RISC-V emulators, and xemu's multi-hart implementation is
  single-threaded.



### H-1 R-001 migration table is not the literal "every site" the plan claims

- Severity: HIGH
- Section: `Implement / Phase 2 — Migration table`
- Type: Correctness / Validation
- Problem:
  The plan asserts the table is built from "every site from
  `rg 'bus\.lock\(\)' xemu/xcore/src`" (lines 593-594) and that after
  Phase 2 "`rg 'bus\.lock\(\)' xemu/xcore/src` returns zero matches"
  (line 632). A multi-line-aware grep shows the executor's grep was
  line-anchored and missed every multi-line chained-call site.
  Specifically, the migration table omits:
  - `xemu/xcore/src/arch/riscv/cpu/inst/base.rs:344-348` —
    `write_bytes` test helper, `core.bus.lock().unwrap().load_ram(...)`
    spread over four lines.
  - `xemu/xcore/src/arch/riscv/cpu/inst/base.rs:351-356` —
    `read_word` test helper, same pattern.
  - `xemu/xcore/src/arch/riscv/cpu/inst/compressed.rs:552-556` —
    `core.bus.lock().unwrap().write(...)` test helper (the table only
    lists line 573 of the same file).
  - `xemu/xcore/src/arch/riscv/cpu/inst/float.rs:1075-1079` —
    `core.bus.lock().unwrap().write(...)` test helper (the table only
    lists line 1085 of the same file).
  - `xemu/xcore/src/arch/riscv/cpu.rs:277-282` — `write_inst` test
    helper, `core.bus.lock().unwrap().write(...)`.
  All five missed sites are in `#[cfg(test)]` modules, so they are
  not on any guest-exec hot path. Compile-time impact: the type
  change to `bus: BusHandle` will break each of these five sites —
  the compiler will catch them. Plan-validation impact: the executor
  cannot reason about "what migration each site needs" if the site
  is absent from the table, and the post-Phase-2 grep gate
  (`rg returns zero matches`) is satisfiable by the literal regex
  but not by the spirit of the gate (the chained sites contain
  `bus\.lock\(\)` only when the multi-line buffer is searched).
- Why it matters:
  R-001 is the HIGH-severity 00_REVIEW finding the plan claims to
  resolve. If "every site is enumerated" turns out to be "every
  line-anchored site", the plan's response is rhetorical, not
  substantive. The next implementation round will fix these sites
  by accident (compile errors) but the Phase 2 commit-by-commit
  checkpoint pattern relies on the table being authoritative. A
  reviewer cannot tell, from the plan alone, that the executor has
  reasoned about test-helper migration patterns.
- Recommendation:
  In `02_PLAN`, rebuild the migration table from a multi-line-aware
  grep (e.g. `rg --multiline --multiline-dotall 'bus[\s]*\.[\s]*lock\(\)' xemu/xcore/src`).
  Add the five missed test-helper rows. For the Phase-2 gate, change
  the assertion to "no chained or inline `bus.lock()` form remains,
  verified by `rg --multiline`". State the migration pattern for each
  (`with` / `with_ref` / `with_guard`).



### H-2 V-UT-5 baseline constant is under-specified — not implementable

- Severity: HIGH
- Section: `Validation / Unit Tests / V-UT-5`
- Type: Validation
- Problem:
  V-UT-5 reads (lines 774-783):
  "Wrap a `Shared` `BusHandle` with a test-only counter ... and run a
  fixed 64-instruction golden trace (dhrystone's inner loop, 1
  iteration). Capture the pre-P1 acquire-count as a frozen constant
  (`const PRE_P1_LOCK_ACQUIRES: usize = N;`); assert post-P1 count
  equals or beats `N`."
  Three things are missing for this to be implementable:
  1. **Which 64 instructions?** "Dhrystone's inner loop" is not
     uniquely defined — `dhry_1` has multiple loops, the compiled
     binary varies by toolchain version, and there is no committed
     fixture file in the repo for "the dhrystone golden trace".
  2. **Where does `N` come from?** The plan does not say which
     branch / commit SHA the executor runs to capture the baseline,
     or how to mechanically reproduce that capture in CI when
     reviewers want to verify the constant.
  3. **What is "the counter"?** The plan says "test-only counter
     (increment on every `lock()` call; `#[cfg(test)]`-only)" but
     does not say where the counter lives — on `BusHandle::Shared`
     itself? On a wrapper type? Inside `with` / `with_guard`'s
     bodies? The implementation choice affects whether nested
     `with_guard`s count as one or two acquires.
- Why it matters:
  R-002 is the second HIGH finding from 00_REVIEW. Its purpose is to
  catch "did this round silently widen the Shared critical section?"
  A test that cannot be implemented without the executor inventing
  a fixture, picking a baseline SHA, and choosing a counter site is
  not a regression gate — it is a wish. If three implementers pick
  three different "dhrystone golden traces", three different baseline
  Ns will land in tree, and the gate will measure nothing useful.
  The plan also names V-UT-5 in C-8 ("V-UT-5 (counter test) proves
  the Shared-arm critical-section acquire count does not increase")
  and in the Acceptance Mapping. If V-UT-5 is unimplementable, C-8
  has no proof.
- Recommendation:
  In `02_PLAN`, specify:
  - **Trace fixture:** a hand-written 64-instruction `.S` file at a
    committed path (e.g.
    `xemu/xcore/tests/fixtures/lock_width_trace.S`),
    deterministically assembled, no toolchain dependency. Or a
    `Vec<u32>` of raw encodings hard-coded in the test file. State
    that the fixture covers at least one of each: `lr`, `sc`,
    `amoadd`, regular `sw`, regular `lw`, page-table walk.
  - **Baseline procedure:** pin the baseline-capture branch
    (`main@<SHA>`) and document the exact `cargo test` invocation
    that prints `N`. Commit the resulting constant with a comment
    citing both the SHA and the trace file.
  - **Counter site:** put the counter on a `#[cfg(test)]` wrapper
    `CountedBus(Bus, AtomicUsize)` (or similar) so the count is
    independent of how `with` / `with_guard` are layered.
  Or, alternatively, replace V-UT-5 with the criterion microbench
  approach from the 00_REVIEW R-002 recommendation (option (b)) and
  make it a hard exit gate.



### H-3 Single-hart `Core::bus` ownership story is hand-waved and not implementable as written

- Severity: HIGH
- Section: `Spec / Architecture` (lines 293-302)
- Type: Design Soundness / Correctness
- Problem:
  The architecture paragraph at lines 293-302 says:
  "the bus physically lives in `CPU`'s `BusHandle::Owned` and each
  `Core` holds its own `BusHandle::Owned` to a separate `Box<Bus>` is
  not the design — rather, when `num_harts == 1` there is one
  `BusHandle` total (on `CPU`) and the single `Core` receives
  `&mut Bus` via `with` / `with_guard` during its step. The `Core`'s
  `bus` field for single-hart mode is a re-borrow handle that is
  reset each call."
  This does not type-check. There is no Rust construct for a struct
  field that holds a "re-borrow handle ... reset each call". A `Core`
  cannot have a `bus: &'? mut Bus` field without a lifetime parameter
  on `Core`, and a struct field cannot be re-borrowed from outside
  per-step. The Data Structure block (lines 348-388) shows
  `BusHandle` as a self-contained enum with no lifetime parameter,
  and `Core`'s `bus` field is never explicitly typed in this plan.
  The actual implementable designs are:
  - (a) `Core` has no `bus` field. `CPU::step` does
    `self.bus.with(|b| self.cores[self.current].step(b))` and the
    bus is a parameter to every `Core::step` / `RVCore::step`
    method. This requires changing the trait shape of `Core` and
    every per-instruction handler — a big refactor not described
    in the plan.
  - (b) `Core` owns its own `BusHandle::Owned(Box<Bus>)` and the
    `CPU`'s `BusHandle` is `Empty` in single-hart mode, with the
    bus passed to `Core` at construction time. But then `CPU::bus()`
    has nothing to return for the public API, breaking G-5.
  - (c) `Core::bus: BusHandle` mirrors `CPU::bus` and both point at
    the same `Arc<Mutex<Bus>>` even in single-hart — i.e. give up
    `Owned` for the per-`Core` field and only use `Owned` on `CPU`.
    But then `Core::checked_read` etc. still hit the mutex; G-1
    fails.
  None of these are described. The plan needs to pick one and
  enumerate the consequences.
- Why it matters:
  This is the central design question of P1. G-1 (zero
  `pthread_mutex_*` on the hot path) lives or dies on whether
  `Core::checked_read` can reach `&mut Bus` without going through a
  mutex. The hot-path call chain is `CPU::step` → `Core::step`
  (via `cores[current]`) → `RVCore::checked_read` → `self.bus...`.
  If `self.bus` on `RVCore` is a `BusHandle::Shared`, the discriminant
  check still goes through but the underlying access is a mutex —
  G-1 fails. If it's a `BusHandle::Owned`, then who owns the
  `Box<Bus>` between `CPU::step` and the next `CPU::step`? The plan
  must answer this.
- Recommendation:
  In `02_PLAN`, replace lines 293-302 with a concrete typed
  description: state `RVCore`'s `bus` field type, state who owns
  the `Box<Bus>` across step boundaries, and state how `CPU::step`
  hands the `&mut Bus` to `cores[self.current]`. If the design is
  (a) above, enumerate the API change to `Core::step` (a `&mut Bus`
  parameter on every step) and add it to the migration table. If
  (b) or (c), state which goal the design forfeits.
  This finding interacts with C-1: if C-1 (a) is adopted (drop
  `BusHandle`), the simplest option is `bus: Rc<RefCell<Bus>>` on
  both `CPU` and `Core`, with a borrow-on-step pattern.



### M-1 R-005 reshape removes the panic but does not deliver type-state

- Severity: MEDIUM
- Section: `API Surface / BusOwner`
- Type: API / Design Soundness
- Problem:
  The 00_REVIEW R-005 recommendation was either (a) split the factory
  into separate `new_owned` / `new_shared` methods on a type that
  cannot be misused, or (b) make the misuse-prone method return a
  `Result`. The 01_PLAN response (lines 90-93, 400-404) is:
  ```
  pub fn into_handles(self, num_harts: usize) -> Vec<BusHandle>;
  ```
  with the comment "Owned is reachable only when `num_harts == 1`,
  encoded structurally (not by runtime check)" (line 384). This is
  internally inconsistent: `num_harts` is a runtime `usize`
  parameter; the body of `into_handles` must do a runtime branch
  (`if num_harts == 1 { Owned } else { Shared }`); the misuse
  surface ("call with the wrong `num_harts`") is preserved.
  The R-005 panic on `clone_for_hart(Owned)` is gone, which is
  progress. But the new shape doesn't deliver "compile-time-knowable
  misuse", which was the stated R-005 goal. A genuine type-state
  factory looks like:
  ```rust
  impl BusOwner {
      pub fn into_owned(self) -> OwnedBusHandle;          // single hart
      pub fn into_shared(self, n: usize) -> Vec<SharedBusHandle>;
  }
  ```
  where `OwnedBusHandle` and `SharedBusHandle` are different types
  and `CPU<Single>` / `CPU<Multi>` accept the matching one.
- Why it matters:
  Reviewer maintainability rule. The plan claims a type-state
  encoding it does not provide. A future contributor reading
  `into_handles(n)` will not know which `n` produces which arm
  unless they read the body. The refactor cost (split into two
  methods returning two types) is small.
- Recommendation:
  In `02_PLAN`, either:
  - (a) Adopt a real type-state split (two methods, two return
    types) and update the Data Structure / API Surface blocks; or
  - (b) Drop the "encoded structurally" claim in the architecture
    text and state plainly that `into_handles` is a runtime branch;
    declare R-005 partially-resolved-with-rationale.
  This finding is moot if C-1 is accepted (no `BusHandle` enum →
  no `BusOwner` factory).



### M-2 G-2 ceiling math is internally inconsistent

- Severity: MEDIUM
- Section: `Spec / Goals (G-2)`
- Type: Spec Alignment / Correctness of claim
- Problem:
  G-2 (lines 209-219) states the ceiling is "35 %" but the bucket
  math offered in the same paragraph derives a different number:
  "macOS pthread uncontended lock+unlock is roughly 20-40 ns; at
  ~2-3 acquisitions per guest instruction and tens-of-M instr/s,
  mutex work is ~1-3 s of a ~9 s dhrystone run, i.e. **10-30 %** of
  wall-clock even if 100 % of the bucket vanishes."
  10-30 % is the bucket math's own ceiling. The 35 % figure is then
  introduced two sentences later as the "theoretical ceiling only if
  the entire mutex bucket drops and no sample redistributes". But
  the mutex bucket itself is only 10-30 % of wall-clock by this
  reasoning, so the ceiling cannot exceed 30 %. The reported
  pthread_mutex_* self-time of 33-40 % must include sampling
  artifacts (PLT stub, CAS-only fast path) that *would* redistribute
  rather than disappear — the same paragraph acknowledges this.
  The exit gate (lines 856-868) consequently has a "≥ 15 %" floor
  with an "expected" band of 20-30 % and a "ceiling" of 35 % that
  the plan's own math says is unreachable.
- Why it matters:
  R-003 was a MEDIUM finding asking for honest gain math. The plan's
  honesty is undercut by the 35 % figure, which reads as marketing.
  More substantively, if the bucket-math ceiling is 30 %, the
  20-30 % "expected" band is the ceiling, not the expected. That
  changes the exit-gate posture: passing at 18 % is "right at the
  bottom of the realistic band", not "below expected but above
  floor".
- Recommendation:
  In `02_PLAN`, drop the 35 % figure or reconcile it with the
  bucket math. The straightforward edit: keep "≥ 15 % required,
  20-30 % expected (which is also the bucket-math ceiling)" and
  delete the 35 % sentence. If the executor wants to defend 35 %,
  the math needs to show how 35 % wall-clock reduction is reachable
  given a 10-30 % bucket; the only mechanism is "removing the mutex
  also removes secondary cache / branch-predictor / `xdb::main`
  sampling overhead", which the plan should quantify (or label
  speculative).



### M-3 V-IT-7 budget is not justified and 2-hart `Shared` torture is a thread-only test

- Severity: MEDIUM
- Section: `Validation / Integration Tests / V-IT-7`
- Type: Validation
- Problem:
  V-IT-7 (lines 801-806) "build a 2-hart `CPU`, run a hand-written
  LR/SC torture for 10 000 iterations where hart 0 does `lr`, hart 1
  stores in the reserved granule, and hart 0's `sc` must fail on
  every round. Must complete within a 50 ms Rust-test budget."
  Two issues:
  1. The 50 ms budget is unjustified. 10 000 LR/SC iterations on a
     single-threaded scheduler at the project's measured throughput
     (~10s of M instructions/s) is 1-3 ms; if the budget is
     advisory, fine, but stating it as a fail condition without
     justification invites flakiness on slow CI.
  2. The test exercises 2-hart cooperation, but xemu's multi-hart
     scheduler is single-threaded round-robin. There is no race —
     hart 0's `lr` and hart 1's `store` are interleaved
     deterministically by `advance_current()`. The test verifies
     reservation-tracking semantics, not lock-contention behaviour.
     The 00_REVIEW R-002 recommendation was to catch silent
     critical-section widening on the `Shared` arm; V-IT-7 does
     not test for that — it tests SC semantics, which V-E-3
     already covers.
- Why it matters:
  V-IT-7 is named in the Acceptance Mapping for G-3 and G-4 (lines
  841, 842) and in C-8's behavioural-test list (line 491). If it
  doesn't actually catch lock-width regressions, C-8 is proven only
  by V-UT-5 — which itself has implementation gaps (H-2). The
  Shared-arm width gate is then weak.
- Recommendation:
  In `02_PLAN`, either drop V-IT-7's budget claim and reposition
  it as "deterministic SC-semantics regression test (covers V-E-3
  at scale)", or replace it with a counter-based test that
  measures `Shared`-arm acquire count per torture iteration and
  fails on increase relative to a baseline. If C-1 is adopted,
  V-IT-7 is moot (no `Shared` arm).



### L-1 V-IT-8 `cargo asm` gate depends on toolchain not declared as a dev dep

- Severity: LOW
- Section: `Validation / Integration Tests / V-IT-8`, `Phase 3 step 3e`
- Type: Validation / Reproducibility
- Problem:
  Step 3e (lines 648-652) and V-IT-8 (lines 807-810) require
  `cargo asm -p xcore xcore::cpu::CPU::step --rust` and reference
  `cargo-show-asm` / `cargo-asm` parenthetically. These are external
  tools not in the project's standard CI toolchain. The plan does
  not state where the asm-check runs (developer machine? CI? as
  part of `make test`?), how the disassembly is matched against
  "no `lock cmpxchg` / `xchg` / `pthread_mutex_*` symbol", or what
  happens if the developer doesn't have `cargo-show-asm` installed.
  The exit gate (line 866) makes V-IT-8 mandatory.
- Why it matters:
  An exit gate that requires an out-of-tree tool with no fallback
  is brittle. The check is genuinely useful (it directly proves
  G-1) but as written it can't be enforced uniformly.
- Recommendation:
  In `02_PLAN`, either:
  - add `cargo-show-asm` to the project's `tools/` install script
    and `make test` flow, with a CI step that runs the disasm-grep;
    or
  - replace V-IT-8 with a runtime equivalent (e.g. perf-stat the
    `pthread_mutex_lock` symbol on `make run` and assert
    `samples == 0`); or
  - downgrade V-IT-8 from exit gate to "Phase 3 evidence
    artefact" (matches R-009's treatment of the criterion bench).



---

## Trade-off Advice

### TR-1 Drop the `BusHandle` enum entirely (alternative to T-1 Option 1)

- Related Plan Item: `T-1`
- Topic: Performance vs Simplicity
- Reviewer Position: Prefer a fourth option not in the plan
- Advice:
  T-1 frames the choice as enum (Option 1) vs `UnsafeCell` (Option
  2) vs trait-object (Option 3). All three preserve the `Shared`
  arm. Given the C-1 finding (no concurrent `Bus` access exists),
  the right Option 4 is "no enum: `bus: Box<Bus>` on `CPU`,
  borrowed `&mut Bus` per `Core::step`". This collapses the enum
  match the plan worries LLVM may not hoist (R-003), removes the
  `Shared` arm's mutex everywhere, and shrinks `BusHandle` /
  `BusGuard` / `ReadBusGuard` / `BusOwner` to nothing.
- Rationale:
  Parsimony. The QEMU BQL parallel cited in T-1 holds because QEMU
  *is* multi-threaded (TCG worker threads). xemu is not. Inheriting
  the BQL shape without inheriting the threading model is
  cargo-culting.
- Required Action:
  Either adopt Option 4 in `02_PLAN`, or justify in writing why
  the `Shared` arm's mutex protects something concrete in this
  codebase.



### TR-2 Adopting reviewer Option C (T-2) is correct; nothing to add

- Related Plan Item: `T-2`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Agree with plan
- Advice:
  The split into `CPU::bus(&self) -> ReadBusGuard` and
  `CPU::bus_mut(&mut self) -> BusGuard` matches HashMap::get /
  get_mut and preserves source compatibility. The external-caller
  audit (lines 436-444) is the right level of evidence.
- Rationale:
  G-5 satisfied; new API surface is small and idiomatic.
- Required Action:
  Keep as-is. (If C-1 collapses `BusHandle`, the same split applies
  with `&Bus` / `&mut Bus` instead of `ReadBusGuard` / `BusGuard`.)



### TR-3 Keep `std::sync::Mutex` for now (T-3) — moot if C-1 lands

- Related Plan Item: `T-3`
- Topic: Performance vs Compatibility
- Reviewer Position: Agree with plan, conditionally
- Advice:
  Plan keeps `std::sync::Mutex` for the `Shared` arm. Correct under
  the current design. If C-1 is accepted (drop the `Shared` arm),
  T-3 disappears.
- Rationale:
  `parking_lot` would add a dependency for a bucket that isn't
  the P1 target.
- Required Action:
  Keep as-is.



### TR-4 Decision 4b (don't fuse `store_op`'s clear into write guard) is correct

- Related Plan Item: `T-4`
- Topic: Semantics vs Performance
- Reviewer Position: Agree with plan
- Advice:
  Plan declines to fuse the post-store `clear_reservation(self.id)`
  into `checked_write`'s guard (lines 738-748) on semantic grounds:
  "store-then-clear" vs "store-and-clear-atomically" are
  observably different on a multi-hart guest. Correct call.
  Decision 4a (merge `checked_read` / `checked_write`'s two locks
  into one) is also correct on the existing design — the
  translate-then-access pair already widens to the same scope on
  `access_bus`.
- Rationale:
  G-3 (multi-hart byte-identical) is preserved by 4b. T-4's MTTCG
  re-split warning still applies but is a future-phase concern.
- Required Action:
  Keep as-is. If C-1 collapses `BusHandle`, the merge in 4a remains
  beneficial: it's now "one direct call vs two direct calls",
  which is a clarity win independent of locking.



---

## Positive Notes

- Response Matrix completeness (lines 178-191) is exemplary: every
  prior R-NNN has a row, a decision, an action, and a test/gate.
  Rejections column is correctly empty (none in this round).
- T-2 adopts reviewer Option C cleanly; the external-caller audit
  (lines 436-449) is concrete (file + line for every caller, with
  `#[cfg(test)]` annotation) and is exactly the evidence R-007
  asked for.
- I-9 (lines 333-342) is a precise, well-scoped invariant. The
  borrow-checker enforcement on `Owned` plus the
  `Cell<bool>` debug guard on `Shared` is the right enforcement
  ladder (compile-time where possible, runtime debug-assert
  otherwise, documented release behaviour).
- The G-2 bucket math (lines 209-219), modulo M-2 above, is the
  kind of bottoms-up reasoning a perf plan should have. Honest
  about `pthread_mutex_*` self-time decomposing into PLT-stub +
  CAS + redistribution.
- T-4 Decision 4b's reasoning for not fusing `store_op`'s clear
  into the write guard (lines 739-746) correctly identifies the
  semantic vs performance trade-off and chooses semantics. This
  matches the user-brief constraint "multi-hart correctness
  preserved".
- Migration table format (file:line | today | pattern | notes)
  is review-friendly. The notes column carries the right level of
  detail (which sites are hot, which are merges, which are
  declined fusions).
- Source URLs in T-1 (lines 689-705) for rvemu / rv8 / Rare /
  riscv-rust / rrs / QEMU MTTCG / LWN / parking_lot — exactly
  what R-008 asked for.

---

## Approval Conditions

### Must Fix

- C-1 (Justify the `Shared` arm's mutex against the actual
  single-threaded multi-hart scheduler, or drop the enum and adopt
  a uniform owned-bus design. 01_MASTER M-001 raises the same
  concern; the next plan must respond.)
- H-1 (Rebuild the migration table from a multi-line-aware grep;
  add the five missing test-helper sites in
  `inst/base.rs:344-356`, `inst/compressed.rs:552-556`,
  `inst/float.rs:1075-1079`, `arch/riscv/cpu.rs:277-282`. Update
  the post-Phase-2 grep gate to use `--multiline`.)
- H-2 (Specify V-UT-5's trace fixture path, baseline-capture SHA
  and procedure, and counter site. Or replace with a criterion
  microbench that's a hard exit gate.)
- H-3 (Replace the "re-borrow handle that is reset each call"
  hand-wave with a typed, implementable description of `RVCore`'s
  `bus` field and the per-step borrow handover from `CPU` to
  `Core`. Pick design (a), (b), or (c) from this finding's
  Recommendation, or a fourth alternative if C-1 is adopted.)

### Should Improve

- M-1 (Either deliver a real type-state factory split or drop
  the "encoded structurally" claim.)
- M-2 (Reconcile the 35 % ceiling with the bucket math, or drop
  the 35 % figure.)
- M-3 (Either justify V-IT-7's 50 ms budget and reposition it as
  a determinism gate, or replace it with a Shared-arm acquire-count
  regression test.)

### Should Improve (LOW)

- L-1 (Declare `cargo-show-asm` as a project dev tool with an
  install path, or downgrade V-IT-8 from exit gate to evidence
  artefact, or replace with a perf-stat runtime equivalent.)

### Trade-off Responses Required

- T-1 — adopt Option 4 (drop `BusHandle`) per TR-1 if C-1 is
  accepted; otherwise keep Option 1 with explicit C-1 justification.
- T-2 — keep as-is (Option C adopted).
- T-3 — keep as-is.
- T-4 — keep as-is (4a merge + 4b non-fuse).

### Ready for Implementation

- No
- Reason: One CRITICAL (C-1: the design's premise — `Shared` arm
  needs `Mutex` — is not justified by the codebase's actual
  concurrency model, and 01_MASTER M-001 explicitly questions it)
  and three HIGH findings (H-1 migration-table gap, H-2 V-UT-5
  under-specification, H-3 single-hart ownership hand-wave). All
  four are mechanical-to-resolve in `02_PLAN` once the C-1
  premise question is answered.
