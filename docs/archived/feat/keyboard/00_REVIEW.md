# `Keyboard (UART Stdin RX)` REVIEW `00`

> Status: Open
> Feature: `keyboard`
> Iteration: `00`
> Owner: Reviewer
> Target Plan: `00_PLAN.md`
> Review Scope:
>
> - Plan Correctness
> - Spec Alignment
> - Design Soundness
> - Validation Adequacy
> - Trade-off Advice

---

## Verdict

- Decision: Rejected
- Blocking Issues: `3`
- Non-Blocking Issues: `1`

## Summary

The scope is directionally right: serial RX on the emulated UART is the correct first input path for OpenSBI / xv6 / Linux bring-up, because the target stack is centered on a serial console rather than a GUI keyboard device. But this round is not implementable yet in the current repo workflow.

The blocking problems are structural, not cosmetic:

- the proposed stdin backend conflicts with the default `xdb` frontend, which already owns host stdin for debugger commands;
- the plan wires host I/O policy into `RVCore::new()`, which is the wrong attachment point for a library constructor used broadly across `xcore`;
- the mode matrix and validation plan are internally inconsistent, especially around interactive mode, batch mode, and piped stdin.

Until those are resolved, implementation would either break the main `make run` workflow, contaminate `xcore` tests with host-terminal behavior, or produce acceptance checks that cannot actually validate the intended behavior.

## Findings

### R-001 `Guest stdin RX conflicts with the current xdb frontend`

- Severity: CRITICAL
- Section: `Architecture / Execution Flow / Trade-offs / Validation`
- Type: Flow
- Problem:
  The plan makes `stdin` the guest UART RX source and explicitly suggests enabling raw mode in `xdb` interactive mode, but the current frontend already consumes host stdin for debugger commands. `xdb_mainloop()` loops on `cli::readline()`, and `cli::readline()` uses `std::io::stdin().read_line(...)`. A background byte reader plus raw terminal mode means the guest UART and the debugger will race over the same file descriptor, and the REPL's current line-oriented input model will stop behaving correctly.
- Why it matters:
  `xemu/Makefile` defaults `make run` to `BATCH=n`, so the primary workflow is exactly the interactive `xdb` mode that this plan would disrupt. In the current architecture, the same stdin stream cannot safely serve both debugger commands and guest keyboard input.
- Recommendation:
  The next PLAN must define a frontend contract that gives guest console input its own path. Viable options include:
  - keep `xdb` interactive mode command-driven and use `TCP` / `PTY` / another attachable backend for guest RX;
  - add a dedicated serial-console runner that does not also run the `xdb` REPL;
  - or explicitly scope this round to a non-`xdb` mode only.

### R-002 `RVCore::new()` is the wrong place to bind a host stdin backend`

- Severity: HIGH
- Section: `API Surface / Implementation Plan / Validation`
- Type: API
- Problem:
  The plan changes `RVCore::new()` so the default machine always constructs `uart0` with `Uart::with_stdin()`. That pushes host-interaction policy into the base `xcore` constructor instead of keeping it in the binary/frontend layer. It also means every test or helper that instantiates `RVCore::new()` inherits a background stdin reader thread and host-environment coupling.
- Why it matters:
  `RVCore::new()` is the default constructor used across `xcore`, including unit tests. Hard-wiring stdin there makes the library less deterministic, makes tests depend on ambient terminal state, and contradicts the plan's own trade-off preference to keep terminal handling at binary level.
- Recommendation:
  Keep `RVCore::new()` as the stable TX-only default. The next PLAN should introduce an explicit machine/UART configuration boundary, for example:
  - a `UartBackend` enum passed from the binary;
  - a dedicated `RVCore::new_with_config(...)`;
  - or bus wiring performed by the frontend before `with_bus(...)`.

### R-003 `The mode matrix and validation plan are internally inconsistent`

- Severity: HIGH
- Section: `Constraints / Failure Flow / Trade-offs / Validation`
- Type: Validation
- Problem:
  The plan states "always-on in interactive mode, TX-only in batch mode", but the validation section expects piped stdin to work via `echo "hello" | X_FILE=echo.bin make run`. Those statements do not fit the current runtime:
  - `make run` defaults to interactive `xdb`, which reads stdin as debugger commands;
  - the plan's own `V-E-3` says batch mode disables RX entirely;
  - and `C-1` overstates raw-mode requirements by treating all stdin as line-buffered, even though non-TTY stdin (pipes/files) is not governed by terminal cooked/raw settings.
- Why it matters:
  As written, the executor could "validate" a scenario that the current frontend cannot exercise, and the acceptance matrix would still leave the actual interactive/batch behavior undefined.
- Recommendation:
  The next PLAN should define an explicit behavior matrix for:
  - interactive `xdb` + TTY stdin;
  - batch mode + TTY stdin;
  - non-TTY stdin (pipe/file);
  - and alternate RX backends (`TCP`, `PTY`, etc.).

  It should also separate:
  - when raw mode is required (`isatty(stdin)`),
  - when stdin RX is enabled,
  - and which executable/workflow owns stdin in each mode.

### R-004 `Terminal restore guarantees are overstated`

- Severity: MEDIUM
- Section: `Invariants / Failure Flow / Acceptance Mapping`
- Type: Invariant
- Problem:
  `I-3` says terminal attributes are restored "under all conditions", but the failure flow explicitly accepts that a killed process may leave the terminal in raw mode. The acceptance mapping also only validates normal exit and `Ctrl-C`, not panic handling despite `G-3` and `C-3`.
- Why it matters:
  The current wording over-claims what the implementation can guarantee and makes the acceptance criteria weaker than the stated invariant.
- Recommendation:
  Narrow the invariant to the cases the implementation can actually handle, for example normal exit + handled signal + panic hook, and explicitly mark uncatchable termination as out of scope. Then add panic-path validation if panic restore remains in scope.

## Trade-off Advice

### TR-1 `Prefer a separate guest-console transport over sharing stdin with xdb`

- Related Plan Item: `T-2 / T-3`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer Option B
- Advice:
  Prefer a backend that decouples guest console input from the debugger frontend, such as `PTY` or `TCP`, or explicitly introduce a serial-only execution mode.
- Rationale:
  The current repo already has an interactive debugger that owns stdin. Reusing that same stream for guest RX is superficially simple but breaks the primary workflow. A separate transport costs a little more wiring but preserves both responsibilities cleanly.
- Required Action:
  The next PLAN should either adopt a non-stdin backend for guest RX in the default debugger workflow or define a separate execution mode where `xdb` is not active.

### TR-2 `Prefer explicit machine configuration over mutating the base constructor`

- Related Plan Item: `Phase 3 / T-3`
- Topic: Flexibility vs Safety
- Reviewer Position: Prefer Option B
- Advice:
  Prefer an explicit configuration surface for host-facing devices instead of changing `RVCore::new()` semantics.
- Rationale:
  The repo is already split into `xcore` and `xdb`. A configuration boundary preserves that layering, keeps tests deterministic, and makes future backends (`stdin`, `TCP`, `PTY`, scripted input) easier to add without reopening constructor semantics.
- Required Action:
  The next PLAN should define how frontends select UART RX policy without changing the default `RVCore::new()` contract.

## Positive Notes

- The plan picks the right functional seam: OpenSBI platform integration relies on console access functions, QEMU `virt` exposes an NS16550-compatible UART, Linux uses a serial console (`ttySx`), and xv6 console input is UART-backed. Serial RX is the correct first input target.
- Reusing the existing `rx_buf -> tick() -> rx_fifo -> irq_line()` path is the right implementation direction once the frontend/configuration boundary is fixed.

## Approval Conditions

### Must Fix

- R-001
- R-002
- R-003

### Should Improve

- R-004

### Trade-off Responses Required

- TR-1
- TR-2

### Ready for Implementation
- No
- Reason: No. The current round still breaks the default `xdb` workflow and does not yet define a safe configuration boundary or a coherent runtime/validation matrix for guest input.
